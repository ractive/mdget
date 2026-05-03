use std::collections::{HashSet, VecDeque};
use std::time::Duration;

use anyhow::Context;
use url::Url;

use crate::extract::{ExtractOptions, Metadata, extract, strip_images, word_count};
use crate::fetch::{FetchOptions, fetch};
use crate::links::extract_links;
use crate::normalize::normalize_url;

/// Options controlling the crawl behaviour.
pub struct CrawlOptions {
    pub fetch_options: FetchOptions,
    pub extract_options: ExtractOptions,
    /// Maximum link depth to follow from the start URL. Default: 1.
    pub max_depth: u32,
    /// Maximum number of pages to fetch (includes the start URL). Default: 20.
    pub max_pages: usize,
    /// Delay between consecutive HTTP requests. Default: 1 second.
    pub delay: Duration,
    /// If true, follow links to different hosts. Default: false.
    pub follow_external: bool,
    /// If true, strip images from the extracted markdown.
    pub no_images: bool,
}

impl Default for CrawlOptions {
    fn default() -> Self {
        Self {
            fetch_options: FetchOptions::default(),
            extract_options: ExtractOptions { raw: false },
            max_depth: 1,
            max_pages: 20,
            delay: Duration::from_secs(1),
            follow_external: false,
            no_images: false,
        }
    }
}

/// The markdown content and metadata for a single crawled page.
#[derive(Debug)]
pub struct CrawlResult {
    pub url: Url,
    pub markdown: String,
    pub title: Option<String>,
    pub metadata: Metadata,
    pub depth: u32,
    pub word_count: usize,
}

/// Progress events emitted by the crawl engine via the `on_page` callback.
pub enum CrawlProgress {
    Fetching {
        url: String,
        depth: u32,
        queue_size: usize,
        fetched: usize,
    },
    Fetched {
        url: String,
        title: Option<String>,
    },
    Skipped {
        url: String,
        reason: String,
    },
    Error {
        url: String,
        error: String,
    },
    Done {
        total: usize,
    },
}

/// Crawl a website breadth-first starting from `start_url`.
///
/// Fetches pages, extracts links from raw HTML, and converts each page to
/// markdown using readability. Calls `on_page` for progress reporting.
/// Returns all successfully fetched pages in the order they were visited.
pub fn crawl<F>(
    start_url: &str,
    options: &CrawlOptions,
    mut on_page: F,
) -> anyhow::Result<Vec<CrawlResult>>
where
    F: FnMut(&CrawlProgress),
{
    let start = Url::parse(start_url).with_context(|| format!("invalid start URL: {start_url}"))?;

    let scheme = start.scheme();
    if scheme != "http" && scheme != "https" {
        anyhow::bail!("unsupported URL scheme '{scheme}' — only http and https are supported");
    }

    let mut start_host = start
        .host_str()
        .with_context(|| format!("start URL has no host: {start_url}"))?
        .to_lowercase();

    // BFS queue: (url, depth).
    let mut queue: VecDeque<(Url, u32)> = VecDeque::new();
    // Visited set keyed on normalized URL strings.
    let mut visited: HashSet<String> = HashSet::new();
    let mut results: Vec<CrawlResult> = Vec::new();

    // Seed the queue with the start URL.
    let start_norm = normalize_url(&start);
    visited.insert(start_norm);
    queue.push_back((start, 0));

    while let Some((url, depth)) = queue.pop_front() {
        if results.len() >= options.max_pages {
            on_page(&CrawlProgress::Skipped {
                url: url.to_string(),
                reason: format!("max_pages ({}) reached", options.max_pages),
            });
            continue;
        }

        on_page(&CrawlProgress::Fetching {
            url: url.to_string(),
            depth,
            queue_size: queue.len(),
            fetched: results.len(),
        });

        // Delay between requests (skipped for the first page for faster startup).
        if !options.delay.is_zero() && !results.is_empty() {
            std::thread::sleep(options.delay);
        }

        // Fetch the page.
        let fetch_result = match fetch(url.as_str(), &options.fetch_options) {
            Ok(r) => r,
            Err(e) => {
                on_page(&CrawlProgress::Error {
                    url: url.to_string(),
                    error: e.to_string(),
                });
                continue;
            }
        };

        let raw_html = &fetch_result.body;
        let final_url = &fetch_result.final_url;

        // Add the final URL to visited too, in case it differs from the requested URL
        // (e.g., after a redirect). Prevents fetching the same page twice.
        let final_norm = normalize_url(final_url);
        visited.insert(final_norm);

        // After a redirect, update the host filter to use the final destination's host.
        // This handles cases like example.com → www.example.com.
        if results.is_empty()
            && let Some(host) = final_url.host_str()
        {
            start_host = host.to_lowercase();
        }

        // Extract links from the raw HTML before readability strips navigation.
        if depth < options.max_depth {
            let discovered = extract_links(raw_html, final_url);
            for link in discovered {
                // Filter by host unless follow_external is set.
                if !options.follow_external {
                    let link_host = link.host_str().unwrap_or("").to_lowercase();
                    if link_host != start_host {
                        continue;
                    }
                }

                let norm = normalize_url(&link);
                if visited.contains(&norm) {
                    continue;
                }

                // Check that adding this to the queue won't push us way beyond max_pages.
                // We allow the queue to grow a bit (we check max_pages at dequeue time),
                // but cap the queue to avoid unbounded memory growth.
                if visited.len() >= options.max_pages.saturating_mul(4) {
                    break;
                }

                visited.insert(norm);
                queue.push_back((link, depth + 1));
            }
        }

        // Extract markdown content via readability.
        let extract_result = match extract(raw_html, final_url, &options.extract_options) {
            Ok(r) => r,
            Err(e) => {
                on_page(&CrawlProgress::Error {
                    url: url.to_string(),
                    error: format!("extraction failed: {e}"),
                });
                continue;
            }
        };

        let markdown = if options.no_images {
            strip_images(&extract_result.markdown)
        } else {
            extract_result.markdown
        };

        let wc = word_count(&markdown);

        on_page(&CrawlProgress::Fetched {
            url: url.to_string(),
            title: extract_result.title.clone(),
        });

        results.push(CrawlResult {
            url: final_url.clone(), // clone needed: final_url is borrowed from fetch_result
            markdown,
            title: extract_result.title,
            metadata: extract_result.metadata,
            depth,
            word_count: wc,
        });
    }

    on_page(&CrawlProgress::Done {
        total: results.len(),
    });

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_http_scheme() {
        let opts = CrawlOptions::default();
        let result = crawl("ftp://example.com/", &opts, |_| {});
        let err = result.expect_err("expected an error");
        assert!(
            err.to_string().contains("unsupported URL scheme"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn rejects_invalid_url() {
        let opts = CrawlOptions::default();
        let result = crawl("not a url", &opts, |_| {});
        let err = result.expect_err("expected an error");
        assert!(
            err.to_string().contains("invalid start URL"),
            "unexpected error: {err}"
        );
    }
}
