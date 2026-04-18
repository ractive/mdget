use assert_cmd::Command;
use predicates::prelude::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

/// Spawn a one-shot HTTP server that returns the given status, content-type, and body.
/// Returns the URL to connect to. The server handles exactly one request then shuts down.
fn spawn_http_server(status: u16, content_type: &str, body: &[u8]) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://127.0.0.1:{}", addr.port());

    let response = format!(
        "HTTP/1.1 {status} OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let mut full_response = response.into_bytes();
    full_response.extend_from_slice(body);

    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            // Read the request (we don't care about the contents)
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(&full_response);
            let _ = stream.flush();
        }
    });

    url
}

#[test]
fn cli_prints_help() {
    Command::cargo_bin("mdget")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Fetch a web page"));
}

#[test]
fn cli_prints_version() {
    Command::cargo_bin("mdget")
        .unwrap()
        .arg("-V")
        .assert()
        .success()
        .stdout(predicate::str::contains("mdget"));
}

#[test]
fn cli_missing_url_exits_with_error() {
    Command::cargo_bin("mdget")
        .unwrap()
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing required argument"));
}

#[test]
fn cli_init_requires_claude_flag() {
    Command::cargo_bin("mdget")
        .unwrap()
        .args(["init"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--claude"));
}

#[test]
fn cli_init_deinit_project_roundtrip() {
    let dir = tempfile::tempdir().unwrap();

    // init installs skill and updates CLAUDE.md
    Command::cargo_bin("mdget")
        .unwrap()
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
    Command::cargo_bin("mdget")
        .unwrap()
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

#[test]
fn cli_init_idempotent() {
    let dir = tempfile::tempdir().unwrap();

    for _ in 0..2 {
        Command::cargo_bin("mdget")
            .unwrap()
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

#[test]
fn cli_deinit_idempotent() {
    let dir = tempfile::tempdir().unwrap();

    // deinit on a clean directory should not error
    Command::cargo_bin("mdget")
        .unwrap()
        .args(["deinit"])
        .current_dir(dir.path())
        .assert()
        .success();
}

#[test]
fn cli_handles_html_content_type() {
    let html = br"<!DOCTYPE html>
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
    let url = spawn_http_server(200, "text/html; charset=utf-8", html);

    Command::cargo_bin("mdget")
        .unwrap()
        .args(["-t", "5", "--raw", &url])
        .assert()
        .success()
        .stdout(predicate::str::contains("paragraph with enough content"));
}

#[test]
fn cli_handles_plain_text() {
    let body = b"Hello, plain world!";
    let url = spawn_http_server(200, "text/plain", body);

    Command::cargo_bin("mdget")
        .unwrap()
        .args(["-t", "5", &url])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello, plain world!"));
}

#[test]
fn cli_handles_json() {
    let body = b"{\"key\":\"value\"}";
    let url = spawn_http_server(200, "application/json", body);

    Command::cargo_bin("mdget")
        .unwrap()
        .args(["-t", "5", &url])
        .assert()
        .success()
        .stdout(predicate::str::contains("```json"))
        .stdout(predicate::str::contains("{\"key\":\"value\"}"));
}

#[test]
fn cli_rejects_binary_content() {
    // Minimal fake PNG bytes (just needs a non-empty body)
    let body = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
    let url = spawn_http_server(200, "image/png", body);

    Command::cargo_bin("mdget")
        .unwrap()
        .args(["-t", "5", &url])
        .assert()
        .failure()
        .stderr(predicate::str::contains("binary content"))
        .stderr(predicate::str::contains("image/png"));
}

#[test]
fn cli_quiet_suppresses_progress() {
    let html = br"<!DOCTYPE html>
<html>
<head><title>Quiet Test</title></head>
<body>
  <article>
    <h1>Quiet Test</h1>
    <p>Content should still appear even when progress messages are suppressed.</p>
  </article>
</body>
</html>";
    let url = spawn_http_server(200, "text/html", html);

    let output = Command::cargo_bin("mdget")
        .unwrap()
        .args(["-q", "-t", "5", "--raw", &url])
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
}

#[test]
fn cli_quiet_still_shows_errors() {
    Command::cargo_bin("mdget")
        .unwrap()
        .args(["-q", "ftp://example.com"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported"));
}

#[test]
fn cli_output_ends_with_newline() {
    let html = br"<!DOCTYPE html>
<html>
<head><title>Newline Test</title></head>
<body>
  <article>
    <h1>Newline Test</h1>
    <p>Verifying that stdout always ends with a newline for pipe-friendliness.</p>
  </article>
</body>
</html>";
    let url = spawn_http_server(200, "text/html", html);

    let output = Command::cargo_bin("mdget")
        .unwrap()
        .args(["-t", "5", "--raw", &url])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stdout.ends_with(b"\n"),
        "stdout must end with a newline"
    );
}
