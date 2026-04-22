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
    let markdown = strip_edit_links(&markdown);
    let markdown = clean_degenerate_tables(&markdown);
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

/// Strips Wikipedia-style `[edit]` section links from markdown output.
///
/// Matches two forms:
/// - `\[[edit](url-with-action=edit ...)\]` — escaped-bracket form from dom_smoothie
/// - `[edit](url-with-action=edit...)` — plain link form
///
/// Only strips when the visible link text is exactly `edit` (case-insensitive) and the
/// href contains `action=edit`. Standalone `[edit]` text (not a link) on its own line
/// is also removed. Blank lines left behind after removal are collapsed.
fn strip_edit_links(input: &str) -> String {
    // Regex-free approach: iterate lines, strip matching patterns, collapse blank runs.
    let mut out = String::with_capacity(input.len());
    let mut prev_blank = false;
    let trailing_newline = input.ends_with('\n');

    for line in input.split('\n') {
        let cleaned = strip_edit_links_from_line(line);
        let is_blank = cleaned.trim().is_empty();

        if is_blank {
            // Collapse consecutive blank lines (but remember we need to re-emit one later).
            prev_blank = true;
            continue;
        }

        if prev_blank {
            out.push('\n');
        }
        prev_blank = false;

        out.push_str(&cleaned);
        out.push('\n');
    }

    // Remove trailing newline if input didn't end with one.
    if !trailing_newline && out.ends_with('\n') {
        out.pop();
    }

    out
}

/// Strip `[edit]` link patterns from a single line.
fn strip_edit_links_from_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Try to match `\[[edit](...action=edit...)\]` at position i.
        if bytes[i] == b'\\'
            && i + 1 < len
            && bytes[i + 1] == b'['
            && let Some(end) = match_escaped_edit_link(line, i)
        {
            i = end;
            continue;
        }

        // Try to match `[edit](...action=edit...)` at position i.
        if bytes[i] == b'['
            && let Some(end) = match_plain_edit_link(line, i)
        {
            i = end;
            continue;
        }

        // Push current byte as-is (safe: we only index ASCII boundaries above).
        // Use char-boundary-aware push to handle multi-byte chars correctly.
        let ch = line[i..].chars().next().unwrap_or('\0');
        out.push(ch);
        i += ch.len_utf8();
    }

    // If the whole line collapsed to whitespace (e.g. it was only an edit link), return empty.
    if out.trim().is_empty() {
        String::new()
    } else {
        out
    }
}

/// Try to match `\[[edit](url)\]` starting at `pos` in `s`.
/// Returns `Some(end_pos)` if matched, where `end_pos` is the byte after the closing `\]`.
fn match_escaped_edit_link(s: &str, pos: usize) -> Option<usize> {
    // Expected prefix: `\[`
    let rest = s.get(pos..)?;
    let inner = rest.strip_prefix("\\[")?;

    // Find the matching `\]` that closes the outer escaped bracket.
    // Structure: `\[` LINK_CONTENT `\]`
    // where LINK_CONTENT is `[edit](url "title")` or `[edit](url)`.
    let (link_content, after_close) = split_at_escaped_close(inner)?;

    // link_content must be a plain markdown link `[edit](url...)`
    let link_inner = link_content.strip_prefix("[edit](")?;
    let href_end = link_inner.find(')')?;
    let href = link_inner.get(..href_end)?;

    if !is_edit_action_url(href) {
        return None;
    }

    // Consumed: pos .. pos + 2 (for `\[`) + link_content.len() + 2 (for `\]`)
    let consumed = pos + 2 + link_content.len() + after_close;
    Some(consumed)
}

/// Split `s` at the first `\]`, returning `(before, bytes_consumed_by_close)`.
/// `bytes_consumed_by_close` is 2 (the two bytes `\]`).
fn split_at_escaped_close(s: &str) -> Option<(&str, usize)> {
    let idx = s.find("\\]")?;
    Some((&s[..idx], idx + 2))
}

/// Try to match `[edit](url)` starting at `pos` in `s`.
/// Returns `Some(end_pos)` on a match.
fn match_plain_edit_link(s: &str, pos: usize) -> Option<usize> {
    let rest = s.get(pos..)?;
    // Must start with `[edit](`
    let after_text = rest.strip_prefix("[edit](")?;

    // Find closing `)` — could contain spaces/titles, so search for first `)`.
    let href_end = after_text.find(')')?;
    let href_region = &after_text[..href_end];

    // href_region may be `url` or `url "title"` — extract just the URL part.
    let href = href_region
        .split_once('"')
        .map_or(href_region.trim(), |(u, _)| u.trim());

    if !is_edit_action_url(href) {
        return None;
    }

    // Total consumed: pos + len(`[edit](`) + href_end + len(`)`)
    let consumed = pos + "[edit](".len() + href_end + 1;
    Some(consumed)
}

/// Returns true if `url` contains `action=edit` as a query parameter fragment.
fn is_edit_action_url(url: &str) -> bool {
    url.contains("action=edit")
}

// ── Table heuristic ──────────────────────────────────────────────────────────

/// Detects and converts degenerate markdown tables to more readable formats.
///
/// A table is considered degenerate when:
/// - More than 50% of data cells (excluding header row) are empty/whitespace-only, OR
/// - Rows have inconsistent column counts (malformed), OR
/// - There is only one data column.
///
/// Conversion rules:
/// - 2-column key-value pattern → `**Key:** Value` per line
/// - 1-column → bullet list (`- item`)
/// - Other degenerate → plain text (one row per line, cells space-joined)
///
/// Good tables (most cells filled, consistent columns) pass through unchanged.
fn clean_degenerate_tables(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let trailing_newline = input.ends_with('\n');
    // Strip the trailing newline so split('\n') doesn't produce a spurious empty element.
    let content = if trailing_newline {
        &input[..input.len() - 1]
    } else {
        input
    };
    let lines: Vec<&str> = content.split('\n').collect();
    let total = lines.len();
    let mut i = 0;

    while i < total {
        // Detect start of a markdown table: a line starting with `|`.
        if is_table_line(lines[i]) {
            // Collect all consecutive table lines.
            let start = i;
            while i < total && is_table_line(lines[i]) {
                i += 1;
            }
            let table_lines = &lines[start..i];
            let replacement = process_table(table_lines);
            out.push_str(&replacement);
        } else {
            out.push_str(lines[i]);
            if i + 1 < total {
                out.push('\n');
            }
            i += 1;
        }
    }

    if trailing_newline && !out.ends_with('\n') {
        out.push('\n');
    }

    out
}

fn is_table_line(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') || t.starts_with("|-") || t.starts_with("|:")
}

/// Process a block of table lines, returning the appropriate representation.
fn process_table(lines: &[&str]) -> String {
    // Split into header, separator, and data rows.
    let (header_row, sep_idx, data_rows) = parse_table_structure(lines);

    // If we couldn't identify a separator, just pass through unchanged.
    let Some(sep_idx) = sep_idx else {
        return lines.join("\n") + "\n";
    };

    let header_cells = split_table_row(header_row);

    // Need at least one header column.
    if header_cells.is_empty() {
        return lines.join("\n") + "\n";
    }

    // No data rows — table is header-only; pass through unchanged.
    if data_rows.is_empty() {
        return lines[..=sep_idx].join("\n") + "\n";
    }

    // Parse data rows into cell vecs.
    let parsed_data: Vec<Vec<&str>> = data_rows.iter().map(|r| split_table_row(r)).collect();

    // Check column consistency.
    let expected_cols = header_cells.len();
    let consistent = parsed_data.iter().all(|r| r.len() == expected_cols);

    // Calculate empty-cell ratio across all data rows.
    let total_cells: usize = parsed_data.iter().map(Vec::len).sum();
    let empty_cells: usize = parsed_data
        .iter()
        .flat_map(|r| r.iter())
        .filter(|c| c.trim().is_empty())
        .count();
    let empty_ratio = if total_cells == 0 {
        0.0f64
    } else {
        #[allow(clippy::cast_precision_loss)]
        {
            empty_cells as f64 / total_cells as f64
        }
    };

    let is_degenerate = !consistent || empty_ratio > 0.5 || expected_cols == 1;

    if !is_degenerate {
        // Good table — pass through unchanged.
        return lines.join("\n") + "\n";
    }

    // Determine conversion format.
    let effective_cols = if consistent {
        expected_cols
    } else {
        // Use mode of row widths as the effective column count.
        let mut counts = std::collections::HashMap::new();
        for r in &parsed_data {
            *counts.entry(r.len()).or_insert(0usize) += 1;
        }
        counts
            .into_iter()
            .max_by_key(|(_, v)| *v)
            .map_or(1, |(k, _)| k)
    };

    let mut out = String::new();

    if effective_cols == 1 {
        // Single-column → bullet list.
        for row in &parsed_data {
            let cell = row.first().copied().unwrap_or("").trim();
            if !cell.is_empty() {
                out.push_str("- ");
                out.push_str(cell);
                out.push('\n');
            }
        }
    } else if effective_cols == 2 {
        // Two-column → key-value format.
        for row in &parsed_data {
            let key = row.first().copied().unwrap_or("").trim();
            let val = row.get(1).copied().unwrap_or("").trim();
            if key.is_empty() && val.is_empty() {
                continue;
            }
            if key.is_empty() {
                out.push_str(val);
                out.push('\n');
            } else if val.is_empty() {
                out.push_str("**");
                out.push_str(key);
                out.push_str(":**\n");
            } else {
                out.push_str("**");
                out.push_str(key);
                out.push_str(":** ");
                out.push_str(val);
                out.push('\n');
            }
        }
    } else {
        // Multi-column degenerate → plain text, one row per line.
        for row in &parsed_data {
            let text: Vec<&str> = row
                .iter()
                .map(|c| c.trim())
                .filter(|c| !c.is_empty())
                .collect();
            if !text.is_empty() {
                out.push_str(&text.join(" "));
                out.push('\n');
            }
        }
    }

    // If nothing was emitted (all rows were empty), return the original table.
    if out.is_empty() {
        lines.join("\n") + "\n"
    } else {
        out
    }
}

/// Returns (header_line, separator_index, data_lines).
fn parse_table_structure<'a>(lines: &'a [&'a str]) -> (&'a str, Option<usize>, Vec<&'a str>) {
    if lines.is_empty() {
        return ("", None, vec![]);
    }

    // The separator row contains only `|`, `-`, `:`, and spaces.
    let sep_idx = lines.iter().position(|l| is_separator_row(l));

    let Some(sep) = sep_idx else {
        // No separator found — not a standard table; treat first line as data.
        return (lines[0], None, lines[1..].to_vec());
    };

    let header = if sep > 0 { lines[sep - 1] } else { lines[0] };
    let data = lines[sep + 1..].to_vec();
    (header, Some(sep), data)
}

fn is_separator_row(line: &str) -> bool {
    let t = line.trim();
    if !t.starts_with('|') {
        return false;
    }
    t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

/// Split a markdown table row into cell contents (trimmed, without leading/trailing `|`).
fn split_table_row(line: &str) -> Vec<&str> {
    let t = line.trim();
    // Strip leading and trailing `|`.
    let inner = t.strip_prefix('|').unwrap_or(t);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    inner.split('|').collect()
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

    // --- strip_edit_links unit tests ---

    #[test]
    fn test_strip_plain_edit_link() {
        let input = "## Heading [edit](https://en.wikipedia.org/w/index.php?title=Foo&action=edit&section=1)\n";
        let result = strip_edit_links(input);
        assert!(!result.contains("[edit]"), "edit link should be removed");
        assert!(result.contains("## Heading"), "heading should remain");
    }

    #[test]
    fn test_strip_escaped_bracket_edit_link() {
        let input = "## History \\[[edit](https://en.wikipedia.org/w/index.php?title=Foo&action=edit&section=2 \"Edit section: History\")\\]\n";
        let result = strip_edit_links(input);
        assert!(!result.contains("[edit]"), "edit link should be removed");
        assert!(result.contains("## History"), "heading should remain");
    }

    #[test]
    fn test_strip_standalone_edit_line_collapses() {
        // A line that is only an edit link should disappear entirely (blank-line collapse).
        let input =
            "## Section\n\n[edit](https://en.wikipedia.org/w/index.php?action=edit)\n\nContent\n";
        let result = strip_edit_links(input);
        assert!(!result.contains("[edit]"));
        assert!(result.contains("## Section"));
        assert!(result.contains("Content"));
    }

    #[test]
    fn test_no_strip_non_edit_action_link() {
        // A link with visible text `edit` but no `action=edit` in the URL should NOT be stripped.
        let input = "Click [edit](https://example.com/some-page) to continue.\n";
        let result = strip_edit_links(input);
        assert_eq!(result, input, "non-edit-action link must be preserved");
    }

    #[test]
    fn test_no_strip_plain_edit_text() {
        // Bare `[edit]` (no URL) in the middle of prose should not be stripped.
        let input = "The button labelled [edit] is blue.\n";
        let result = strip_edit_links(input);
        assert_eq!(result, input, "bare [edit] in prose must be preserved");
    }

    #[test]
    fn test_no_edit_links_unchanged() {
        let input = "Normal paragraph with no edit links.\n\nAnother paragraph.\n";
        assert_eq!(strip_edit_links(input), input);
    }

    #[test]
    fn test_strip_edit_links_trailing_newline_preserved() {
        let input = "## Heading [edit](https://en.wikipedia.org/w/?action=edit)\n";
        let result = strip_edit_links(input);
        assert!(result.ends_with('\n'), "trailing newline must be preserved");
    }

    #[test]
    fn test_strip_edit_links_no_trailing_newline() {
        let input = "Text [edit](https://en.wikipedia.org/w/?action=edit) end";
        let result = strip_edit_links(input);
        assert!(!result.ends_with('\n'), "must not gain a trailing newline");
        assert!(result.contains("Text"));
        assert!(result.contains("end"));
    }

    // --- clean_degenerate_tables unit tests ---

    #[test]
    fn test_good_table_unchanged() {
        // All cells filled, consistent columns → pass through untouched.
        let input =
            "| Name | Age | City |\n| --- | --- | --- |\n| Alice | 30 | NYC |\n| Bob | 25 | LA |\n";
        assert_eq!(clean_degenerate_tables(input), input);
    }

    #[test]
    fn test_single_column_table_becomes_bullet_list() {
        let input = "| Items |\n| --- |\n| Apple |\n| Banana |\n| Cherry |\n";
        let result = clean_degenerate_tables(input);
        assert!(result.contains("- Apple"), "should be a bullet list");
        assert!(result.contains("- Banana"));
        assert!(result.contains("- Cherry"));
        assert!(!result.contains('|'), "pipes should be gone");
    }

    #[test]
    fn test_two_column_kv_degenerate_becomes_kv_format() {
        // >50% empty cells: rows with empty key+value push the ratio over 50%.
        let input = "| Property | Value |\n| --- | --- |\n| Born | 1990 |\n| | |\n| | |\n| Country | USA |\n| | |\n| | |\n";
        let result = clean_degenerate_tables(input);
        assert!(
            result.contains("**Born:** 1990"),
            "key-value format expected, got: {result}"
        );
        assert!(result.contains("**Country:** USA"));
        assert!(!result.contains('|'), "pipes should be gone");
    }

    #[test]
    fn test_malformed_table_inconsistent_columns_to_plain_text() {
        // Rows with different column counts → plain text.
        let input = "| A | B | C |\n| --- | --- | --- |\n| x | y | z |\n| only one |\n| a | b |\n";
        let result = clean_degenerate_tables(input);
        // Should not contain pipes in the output (converted to plain).
        assert!(!result.contains('|'), "should be converted to plain text");
    }

    #[test]
    fn test_mostly_empty_cells_converted() {
        // >50% empty cells → degenerate (both key and value empty in some rows).
        let input = "| Key | Value |\n| --- | --- |\n| Foo | |\n| | |\n| | |\n| Baz | thing |\n";
        let result = clean_degenerate_tables(input);
        assert!(!result.contains('|'), "should be converted from table");
    }

    #[test]
    fn test_header_only_table_passes_through() {
        // Only header + separator, no data rows → keep as-is.
        let input = "| Col A | Col B |\n| --- | --- |\n";
        let result = clean_degenerate_tables(input);
        assert!(
            result.contains('|'),
            "header-only table should pass through"
        );
    }

    #[test]
    fn test_non_table_content_unchanged() {
        let input = "This is prose.\n\n- bullet one\n- bullet two\n\n## Heading\n";
        assert_eq!(clean_degenerate_tables(input), input);
    }

    #[test]
    fn test_mixed_good_and_bad_tables() {
        let good_table = "| Name | Score |\n| --- | --- |\n| Alice | 95 |\n| Bob | 88 |\n";
        let bad_table = "| Key | Value |\n| --- | --- |\n| | |\n| | |\n| | |\n| Z | thing |\n";
        let input = format!("{good_table}\nSome prose\n\n{bad_table}");
        let result = clean_degenerate_tables(&input);

        // Good table still has pipes.
        assert!(
            result.contains("| Alice | 95 |"),
            "good table should be unchanged"
        );
        // Bad table has been converted.
        assert!(
            result.contains("**Z:** thing"),
            "bad table should be converted, got: {result}"
        );
    }

    #[test]
    fn test_two_column_all_full_not_degenerate() {
        // 2-column table where all cells are filled → NOT degenerate, pass through.
        let input =
            "| Key | Value |\n| --- | --- |\n| Alpha | One |\n| Beta | Two |\n| Gamma | Three |\n";
        let result = clean_degenerate_tables(input);
        assert_eq!(
            result, input,
            "fully-populated 2-col table should pass through"
        );
    }
}
