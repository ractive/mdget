use anyhow::Context;
use serde::Deserialize;
use url::Url;

// ---------------------------------------------------------------------------
// Deserialization structs
// ---------------------------------------------------------------------------

/// Represents `<urlset><url><loc>...</loc></url></urlset>`.
#[derive(Deserialize)]
struct UrlSet {
    #[serde(rename = "url", default)]
    urls: Vec<SitemapUrl>,
}

#[derive(Deserialize)]
struct SitemapUrl {
    loc: String,
}

/// Represents `<sitemapindex><sitemap><loc>...</loc></sitemap></sitemapindex>`.
#[derive(Deserialize)]
struct SitemapIndex {
    #[serde(rename = "sitemap", default)]
    sitemaps: Vec<SitemapEntry>,
}

#[derive(Deserialize)]
struct SitemapEntry {
    loc: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Fetches and parses `{base_url}/sitemap.xml`, returning all discovered page
/// URLs.
///
/// Supports two formats:
/// - `<urlset>` — a flat list of `<url><loc>` entries.
/// - `<sitemapindex>` — a list of child sitemaps; each is fetched one level deep.
///
/// URLs that are not valid http/https are silently filtered out.
pub fn fetch_sitemap_urls(
    client: &reqwest::blocking::Client,
    base_url: &Url,
    quiet: bool,
) -> anyhow::Result<Vec<Url>> {
    let sitemap_url = base_url
        .join("/sitemap.xml")
        .context("failed to construct sitemap.xml URL")?;

    if !quiet {
        eprintln!("  Fetching sitemap: {sitemap_url}");
    }

    let body = fetch_text(client, sitemap_url.as_str())?;

    let mut urls = parse_urlset(&body);

    if urls.is_empty() {
        // Try as sitemapindex.
        let entries = parse_sitemapindex(&body);
        for entry_url_str in entries {
            match fetch_child_sitemap(client, &entry_url_str, quiet) {
                Ok(child_urls) => urls.extend(child_urls),
                Err(e) => {
                    if !quiet {
                        eprintln!("  Warning: failed to fetch child sitemap {entry_url_str}: {e}");
                    }
                }
            }
        }
    }

    // Filter to valid http/https URLs.
    let valid: Vec<Url> = urls
        .into_iter()
        .filter_map(|s| {
            let u = Url::parse(&s).ok()?;
            if u.scheme() == "http" || u.scheme() == "https" {
                Some(u)
            } else {
                None
            }
        })
        .collect();

    Ok(valid)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn fetch_text(client: &reqwest::blocking::Client, url: &str) -> anyhow::Result<String> {
    let response = client
        .get(url)
        .send()
        .with_context(|| format!("failed to fetch {url}"))?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP {} fetching {url}", response.status());
    }

    response
        .text()
        .with_context(|| format!("failed to read response body from {url}"))
}

fn fetch_child_sitemap(
    client: &reqwest::blocking::Client,
    url: &str,
    quiet: bool,
) -> anyhow::Result<Vec<String>> {
    if !quiet {
        eprintln!("  Fetching child sitemap: {url}");
    }
    let body = fetch_text(client, url)?;
    Ok(parse_urlset(&body))
}

/// Attempts to deserialize `xml` as a `<urlset>`.  Returns loc strings.
fn parse_urlset(xml: &str) -> Vec<String> {
    match quick_xml::de::from_str::<UrlSet>(xml) {
        Ok(set) => set.urls.into_iter().map(|u| u.loc).collect(),
        Err(_) => Vec::new(),
    }
}

/// Attempts to deserialize `xml` as a `<sitemapindex>`.  Returns loc strings.
fn parse_sitemapindex(xml: &str) -> Vec<String> {
    match quick_xml::de::from_str::<SitemapIndex>(xml) {
        Ok(index) => index.sitemaps.into_iter().map(|e| e.loc).collect(),
        Err(_) => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_urlset_basic() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>https://example.com/page1</loc></url>
  <url><loc>https://example.com/page2</loc></url>
</urlset>"#;
        let locs = parse_urlset(xml);
        assert_eq!(
            locs,
            vec!["https://example.com/page1", "https://example.com/page2"]
        );
    }

    #[test]
    fn parse_sitemapindex_basic() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <sitemap><loc>https://example.com/sitemap-pages.xml</loc></sitemap>
</sitemapindex>"#;
        let locs = parse_sitemapindex(xml);
        assert_eq!(locs, vec!["https://example.com/sitemap-pages.xml"]);
    }
}
