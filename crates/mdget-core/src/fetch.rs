use anyhow::Context;
use url::Url;

pub struct FetchOptions {
    pub timeout_secs: u64,
    pub user_agent: Option<String>,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            user_agent: None,
        }
    }
}

pub struct FetchResult {
    pub body: String,
    pub final_url: Url,
    pub content_type: Option<String>,
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

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(options.timeout_secs))
        .user_agent(user_agent)
        .build()
        .context("failed to build HTTP client")?;

    let response = client
        .get(url)
        .send()
        .with_context(|| format!("failed to fetch URL: {url}"))?;

    let final_url = response.url().clone();

    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("HTTP {status} fetching URL: {url}");
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(std::string::ToString::to_string);

    let body = response
        .text()
        .with_context(|| format!("failed to read response body from: {url}"))?;

    Ok(FetchResult {
        body,
        final_url,
        content_type,
    })
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
