use anyhow::Context;
use dom_smoothie::{Config, ParsePolicy, Readability, TextMode};
use url::Url;

pub struct ExtractOptions {
    /// If true, skip the readability algorithm and convert the full HTML document.
    pub raw: bool,
}

pub struct ExtractResult {
    pub markdown: String,
    pub title: Option<String>,
}

pub fn extract(html: &str, url: &Url, options: &ExtractOptions) -> anyhow::Result<ExtractResult> {
    let url_str = url.as_str();

    let cfg = Config {
        text_mode: TextMode::Markdown,
        ..Default::default()
    };
    let mut readability = Readability::new(html, Some(url_str), Some(cfg))
        .context("failed to initialize readability parser")?;

    let article = if options.raw {
        // ParsePolicy::Raw skips readability heuristics and converts the full HTML.
        readability
            .parse_with_policy(ParsePolicy::Raw)
            .context("failed to parse HTML with raw policy")?
    } else {
        readability
            .parse()
            .context("failed to extract article content")?
    };

    let markdown = clean_markdown_escapes(article.text_content.as_ref());
    let title = Some(article.title).filter(|t| !t.is_empty());
    Ok(ExtractResult { markdown, title })
}

/// Strips unnecessary backslash escapes introduced by dom_smoothie's markdown serialiser.
///
/// dom_smoothie escapes characters that CommonMark does not require to be escaped in
/// normal prose (e.g. `\.`, `\(`, `\)`).  This function removes those superfluous
/// escapes while preserving the one case where keeping `\.` matters: an ordered-list
/// marker at the start of a line (`1\.`, `12\.`, …).
fn clean_markdown_escapes(input: &str) -> String {
    // Pre-allocate roughly the same capacity; escapes we remove shrink the string slightly.
    let mut out = String::with_capacity(input.len());

    // Split on '\n' and rejoin, inserting the separator between pieces only (not after
    // the last one), so the output has the exact same trailing-newline behaviour as the
    // input regardless of whether it ends with '\n' or not.
    let mut lines = input.split('\n');

    if let Some(first) = lines.next() {
        clean_line(first, &mut out);
        for line in lines {
            out.push('\n');
            clean_line(line, &mut out);
        }
    }

    out
}

/// Process a single line, appending the cleaned result to `out`.
fn clean_line(line: &str, out: &mut String) {
    // Determine whether this line begins with one or more digits followed by `\.`
    // (i.e. an ordered-list marker).  If so, we must leave that first `\.` intact.
    let ordered_list_prefix_len = ordered_list_prefix(line);

    let mut chars = line.char_indices().peekable();

    while let Some((byte_offset, ch)) = chars.next() {
        if ch == '\\'
            && let Some(&(_, next)) = chars.peek()
        {
            match next {
                // These characters never need escaping in CommonMark prose.
                '(' | ')' | '{' | '}' | '"' => {
                    out.push(next);
                    chars.next();
                    continue;
                }
                '!' => {
                    // `!` only matters in image syntax `![`, so preserve the
                    // escape when followed by `[`, otherwise drop it.
                    chars.next();
                    let followed_by_bracket = chars.peek().is_some_and(|&(_, after)| after == '[');
                    if followed_by_bracket {
                        out.push('\\');
                    }
                    out.push('!');
                    continue;
                }
                '.' => {
                    // Keep the escape only when this `\.` is the dot of an ordered-list marker
                    // at the very start of the line (e.g. `1\.` or `12\.`).
                    // `ordered_list_prefix_len` is the byte offset of the backslash in that case.
                    if ordered_list_prefix_len > 0 && byte_offset == ordered_list_prefix_len {
                        out.push('\\');
                        out.push('.');
                    } else {
                        out.push('.');
                    }
                    chars.next();
                    continue;
                }
                _ => {}
            }
        }
        out.push(ch);
    }
}

/// If `line` starts with one or more ASCII digits immediately followed by `\.`,
/// returns the byte offset of the `\` (i.e. the number of digit bytes).
/// Otherwise returns 0.
fn ordered_list_prefix(line: &str) -> usize {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    // Must have at least one digit and be followed by `\.`
    if i > 0 && i + 1 < bytes.len() && bytes[i] == b'\\' && bytes[i + 1] == b'.' {
        i // byte offset of the backslash
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_HTML: &str = r"<!DOCTYPE html>
<html>
<head><title>Test Page</title></head>
<body>
  <article>
    <h1>Hello World</h1>
    <p>This is a test paragraph with enough content to pass the readability threshold.
    We need quite a bit of text here because readability algorithms typically require
    a minimum amount of content before they consider a page readable. Adding more
    sentences helps ensure the extraction works correctly in all cases.</p>
    <p>Another paragraph to ensure we have enough content. The readability algorithm
    needs sufficient text to identify the main content area of the page. Without enough
    text, it may fail to extract anything meaningful from the document.</p>
  </article>
</body>
</html>";

    #[test]
    fn test_extract_readability_mode() {
        let url = Url::parse("https://example.com/article").unwrap();
        let opts = ExtractOptions { raw: false };
        let result = extract(SIMPLE_HTML, &url, &opts).unwrap();
        assert!(result.markdown.contains("Hello World") || result.markdown.contains("paragraph"));
    }

    #[test]
    fn test_extract_raw_mode() {
        let url = Url::parse("https://example.com/article").unwrap();
        let opts = ExtractOptions { raw: true };
        let result = extract(SIMPLE_HTML, &url, &opts).unwrap();
        assert!(!result.markdown.is_empty());
    }

    #[test]
    fn test_extract_title() {
        let url = Url::parse("https://example.com/article").unwrap();
        let opts = ExtractOptions { raw: true };
        let result = extract(SIMPLE_HTML, &url, &opts).unwrap();
        assert_eq!(result.title.as_deref(), Some("Test Page"));
    }

    // --- clean_markdown_escapes unit tests ---

    #[test]
    fn test_clean_dot_in_normal_text() {
        // `\.` in the middle of a word should become `.`
        assert_eq!(clean_markdown_escapes("Hello\\."), "Hello.");
    }

    #[test]
    fn test_clean_parentheses() {
        // `\(` and `\)` should always be unescaped
        assert_eq!(clean_markdown_escapes("foo \\(bar\\)"), "foo (bar)");
    }

    #[test]
    fn test_ordered_list_marker_preserved() {
        // `1\.` at the start of a line must keep its backslash (ordered list prevention)
        assert_eq!(clean_markdown_escapes("1\\. item"), "1\\. item");
    }

    #[test]
    fn test_ordered_list_multi_digit_preserved() {
        // Multi-digit numbers are also preserved
        assert_eq!(clean_markdown_escapes("12\\. item"), "12\\. item");
    }

    #[test]
    fn test_dot_after_digits_not_at_line_start() {
        // `1\.` that is NOT at the start of the line should be unescaped
        assert_eq!(
            clean_markdown_escapes("word 1\\. something"),
            "word 1. something"
        );
    }

    #[test]
    fn test_other_escapes_untouched() {
        // Backslash before characters we do not handle (e.g. `*`) must be left as-is
        assert_eq!(clean_markdown_escapes("\\*bold\\*"), "\\*bold\\*");
    }

    #[test]
    fn test_multiline_mixed() {
        let input = "Hello\\.\n1\\. first\nnot list 1\\. here";
        let expected = "Hello.\n1\\. first\nnot list 1. here";
        assert_eq!(clean_markdown_escapes(input), expected);
    }

    #[test]
    fn test_trailing_newline_preserved() {
        assert_eq!(clean_markdown_escapes("Hello\\.\n"), "Hello.\n");
    }

    #[test]
    fn test_no_trailing_newline_preserved() {
        assert_eq!(clean_markdown_escapes("Hello\\."), "Hello.");
    }

    #[test]
    fn test_clean_exclamation_mark() {
        // Standalone `\!` should be unescaped
        assert_eq!(clean_markdown_escapes("Hello\\!"), "Hello!");
        assert_eq!(clean_markdown_escapes("wow\\! amazing\\!"), "wow! amazing!");
    }

    #[test]
    fn test_exclamation_before_bracket_preserved() {
        // `\![` is image syntax — keep the escape
        assert_eq!(
            clean_markdown_escapes("\\![alt](img.png)"),
            "\\![alt](img.png)"
        );
    }

    #[test]
    fn test_clean_braces_and_quotes() {
        assert_eq!(clean_markdown_escapes("\\{foo\\}"), "{foo}");
        assert_eq!(
            clean_markdown_escapes("said \\\"hello\\\""),
            "said \"hello\""
        );
    }
}
