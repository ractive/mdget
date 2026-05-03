use std::collections::HashSet;

use url::Url;

/// Extract all unique HTTP/HTTPS links from `<a href="...">` tags in raw HTML.
///
/// Scans the HTML string for `href="..."` and `href='...'` patterns,
/// resolves relative URLs against `base_url`, strips fragments, and
/// deduplicates the results. Only `<a>` tags are followed — `<link>`,
/// `<base>`, and other elements are skipped to avoid stylesheet/font URLs.
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

        // Verify the href belongs to an <a> tag by scanning backward to
        // find the nearest `<` and checking the tag name.
        if !is_anchor_tag(bytes, href_pos) {
            continue;
        }

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

/// Return `true` if the `href` at `href_pos` belongs to an `<a>` tag.
///
/// Scans backward from `href_pos` to find the nearest `<`, then checks
/// whether the tag name that follows is `a` (case-insensitive). Returns
/// `false` if no `<` is found or if the tag name is anything other than `a`.
fn is_anchor_tag(bytes: &[u8], href_pos: usize) -> bool {
    // Scan backward to find the last `<` before href_pos.
    let Some(lt_pos) = bytes[..href_pos].iter().rposition(|&b| b == b'<') else {
        return false;
    };

    // The tag name starts right after `<`.
    // Skip optional `/` for closing tags (they have no href, but be safe).
    let name_start = lt_pos + 1;
    if name_start >= bytes.len() {
        return false;
    }

    // Extract the tag name: bytes up to the first whitespace, `>`, or `/`.
    let name_bytes = bytes[name_start..]
        .iter()
        .take_while(|&&b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'>' | b'/'))
        .copied()
        .collect::<Vec<u8>>();

    // Tag name must be exactly "a" (case-insensitive).
    matches!(name_bytes.as_slice(), [b'a' | b'A'])
}

/// Return `true` if the URL points to a known static asset (CSS, JS, fonts,
/// images, audio/video, archives, binary documents, etc.).
///
/// Uses only `Url::path()` so query strings never confuse extension detection.
/// Extensionless URLs and `.html` / `.htm` paths return `false`.
pub fn is_static_asset_url(url: &Url) -> bool {
    let path = url.path();

    // Extract the last path segment to avoid matching directory-like paths.
    let last_segment = path.rsplit('/').next().unwrap_or("");

    // Find the extension: everything after the last `.` in the last segment.
    let Some((_, ext)) = last_segment.rsplit_once('.') else {
        return false; // no extension → not a static asset
    };

    matches!(
        ext.to_ascii_lowercase().as_str(),
        // CSS
        "css"
        // Scripts
        | "js" | "mjs" | "cjs"
        // Fonts
        | "woff2" | "woff" | "ttf" | "eot" | "otf"
        // Images
        | "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" | "webp" | "avif"
        | "bmp" | "tiff" | "tif"
        // Audio / Video
        | "mp3" | "mp4" | "webm" | "ogg" | "wav" | "flac" | "aac" | "avi"
        | "mkv" | "mov"
        // Archives
        | "zip" | "tar" | "gz" | "bz2" | "xz" | "rar" | "7z"
        // Binary documents
        | "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx"
        // Data
        | "json" | "xml" | "csv"
        // Source maps
        | "map"
        // Binaries
        | "exe" | "dmg" | "deb" | "rpm" | "apk" | "msi" | "bin" | "iso" | "img"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Url {
        Url::parse("https://example.com/page").unwrap()
    }

    // --- extract_links: tag-name filtering ---

    #[test]
    fn link_tag_stylesheet_not_extracted() {
        // <link rel="stylesheet" href="..."> must be ignored
        let html = r#"<link rel="stylesheet" href="/style.css"><a href="/about">About</a>"#;
        let links = extract_links(html, &base());
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].as_str(), "https://example.com/about");
    }

    #[test]
    fn link_tag_preload_font_not_extracted() {
        let html = r#"<link rel="preload" href="/font.woff2" as="font">"#;
        let links = extract_links(html, &base());
        assert!(links.is_empty());
    }

    #[test]
    fn link_tag_icon_not_extracted() {
        let html = r#"<link rel="icon" href="/favicon.ico">"#;
        let links = extract_links(html, &base());
        assert!(links.is_empty());
    }

    // --- is_static_asset_url ---

    #[test]
    fn static_asset_css_detected() {
        let url = Url::parse("https://example.com/styles/main.css").unwrap();
        assert!(is_static_asset_url(&url));
    }

    #[test]
    fn static_asset_js_detected() {
        let url = Url::parse("https://example.com/bundle.js").unwrap();
        assert!(is_static_asset_url(&url));
    }

    #[test]
    fn static_asset_mjs_detected() {
        let url = Url::parse("https://example.com/module.mjs").unwrap();
        assert!(is_static_asset_url(&url));
    }

    #[test]
    fn static_asset_font_woff2_detected() {
        let url = Url::parse("https://example.com/fonts/inter.woff2").unwrap();
        assert!(is_static_asset_url(&url));
    }

    #[test]
    fn static_asset_font_ttf_detected() {
        let url = Url::parse("https://example.com/fonts/roboto.ttf").unwrap();
        assert!(is_static_asset_url(&url));
    }

    #[test]
    fn static_asset_png_detected() {
        let url = Url::parse("https://example.com/images/logo.png").unwrap();
        assert!(is_static_asset_url(&url));
    }

    #[test]
    fn static_asset_svg_detected() {
        let url = Url::parse("https://example.com/icons/arrow.svg").unwrap();
        assert!(is_static_asset_url(&url));
    }

    #[test]
    fn static_asset_pdf_detected() {
        let url = Url::parse("https://example.com/docs/manual.pdf").unwrap();
        assert!(is_static_asset_url(&url));
    }

    #[test]
    fn static_asset_zip_detected() {
        let url = Url::parse("https://example.com/releases/app.zip").unwrap();
        assert!(is_static_asset_url(&url));
    }

    #[test]
    fn static_asset_mp4_detected() {
        let url = Url::parse("https://example.com/video/intro.mp4").unwrap();
        assert!(is_static_asset_url(&url));
    }

    #[test]
    fn static_asset_json_detected() {
        let url = Url::parse("https://example.com/api/data.json").unwrap();
        assert!(is_static_asset_url(&url));
    }

    #[test]
    fn static_asset_map_detected() {
        let url = Url::parse("https://example.com/bundle.js.map").unwrap();
        assert!(is_static_asset_url(&url));
    }

    #[test]
    fn html_not_a_static_asset() {
        let url = Url::parse("https://example.com/page.html").unwrap();
        assert!(!is_static_asset_url(&url));
    }

    #[test]
    fn htm_not_a_static_asset() {
        let url = Url::parse("https://example.com/page.htm").unwrap();
        assert!(!is_static_asset_url(&url));
    }

    #[test]
    fn extensionless_url_not_a_static_asset() {
        let url = Url::parse("https://example.com/about").unwrap();
        assert!(!is_static_asset_url(&url));
    }

    #[test]
    fn root_url_not_a_static_asset() {
        let url = Url::parse("https://example.com/").unwrap();
        assert!(!is_static_asset_url(&url));
    }

    #[test]
    fn query_string_does_not_confuse_extension_detection() {
        // Path is "/page" (no extension); query is "?file=data.css" — should NOT be detected
        let url = Url::parse("https://example.com/page?file=data.css").unwrap();
        assert!(!is_static_asset_url(&url));
    }

    #[test]
    fn extension_detection_case_insensitive() {
        let url = Url::parse("https://example.com/IMAGE.PNG").unwrap();
        assert!(is_static_asset_url(&url));
    }

    // --- existing extract_links tests ---

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
