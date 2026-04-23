use std::io::Read as _;

use anyhow::Context;
use url::Url;

const MAX_TOTAL_HOPS: usize = 10;
const MAX_RESPONSE_SIZE: usize = 50 * 1024 * 1024; // 50 MB
/// Only scan this many bytes for a meta refresh tag — it must be in <head>.
const META_REFRESH_SCAN_BYTES: usize = 4096;

/// Read the response body with a hard size cap. Checks Content-Length first
/// as a fast pre-flight, then enforces the limit while reading (protects
/// against lying Content-Length or chunked-encoding).
fn read_body_limited(response: reqwest::blocking::Response, url: &Url) -> anyhow::Result<String> {
    if let Some(cl) = response.content_length()
        && cl > MAX_RESPONSE_SIZE as u64
    {
        anyhow::bail!(
            "response too large ({cl} bytes, limit is {MAX_RESPONSE_SIZE} bytes) from: {url}",
        );
    }
    let mut body_bytes = Vec::new();
    response
        .take(MAX_RESPONSE_SIZE as u64 + 1)
        .read_to_end(&mut body_bytes)
        .with_context(|| format!("failed to read response body from: {url}"))?;
    if body_bytes.len() > MAX_RESPONSE_SIZE {
        anyhow::bail!("response too large (>{MAX_RESPONSE_SIZE} bytes) from: {url}");
    }
    Ok(String::from_utf8_lossy(&body_bytes).into_owned())
}

pub struct FetchOptions {
    pub timeout_secs: u64,
    pub user_agent: Option<String>,
    pub retries: u32,
    pub quiet: bool,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            user_agent: None,
            retries: 0,
            quiet: false,
        }
    }
}

pub struct FetchResult {
    pub body: String,
    pub final_url: Url,
    pub content_type: Option<String>,
    /// Redirect hops followed to reach `final_url` (HTTP 3xx and meta-refresh).
    pub redirect_chain: Vec<String>,
}

pub fn fetch(url: &str, options: &FetchOptions) -> anyhow::Result<FetchResult> {
    // Validate URL before making the request.
    let parsed = Url::parse(url).with_context(|| format!("invalid URL: {url}"))?;

    // Reject non-HTTP(S) schemes early — reqwest would produce a confusing error.
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        anyhow::bail!("unsupported URL scheme '{scheme}' — only http and https are supported");
    }

    let user_agent = options
        .user_agent
        .as_deref()
        .unwrap_or(concat!("mdget/", env!("CARGO_PKG_VERSION")));

    // Disable automatic redirect following so we can report the chain ourselves.
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(options.timeout_secs))
        .user_agent(user_agent)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("failed to build HTTP client")?;

    let mut redirect_chain: Vec<String> = Vec::new();
    let result = fetch_with_retries(url, options, &client, &mut redirect_chain)?;

    Ok(result)
}

/// Execute a GET with retry/backoff, manual redirect following, and meta-refresh
/// chasing. `redirect_chain` is populated with every hop URL.
fn fetch_with_retries(
    url: &str,
    options: &FetchOptions,
    client: &reqwest::blocking::Client,
    redirect_chain: &mut Vec<String>,
) -> anyhow::Result<FetchResult> {
    let max_attempts = options.retries + 1;
    let mut last_err: Option<anyhow::Error> = None;

    for attempt in 0..max_attempts {
        if attempt > 0 {
            let delay_secs = 1u64 << (attempt - 1); // 1, 2, 4 …
            if !options.quiet {
                eprintln!("Retrying {url} ({attempt}/{})...", options.retries);
            }
            std::thread::sleep(std::time::Duration::from_secs(delay_secs));
        }

        match do_fetch(url, options, client, redirect_chain) {
            Ok(result) => return Ok(result),
            Err(e) => {
                // Check if this is a non-retryable client error (4xx).
                // We encode 4xx errors with a prefix so we can identify them here.
                if is_client_error(&e) {
                    return Err(e);
                }
                last_err = Some(e);
                // Clear redirect chain for the next attempt so we don't
                // accumulate hops from failed attempts.
                redirect_chain.clear();
            }
        }
    }

    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("fetch failed: {url}")))
}

/// Returns true when the error represents a 4xx HTTP status — these should not
/// be retried because the client sent a bad request.
fn is_client_error(e: &anyhow::Error) -> bool {
    e.to_string().contains("HTTP 4")
}

/// Performs the actual HTTP GET, follows 3xx redirects manually (up to
/// `MAX_TOTAL_HOPS`), checks for meta-refresh in the final response body, and
/// returns a `FetchResult`.
fn do_fetch(
    start_url: &str,
    options: &FetchOptions,
    client: &reqwest::blocking::Client,
    redirect_chain: &mut Vec<String>,
) -> anyhow::Result<FetchResult> {
    let mut current_url =
        Url::parse(start_url).with_context(|| format!("invalid URL: {start_url}"))?;

    let mut hops: usize = 0;

    // --- Phase 1: follow HTTP 3xx redirects ---
    let response =
        follow_http_redirects(&mut current_url, options, client, redirect_chain, &mut hops)?;

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(std::string::ToString::to_string);

    let body = read_body_limited(response, &current_url)?;

    // --- Phase 2: follow meta-refresh hops ---
    let (final_body, final_url, final_content_type) = follow_meta_refresh(
        body,
        current_url,
        content_type,
        options,
        client,
        redirect_chain,
        &mut hops,
    )?;

    Ok(FetchResult {
        body: final_body,
        final_url,
        content_type: final_content_type,
        redirect_chain: redirect_chain.clone(), // clone needed: caller owns redirect_chain across retries
    })
}

/// Sends a GET to `current_url`, following 3xx redirects manually until a
/// non-redirect response is received or `MAX_TOTAL_HOPS` is reached.
fn follow_http_redirects(
    current_url: &mut Url,
    options: &FetchOptions,
    client: &reqwest::blocking::Client,
    redirect_chain: &mut Vec<String>,
    hops: &mut usize,
) -> anyhow::Result<reqwest::blocking::Response> {
    loop {
        let response = client
            .get(current_url.as_str())
            .send()
            .with_context(|| format!("failed to fetch URL: {current_url}"))?;

        let status = response.status();

        if status.is_redirection() {
            *hops += 1;
            if *hops > MAX_TOTAL_HOPS {
                anyhow::bail!("too many redirects (>{MAX_TOTAL_HOPS} hops)");
            }

            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .with_context(|| {
                    format!("redirect response missing Location header from: {current_url}")
                })?
                .to_str()
                .with_context(|| {
                    format!("Location header is not valid UTF-8 from: {current_url}")
                })?;

            // Resolve relative Location against the current URL.
            let next_url = current_url
                .join(location)
                .with_context(|| format!("invalid Location header '{location}'"))?;

            if !options.quiet {
                eprintln!("  \u{2192} {next_url}");
            }

            redirect_chain.push(next_url.to_string());
            *current_url = next_url;
            continue;
        }

        // Non-redirect: check for errors.
        if status.is_client_error() {
            anyhow::bail!("HTTP {status} fetching URL: {current_url}");
        }
        if !status.is_success() {
            anyhow::bail!("HTTP {status} fetching URL: {current_url}");
        }

        return Ok(response);
    }
}

/// Checks `body` for a `<meta http-equiv="refresh">` tag. If found and within
/// the hop limit, fetches the target URL and recurses.
fn follow_meta_refresh(
    body: String,
    current_url: Url,
    content_type: Option<String>,
    options: &FetchOptions,
    client: &reqwest::blocking::Client,
    redirect_chain: &mut Vec<String>,
    hops: &mut usize,
) -> anyhow::Result<(String, Url, Option<String>)> {
    // Only HTML content can carry meta-refresh tags.
    let is_html = content_type.as_deref().is_none_or(|ct| ct.contains("html")); // assume HTML if content-type unknown

    if !is_html {
        return Ok((body, current_url, content_type));
    }

    let Some(target_url) = extract_meta_refresh_url(&body, &current_url)? else {
        return Ok((body, current_url, content_type));
    };

    *hops += 1;
    if *hops > MAX_TOTAL_HOPS {
        anyhow::bail!("too many redirects (>{MAX_TOTAL_HOPS} hops)");
    }

    if !options.quiet {
        eprintln!("  \u{21AA} meta refresh \u{2192} {target_url}");
    }
    redirect_chain.push(target_url.to_string());

    // Fetch the meta-refresh target (HTTP redirects within this hop are also
    // followed, so we pass redirect_chain through).
    let mut next_url = target_url;
    let response = follow_http_redirects(&mut next_url, options, client, redirect_chain, hops)?;

    let next_content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(std::string::ToString::to_string);

    let next_body = read_body_limited(response, &next_url)?;

    // Recurse in case of chained meta-refreshes.
    follow_meta_refresh(
        next_body,
        next_url,
        next_content_type,
        options,
        client,
        redirect_chain,
        hops,
    )
}

/// Scans the first `META_REFRESH_SCAN_BYTES` of `body` for a
/// `<meta http-equiv="refresh" content="N; url=TARGET">` tag.
///
/// Returns `None` if no meta-refresh is found, or the resolved absolute URL
/// if one is found.
fn extract_meta_refresh_url(body: &str, base_url: &Url) -> anyhow::Result<Option<Url>> {
    // Work on a lowercase slice — meta tags must be in <head> so 4 KiB is plenty.
    let scan_len = body.len().min(META_REFRESH_SCAN_BYTES);
    let window = body[..scan_len].to_ascii_lowercase();

    // Find `http-equiv="refresh"` or `http-equiv='refresh'`.
    let equiv_pos = window
        .find("http-equiv=\"refresh\"")
        .or_else(|| window.find("http-equiv='refresh'"));

    let tag_start = match equiv_pos {
        Some(pos) => {
            // Walk back to the opening '<' of the enclosing tag.
            window[..pos].rfind('<').unwrap_or(pos)
        }
        None => return Ok(None),
    };

    // Slice the tag from its '<' to the next '>' (or end of scan window).
    let tag_slice = &window[tag_start..];
    let tag_end = tag_slice.find('>').map_or(tag_slice.len(), |i| i + 1);
    let tag = &tag_slice[..tag_end];

    // Extract the value of the `content` attribute.
    let content_value = extract_attr_value(tag, "content")?;
    let Some(content) = content_value else {
        return Ok(None);
    };

    // Parse "N; url=TARGET" or "N;url=TARGET" (N may be a float).
    let url_part = if let Some(idx) = content.find(';') {
        &content[idx + 1..]
    } else {
        return Ok(None);
    };

    let url_part = url_part.trim();
    let target = if let Some(stripped) = url_part.strip_prefix("url=") {
        stripped.trim().trim_matches('\'').trim_matches('"')
    } else {
        return Ok(None);
    };

    if target.is_empty() {
        return Ok(None);
    }

    // Resolve against the base URL (handles relative paths).
    let resolved = base_url
        .join(target)
        .with_context(|| format!("invalid meta-refresh URL '{target}'"))?;

    Ok(Some(resolved))
}

/// Extracts the value of `attr` from a lowercase HTML tag snippet.
/// Handles both `attr="value"` and `attr='value'` forms.
fn extract_attr_value<'a>(tag: &'a str, attr: &str) -> anyhow::Result<Option<&'a str>> {
    for (quote, label) in [('"', "double-quote"), ('\'', "single-quote")] {
        let needle = format!("{attr}={quote}");
        if let Some(start) = tag.find(needle.as_str()) {
            let after = &tag[start + needle.len()..];
            let end = after
                .find(quote)
                .with_context(|| format!("unclosed {attr} attribute ({label})"))?;
            return Ok(Some(&after[..end]));
        }
    }

    Ok(None)
}

/// Read a local HTML file and return a `FetchResult` suitable for the extraction pipeline.
///
/// The `content_type` is guessed from the file extension:
/// - `.html` / `.htm` / `.xhtml` → `text/html`
/// - `.txt` → `text/plain`
/// - `.json` → `application/json`
/// - anything else → `text/html` (default, since most local files will be saved web pages)
pub fn read_local(path: &std::path::Path) -> anyhow::Result<FetchResult> {
    let body = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read local file: {}", path.display()))?;

    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("failed to determine current directory")?
            .join(path)
    };

    let final_url = Url::from_file_path(&abs_path).map_err(|()| {
        anyhow::anyhow!("failed to convert path to file URL: {}", abs_path.display())
    })?;

    let content_type = guess_content_type(path);

    Ok(FetchResult {
        body,
        final_url,
        content_type: Some(content_type.to_string()),
        redirect_chain: Vec::new(),
    })
}

fn guess_content_type(path: &std::path::Path) -> &'static str {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext.to_ascii_lowercase().as_str() {
        "txt" => "text/plain",
        "json" => "application/json",
        // Default to text/html — covers .html, .htm, .xhtml, and extensionless files
        // (most local files will be saved web pages).
        _ => "text/html",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Url {
        Url::parse("https://example.com/").unwrap()
    }

    #[test]
    fn no_meta_refresh_returns_none() {
        let body = "<html><head><title>Hi</title></head></html>";
        let result = extract_meta_refresh_url(body, &base()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn meta_refresh_double_quotes() {
        let body = r#"<html><head>
            <meta http-equiv="refresh" content="0; url=https://target.example.com/page">
        </head></html>"#;
        let result = extract_meta_refresh_url(body, &base()).unwrap();
        assert_eq!(result.unwrap().as_str(), "https://target.example.com/page");
    }

    #[test]
    fn meta_refresh_single_quotes() {
        let body = r"<html><head>
            <meta http-equiv='refresh' content='0;url=https://other.example.com/'>
        </head></html>";
        let result = extract_meta_refresh_url(body, &base()).unwrap();
        assert_eq!(result.unwrap().as_str(), "https://other.example.com/");
    }

    #[test]
    fn meta_refresh_relative_url() {
        let body = r#"<meta http-equiv="refresh" content="0; url=/new/path">"#;
        let result = extract_meta_refresh_url(body, &base()).unwrap();
        assert_eq!(result.unwrap().as_str(), "https://example.com/new/path");
    }

    #[test]
    fn meta_refresh_no_url_part_returns_none() {
        // Content without a url= part should not produce a redirect.
        let body = r#"<meta http-equiv="refresh" content="30">"#;
        let result = extract_meta_refresh_url(body, &base()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn is_client_error_detects_4xx() {
        let err = anyhow::anyhow!("HTTP 404 Not Found fetching URL: https://example.com");
        assert!(is_client_error(&err));
    }

    #[test]
    fn is_client_error_ignores_5xx() {
        let err = anyhow::anyhow!("HTTP 503 Service Unavailable fetching URL: https://example.com");
        assert!(!is_client_error(&err));
    }
}
