use std::collections::HashSet;

use url::Url;

/// Extract all unique HTTP/HTTPS links from raw HTML.
///
/// Scans the HTML string for `href="..."` and `href='...'` patterns,
/// resolves relative URLs against `base_url`, strips fragments, and
/// deduplicates the results.
///
/// This operates on raw HTML *before* readability processing, so navigation
/// links and sidebars are included — which is what we want for crawling.
pub fn extract_links(html: &str, base_url: &Url) -> Vec<Url> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut links: Vec<Url> = Vec::new();

    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Scan for `href` (case-insensitive).
        let Some(href_pos) = find_icase(html, i, b"href") else {
            break;
        };

        i = href_pos + 4; // advance past "href"

        // Skip whitespace.
        while i < len && bytes[i] == b' ' {
            i += 1;
        }

        // Must be followed by '='.
        if i >= len || bytes[i] != b'=' {
            continue;
        }
        i += 1; // skip '='

        // Skip whitespace.
        while i < len && bytes[i] == b' ' {
            i += 1;
        }

        if i >= len {
            break;
        }

        // Accept double-quote or single-quote.
        let quote = bytes[i];
        if quote != b'"' && quote != b'\'' {
            continue;
        }
        i += 1; // skip opening quote

        // Find closing quote.
        let value_start = i;
        let Some(close_offset) = html[i..].find(quote as char) else {
            break;
        };
        let href_value = &html[value_start..value_start + close_offset];
        i = value_start + close_offset + 1; // advance past closing quote

        // Skip empty, fragment-only, and non-http(s) hrefs.
        let trimmed = href_value.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Resolve the href against the base URL.
        let Ok(resolved) = base_url.join(trimmed) else {
            continue;
        };

        // Only keep http/https.
        if resolved.scheme() != "http" && resolved.scheme() != "https" {
            continue;
        }

        // Strip fragment.
        let mut clean = resolved;
        clean.set_fragment(None);

        // Deduplicate by string representation.
        let key = clean.to_string();
        if seen.insert(key) {
            links.push(clean);
        }
    }

    links
}

/// Find `needle` in `haystack` starting at `from`, case-insensitively (ASCII only).
/// Returns the byte offset of the match start, or `None`.
fn find_icase(haystack: &str, from: usize, needle: &[u8]) -> Option<usize> {
    let bytes = haystack.as_bytes();
    let nlen = needle.len();
    if from + nlen > bytes.len() {
        return None;
    }
    'outer: for i in from..=(bytes.len() - nlen) {
        for (j, &nb) in needle.iter().enumerate() {
            if !bytes[i + j].eq_ignore_ascii_case(&nb) {
                continue 'outer;
            }
        }
        return Some(i);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Url {
        Url::parse("https://example.com/page").unwrap()
    }

    #[test]
    fn extracts_absolute_link() {
        let html = r#"<a href="https://other.com/article">click</a>"#;
        let links = extract_links(html, &base());
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].as_str(), "https://other.com/article");
    }

    #[test]
    fn extracts_relative_link() {
        let html = r#"<a href="/about">about</a>"#;
        let links = extract_links(html, &base());
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].as_str(), "https://example.com/about");
    }

    #[test]
    fn strips_fragment_from_link() {
        let html = r#"<a href="/page#section">section</a>"#;
        let links = extract_links(html, &base());
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].as_str(), "https://example.com/page");
    }

    #[test]
    fn fragment_only_link_is_skipped() {
        let html = r##"<a href="#top">top</a>"##;
        let links = extract_links(html, &base());
        assert!(links.is_empty());
    }

    #[test]
    fn mailto_is_filtered_out() {
        let html = r#"<a href="mailto:user@example.com">email</a>"#;
        let links = extract_links(html, &base());
        assert!(links.is_empty());
    }

    #[test]
    fn javascript_is_filtered_out() {
        let html = r#"<a href="javascript:void(0)">click</a>"#;
        let links = extract_links(html, &base());
        assert!(links.is_empty());
    }

    #[test]
    fn tel_is_filtered_out() {
        let html = r#"<a href="tel:+1234567890">call</a>"#;
        let links = extract_links(html, &base());
        assert!(links.is_empty());
    }

    #[test]
    fn deduplicates_same_url() {
        let html = r#"<a href="/page">one</a><a href="/page">two</a>"#;
        let links = extract_links(html, &base());
        assert_eq!(links.len(), 1);
    }

    #[test]
    fn handles_single_quotes() {
        let html = r"<a href='/path/to/page'>link</a>";
        let links = extract_links(html, &base());
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].as_str(), "https://example.com/path/to/page");
    }

    #[test]
    fn multiple_links_extracted() {
        let html = r#"
            <nav>
                <a href="/home">Home</a>
                <a href="/about">About</a>
                <a href="https://external.com/">External</a>
            </nav>
        "#;
        let links = extract_links(html, &base());
        assert_eq!(links.len(), 3);
    }

    #[test]
    fn case_insensitive_href() {
        let html = r#"<a HREF="/page">link</a>"#;
        let links = extract_links(html, &base());
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].as_str(), "https://example.com/page");
    }

    #[test]
    fn empty_href_skipped() {
        let html = r#"<a href="">empty</a>"#;
        let links = extract_links(html, &base());
        assert!(links.is_empty());
    }

    #[test]
    fn dedup_fragment_variants() {
        // "/page" and "/page#anchor" both normalize to "/page" after fragment strip
        let html = r#"<a href="/page">link</a><a href="/page#anchor">anchored</a>"#;
        let links = extract_links(html, &base());
        assert_eq!(links.len(), 1);
    }
}
