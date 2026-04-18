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

    let markdown = article.text_content.to_string();
    let title = Some(article.title).filter(|t| !t.is_empty());
    Ok(ExtractResult { markdown, title })
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
}
