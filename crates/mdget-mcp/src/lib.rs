use anyhow::Context as _;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerInfo};
use rmcp::{ServerHandler, ServiceExt as _, schemars, tool, tool_handler, tool_router};
use serde::Serialize;

/// MCP server that exposes mdget functionality as tools.
#[derive(Debug, Clone)]
pub struct MdgetServer;

// ── Parameter types ─────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FetchMarkdownParams {
    /// URL to fetch (must be http or https)
    pub url: String,

    /// Skip readability extraction, convert full HTML
    #[serde(default)]
    pub raw: bool,

    /// Prepend YAML frontmatter with title, URL, date, word count, and metadata
    #[serde(default)]
    pub include_metadata: bool,

    /// Strip image references from markdown output
    #[serde(default)]
    pub no_images: bool,

    /// Truncate output to N characters (at paragraph/sentence boundary)
    pub max_length: Option<usize>,

    /// HTTP timeout in seconds (default 30)
    pub timeout: Option<u64>,

    /// Retry count for transient errors like 5xx, timeouts (default 2)
    pub retries: Option<u32>,

    /// Override the User-Agent header for this request
    #[serde(default)]
    pub user_agent: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FetchMetadataParams {
    /// URL to fetch (must be http or https)
    pub url: String,

    /// HTTP timeout in seconds (default 30)
    pub timeout: Option<u64>,

    /// Retry count for transient errors like 5xx, timeouts (default 2)
    pub retries: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BatchFetchParams {
    /// URLs to fetch (must be http or https)
    pub urls: Vec<String>,

    /// Skip readability extraction, convert full HTML
    #[serde(default)]
    pub raw: bool,

    /// Prepend YAML frontmatter with title, URL, date, word count, and metadata
    #[serde(default)]
    pub include_metadata: bool,

    /// Strip image references from markdown output
    #[serde(default)]
    pub no_images: bool,

    /// Truncate each output to N characters (at paragraph/sentence boundary)
    pub max_length: Option<usize>,

    /// HTTP timeout in seconds per URL (default 30)
    pub timeout: Option<u64>,

    /// Retry count for transient errors like 5xx, timeouts (default 2)
    pub retries: Option<u32>,

    /// Override the User-Agent header for this request
    #[serde(default)]
    pub user_agent: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CrawlSiteParams {
    /// Starting URL to crawl from (must be http or https)
    pub url: String,

    /// Maximum link depth to follow (0 = start page only, default 1)
    pub depth: Option<u32>,

    /// Maximum number of pages to fetch (default 20, max 200)
    pub max_pages: Option<usize>,

    /// Seconds to wait between requests (default 1)
    pub delay: Option<u64>,

    /// Only follow links whose URL path starts with this prefix (e.g. /docs/).
    /// Auto-inferred from start URL when not set.
    pub path_prefix: Option<String>,

    /// Include YAML frontmatter metadata in each page's content
    #[serde(default)]
    pub include_metadata: bool,

    /// Truncate each page's content to N characters (default 50000). Use 0 for no limit.
    pub max_length: Option<usize>,

    /// HTTP timeout in seconds per request (default 30)
    pub timeout: Option<u64>,
}

// ── Result types ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct CrawlSiteResult {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    word_count: Option<usize>,
    depth: u32,
}

// ── Batch result type ───────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct BatchResult {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    word_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    excerpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    byline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

const MAX_BATCH_URLS: usize = 50;

// ── Tool implementations ────────────────────────────────────────────────

#[tool_router]
#[allow(clippy::unused_self)] // &self required by the #[tool] proc-macro contract
impl MdgetServer {
    /// Fetch a URL and return its content as clean markdown. Uses a readability
    /// algorithm to extract the main content (like browser reader mode) unless
    /// `raw` is true.
    #[tool(
        name = "fetch_markdown",
        description = "Fetch a web page and return its content as clean markdown. Extracts main content via readability (like browser reader mode). Use `raw: true` to convert full HTML instead."
    )]
    fn fetch_markdown(
        &self,
        Parameters(params): Parameters<FetchMarkdownParams>,
    ) -> Result<String, String> {
        validate_url(&params.url)?;
        if let Some(t) = params.timeout {
            validate_timeout(t)?;
        }
        if let Some(r) = params.retries {
            validate_retries(r)?;
        }

        let fetch_opts = mdget_core::FetchOptions {
            timeout_secs: params.timeout.unwrap_or(30),
            retries: params.retries.unwrap_or(2),
            quiet: true,
            user_agent: params.user_agent,
        };

        // `mdget_core::fetch` uses `reqwest::blocking` which must not run on a
        // tokio worker thread. `block_in_place` parks the current thread as a
        // blocking thread so the runtime can schedule other async work.
        let fetch_result =
            tokio::task::block_in_place(|| mdget_core::fetch(&params.url, &fetch_opts))
                .map_err(format_error)?;

        let (markdown, metadata, _title) = extract_content(&fetch_result, params.raw)?;

        // Post-processing pipeline (mirrors CLI)
        let markdown = if params.no_images {
            mdget_core::strip_images(&markdown)
        } else {
            markdown
        };

        let wc = mdget_core::word_count(&markdown);

        let markdown = if let Some(max) = params.max_length {
            if max > 0 {
                mdget_core::truncate_output(&markdown, max)
            } else {
                markdown
            }
        } else {
            markdown
        };

        if params.include_metadata {
            let frontmatter = mdget_core::format_metadata_frontmatter(
                &metadata,
                fetch_result.final_url.as_str(),
                wc,
            );
            Ok(format!("{frontmatter}\n{markdown}"))
        } else {
            Ok(markdown)
        }
    }

    /// Fetch a URL and return only YAML metadata (title, word count, excerpt, etc.).
    /// Useful for triaging URLs before committing to a full fetch.
    #[tool(
        name = "fetch_metadata",
        description = "Fetch a web page and return only its YAML metadata frontmatter (title, word count, excerpt, etc.). Useful for triaging URLs before a full fetch."
    )]
    fn fetch_metadata(
        &self,
        Parameters(params): Parameters<FetchMetadataParams>,
    ) -> Result<String, String> {
        validate_url(&params.url)?;
        if let Some(t) = params.timeout {
            validate_timeout(t)?;
        }
        if let Some(r) = params.retries {
            validate_retries(r)?;
        }

        let fetch_opts = mdget_core::FetchOptions {
            timeout_secs: params.timeout.unwrap_or(30),
            retries: params.retries.unwrap_or(2),
            quiet: true,
            ..Default::default()
        };

        let fetch_result =
            tokio::task::block_in_place(|| mdget_core::fetch(&params.url, &fetch_opts))
                .map_err(format_error)?;
        let (markdown, metadata, _title) = extract_content(&fetch_result, false)?;
        let wc = mdget_core::word_count(&markdown);

        Ok(mdget_core::format_metadata_frontmatter(
            &metadata,
            fetch_result.final_url.as_str(),
            wc,
        ))
    }

    /// Fetch multiple URLs in parallel and return all results. Each result
    /// contains the URL, content (or error), and title.
    #[tool(
        name = "batch_fetch",
        description = "Fetch multiple web pages in parallel and return all results. Each result contains url, title, content (or error), plus metadata: word_count, excerpt, language, byline."
    )]
    fn batch_fetch(
        &self,
        Parameters(params): Parameters<BatchFetchParams>,
    ) -> Result<String, String> {
        if params.urls.is_empty() {
            return Err("urls array must not be empty".to_string());
        }
        if params.urls.len() > MAX_BATCH_URLS {
            return Err(format!(
                "urls array must contain at most {MAX_BATCH_URLS} entries"
            ));
        }
        for url in &params.urls {
            validate_url(url)?;
        }
        if let Some(t) = params.timeout {
            validate_timeout(t)?;
        }
        if let Some(r) = params.retries {
            validate_retries(r)?;
        }

        let fetch_opts = mdget_core::FetchOptions {
            timeout_secs: params.timeout.unwrap_or(30),
            retries: params.retries.unwrap_or(2),
            quiet: true,
            user_agent: params.user_agent,
        };

        let results: Vec<BatchResult> = std::thread::scope(|s| {
            let handles: Vec<_> = params
                .urls
                .iter()
                .map(|url| {
                    let opts = &fetch_opts;
                    let raw = params.raw;
                    let no_images = params.no_images;
                    let include_metadata = params.include_metadata;
                    let max_length = params.max_length;
                    s.spawn(move || {
                        process_single_url(url, opts, raw, no_images, include_metadata, max_length)
                    })
                })
                .collect();

            handles
                .into_iter()
                .map(|h| {
                    h.join().unwrap_or_else(|_| BatchResult {
                        url: String::from("<unknown>"),
                        title: None,
                        content: None,
                        word_count: None,
                        excerpt: None,
                        language: None,
                        byline: None,
                        error: Some("processing thread panicked".to_string()),
                    })
                })
                .collect()
        });

        serde_json::to_string_pretty(&results)
            .map_err(|e| format!("failed to serialize results: {e}"))
    }

    /// Crawl a website breadth-first, following links up to a configurable depth.
    /// Returns an array of pages with URL, title, content, and word count.
    #[tool(
        name = "crawl_site",
        description = "Crawl a website breadth-first, following links up to a configurable depth. Returns an array of {url, title, content, word_count, depth} results. Great for exploring documentation sites. Set `max_length` to control per-page content size (default 50000 chars)."
    )]
    fn crawl_site(
        &self,
        Parameters(params): Parameters<CrawlSiteParams>,
    ) -> Result<String, String> {
        validate_url(&params.url)?;
        if let Some(t) = params.timeout {
            validate_timeout(t)?;
        }

        let max_pages = params.max_pages.unwrap_or(20);
        if max_pages == 0 {
            return Err("max_pages must be greater than 0".to_string());
        }
        if max_pages > 200 {
            return Err("max_pages must be 200 or less".to_string());
        }

        let depth = params.depth.unwrap_or(1);
        let delay = params.delay.unwrap_or(1);
        let max_length = params.max_length.unwrap_or(50000);

        // Auto-infer path prefix from start URL if not explicitly set
        let path_prefix = if params.path_prefix.is_some() {
            params.path_prefix
        } else {
            url::Url::parse(&params.url)
                .ok()
                .and_then(|u| mdget_core::infer_path_prefix(&u))
        };

        let options = mdget_core::CrawlOptions {
            fetch_options: mdget_core::FetchOptions {
                timeout_secs: params.timeout.unwrap_or(30),
                quiet: true,
                ..Default::default()
            },
            extract_options: mdget_core::ExtractOptions { raw: false },
            max_depth: depth,
            max_pages,
            delay: std::time::Duration::from_secs(delay),
            no_images: true, // agents don't need images
            path_prefix,
            ..Default::default()
        };

        let crawl_results =
            tokio::task::block_in_place(|| mdget_core::crawl(&params.url, &options, |_| {}))
                .map_err(format_error)?;

        let results: Vec<CrawlSiteResult> = crawl_results
            .into_iter()
            .map(|r| {
                let content = if params.include_metadata {
                    let frontmatter = mdget_core::format_metadata_frontmatter(
                        &r.metadata,
                        r.url.as_str(),
                        r.word_count,
                    );
                    format!("{frontmatter}\n{}", r.markdown)
                } else {
                    r.markdown
                };
                let content = if max_length > 0 {
                    mdget_core::truncate_output(&content, max_length)
                } else {
                    content
                };

                CrawlSiteResult {
                    url: r.url.to_string(),
                    title: r.title,
                    content,
                    word_count: Some(r.word_count),
                    depth: r.depth,
                }
            })
            .collect();

        serde_json::to_string_pretty(&results)
            .map_err(|e| format!("failed to serialize results: {e}"))
    }
}

// ── ServerHandler impl ──────────────────────────────────────────────────

#[tool_handler]
impl ServerHandler for MdgetServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("mdget", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Fetch web pages and convert them to clean markdown. \
                 Tools: fetch_markdown (single URL), fetch_metadata (YAML metadata only), \
                 batch_fetch (multiple URLs in parallel), \
                 crawl_site (crawl a website breadth-first).",
            )
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn validate_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("invalid URL '{url}': {e}"))?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("URLs with embedded credentials are not supported".to_string());
    }
    match parsed.scheme() {
        "http" | "https" => Ok(()),
        s => Err(format!(
            "unsupported URL scheme '{s}': only http and https are allowed"
        )),
    }
}

fn validate_timeout(t: u64) -> Result<(), String> {
    if t == 0 {
        return Err("timeout must be greater than 0".to_string());
    }
    if t > 300 {
        return Err("timeout must be 300 seconds or less".to_string());
    }
    Ok(())
}

fn validate_retries(r: u32) -> Result<(), String> {
    if r > 10 {
        return Err("retries must be 10 or less".to_string());
    }
    Ok(())
}

#[allow(clippy::needless_pass_by_value)] // owned Error required for map_err(format_error) pattern
fn format_error(e: anyhow::Error) -> String {
    // Strip anyhow context chain down to a clean user-facing message.
    // Never expose file paths or stack traces.
    format!("{e:#}")
}

fn extract_content(
    fetch_result: &mdget_core::FetchResult,
    raw: bool,
) -> Result<(String, mdget_core::Metadata, Option<String>), String> {
    let content_type = fetch_result.content_type.as_deref().unwrap_or("");
    let mime = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    match mime.as_str() {
        "text/html" | "application/xhtml+xml" | "" => {
            let r = mdget_core::extract(
                &fetch_result.body,
                &fetch_result.final_url,
                &mdget_core::ExtractOptions { raw },
            )
            .map_err(format_error)?;
            Ok((r.markdown, r.metadata, r.title))
        }
        "text/plain" => Ok((
            fetch_result.body.clone(),
            mdget_core::Metadata::default(),
            None,
        )),
        "application/json" => Ok((
            format!("```json\n{}\n```", fetch_result.body),
            mdget_core::Metadata::default(),
            None,
        )),
        "application/pdf" => {
            Err("PDF content detected; mdget cannot extract text from PDFs".to_string())
        }
        "application/rss+xml" | "application/atom+xml" => {
            Err("RSS/Atom feed detected — use a feed parser instead of mdget".to_string())
        }
        "application/xml" | "text/xml" => {
            let trimmed = fetch_result.body.trim_start();
            if trimmed.starts_with("<rss")
                || trimmed.starts_with("<feed")
                || (trimmed.starts_with("<?xml")
                    && (trimmed.contains("<rss") || trimmed.contains("<feed")))
            {
                Err("RSS/Atom feed detected — use a feed parser instead of mdget".to_string())
            } else {
                // Regular XML — attempt HTML extraction
                let r = mdget_core::extract(
                    &fetch_result.body,
                    &fetch_result.final_url,
                    &mdget_core::ExtractOptions { raw },
                )
                .map_err(format_error)?;
                Ok((r.markdown, r.metadata, r.title))
            }
        }
        t if t.starts_with("image/")
            || t.starts_with("audio/")
            || t.starts_with("video/")
            || matches!(
                t,
                "application/octet-stream" | "application/zip" | "application/gzip"
            ) =>
        {
            Err(format!(
                "binary content ({mime}); mdget only processes HTML pages"
            ))
        }
        _ => {
            let r = mdget_core::extract(
                &fetch_result.body,
                &fetch_result.final_url,
                &mdget_core::ExtractOptions { raw },
            )
            .map_err(format_error)?;
            Ok((r.markdown, r.metadata, r.title))
        }
    }
}

fn process_single_url(
    url: &str,
    opts: &mdget_core::FetchOptions,
    raw: bool,
    no_images: bool,
    include_metadata: bool,
    max_length: Option<usize>,
) -> BatchResult {
    let fetch_result = match mdget_core::fetch(url, opts) {
        Ok(r) => r,
        Err(e) => {
            return BatchResult {
                url: url.to_string(),
                title: None,
                content: None,
                word_count: None,
                excerpt: None,
                language: None,
                byline: None,
                error: Some(format_error(e)),
            };
        }
    };

    let (markdown, metadata, title) = match extract_content(&fetch_result, raw) {
        Ok(r) => r,
        Err(e) => {
            return BatchResult {
                url: url.to_string(),
                title: None,
                content: None,
                word_count: None,
                excerpt: None,
                language: None,
                byline: None,
                error: Some(e),
            };
        }
    };

    let markdown = if no_images {
        mdget_core::strip_images(&markdown)
    } else {
        markdown
    };

    let wc = mdget_core::word_count(&markdown);

    let markdown = if let Some(max) = max_length {
        if max > 0 {
            mdget_core::truncate_output(&markdown, max)
        } else {
            markdown
        }
    } else {
        markdown
    };

    let content = if include_metadata {
        let frontmatter =
            mdget_core::format_metadata_frontmatter(&metadata, fetch_result.final_url.as_str(), wc);
        format!("{frontmatter}\n{markdown}")
    } else {
        markdown
    };

    BatchResult {
        url: url.to_string(),
        title,
        content: Some(content),
        word_count: Some(wc),
        excerpt: metadata.excerpt,
        language: metadata.language,
        byline: metadata.byline,
        error: None,
    }
}

/// Start the MCP server on stdio (blocking). Creates a tokio runtime internally,
/// so callers don't need to depend on tokio. Blocks until the client disconnects.
///
/// # Errors
///
/// Returns an error if the server fails to start or encounters a transport error.
pub fn run_server() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to create tokio runtime")?
        .block_on(async {
            let server = MdgetServer;
            let service = server
                .serve(rmcp::transport::stdio())
                .await
                .map_err(|e| anyhow::anyhow!("failed to start MCP server: {e}"))?;
            service
                .waiting()
                .await
                .map_err(|e| anyhow::anyhow!("MCP server error: {e}"))?;
            Ok(())
        })
}
