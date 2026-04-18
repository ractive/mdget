use url::Url;

/// Generate a filename for saving a fetched page.
///
/// Priority:
/// 1. Page title (slugified, max 80 chars) → `"{slug}.md"`
/// 2. Last non-empty URL path segment (slugified) → `"{slug}.md"`
/// 3. Fallback: `"{hostname}-{YYYYMMDD}.md"` using current date
pub fn generate_filename(title: Option<&str>, url: &Url) -> String {
    // Priority 1: title
    if let Some(t) = title {
        let slug = slugify(t, 80);
        if !slug.is_empty() {
            return format!("{slug}.md");
        }
    }

    // Priority 2: last non-empty path segment
    if let Some(segment) = last_path_segment(url) {
        let slug = slugify(segment, 80);
        if !slug.is_empty() {
            return format!("{slug}.md");
        }
    }

    // Priority 3: hostname + date fallback
    let hostname = url.host_str().unwrap_or("page");
    let date = today_yyyymmdd();
    format!("{hostname}-{date}.md")
}

/// Slugify a string: lowercase, non-alphanumeric chars become hyphens,
/// consecutive hyphens collapsed, leading/trailing hyphens trimmed,
/// truncated to `max_len` characters.
fn slugify(s: &str, max_len: usize) -> String {
    let lowered = s.to_lowercase();
    let mut slug = String::with_capacity(lowered.len());
    let mut prev_hyphen = false;

    for ch in lowered.chars() {
        if ch.is_alphanumeric() {
            slug.push(ch);
            prev_hyphen = false;
        } else if !prev_hyphen {
            slug.push('-');
            prev_hyphen = true;
        }
    }

    // Trim trailing hyphen added by the loop
    let slug = slug.trim_matches('-');

    // Truncate at a character boundary, avoiding a trailing hyphen after truncation.
    let truncated = truncate_at_char(slug, max_len);
    truncated.trim_matches('-').to_string()
}

/// Truncate `s` to at most `max_len` characters (not bytes).
fn truncate_at_char(s: &str, max_len: usize) -> &str {
    if s.chars().count() <= max_len {
        return s;
    }
    // Find the byte index of the max_len-th character.
    match s.char_indices().nth(max_len) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

/// Extract the last non-empty path segment from a URL.
fn last_path_segment(url: &Url) -> Option<&str> {
    url.path_segments()
        .and_then(|mut segments| segments.rfind(|s| !s.is_empty()))
}

/// Return today's date formatted as YYYYMMDD using the system clock.
fn today_yyyymmdd() -> String {
    // Use std::time to compute a simple UTC date without pulling in chrono.
    // We compute the number of days since the Unix epoch and convert to a
    // proleptic Gregorian date (Y-M-D).
    use std::time::{SystemTime, UNIX_EPOCH};

    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let days = secs / 86400;
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}{month:02}{day:02}")
}

/// Convert a count of days since 1970-01-01 to a (year, month, day) tuple.
/// Uses the Gregorian calendar algorithm (Richards, 2013).
fn days_to_ymd(days: u64) -> (u32, u32, u32) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    // All values fit in u32 for any plausible calendar date.
    #[allow(clippy::cast_possible_truncation)]
    (y as u32, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_basic() {
        assert_eq!(slugify("Hello World", 80), "hello-world");
    }

    #[test]
    fn test_slugify_special_chars() {
        assert_eq!(slugify("Rust & Cargo: A Guide!", 80), "rust-cargo-a-guide");
    }

    #[test]
    fn test_slugify_consecutive_hyphens() {
        assert_eq!(slugify("foo   bar", 80), "foo-bar");
    }

    #[test]
    fn test_slugify_truncates() {
        let long = "a".repeat(100);
        let result = slugify(&long, 80);
        assert_eq!(result.chars().count(), 80);
    }

    #[test]
    fn test_slugify_trim_hyphens_after_truncation() {
        // Truncating mid-separator should not leave trailing hyphen.
        let s = "hello-world-foo";
        let result = slugify(s, 7); // "hello-w" — no trailing hyphen
        assert!(!result.ends_with('-'));
        assert!(!result.starts_with('-'));
    }

    #[test]
    fn test_generate_filename_from_title() {
        let url = Url::parse("https://example.com/some/path").unwrap();
        let name = generate_filename(Some("My Article Title"), &url);
        assert_eq!(name, "my-article-title.md");
    }

    #[test]
    fn test_generate_filename_from_url_path() {
        let url = Url::parse("https://example.com/articles/my-post").unwrap();
        let name = generate_filename(None, &url);
        assert_eq!(name, "my-post.md");
    }

    #[test]
    fn test_generate_filename_fallback() {
        let url = Url::parse("https://example.com/").unwrap();
        let name = generate_filename(None, &url);
        // Should be "example.com-YYYYMMDD.md"
        assert!(name.starts_with("example.com-"));
        assert!(
            std::path::Path::new(&name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        );
        // Date part should be 8 digits
        let date_part = &name["example.com-".len()..name.len() - 3];
        assert_eq!(date_part.len(), 8);
        assert!(date_part.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_generate_filename_empty_title_falls_back_to_url() {
        let url = Url::parse("https://example.com/my-page").unwrap();
        let name = generate_filename(Some(""), &url);
        assert_eq!(name, "my-page.md");
    }

    #[test]
    fn test_generate_filename_title_special_chars() {
        let url = Url::parse("https://example.com/").unwrap();
        let name = generate_filename(Some("C++ & Rust: 2024!"), &url);
        assert_eq!(name, "c-rust-2024.md");
    }

    #[test]
    fn test_days_to_ymd_epoch() {
        // Day 0 = 1970-01-01
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn test_days_to_ymd_known_date() {
        // 2024-01-01: days since epoch = 19723
        assert_eq!(days_to_ymd(19723), (2024, 1, 1));
    }
}
