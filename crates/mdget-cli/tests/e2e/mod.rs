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

// ===========================================================================
// Output control flags (iteration 4)
// ===========================================================================

const TEST_HTML_WITH_IMAGES: &[u8] = br#"<!DOCTYPE html>
<html lang="en">
<head>
  <title>Image Test Article</title>
  <meta name="author" content="Jane Author">
  <meta name="description" content="An article with images for testing">
  <meta property="article:published_time" content="2026-04-15">
  <meta property="og:site_name" content="Test News">
</head>
<body>
  <article>
    <h1>Image Test Article</h1>
    <p>First paragraph with enough content to pass readability. We need quite a bit
    of text here because readability algorithms typically require a minimum amount of
    content before they consider a page readable.</p>
    <img src="https://example.com/photo.jpg" alt="A photo">
    <p>Second paragraph also needs content. The readability algorithm needs sufficient
    text to identify the main content area of the page without which it may fail.</p>
    <img src="https://example.com/chart.png" alt="A chart">
    <p>Third paragraph for good measure to ensure extraction works properly and
    we get enough content for meaningful word count testing across all scenarios.</p>
  </article>
</body>
</html>"#;

// ---------------------------------------------------------------------------
// 29. cli_include_metadata
// ---------------------------------------------------------------------------
#[test]
fn cli_include_metadata() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/article")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML_WITH_IMAGES)
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--include-metadata",
            &format!("{}/article", server.url()),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Frontmatter should be present
    assert!(
        stdout.starts_with("---\n"),
        "output should start with YAML frontmatter: {stdout}"
    );
    assert!(
        stdout.contains("title:"),
        "frontmatter should contain title"
    );
    assert!(
        stdout.contains("source:"),
        "frontmatter should contain source"
    );
    assert!(
        stdout.contains("fetched:"),
        "frontmatter should contain fetched"
    );
    assert!(
        stdout.contains("word_count:"),
        "frontmatter should contain word_count"
    );
    // Body should also be present after the frontmatter
    assert!(
        stdout.contains("First paragraph"),
        "body should appear after frontmatter"
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 30. cli_metadata_only
// ---------------------------------------------------------------------------
#[test]
fn cli_metadata_only() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/article")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML_WITH_IMAGES)
        .create();

    let output = mdget()
        .args(["-t", "5", "-m", &format!("{}/article", server.url())])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Frontmatter should be present
    assert!(
        stdout.starts_with("---\n"),
        "output should start with YAML frontmatter"
    );
    assert!(stdout.contains("title:"));
    assert!(stdout.contains("word_count:"));
    // Body should NOT be present
    assert!(
        !stdout.contains("First paragraph"),
        "body should not appear in metadata-only mode"
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 31. cli_no_images
// ---------------------------------------------------------------------------
#[test]
fn cli_no_images() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/article")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML_WITH_IMAGES)
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--no-images",
            &format!("{}/article", server.url()),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Images should be stripped
    assert!(
        !stdout.contains("!["),
        "image references should be stripped: {stdout}"
    );
    // Regular content should remain
    assert!(stdout.contains("paragraph"), "text content should remain");

    mock.assert();
}

// ---------------------------------------------------------------------------
// 32. cli_max_length
// ---------------------------------------------------------------------------
#[test]
fn cli_max_length() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/article")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML)
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--max-length",
            "50",
            &format!("{}/article", server.url()),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[Truncated]"),
        "truncated output should contain [Truncated] marker"
    );
    // Output should be reasonably short
    assert!(
        stdout.len() <= 80,
        "output should be near max-length (got {} chars)",
        stdout.len()
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 33. cli_max_length_no_truncation_when_short
// ---------------------------------------------------------------------------
#[test]
fn cli_max_length_no_truncation_when_short() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/article")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML)
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--max-length",
            "100000",
            &format!("{}/article", server.url()),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("[Truncated]"),
        "should not truncate when content fits within max-length"
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 34. cli_include_metadata_with_no_images
// ---------------------------------------------------------------------------
#[test]
fn cli_include_metadata_with_no_images() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/article")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML_WITH_IMAGES)
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--include-metadata",
            "--no-images",
            &format!("{}/article", server.url()),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Has metadata
    assert!(stdout.starts_with("---\n"));
    assert!(stdout.contains("title:"));
    // No images
    assert!(
        !stdout.contains("!["),
        "images should be stripped even with metadata"
    );
    // Has body
    assert!(stdout.contains("paragraph"));

    mock.assert();
}

// ---------------------------------------------------------------------------
// 35. cli_include_metadata_no_images_max_length
// ---------------------------------------------------------------------------
#[test]
fn cli_include_metadata_no_images_max_length() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/article")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML_WITH_IMAGES)
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--include-metadata",
            "--no-images",
            "--max-length",
            "100",
            &format!("{}/article", server.url()),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Has metadata frontmatter
    assert!(stdout.starts_with("---\n"), "should have frontmatter");
    // Has truncation marker
    assert!(
        stdout.contains("[Truncated]"),
        "should be truncated: {stdout}"
    );
    // No images
    assert!(!stdout.contains("!["), "images should be stripped");

    mock.assert();
}

// ---------------------------------------------------------------------------
// 36. cli_metadata_only_with_no_images
// ---------------------------------------------------------------------------
#[test]
fn cli_metadata_only_with_no_images() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/article")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML_WITH_IMAGES)
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "-m",
            "--no-images",
            &format!("{}/article", server.url()),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should have frontmatter
    assert!(stdout.starts_with("---\n"));
    assert!(stdout.contains("word_count:"));
    // Should not have body
    assert!(!stdout.contains("paragraph"));

    mock.assert();
}

// ---------------------------------------------------------------------------
// 37. cli_metadata_only_batch
// ---------------------------------------------------------------------------
#[test]
fn cli_metadata_only_batch() {
    let mut server = Server::new();
    let mock1 = server
        .mock("GET", "/page1")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML)
        .create();
    let mock2 = server
        .mock("GET", "/page2")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML_WITH_IMAGES)
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "-m",
            &format!("{}/page1", server.url()),
            &format!("{}/page2", server.url()),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should have multiple frontmatter blocks separated by ---
    let frontmatter_count = stdout.matches("title:").count();
    assert!(
        frontmatter_count >= 2,
        "should have metadata for both pages, got {frontmatter_count}"
    );

    mock1.assert();
    mock2.assert();
}

// ---------------------------------------------------------------------------
// 38. cli_metadata_with_local_file
// ---------------------------------------------------------------------------
#[test]
fn cli_metadata_with_local_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("page.html");
    std::fs::write(&file_path, TEST_HTML).unwrap();

    let output = mdget()
        .args(["--include-metadata", file_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("---\n"), "should have frontmatter");
    assert!(stdout.contains("title:"));
    assert!(stdout.contains("source: \"file://"));
    assert!(stdout.contains("word_count:"));
    assert!(
        stdout.contains("paragraph"),
        "body should follow frontmatter"
    );
}

// ---------------------------------------------------------------------------
// 39. cli_retries_on_5xx_exhausted
// ---------------------------------------------------------------------------
#[test]
fn cli_retries_on_5xx_exhausted() {
    let mut server = Server::new();
    // Server always returns 500; with --retries 1 we expect 2 total requests.
    let mock = server
        .mock("GET", "/fail")
        .with_status(500)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(b"Internal Server Error")
        .expect(2)
        .create();

    let url = format!("{}/fail", server.url());
    let output = mdget()
        .args(["--retries", "1", "-q", &url])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "should fail when all retries exhausted"
    );
    mock.assert();
}

// ---------------------------------------------------------------------------
// 40. cli_no_retry_on_4xx
// ---------------------------------------------------------------------------
#[test]
fn cli_no_retry_on_4xx() {
    let mut server = Server::new();
    // 4xx responses should not be retried; expect exactly 1 request.
    let mock = server
        .mock("GET", "/notfound")
        .with_status(404)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(b"Not Found")
        .expect(1)
        .create();

    let url = format!("{}/notfound", server.url());
    let output = mdget()
        .args(["--retries", "2", "-q", &url])
        .output()
        .unwrap();

    assert!(!output.status.success(), "should fail on 404");
    mock.assert();
}

// ---------------------------------------------------------------------------
// 41. cli_reports_redirect_chain
// ---------------------------------------------------------------------------
#[test]
fn cli_reports_redirect_chain() {
    let mut server = Server::new();

    let end_url = format!("{}/end", server.url());
    let middle_url = format!("{}/middle", server.url());

    let mock_start = server
        .mock("GET", "/start")
        .with_status(301)
        .with_header("Location", &middle_url)
        .with_body(b"")
        .create();
    let mock_middle = server
        .mock("GET", "/middle")
        .with_status(301)
        .with_header("Location", &end_url)
        .with_body(b"")
        .create();
    let mock_end = server
        .mock("GET", "/end")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML)
        .create();

    let url = format!("{}/start", server.url());
    let output = mdget().args(["--retries", "0", &url]).output().unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains('\u{2192}'),
        "stderr should contain redirect arrow →, got: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "stdout should have article content");

    mock_start.assert();
    mock_middle.assert();
    mock_end.assert();
}

// ---------------------------------------------------------------------------
// 42. cli_follows_meta_refresh
// ---------------------------------------------------------------------------
#[test]
fn cli_follows_meta_refresh() {
    let mut server = Server::new();

    let target_url = format!("{}/target", server.url());
    let refresh_html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
  <meta http-equiv="refresh" content="0; url={target_url}">
  <title>Redirecting</title>
</head>
<body><p>Redirecting...</p></body>
</html>"#
    );

    let mock_refresh = server
        .mock("GET", "/refresh")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(refresh_html.as_bytes())
        .create();
    let mock_target = server
        .mock("GET", "/target")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML)
        .create();

    let url = format!("{}/refresh", server.url());
    let output = mdget().args(["--retries", "0", &url]).output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("paragraph"),
        "stdout should contain article content, got: {stdout}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("meta refresh"),
        "stderr should mention meta refresh, got: {stderr}"
    );

    mock_refresh.assert();
    mock_target.assert();
}

// ---------------------------------------------------------------------------
// 43. cli_pdf_content_type_error
// ---------------------------------------------------------------------------
#[test]
fn cli_pdf_content_type_error() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/doc.pdf")
        .with_status(200)
        .with_header("Content-Type", "application/pdf")
        .with_body(b"%PDF-1.4 fake pdf bytes")
        .create();

    let url = format!("{}/doc.pdf", server.url());
    let output = mdget()
        .args(["--retries", "0", "-q", &url])
        .output()
        .unwrap();

    assert!(!output.status.success(), "should fail for PDF content");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_uppercase().contains("PDF"),
        "stderr should mention PDF, got: {stderr}"
    );
    assert!(
        stderr.contains("pdftotext"),
        "stderr should mention pdftotext, got: {stderr}"
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 44. cli_retries_flag_in_help
// ---------------------------------------------------------------------------
#[test]
fn cli_retries_flag_in_help() {
    let output = mdget().args(["--help"]).output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--retries"),
        "help output should document --retries flag"
    );
}

// ---------------------------------------------------------------------------
// 45. cli_redirect_chain_with_meta_refresh
// ---------------------------------------------------------------------------
#[test]
fn cli_redirect_chain_with_meta_refresh() {
    let mut server = Server::new();

    let final_url = format!("{}/final", server.url());
    let redir_url = format!("{}/redir", server.url());

    let refresh_html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
  <meta http-equiv="refresh" content="0; url={final_url}">
  <title>Refreshing</title>
</head>
<body><p>Please wait...</p></body>
</html>"#
    );

    let mock_start = server
        .mock("GET", "/start")
        .with_status(301)
        .with_header("Location", &redir_url)
        .with_body(b"")
        .create();
    let mock_redir = server
        .mock("GET", "/redir")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(refresh_html.as_bytes())
        .create();
    let mock_final = server
        .mock("GET", "/final")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML)
        .create();

    let url = format!("{}/start", server.url());
    let output = mdget().args(["--retries", "0", &url]).output().unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains('\u{2192}'),
        "stderr should contain redirect arrow → for HTTP redirect, got: {stderr}"
    );
    assert!(
        stderr.to_lowercase().contains("meta refresh"),
        "stderr should mention meta refresh, got: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "stdout should have article content");

    mock_start.assert();
    mock_redir.assert();
    mock_final.assert();
}

// ---------------------------------------------------------------------------
// 46. cli_retries_zero_no_retry
// ---------------------------------------------------------------------------
#[test]
fn cli_retries_zero_no_retry() {
    let mut server = Server::new();
    // With --retries 0 there should be exactly 1 request even on 500.
    let mock = server
        .mock("GET", "/always500")
        .with_status(500)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(b"Internal Server Error")
        .expect(1)
        .create();

    let url = format!("{}/always500", server.url());
    let output = mdget()
        .args(["--retries", "0", "-q", &url])
        .output()
        .unwrap();

    assert!(!output.status.success(), "should fail on 500");
    mock.assert();
}

// ---------------------------------------------------------------------------
// 47. cli_help_contains_cookbook
// ---------------------------------------------------------------------------
#[test]
fn cli_help_contains_cookbook() {
    mdget()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("COOKBOOK:"))
        .stdout(predicate::str::contains("BEHAVIOR NOTES:"))
        .stdout(predicate::str::contains("AGENT TIPS:"));
}

// ===========================================================================
// Crawl subcommand tests (iteration 7)
// ===========================================================================

/// Build a self-contained HTML page suitable for readability extraction.
/// `links` are rendered as <a href> elements in a <nav> block.
fn crawl_page(title: &str, body: &str, links: &[&str]) -> String {
    let link_html: String = links.iter().fold(String::new(), |mut acc, l| {
        use std::fmt::Write as _;
        let _ = write!(acc, r#"<a href="{l}">link</a> "#);
        acc
    });
    format!(
        r"<!DOCTYPE html>
<html><head><title>{title}</title></head>
<body><article>
  <h1>{title}</h1>
  <p>{body} This paragraph has enough content for readability to work.
  Extra sentences help ensure the extraction threshold is met by the algorithm.</p>
  <nav>{link_html}</nav>
</article></body></html>"
    )
}

// ---------------------------------------------------------------------------
// 48. crawl_basic
// ---------------------------------------------------------------------------
#[test]
fn crawl_basic() {
    let mut server = Server::new();

    let page2_url = format!("{}/page2", server.url());
    let root_html = crawl_page("Root Page", "Root body text.", &[&page2_url]);
    let page2_html = crawl_page("Page Two", "Page two body text.", &[]);

    let mock_root = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(root_html)
        .create();
    let mock_page2 = server
        .mock("GET", "/page2")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(page2_html)
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--retries",
            "0",
            "crawl",
            "--delay",
            "0",
            &server.url(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "crawl should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Both pages should have frontmatter with source:
    let source_count = stdout.matches("source:").count();
    assert!(
        source_count >= 2,
        "expected frontmatter for both pages (2 source: lines), got {source_count}: {stdout}"
    );

    mock_root.assert();
    mock_page2.assert();
}

// ---------------------------------------------------------------------------
// 49. crawl_depth_zero
// ---------------------------------------------------------------------------
#[test]
fn crawl_depth_zero() {
    let mut server = Server::new();

    let page2_url = format!("{}/page2", server.url());
    let root_html = crawl_page("Root Page", "Root body text.", &[&page2_url]);

    let mock_root = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(root_html)
        .create();
    // page2 should NOT be requested when depth=0
    let mock_page2 = server
        .mock("GET", "/page2")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(crawl_page("Page Two", "Page two.", &[]))
        .expect(0)
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--retries",
            "0",
            "crawl",
            "--delay",
            "0",
            "--depth",
            "0",
            &server.url(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "crawl should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let source_count = stdout.matches("source:").count();
    assert_eq!(
        source_count, 1,
        "depth=0 should fetch only the start page, got {source_count} source: lines: {stdout}"
    );

    mock_root.assert();
    mock_page2.assert();
}

// ---------------------------------------------------------------------------
// 50. crawl_max_pages
// ---------------------------------------------------------------------------
#[test]
fn crawl_max_pages() {
    let mut server = Server::new();

    // Build a chain: root → /p1 → /p2 → /p3 → /p4
    let p4_url = format!("{}/p4", server.url());
    let p3_url = format!("{}/p3", server.url());
    let p2_url = format!("{}/p2", server.url());
    let p1_url = format!("{}/p1", server.url());

    let mock_root = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(crawl_page("Root", "Root body.", &[&p1_url]))
        .create();
    let mock_p1 = server
        .mock("GET", "/p1")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(crawl_page("P1", "P1 body.", &[&p2_url]))
        .create();
    // With --max-pages 2 and --depth 2, only root + p1 should be fetched.
    // p2 and beyond should not be requested.
    let mock_p2 = server
        .mock("GET", "/p2")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(crawl_page("P2", "P2 body.", &[&p3_url]))
        .expect(0)
        .create();
    let mock_p3 = server
        .mock("GET", "/p3")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(crawl_page("P3", "P3 body.", &[&p4_url]))
        .expect(0)
        .create();
    let mock_p4 = server
        .mock("GET", "/p4")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(crawl_page("P4", "P4 body.", &[]))
        .expect(0)
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--retries",
            "0",
            "crawl",
            "--delay",
            "0",
            "--max-pages",
            "2",
            "--depth",
            "2",
            &server.url(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "crawl should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let source_count = stdout.matches("source:").count();
    assert_eq!(
        source_count, 2,
        "expected exactly 2 pages with --max-pages 2, got {source_count}: {stdout}"
    );

    mock_root.assert();
    mock_p1.assert();
    mock_p2.assert();
    mock_p3.assert();
    mock_p4.assert();
}

// ---------------------------------------------------------------------------
// 51. crawl_same_host_only
// ---------------------------------------------------------------------------
#[test]
fn crawl_same_host_only() {
    let mut server = Server::new();

    // Link to a real external domain that should be filtered out.
    // We use https://example.com which has a different host than 127.0.0.1,
    // so the crawl engine (which compares host_str()) will skip it.
    let external_url = "https://example.com/external-page";
    let same_host_url = format!("{}/internal", server.url());

    let root_html = crawl_page("Root", "Root body.", &[external_url, &same_host_url]);
    let internal_html = crawl_page("Internal", "Internal body.", &[]);

    let mock_root = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(root_html)
        .create();
    let mock_internal = server
        .mock("GET", "/internal")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(internal_html)
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--retries",
            "0",
            "crawl",
            "--delay",
            "0",
            &server.url(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "crawl should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Only same-host pages should appear as fetched sources.
    // example.com should not appear as a `source:` value (it may appear in link text).
    let source_lines: Vec<&str> = stdout
        .lines()
        .filter(|l| l.starts_with("source:"))
        .collect();
    assert!(
        source_lines.iter().all(|l| !l.contains("example.com")),
        "external page should not have been fetched: {source_lines:?}"
    );
    // Both same-host pages should have been fetched (root + internal).
    assert_eq!(
        source_lines.len(),
        2,
        "expected exactly 2 same-host pages (root + internal), got: {source_lines:?}"
    );

    mock_root.assert();
    mock_internal.assert();
}

// ---------------------------------------------------------------------------
// 52. crawl_output_dir
// ---------------------------------------------------------------------------
#[test]
fn crawl_output_dir() {
    let mut server = Server::new();

    let page2_url = format!("{}/page2", server.url());
    let root_html = crawl_page("Root Page", "Root body text.", &[&page2_url]);
    let page2_html = crawl_page("Page Two", "Page two body.", &[]);

    let mock_root = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(root_html)
        .create();
    let mock_page2 = server
        .mock("GET", "/page2")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(page2_html)
        .create();

    let dir = tempfile::tempdir().unwrap();
    let output_dir = dir.path().to_str().unwrap();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--retries",
            "0",
            "crawl",
            "--delay",
            "0",
            "--output-dir",
            output_dir,
            &server.url(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "crawl should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Should have created index.md (for root /) and page2.md
    let index_file = dir.path().join("index.md");
    let page2_file = dir.path().join("page2.md");

    assert!(
        index_file.exists(),
        "index.md should exist in output dir: {:?}",
        dir.path()
    );
    assert!(
        page2_file.exists(),
        "page2.md should exist in output dir: {:?}",
        dir.path()
    );

    // Files should contain frontmatter
    let index_content = std::fs::read_to_string(&index_file).unwrap();
    assert!(
        index_content.starts_with("---\n"),
        "index.md should have frontmatter"
    );
    let page2_content = std::fs::read_to_string(&page2_file).unwrap();
    assert!(
        page2_content.starts_with("---\n"),
        "page2.md should have frontmatter"
    );

    mock_root.assert();
    mock_page2.assert();
}

// ---------------------------------------------------------------------------
// 53. crawl_auto_filename
// ---------------------------------------------------------------------------
#[test]
fn crawl_auto_filename() {
    let mut server = Server::new();

    let page2_url = format!("{}/page2", server.url());
    let root_html = crawl_page("Root Auto Page", "Root body text.", &[&page2_url]);
    let page2_html = crawl_page("Second Auto Page", "Page two body.", &[]);

    let mock_root = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(root_html)
        .create();
    let mock_page2 = server
        .mock("GET", "/page2")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(page2_html)
        .create();

    let dir = tempfile::tempdir().unwrap();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--retries",
            "0",
            "crawl",
            "--delay",
            "0",
            "-O",
            &server.url(),
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "crawl should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Two .md files should have been created
    let md_files: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .collect();

    assert_eq!(
        md_files.len(),
        2,
        "expected 2 .md files, found: {:?}",
        md_files
            .iter()
            .map(std::fs::DirEntry::file_name)
            .collect::<Vec<_>>()
    );

    mock_root.assert();
    mock_page2.assert();
}

// ---------------------------------------------------------------------------
// 54. crawl_stdout_has_frontmatter
// ---------------------------------------------------------------------------
#[test]
fn crawl_stdout_has_frontmatter() {
    let mut server = Server::new();

    let mock = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(crawl_page("FM Test", "Frontmatter test content.", &[]))
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--retries",
            "0",
            "crawl",
            "--delay",
            "0",
            "--depth",
            "0",
            &server.url(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "crawl should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("---\n"),
        "crawl stdout should start with YAML frontmatter: {stdout}"
    );
    assert!(
        stdout.contains("source:"),
        "crawl stdout should contain source: field: {stdout}"
    );
    assert!(
        stdout.contains("title:"),
        "crawl stdout should contain title: field: {stdout}"
    );
    // Should also have the closing --- of the frontmatter block
    assert!(
        stdout.matches("---").count() >= 2,
        "frontmatter should have both opening and closing --- fences: {stdout}"
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 55. crawl_quiet_suppresses_progress
// ---------------------------------------------------------------------------
#[test]
fn crawl_quiet_suppresses_progress() {
    let mut server = Server::new();

    let mock = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(crawl_page("Quiet Test", "Quiet mode test content.", &[]))
        .create();

    let output = mdget()
        .args([
            "-q",
            "-t",
            "5",
            "--retries",
            "0",
            "crawl",
            "--delay",
            "0",
            "--depth",
            "0",
            &server.url(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "crawl should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.is_empty(),
        "quiet mode should produce no stderr output, got: {stderr}"
    );

    // stdout should still have content
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.is_empty(),
        "stdout should have content even in quiet mode"
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 56. crawl_follow_external
// ---------------------------------------------------------------------------
// The previous version of this test used two local mockito servers that both
// bind to 127.0.0.1. Because host_str() doesn't include the port, both are
// treated as same-host by the crawler, making the test pass even without
// --follow-external. This rewrite uses a real external hostname to properly
// exercise the flag.
#[test]
fn crawl_follow_external() {
    let mut server1 = Server::new();

    // Link to an external host — unreachable in tests, but that's fine:
    // we're testing whether the crawler *attempts* it with --follow-external.
    let root_html = crawl_page("Root", "Root body.", &["https://external.example.com/page"]);

    // WITHOUT --follow-external: root is served once; external link is skipped.
    let mock_root_no_follow = server1
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(root_html.clone())
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--retries",
            "0",
            "crawl",
            "--delay",
            "0",
            &server1.url(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "crawl without --follow-external should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let source_count = stdout.matches("source:").count();
    assert_eq!(
        source_count, 1,
        "without --follow-external, only root should be crawled (got {source_count})"
    );
    mock_root_no_follow.assert();

    // WITH --follow-external: crawler attempts the external URL.
    // The fetch will fail (external.example.com is not a mock), but the root
    // still succeeds and the external URL should appear in stderr progress output.
    let mock_root_follow = server1
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(root_html)
        .create();

    let output2 = mdget()
        .args([
            "-t",
            "2",
            "--retries",
            "0",
            "crawl",
            "--delay",
            "0",
            "--follow-external",
            &server1.url(),
        ])
        .output()
        .unwrap();

    assert!(
        output2.status.success(),
        "crawl with --follow-external should succeed even if external fetch fails; stderr: {}",
        String::from_utf8_lossy(&output2.stderr)
    );
    let stderr2 = String::from_utf8_lossy(&output2.stderr);
    assert!(
        stderr2.contains("external.example.com"),
        "with --follow-external, the external URL should appear in progress output; stderr: {stderr2}"
    );
    mock_root_follow.assert();
}

// ===========================================================================
// robots.txt and sitemap.xml tests (iteration 7b)
// ===========================================================================

// ---------------------------------------------------------------------------
// 57. robots_txt_blocks_url
// ---------------------------------------------------------------------------
#[test]
fn robots_txt_blocks_url() {
    let mut server = Server::new();

    let public_url = format!("{}/public/", server.url());
    let private_url = format!("{}/private/", server.url());

    let robots_body = "User-agent: *\nDisallow: /private/\n";
    let root_html = crawl_page("Root", "Root body.", &[&public_url, &private_url]);
    let public_html = crawl_page("Public Page", "Public page content.", &[]);
    let private_html = crawl_page("Private Page", "Private page content.", &[]);

    let mock_robots = server
        .mock("GET", "/robots.txt")
        .with_status(200)
        .with_header("Content-Type", "text/plain")
        .with_body(robots_body)
        .create();
    let mock_root = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(root_html)
        .create();
    let mock_public = server
        .mock("GET", "/public/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(public_html)
        .create();
    // /private/ should NOT be fetched because robots.txt disallows it.
    let mock_private = server
        .mock("GET", "/private/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(private_html)
        .expect(0)
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--retries",
            "0",
            "crawl",
            "--delay",
            "0",
            "--depth",
            "1",
            &server.url(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "crawl should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Public page content"),
        "public page should be in output: {stdout}"
    );
    assert!(
        !stdout.contains("Private page content"),
        "private page should be blocked by robots.txt: {stdout}"
    );

    mock_robots.assert();
    mock_root.assert();
    mock_public.assert();
    mock_private.assert();
}

// ---------------------------------------------------------------------------
// 58. robots_txt_ignore_flag
// ---------------------------------------------------------------------------
#[test]
fn robots_txt_ignore_flag() {
    let mut server = Server::new();

    let public_url = format!("{}/public/", server.url());
    let private_url = format!("{}/private/", server.url());

    let robots_body = "User-agent: *\nDisallow: /private/\n";
    let root_html = crawl_page("Root", "Root body.", &[&public_url, &private_url]);
    let public_html = crawl_page("Public Page", "Public page content.", &[]);
    let private_html = crawl_page("Private Page", "Private page content.", &[]);

    let _mock_robots = server
        .mock("GET", "/robots.txt")
        .with_status(200)
        .with_header("Content-Type", "text/plain")
        .with_body(robots_body)
        .create();
    let mock_root = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(root_html)
        .create();
    let mock_public = server
        .mock("GET", "/public/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(public_html)
        .create();
    let mock_private = server
        .mock("GET", "/private/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(private_html)
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--retries",
            "0",
            "crawl",
            "--delay",
            "0",
            "--depth",
            "1",
            "--ignore-robots",
            &server.url(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "crawl should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Public page content"),
        "public page should be in output: {stdout}"
    );
    assert!(
        stdout.contains("Private page content"),
        "private page should be fetched with --ignore-robots: {stdout}"
    );

    mock_root.assert();
    mock_public.assert();
    mock_private.assert();
}

// ---------------------------------------------------------------------------
// 59. robots_txt_crawl_delay
// ---------------------------------------------------------------------------
#[test]
fn robots_txt_crawl_delay() {
    let mut server = Server::new();

    // robots.txt specifies a large crawl delay, but we only fetch depth 0
    // so the crawl completes quickly anyway.
    let robots_body = "User-agent: *\nCrawl-delay: 60\n";
    let root_html = crawl_page("Root", "Root body text for delay test.", &[]);

    let mock_robots = server
        .mock("GET", "/robots.txt")
        .with_status(200)
        .with_header("Content-Type", "text/plain")
        .with_body(robots_body)
        .create();
    let mock_root = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(root_html)
        .create();

    // With depth 0 there's only one page, so the crawl delay never fires.
    let output = mdget()
        .args([
            "-t",
            "5",
            "--retries",
            "0",
            "crawl",
            "--delay",
            "0",
            "--depth",
            "0",
            &server.url(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "crawl should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Root body text for delay test"),
        "root page should be fetched: {stdout}"
    );

    // The robots.txt crawl-delay should be reported in stderr progress output.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("crawl-delay"),
        "crawl-delay from robots.txt should be reported; stderr: {stderr}"
    );

    mock_robots.assert();
    mock_root.assert();
}

// ---------------------------------------------------------------------------
// 60. sitemap_discovery
// ---------------------------------------------------------------------------
#[test]
fn sitemap_discovery() {
    let mut server = Server::new();

    let page1_url = format!("{}/page1", server.url());
    let page2_url = format!("{}/page2", server.url());

    let sitemap_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>{page1_url}</loc></url>
  <url><loc>{page2_url}</loc></url>
</urlset>"#
    );

    let page1_html = crawl_page("Sitemap Page One", "Sitemap page one content.", &[]);
    let page2_html = crawl_page("Sitemap Page Two", "Sitemap page two content.", &[]);

    let mock_robots = server.mock("GET", "/robots.txt").with_status(404).create();
    let mock_sitemap = server
        .mock("GET", "/sitemap.xml")
        .with_status(200)
        .with_header("Content-Type", "application/xml")
        .with_body(sitemap_xml)
        .create();
    let mock_root = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(crawl_page("Root", "Root body.", &[]))
        .create();
    let mock_page1 = server
        .mock("GET", "/page1")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(page1_html)
        .create();
    let mock_page2 = server
        .mock("GET", "/page2")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(page2_html)
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--retries",
            "0",
            "crawl",
            "--delay",
            "0",
            "--depth",
            "0",
            "--sitemap",
            &server.url(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "crawl should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Sitemap page one content"),
        "sitemap page 1 should be in output: {stdout}"
    );
    assert!(
        stdout.contains("Sitemap page two content"),
        "sitemap page 2 should be in output: {stdout}"
    );

    mock_robots.assert();
    mock_sitemap.assert();
    mock_root.assert();
    mock_page1.assert();
    mock_page2.assert();
}

// ---------------------------------------------------------------------------
// 61. sitemap_index_nested
// ---------------------------------------------------------------------------
#[test]
fn sitemap_index_nested() {
    let mut server = Server::new();

    let child_sitemap_url = format!("{}/sitemap-pages.xml", server.url());
    let article_url = format!("{}/article", server.url());

    let sitemap_index_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <sitemap><loc>{child_sitemap_url}</loc></sitemap>
</sitemapindex>"#
    );

    let child_sitemap_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>{article_url}</loc></url>
</urlset>"#
    );

    let article_html = crawl_page(
        "Nested Sitemap Article",
        "Nested sitemap article content.",
        &[],
    );

    let mock_robots = server.mock("GET", "/robots.txt").with_status(404).create();
    let mock_sitemap = server
        .mock("GET", "/sitemap.xml")
        .with_status(200)
        .with_header("Content-Type", "application/xml")
        .with_body(sitemap_index_xml)
        .create();
    let mock_child_sitemap = server
        .mock("GET", "/sitemap-pages.xml")
        .with_status(200)
        .with_header("Content-Type", "application/xml")
        .with_body(child_sitemap_xml)
        .create();
    let mock_root = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(crawl_page("Root", "Root body.", &[]))
        .create();
    let mock_article = server
        .mock("GET", "/article")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(article_html)
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--retries",
            "0",
            "crawl",
            "--delay",
            "0",
            "--depth",
            "0",
            "--sitemap",
            &server.url(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "crawl should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Nested sitemap article content"),
        "article from nested sitemap should be in output: {stdout}"
    );

    mock_robots.assert();
    mock_sitemap.assert();
    mock_child_sitemap.assert();
    mock_root.assert();
    mock_article.assert();
}

// ---------------------------------------------------------------------------
// 62. crawl_single_segment_prefix
// ---------------------------------------------------------------------------
#[test]
fn crawl_single_segment_prefix() {
    let mut server = Server::new();

    // Start URL is /docs (single segment) — crawler should infer prefix "/docs/"
    // and only follow links under /docs/.
    let docs_page_url = format!("{}/docs/getting-started", server.url());
    let blog_url = format!("{}/blog/news", server.url());

    let docs_html = crawl_page(
        "Docs Root",
        "Documentation root page.",
        &[&docs_page_url, &blog_url],
    );
    let getting_started_html = crawl_page("Getting Started", "Getting started content.", &[]);

    let mock_docs = server
        .mock("GET", "/docs")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(docs_html)
        .create();
    let mock_getting_started = server
        .mock("GET", "/docs/getting-started")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(getting_started_html)
        .create();
    // /blog/news should NOT be fetched because it's outside /docs/ prefix
    let mock_blog = server
        .mock("GET", "/blog/news")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(crawl_page("Blog", "Blog content.", &[]))
        .expect(0)
        .create();

    let url = format!("{}/docs", server.url());
    let output = mdget()
        .args(["-t", "5", "--retries", "0", "crawl", "--delay", "0", &url])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "crawl should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Documentation root page"),
        "docs root should be in output: {stdout}"
    );
    assert!(
        stdout.contains("Getting started content"),
        "getting-started under /docs/ should be in output: {stdout}"
    );
    assert!(
        !stdout.contains("Blog content"),
        "blog page outside /docs/ should NOT be in output: {stdout}"
    );

    mock_docs.assert();
    mock_getting_started.assert();
    mock_blog.assert();
}

// ---------------------------------------------------------------------------
// 63. crawl_redirect_dedup
// ---------------------------------------------------------------------------
#[test]
fn crawl_redirect_dedup() {
    let mut server = Server::new();

    // /page redirects to /canonical — crawler should not produce duplicate results.
    let page_url = format!("{}/page", server.url());
    let canonical_url = format!("{}/canonical", server.url());

    let root_html = crawl_page("Root", "Root body.", &[&page_url, &canonical_url]);
    let canonical_html = crawl_page("Canonical Page", "Canonical page content.", &[]);

    let mock_root = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(root_html)
        .create();
    let mock_page = server
        .mock("GET", "/page")
        .with_status(301)
        .with_header("Location", &canonical_url)
        .with_body("")
        .create();
    // The canonical URL will be hit once via the redirect follow-through from /page,
    // and again when the BFS dequeues /canonical directly. The dedup prevents adding
    // duplicate results, but cannot prevent the redirect follow-through HTTP request.
    let mock_canonical = server
        .mock("GET", "/canonical")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(canonical_html)
        .expect_at_least(1)
        .create();

    let output = mdget()
        .args([
            "-t",
            "5",
            "--retries",
            "0",
            "crawl",
            "--delay",
            "0",
            &server.url(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "crawl should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // The canonical page should only be fetched once — check that the source URL
    // appears exactly once in the frontmatter. "Canonical page content" appears
    // twice per result (in excerpt + body), so we check the source field instead.
    let source_count = stdout.matches("/canonical\"").count();
    assert_eq!(
        source_count, 1,
        "canonical URL should appear as source exactly once (no duplicates from redirect), got {source_count}: {stdout}"
    );

    mock_root.assert();
    mock_page.assert();
    mock_canonical.assert();
}
