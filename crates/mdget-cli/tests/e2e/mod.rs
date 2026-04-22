use assert_cmd::Command;
use mockito::Server;
use predicates::prelude::*;

const TEST_HTML: &[u8] = br"<!DOCTYPE html>
<html>
<head><title>Test Article</title></head>
<body>
  <article>
    <h1>Test Article</h1>
    <p>This is a paragraph with enough content for readability extraction to work correctly.</p>
    <p>Another paragraph with more content to ensure the article is long enough to be extracted.</p>
  </article>
</body>
</html>";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn mdget() -> Command {
    Command::cargo_bin("mdget").unwrap()
}

// ---------------------------------------------------------------------------
// 1. cli_prints_help
// ---------------------------------------------------------------------------
#[test]
fn cli_prints_help() {
    mdget()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Fetch web pages"));
}

// ---------------------------------------------------------------------------
// 2. cli_prints_version
// ---------------------------------------------------------------------------
#[test]
fn cli_prints_version() {
    mdget()
        .arg("-V")
        .assert()
        .success()
        .stdout(predicate::str::contains("mdget"));
}

// ---------------------------------------------------------------------------
// 3. cli_missing_url_exits_with_error
// ---------------------------------------------------------------------------
#[test]
fn cli_missing_url_exits_with_error() {
    mdget()
        .assert()
        .failure()
        .stderr(predicate::str::contains("no inputs provided"));
}

// ---------------------------------------------------------------------------
// 4. cli_init_requires_claude_flag
// ---------------------------------------------------------------------------
#[test]
fn cli_init_requires_claude_flag() {
    mdget()
        .args(["init"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--claude"));
}

// ---------------------------------------------------------------------------
// 5. cli_init_deinit_project_roundtrip
// ---------------------------------------------------------------------------
#[test]
fn cli_init_deinit_project_roundtrip() {
    let dir = tempfile::tempdir().unwrap();

    // init installs skill and updates CLAUDE.md
    mdget()
        .args(["init", "--claude"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Installed skill"))
        .stderr(predicate::str::contains("Updated CLAUDE.md"));

    // SKILL.md was created
    assert!(dir.path().join(".claude/skills/mdget/SKILL.md").exists());

    // CLAUDE.md contains managed section
    let claude_md = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    assert!(claude_md.contains("<!-- mdget:start -->"));
    assert!(claude_md.contains("<!-- mdget:end -->"));

    // deinit removes skill and strips managed section
    mdget()
        .args(["deinit"])
        .current_dir(dir.path())
        .assert()
        .success();

    // SKILL.md is gone
    assert!(!dir.path().join(".claude/skills/mdget/SKILL.md").exists());

    // CLAUDE.md no longer contains managed section (or was removed)
    if dir.path().join("CLAUDE.md").exists() {
        let after = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert!(!after.contains("<!-- mdget:start -->"));
    }
}

// ---------------------------------------------------------------------------
// 6. cli_init_idempotent
// ---------------------------------------------------------------------------
#[test]
fn cli_init_idempotent() {
    let dir = tempfile::tempdir().unwrap();

    for _ in 0..2 {
        mdget()
            .args(["init", "--claude"])
            .current_dir(dir.path())
            .assert()
            .success();
    }

    // Managed section should appear exactly once
    let claude_md = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    assert_eq!(
        claude_md.matches("<!-- mdget:start -->").count(),
        1,
        "managed section should appear exactly once after double init"
    );
}

// ---------------------------------------------------------------------------
// 7. cli_deinit_idempotent
// ---------------------------------------------------------------------------
#[test]
fn cli_deinit_idempotent() {
    let dir = tempfile::tempdir().unwrap();

    // deinit on a clean directory should not error
    mdget()
        .args(["deinit"])
        .current_dir(dir.path())
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// 8. cli_handles_html_content_type
// ---------------------------------------------------------------------------
#[test]
fn cli_handles_html_content_type() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML)
        .create();

    mdget()
        .args(["-t", "5", "--raw", &server.url()])
        .assert()
        .success()
        .stdout(predicate::str::contains("paragraph with enough content"));

    mock.assert();
}

// ---------------------------------------------------------------------------
// 9. cli_handles_plain_text
// ---------------------------------------------------------------------------
#[test]
fn cli_handles_plain_text() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/plain")
        .with_body("Hello, plain world!")
        .create();

    mdget()
        .args(["-t", "5", &server.url()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello, plain world!"));

    mock.assert();
}

// ---------------------------------------------------------------------------
// 10. cli_handles_json
// ---------------------------------------------------------------------------
#[test]
fn cli_handles_json() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body("{\"key\":\"value\"}")
        .create();

    mdget()
        .args(["-t", "5", &server.url()])
        .assert()
        .success()
        .stdout(predicate::str::contains("```json"))
        .stdout(predicate::str::contains("{\"key\":\"value\"}"));

    mock.assert();
}

// ---------------------------------------------------------------------------
// 11. cli_rejects_binary_content
// ---------------------------------------------------------------------------
#[test]
fn cli_rejects_binary_content() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "image/png")
        .with_body(b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR".as_ref())
        .create();

    mdget()
        .args(["-t", "5", &server.url()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("binary content"))
        .stderr(predicate::str::contains("image/png"));

    mock.assert();
}

// ---------------------------------------------------------------------------
// 12. cli_quiet_suppresses_progress
// ---------------------------------------------------------------------------
#[test]
fn cli_quiet_suppresses_progress() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html")
        .with_body(TEST_HTML)
        .create();

    let output = mdget()
        .args(["-q", "-t", "5", "--raw", &server.url()])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Fetching"),
        "stderr should not contain 'Fetching' in quiet mode, got: {stderr}"
    );
    assert!(
        !stderr.contains("Extracting"),
        "stderr should not contain 'Extracting' in quiet mode, got: {stderr}"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.is_empty(),
        "stdout should contain content even in quiet mode"
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 13. cli_quiet_still_shows_errors
// ---------------------------------------------------------------------------
#[test]
fn cli_quiet_still_shows_errors() {
    mdget()
        .args(["-q", "ftp://example.com"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported"));
}

// ---------------------------------------------------------------------------
// 14. cli_output_ends_with_newline
// ---------------------------------------------------------------------------
#[test]
fn cli_output_ends_with_newline() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html")
        .with_body(TEST_HTML)
        .create();

    let output = mdget()
        .args(["-t", "5", "--raw", &server.url()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stdout.ends_with(b"\n"),
        "stdout must end with a newline"
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 15. cli_strips_wikipedia_edit_links
// ---------------------------------------------------------------------------
#[test]
fn cli_strips_wikipedia_edit_links() {
    let html = br#"<!DOCTYPE html>
<html>
<head><title>Wiki Test</title></head>
<body>
  <article>
    <h2>History</h2>
    <a href="https://en.wikipedia.org/w/index.php?title=Foo&amp;action=edit&amp;section=1">[edit]</a>
    <p>This is the history section with enough text for readability to extract it.
    We need multiple sentences to ensure the content passes the extraction threshold.</p>
    <h2>Geography</h2>
    <a href="https://en.wikipedia.org/w/index.php?title=Foo&amp;action=edit&amp;section=2">[edit]</a>
    <p>This is the geography section with enough text for readability to extract it.
    Again we add extra content to make sure readability is happy with the length.</p>
  </article>
</body>
</html>"#;

    let mut server = Server::new();
    let mock = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(html.as_ref())
        .create();

    let output = mdget()
        .args(["-t", "5", "--raw", &server.url()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("[edit]"),
        "edit links should be stripped, got: {stdout}"
    );
    assert!(
        stdout.contains("History") || stdout.contains("Geography"),
        "headings should remain"
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 16. cli_cleans_degenerate_table
// ---------------------------------------------------------------------------
#[test]
fn cli_cleans_degenerate_table() {
    let html = br"<!DOCTYPE html>
<html>
<head><title>Table Test</title></head>
<body>
  <article>
    <p>Content before table to ensure readability extraction works properly.
    We need enough text here to pass the content threshold.</p>
    <table>
      <tr><th>Property</th><th>Value</th></tr>
      <tr><td>Name</td><td>Alice</td></tr>
      <tr><td></td><td></td></tr>
      <tr><td></td><td></td></tr>
      <tr><td></td><td></td></tr>
      <tr><td>Country</td><td>USA</td></tr>
    </table>
    <p>Content after table to ensure there is enough text for extraction.
    Additional sentences help the readability algorithm identify this as content.</p>
  </article>
</body>
</html>";

    let mut server = Server::new();
    let mock = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(html.as_ref())
        .create();

    let output = mdget()
        .args(["-t", "5", "--raw", &server.url()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("**Name:** Alice") || stdout.contains("**Country:** USA"),
        "degenerate table should be converted to key-value format, got: {stdout}"
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 17. cli_multiple_urls
// ---------------------------------------------------------------------------
#[test]
fn cli_multiple_urls() {
    let html1 = br"<!DOCTYPE html>
<html><head><title>First Page</title></head>
<body><article>
  <h1>First Page</h1>
  <p>Content of the first page with enough text for readability extraction.</p>
  <p>Second paragraph to ensure the article passes the extraction threshold.</p>
</article></body></html>";

    let html2 = br"<!DOCTYPE html>
<html><head><title>Second Page</title></head>
<body><article>
  <h1>Second Page</h1>
  <p>Content of the second page with enough text for readability extraction.</p>
  <p>Second paragraph to ensure the article passes the extraction threshold.</p>
</article></body></html>";

    let mut server = Server::new();
    let mock1 = server
        .mock("GET", "/first")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(html1.as_ref())
        .create();
    let mock2 = server
        .mock("GET", "/second")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(html2.as_ref())
        .create();

    let url1 = format!("{}/first", server.url());
    let url2 = format!("{}/second", server.url());

    let output = mdget()
        .args(["-t", "5", "--raw", &url1, &url2])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Content of the first page"),
        "output should contain first page content, got: {stdout}"
    );
    assert!(
        stdout.contains("Content of the second page"),
        "output should contain second page content, got: {stdout}"
    );
    assert!(
        stdout.contains("---"),
        "output should contain separator between pages, got: {stdout}"
    );

    // First page content should appear before the separator
    let sep_pos = stdout.find("---").unwrap();
    let first_pos = stdout.find("Content of the first page").unwrap();
    let second_pos = stdout.find("Content of the second page").unwrap();
    assert!(
        first_pos < sep_pos,
        "first page content should appear before separator"
    );
    assert!(
        second_pos > sep_pos,
        "second page content should appear after separator"
    );

    mock1.assert();
    mock2.assert();
}

// ---------------------------------------------------------------------------
// 18. cli_local_html_file
// ---------------------------------------------------------------------------
#[test]
fn cli_local_html_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("page.html");
    std::fs::write(&file_path, TEST_HTML).unwrap();

    mdget()
        .args(["--raw", file_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("paragraph with enough content"));
}

// ---------------------------------------------------------------------------
// 19. cli_file_uri
// ---------------------------------------------------------------------------
#[test]
fn cli_file_uri() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("page.html");
    std::fs::write(&file_path, TEST_HTML).unwrap();

    let file_uri = url::Url::from_file_path(&file_path).unwrap().to_string();

    mdget()
        .args(["--raw", &file_uri])
        .assert()
        .success()
        .stdout(predicate::str::contains("paragraph with enough content"));
}

// ---------------------------------------------------------------------------
// 20. cli_input_file
// ---------------------------------------------------------------------------
#[test]
fn cli_input_file() {
    let html_a = br"<!DOCTYPE html>
<html><head><title>Page A</title></head>
<body><article>
  <h1>Page A</h1>
  <p>Content of page A with enough text for readability extraction to succeed.</p>
  <p>Extra paragraph to ensure the content threshold is met by readability.</p>
</article></body></html>";

    let html_b = br"<!DOCTYPE html>
<html><head><title>Page B</title></head>
<body><article>
  <h1>Page B</h1>
  <p>Content of page B with enough text for readability extraction to succeed.</p>
  <p>Extra paragraph to ensure the content threshold is met by readability.</p>
</article></body></html>";

    let mut server = Server::new();
    let mock_a = server
        .mock("GET", "/a")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(html_a.as_ref())
        .create();
    let mock_b = server
        .mock("GET", "/b")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(html_b.as_ref())
        .create();

    let url_a = format!("{}/a", server.url());
    let url_b = format!("{}/b", server.url());

    let dir = tempfile::tempdir().unwrap();
    let urls_file = dir.path().join("urls.txt");
    // Include a blank line and a comment — these should be ignored
    std::fs::write(
        &urls_file,
        format!("{url_a}\n\n# this is a comment\n{url_b}\n"),
    )
    .unwrap();

    let output = mdget()
        .args(["-t", "5", "--raw", "-i", urls_file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Content of page A"),
        "output should contain Page A content, got: {stdout}"
    );
    assert!(
        stdout.contains("Content of page B"),
        "output should contain Page B content, got: {stdout}"
    );

    mock_a.assert();
    mock_b.assert();
}

// ---------------------------------------------------------------------------
// 21. cli_mixed_inputs
// ---------------------------------------------------------------------------
#[test]
fn cli_mixed_inputs() {
    let html_local = br"<!DOCTYPE html>
<html><head><title>Local Page</title></head>
<body><article>
  <h1>Local Page</h1>
  <p>Content from a local HTML file with enough text for readability extraction.</p>
  <p>Second paragraph to ensure the article passes the content threshold.</p>
</article></body></html>";

    let html_remote = br"<!DOCTYPE html>
<html><head><title>Remote Page</title></head>
<body><article>
  <h1>Remote Page</h1>
  <p>Content from a remote URL with enough text for readability extraction.</p>
  <p>Second paragraph to ensure the article passes the content threshold.</p>
</article></body></html>";

    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("local.html");
    std::fs::write(&file_path, html_local).unwrap();

    let mut server = Server::new();
    let mock = server
        .mock("GET", "/remote")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(html_remote.as_ref())
        .create();

    let remote_url = format!("{}/remote", server.url());

    let output = mdget()
        .args(["-t", "5", "--raw", file_path.to_str().unwrap(), &remote_url])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Content from a local HTML file"),
        "output should contain local page content, got: {stdout}"
    );
    assert!(
        stdout.contains("Content from a remote URL"),
        "output should contain remote page content, got: {stdout}"
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 22. cli_batch_preserves_order
// ---------------------------------------------------------------------------
#[test]
fn cli_batch_preserves_order() {
    let html1 = br"<!DOCTYPE html>
<html><head><title>Alpha</title></head>
<body><article>
  <h1>Alpha Content</h1>
  <p>This is the alpha page content for ordering test verification.</p>
  <p>Second paragraph to ensure readability extraction passes the threshold.</p>
</article></body></html>";

    let html2 = br"<!DOCTYPE html>
<html><head><title>Beta</title></head>
<body><article>
  <h1>Beta Content</h1>
  <p>This is the beta page content for ordering test verification.</p>
  <p>Second paragraph to ensure readability extraction passes the threshold.</p>
</article></body></html>";

    let mut server = Server::new();
    let mock1 = server
        .mock("GET", "/alpha")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(html1.as_ref())
        .create();
    let mock2 = server
        .mock("GET", "/beta")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(html2.as_ref())
        .create();

    let url1 = format!("{}/alpha", server.url());
    let url2 = format!("{}/beta", server.url());

    // Pass alpha first, beta second — verify output order matches
    let output = mdget()
        .args(["-t", "5", "--raw", &url1, &url2])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let alpha_pos = stdout.find("alpha page content").unwrap_or(usize::MAX);
    let beta_pos = stdout.find("beta page content").unwrap_or(usize::MAX);
    let sep_pos = stdout.find("---").unwrap_or(usize::MAX);

    assert!(
        alpha_pos != usize::MAX,
        "alpha page content not found in output: {stdout}"
    );
    assert!(
        beta_pos != usize::MAX,
        "beta page content not found in output: {stdout}"
    );
    assert!(
        alpha_pos < sep_pos,
        "alpha content should appear before separator"
    );
    assert!(
        beta_pos > sep_pos,
        "beta content should appear after separator"
    );

    mock1.assert();
    mock2.assert();
}

// ---------------------------------------------------------------------------
// 23. cli_batch_per_input_error
// ---------------------------------------------------------------------------
#[test]
fn cli_batch_per_input_error() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/good")
        .with_status(200)
        .with_header("Content-Type", "text/plain")
        .with_body("good content")
        .create();

    let good_url = format!("{}/good", server.url());
    // Port 1 is privileged/reserved — connection should be refused immediately
    let bad_url = "http://127.0.0.1:1/nope";

    let output = mdget()
        .args(["-t", "5", &good_url, bad_url])
        .output()
        .unwrap();

    // Should exit 1 because one input failed
    assert!(
        !output.status.success(),
        "should exit non-zero when an input fails"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("good content"),
        "successful input output should appear, got stdout: {stdout}"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Error:"),
        "error for bad URL should be reported on stderr, got: {stderr}"
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 24. cli_output_flag_with_multiple_inputs_fails
// ---------------------------------------------------------------------------
#[test]
fn cli_output_flag_with_multiple_inputs_fails() {
    // We pass two URLs with -o; this should be rejected before any network call
    mdget()
        .args([
            "-o",
            "out.md",
            "http://127.0.0.1:1/nope1",
            "http://127.0.0.1:1/nope2",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot use -o"));
}

// ---------------------------------------------------------------------------
// 25. cli_auto_filename_batch
// ---------------------------------------------------------------------------
#[test]
fn cli_auto_filename_batch() {
    let html1 = br"<!DOCTYPE html>
<html><head><title>Auto File One</title></head>
<body><article>
  <h1>Auto File One</h1>
  <p>Content for auto filename test with enough text for readability.</p>
  <p>Second paragraph to ensure the content threshold is met.</p>
</article></body></html>";

    let html2 = br"<!DOCTYPE html>
<html><head><title>Auto File Two</title></head>
<body><article>
  <h1>Auto File Two</h1>
  <p>Content for auto filename test with enough text for readability.</p>
  <p>Second paragraph to ensure the content threshold is met.</p>
</article></body></html>";

    let mut server = Server::new();
    let mock1 = server
        .mock("GET", "/one")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(html1.as_ref())
        .create();
    let mock2 = server
        .mock("GET", "/two")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(html2.as_ref())
        .create();

    let url1 = format!("{}/one", server.url());
    let url2 = format!("{}/two", server.url());

    let dir = tempfile::tempdir().unwrap();

    let output = mdget()
        .args(["-t", "5", "--raw", "-O", &url1, &url2])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Two .md files should have been created in the temp dir
    let md_files: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .collect();

    assert_eq!(
        md_files.len(),
        2,
        "expected 2 .md files created, found: {:?}",
        md_files
            .iter()
            .map(std::fs::DirEntry::file_name)
            .collect::<Vec<_>>()
    );

    mock1.assert();
    mock2.assert();
}

// ---------------------------------------------------------------------------
// 26. cli_jobs_flag
// ---------------------------------------------------------------------------
#[test]
fn cli_jobs_flag() {
    let html1 = br"<!DOCTYPE html>
<html><head><title>Jobs Test One</title></head>
<body><article>
  <h1>Jobs Test One</h1>
  <p>Content for parallel jobs test with enough text for readability extraction.</p>
  <p>Second paragraph to ensure the content threshold is satisfied.</p>
</article></body></html>";

    let html2 = br"<!DOCTYPE html>
<html><head><title>Jobs Test Two</title></head>
<body><article>
  <h1>Jobs Test Two</h1>
  <p>Content for parallel jobs test with enough text for readability extraction.</p>
  <p>Second paragraph to ensure the content threshold is satisfied.</p>
</article></body></html>";

    let mut server = Server::new();
    let mock1 = server
        .mock("GET", "/j1")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(html1.as_ref())
        .create();
    let mock2 = server
        .mock("GET", "/j2")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(html2.as_ref())
        .create();

    let url1 = format!("{}/j1", server.url());
    let url2 = format!("{}/j2", server.url());

    let output = mdget()
        .args(["-t", "5", "--raw", "-j", "2", &url1, &url2])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "should succeed with -j 2; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Content for parallel jobs test"),
        "at least one page should appear in output: {stdout}"
    );

    mock1.assert();
    mock2.assert();
}

// ---------------------------------------------------------------------------
// 27. cli_local_plain_text_file
// ---------------------------------------------------------------------------
#[test]
fn cli_local_plain_text_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("notes.txt");
    std::fs::write(&file_path, "Hello from a local text file!").unwrap();

    mdget()
        .args([file_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello from a local text file!"));
}

// ---------------------------------------------------------------------------
// 28. cli_input_file_with_local_paths
// ---------------------------------------------------------------------------
#[test]
fn cli_input_file_with_local_paths() {
    let dir = tempfile::tempdir().unwrap();
    let html_path = dir.path().join("article.html");
    std::fs::write(&html_path, TEST_HTML).unwrap();

    let input_list = dir.path().join("inputs.txt");
    std::fs::write(&input_list, format!("{}\n", html_path.to_str().unwrap())).unwrap();

    mdget()
        .args(["--raw", "-i", input_list.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("paragraph with enough content"));
}
