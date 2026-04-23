use url::Url;

fn main() {
    let html = r#"<!DOCTYPE html>
<html>
<head><title>Image Test</title></head>
<body>
  <article>
    <h1>Article with Images</h1>
    <p>This is a paragraph with content to pass readability.</p>
    <img src="test.png" alt="A test image" />
    <p>Another paragraph after the image with more content here.</p>
  </article>
</body>
</html>"#;
    
    let url = Url::parse("https://example.com/test").unwrap();
    let opts = mdget_core::ExtractOptions { raw: true };
    let result = mdget_core::extract(html, &url, &opts).unwrap();
    
    println!("=== MARKDOWN OUTPUT ===\n{}", result.markdown);
}
