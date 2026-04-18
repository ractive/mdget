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
}

pub fn fetch(url: &str, options: &FetchOptions) -> anyhow::Result<FetchResult> {
    // Validate URL before making the request.
    Url::parse(url).with_context(|| format!("invalid URL: {url}"))?;

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

    let body = response
        .text()
        .with_context(|| format!("failed to read response body from: {url}"))?;

    Ok(FetchResult { body, final_url })
}
