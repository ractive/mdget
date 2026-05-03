use url::Url;

/// Normalize a URL for deduplication during crawling.
///
/// Steps applied:
/// 1. Strip fragment (`#...`)
/// 2. Lowercase scheme and host (via Url's built-in normalization)
/// 3. Remove default ports (80 for http, 443 for https)
/// 4. Normalize path: remove trailing slash UNLESS path is just `/`
/// 5. Sort query parameters alphabetically
/// 6. Decode unnecessary percent-encoding (unreserved ASCII characters)
pub fn normalize_url(url: &Url) -> String {
    let scheme = url.scheme();

    let host = url.host_str().unwrap_or("");

    // Only include port if it's non-default.
    let port_str = match url.port() {
        Some(80) if scheme == "http" => String::new(),
        Some(443) if scheme == "https" => String::new(),
        Some(p) => format!(":{p}"),
        None => String::new(),
    };

    // Normalize path: strip trailing slash unless path is exactly "/".
    let raw_path = url.path();
    let path = if raw_path.len() > 1 && raw_path.ends_with('/') {
        raw_path.trim_end_matches('/')
    } else {
        raw_path
    };
    let path = decode_unreserved(path);

    // Sort query parameters alphabetically.
    let query = build_sorted_query(url);

    // Assemble the normalized URL (no fragment).
    let mut result = format!("{scheme}://{host}{port_str}{path}");
    if !query.is_empty() {
        result.push('?');
        result.push_str(&query);
    }

    result
}

/// Decode percent-encoded unreserved ASCII characters (ALPHA / DIGIT / "-" / "." / "_" / "~").
/// These never need encoding per RFC 3986.
fn decode_unreserved(input: &str) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;

    while i < len {
        if bytes[i] == b'%' && i + 2 < len {
            let hi = hex_val(bytes[i + 1]);
            let lo = hex_val(bytes[i + 2]);
            if let (Some(h), Some(l)) = (hi, lo) {
                let byte = (h << 4) | l;
                if is_unreserved(byte) {
                    out.push(byte as char);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }

    out
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')
}

/// Collect query parameters, sort them alphabetically by key then value, and
/// reassemble into a query string.
fn build_sorted_query(url: &Url) -> String {
    let mut pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    if pairs.is_empty() {
        return String::new();
    }

    pairs.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    pairs
        .iter()
        .map(|(k, v)| {
            if v.is_empty() {
                k.clone()
            } else {
                format!("{k}={v}")
            }
        })
        .collect::<Vec<_>>()
        .join("&")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn strips_fragment() {
        let url = parse("https://example.com/page#section");
        assert_eq!(normalize_url(&url), "https://example.com/page");
    }

    #[test]
    fn strips_fragment_preserves_query() {
        let url = parse("https://example.com/page?q=1#anchor");
        assert_eq!(normalize_url(&url), "https://example.com/page?q=1");
    }

    #[test]
    fn removes_default_http_port() {
        let url = parse("http://example.com:80/path");
        assert_eq!(normalize_url(&url), "http://example.com/path");
    }

    #[test]
    fn removes_default_https_port() {
        let url = parse("https://example.com:443/path");
        assert_eq!(normalize_url(&url), "https://example.com/path");
    }

    #[test]
    fn keeps_non_default_port() {
        let url = parse("https://example.com:8080/path");
        assert_eq!(normalize_url(&url), "https://example.com:8080/path");
    }

    #[test]
    fn removes_trailing_slash_from_path() {
        let url = parse("https://example.com/path/");
        assert_eq!(normalize_url(&url), "https://example.com/path");
    }

    #[test]
    fn keeps_root_slash() {
        let url = parse("https://example.com/");
        assert_eq!(normalize_url(&url), "https://example.com/");
    }

    #[test]
    fn sorts_query_parameters() {
        let url = parse("https://example.com/search?z=last&a=first&m=middle");
        assert_eq!(
            normalize_url(&url),
            "https://example.com/search?a=first&m=middle&z=last"
        );
    }

    #[test]
    fn decodes_unreserved_percent_encoding() {
        // %41 = 'A', %2D = '-' — both are unreserved
        let url = parse("https://example.com/%41path%2Dmore");
        assert_eq!(normalize_url(&url), "https://example.com/Apath-more");
    }

    #[test]
    fn preserves_reserved_percent_encoding() {
        // %20 = space — reserved, should remain encoded
        let url = parse("https://example.com/path%20with%20spaces");
        assert_eq!(
            normalize_url(&url),
            "https://example.com/path%20with%20spaces"
        );
    }

    #[test]
    fn no_query_no_fragment() {
        let url = parse("https://example.com/article");
        assert_eq!(normalize_url(&url), "https://example.com/article");
    }

    #[test]
    fn dedup_identical_after_normalization() {
        let a = normalize_url(&parse("https://example.com/page#top"));
        let b = normalize_url(&parse("https://example.com/page/"));
        // These should differ (different paths), just checking they both normalize cleanly
        assert_eq!(a, "https://example.com/page");
        assert_eq!(b, "https://example.com/page");
        assert_eq!(a, b);
    }
}
