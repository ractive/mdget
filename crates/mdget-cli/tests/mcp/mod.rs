//! Integration tests for `mdget serve` — the MCP stdio server.
//!
//! Each test spawns the binary as a child process, performs the MCP handshake
//! over stdin/stdout, sends one or more tool calls, and asserts on the JSON
//! responses. HTTP responses are served by a `mockito::Server` running in the
//! same process.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;

use assert_cmd::cargo::cargo_bin;
use mockito::Server;
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Test HTML fixtures
// ---------------------------------------------------------------------------

const TEST_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <title>Test Article</title>
  <meta name="author" content="Test Author">
  <meta name="description" content="A test article for integration testing">
</head>
<body>
  <article>
    <h1>Test Article</h1>
    <p>This is the first paragraph with enough content for readability extraction to work
    correctly. The algorithm needs a minimum amount of text to consider this readable.</p>
    <p>This is the second paragraph with more content to ensure the article body is long
    enough to pass the readability threshold used by the extraction algorithm.</p>
    <p>A third paragraph adds even more content so that word count and excerpt fields
    are populated in the metadata frontmatter output.</p>
  </article>
</body>
</html>"#;

const TEST_HTML_WITH_IMAGES: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <title>Image Article</title>
  <meta name="author" content="Image Author">
</head>
<body>
  <article>
    <h1>Image Article</h1>
    <p>First paragraph with enough content to pass readability. We include quite a bit
    of text here because readability algorithms require a minimum amount of content
    before they consider a page readable and worth extracting.</p>
    <img src="https://example.com/photo.jpg" alt="A photo">
    <p>Second paragraph also needs content. The readability algorithm needs sufficient
    text to identify the main content area of the page without which it may fail to
    extract the article body we are testing here.</p>
    <img src="https://example.com/chart.png" alt="A chart">
    <p>Third paragraph for good measure to ensure extraction works properly and we
    get enough content for word count testing.</p>
  </article>
</body>
</html>"#;

// ---------------------------------------------------------------------------
// MCP client helper
// ---------------------------------------------------------------------------

/// Wraps a `mdget serve` child process and exposes send/receive helpers.
struct McpClient {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
    next_id: u64,
}

impl McpClient {
    /// Spawn `mdget serve`, perform the handshake, and return a ready client.
    fn new() -> Self {
        let mut child = Command::new(cargo_bin("mdget"))
            .arg("serve")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn mdget serve");

        let stdin = child.stdin.take().expect("no stdin");
        let stdout = child.stdout.take().expect("no stdout");

        // ChildStdout has no built-in read timeout; tests rely on CI-level
        // timeouts and short HTTP timeouts in each tool call invocation.
        let reader = BufReader::new(stdout);

        let mut client = McpClient {
            child,
            stdin,
            reader,
            next_id: 1,
        };

        // --- Handshake ---
        // 1. Send initialize
        let init_req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }
        });
        client.send_raw(&init_req);

        // 2. Read initialize response (unused by handshake path)
        let _init_resp = client.read_line_json();

        // 3. Send initialized notification (no response expected)
        let notif = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        client.send_raw(&notif);

        client.next_id = 2;
        client
    }

    /// Write a JSON value as a newline-delimited message to stdin.
    fn send_raw(&mut self, msg: &Value) {
        let line = serde_json::to_string(msg).expect("serialize");
        self.stdin
            .write_all(line.as_bytes())
            .expect("write to stdin");
        self.stdin.write_all(b"\n").expect("write newline");
        self.stdin.flush().expect("flush stdin");
    }

    /// Send a JSON-RPC request with an auto-incremented id and return the id.
    ///
    /// `params` is taken by value because it is moved into the `json!` macro
    /// which serialises it inline — clippy's `needless_pass_by_value` is a
    /// false-positive here due to macro expansion hiding the move.
    #[allow(clippy::needless_pass_by_value)]
    fn send_request(&mut self, method: &str, params: Value) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        self.send_raw(&req);
        id
    }

    /// Send a `tools/call` request and return (id, response).
    ///
    /// `arguments` is taken by value for the same reason as `send_request`.
    #[allow(clippy::needless_pass_by_value)]
    fn call_tool(&mut self, name: &str, arguments: Value) -> (u64, Value) {
        let id = self.send_request("tools/call", json!({"name": name, "arguments": arguments}));
        let resp = self.read_line_json();
        (id, resp)
    }

    /// Read one newline-delimited JSON line from stdout.
    fn read_line_json(&mut self) -> Value {
        let mut line = String::new();
        self.reader.read_line(&mut line).expect("read from stdout");
        serde_json::from_str(line.trim()).expect("parse JSON response")
    }

    /// Extract the text content from a successful tool call result.
    /// Returns the `text` field of the first content item.
    fn result_text(response: &Value) -> &str {
        response["result"]["content"][0]["text"]
            .as_str()
            .expect("expected text content in result")
    }

    /// Return true if the response has `isError: true`.
    fn is_error(response: &Value) -> bool {
        response["result"]["isError"].as_bool().unwrap_or(false)
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        // Kill the child to avoid zombie processes.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ---------------------------------------------------------------------------
// Helper: quick connection-refused check
// ---------------------------------------------------------------------------

/// Return true if port 1 on localhost refuses connections fast enough to use
/// in a short-timeout test. Skip the test if not, to avoid hanging in unusual
/// CI environments.
fn port_1_refused_quickly() -> bool {
    TcpStream::connect_timeout(&"127.0.0.1:1".parse().unwrap(), Duration::from_millis(200)).is_err()
}

// ---------------------------------------------------------------------------
// 1. mcp_initialize
// ---------------------------------------------------------------------------
#[test]
fn mcp_initialize() {
    let mut child = Command::new(cargo_bin("mdget"))
        .arg("serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");

    let mut stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);

    let req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }
    });
    stdin
        .write_all(format!("{}\n", serde_json::to_string(&req).unwrap()).as_bytes())
        .unwrap();
    stdin.flush().unwrap();

    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let resp: Value = serde_json::from_str(line.trim()).unwrap();

    assert_eq!(resp["id"], 1, "id should be 1");

    let server_info = &resp["result"]["serverInfo"];
    assert_eq!(
        server_info["name"].as_str().unwrap(),
        "mdget",
        "serverInfo.name should be 'mdget'"
    );

    assert!(
        resp["result"]["capabilities"]["tools"].is_object(),
        "capabilities.tools should be present"
    );

    let instructions = resp["result"]["instructions"].as_str().unwrap_or("");
    assert!(!instructions.is_empty(), "instructions should not be empty");
    assert!(
        instructions.contains("fetch"),
        "instructions should mention fetch: {instructions}"
    );

    let _ = child.kill();
    let _ = child.wait();
}

// ---------------------------------------------------------------------------
// 2. mcp_tools_list
// ---------------------------------------------------------------------------
#[test]
fn mcp_tools_list() {
    let mut client = McpClient::new();
    let id = client.send_request("tools/list", json!({}));
    let resp = client.read_line_json();

    assert_eq!(resp["id"], id);

    let tools = resp["result"]["tools"]
        .as_array()
        .expect("tools should be an array");

    assert_eq!(tools.len(), 3, "expected exactly 3 tools");

    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert!(
        names.contains(&"fetch_markdown"),
        "tools should include fetch_markdown: {names:?}"
    );
    assert!(
        names.contains(&"fetch_metadata"),
        "tools should include fetch_metadata: {names:?}"
    );
    assert!(
        names.contains(&"batch_fetch"),
        "tools should include batch_fetch: {names:?}"
    );

    // Verify fetch_markdown has 'url' as a required property
    let fetch_md = tools
        .iter()
        .find(|t| t["name"] == "fetch_markdown")
        .unwrap();
    let required = fetch_md["inputSchema"]["required"]
        .as_array()
        .expect("inputSchema.required should be an array");
    assert!(
        required.iter().any(|r| r == "url"),
        "fetch_markdown should require 'url'"
    );

    // Verify batch_fetch has 'urls' as a required property
    let batch = tools.iter().find(|t| t["name"] == "batch_fetch").unwrap();
    let batch_required = batch["inputSchema"]["required"]
        .as_array()
        .expect("batch_fetch inputSchema.required should be an array");
    assert!(
        batch_required.iter().any(|r| r == "urls"),
        "batch_fetch should require 'urls'"
    );
}

// ---------------------------------------------------------------------------
// 3. mcp_fetch_markdown
// ---------------------------------------------------------------------------
#[test]
fn mcp_fetch_markdown() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/article")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML)
        .create();

    let url = format!("{}/article", server.url());
    let mut client = McpClient::new();
    let (_id, resp) = client.call_tool(
        "fetch_markdown",
        json!({"url": url, "timeout": 10, "retries": 0}),
    );

    assert!(
        !McpClient::is_error(&resp),
        "fetch_markdown should succeed: {resp:?}"
    );

    let text = McpClient::result_text(&resp);
    assert!(
        text.contains("first paragraph"),
        "response should contain article content: {text}"
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 4. mcp_fetch_markdown_with_metadata
// ---------------------------------------------------------------------------
#[test]
fn mcp_fetch_markdown_with_metadata() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/article")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML)
        .create();

    let url = format!("{}/article", server.url());
    let mut client = McpClient::new();
    let (_id, resp) = client.call_tool(
        "fetch_markdown",
        json!({"url": url, "include_metadata": true, "timeout": 10, "retries": 0}),
    );

    assert!(!McpClient::is_error(&resp), "should succeed: {resp:?}");

    let text = McpClient::result_text(&resp);
    assert!(
        text.starts_with("---\n"),
        "response should start with YAML frontmatter: {text}"
    );
    assert!(text.contains("title:"), "frontmatter should contain title");
    assert!(
        text.contains("source:"),
        "frontmatter should contain source"
    );
    assert!(
        text.contains("word_count:"),
        "frontmatter should contain word_count"
    );
    assert!(
        text.contains("paragraph"),
        "article body should follow frontmatter"
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 5. mcp_fetch_markdown_no_images
// ---------------------------------------------------------------------------
#[test]
fn mcp_fetch_markdown_no_images() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/article")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML_WITH_IMAGES)
        .create();

    let url = format!("{}/article", server.url());
    let mut client = McpClient::new();
    let (_id, resp) = client.call_tool(
        "fetch_markdown",
        json!({"url": url, "no_images": true, "timeout": 10, "retries": 0}),
    );

    assert!(!McpClient::is_error(&resp), "should succeed: {resp:?}");

    let text = McpClient::result_text(&resp);
    assert!(
        !text.contains("!["),
        "image references should be stripped: {text}"
    );
    assert!(
        text.contains("paragraph"),
        "text content should remain after image stripping"
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 6. mcp_fetch_markdown_max_length
// ---------------------------------------------------------------------------
#[test]
fn mcp_fetch_markdown_max_length() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/article")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML)
        .create();

    let url = format!("{}/article", server.url());
    let mut client = McpClient::new();
    let (_id, resp) = client.call_tool(
        "fetch_markdown",
        json!({"url": url, "max_length": 50, "timeout": 10, "retries": 0}),
    );

    assert!(!McpClient::is_error(&resp), "should succeed: {resp:?}");

    let text = McpClient::result_text(&resp);
    assert!(
        text.contains("[Truncated]"),
        "truncated output should contain [Truncated] marker: {text}"
    );
    assert!(
        text.len() <= 80,
        "output should be near max_length (got {} chars): {text}",
        text.len()
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 7. mcp_fetch_markdown_raw
// ---------------------------------------------------------------------------
#[test]
fn mcp_fetch_markdown_raw() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/article")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML)
        .create();

    let url = format!("{}/article", server.url());
    let mut client = McpClient::new();
    let (_id, resp) = client.call_tool(
        "fetch_markdown",
        json!({"url": url, "raw": true, "timeout": 10, "retries": 0}),
    );

    assert!(!McpClient::is_error(&resp), "should succeed: {resp:?}");

    let text = McpClient::result_text(&resp);
    // Raw mode converts the full HTML — should contain article paragraphs and
    // the page heading or title in some form.
    assert!(
        text.contains("paragraph"),
        "raw output should contain page text: {text}"
    );
    assert!(
        text.contains("Test Article") || text.contains("first paragraph"),
        "raw output should contain page content: {text}"
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 8. mcp_fetch_metadata
// ---------------------------------------------------------------------------
#[test]
fn mcp_fetch_metadata() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/article")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML)
        .create();

    let url = format!("{}/article", server.url());
    let mut client = McpClient::new();
    let (_id, resp) = client.call_tool(
        "fetch_metadata",
        json!({"url": url, "timeout": 10, "retries": 0}),
    );

    assert!(!McpClient::is_error(&resp), "should succeed: {resp:?}");

    let text = McpClient::result_text(&resp);
    assert!(
        text.starts_with("---\n"),
        "metadata response should be YAML frontmatter: {text}"
    );
    assert!(text.contains("title:"), "should contain title field");
    assert!(text.contains("source:"), "should contain source field");
    assert!(
        text.contains("word_count:"),
        "should contain word_count field"
    );

    // fetch_metadata returns ONLY frontmatter (starts with --- and ends with ---).
    // Find the closing --- and verify nothing meaningful follows it.
    let closing = text.rfind("\n---").unwrap_or(0);
    let after_closing = text[closing + 4..].trim();
    assert!(
        after_closing.is_empty(),
        "fetch_metadata should return only frontmatter, got extra content after closing ---: '{after_closing}'"
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 9. mcp_batch_fetch
// ---------------------------------------------------------------------------
#[test]
fn mcp_batch_fetch() {
    let html1 = r"<!DOCTYPE html>
<html><head><title>First Page</title></head>
<body><article>
  <h1>First Page</h1>
  <p>Content of the first page with enough text for readability extraction to succeed.</p>
  <p>Second paragraph to ensure the article passes the readability content threshold.</p>
</article></body></html>";

    let html2 = r"<!DOCTYPE html>
<html><head><title>Second Page</title></head>
<body><article>
  <h1>Second Page</h1>
  <p>Content of the second page with enough text for readability extraction to succeed.</p>
  <p>Second paragraph to ensure the article passes the readability content threshold.</p>
</article></body></html>";

    let mut server = Server::new();
    let mock1 = server
        .mock("GET", "/first")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(html1)
        .create();
    let mock2 = server
        .mock("GET", "/second")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(html2)
        .create();

    let url1 = format!("{}/first", server.url());
    let url2 = format!("{}/second", server.url());

    let mut client = McpClient::new();
    let (_id, resp) = client.call_tool(
        "batch_fetch",
        json!({"urls": [url1, url2], "timeout": 10, "retries": 0}),
    );

    assert!(!McpClient::is_error(&resp), "should succeed: {resp:?}");

    let text = McpClient::result_text(&resp);
    let results: Vec<Value> =
        serde_json::from_str(text).expect("batch_fetch result should be a JSON array");

    assert_eq!(results.len(), 2, "should have 2 results");

    for result in &results {
        assert!(
            result["url"].is_string(),
            "each result should have a url field: {result}"
        );
        assert!(
            result["content"].is_string(),
            "each result should have a content field: {result}"
        );
        assert!(
            result["error"].is_null(),
            "successful result should not have error: {result}"
        );
    }

    let all_content: String = results
        .iter()
        .map(|r| r["content"].as_str().unwrap_or(""))
        .collect::<Vec<_>>()
        .join(" ");

    assert!(
        all_content.contains("first page"),
        "combined content should contain first page text: {all_content}"
    );
    assert!(
        all_content.contains("second page"),
        "combined content should contain second page text: {all_content}"
    );

    mock1.assert();
    mock2.assert();
}

// ---------------------------------------------------------------------------
// 10. mcp_batch_fetch_partial_failure
// ---------------------------------------------------------------------------
#[test]
fn mcp_batch_fetch_partial_failure() {
    let mut server = Server::new();
    let mock = server
        .mock("GET", "/good")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(TEST_HTML)
        .create();

    let good_url = format!("{}/good", server.url());
    // Port 1 is reserved/privileged — connections are refused immediately.
    // This passes URL validation (http://) but fails at the network level.
    let bad_url = "http://127.0.0.1:1/nope";

    let mut client = McpClient::new();
    let (_id, resp) = client.call_tool(
        "batch_fetch",
        json!({"urls": [good_url, bad_url], "timeout": 2, "retries": 0}),
    );

    // The batch tool itself succeeds; individual items carry per-url errors
    assert!(
        !McpClient::is_error(&resp),
        "batch_fetch should not return a top-level error for partial failure: {resp:?}"
    );

    let text = McpClient::result_text(&resp);
    let results: Vec<Value> = serde_json::from_str(text).expect("result should be JSON array");

    assert_eq!(results.len(), 2, "should have 2 results");

    let has_content = results.iter().any(|r| r["content"].is_string());
    let has_error = results.iter().any(|r| r["error"].is_string());

    assert!(
        has_content,
        "at least one result should have content: {results:?}"
    );
    assert!(
        has_error,
        "at least one result should have an error: {results:?}"
    );

    mock.assert();
}

// ---------------------------------------------------------------------------
// 11. mcp_fetch_markdown_invalid_url
// ---------------------------------------------------------------------------
#[test]
fn mcp_fetch_markdown_invalid_url() {
    let mut client = McpClient::new();
    let (_id, resp) = client.call_tool("fetch_markdown", json!({"url": "not-a-url"}));

    assert!(
        McpClient::is_error(&resp),
        "invalid URL should produce isError: true: {resp:?}"
    );

    let text = McpClient::result_text(&resp);
    assert!(
        text.contains("invalid URL") || text.contains("unsupported"),
        "error text should describe the problem: {text}"
    );
}

// ---------------------------------------------------------------------------
// 12. mcp_fetch_markdown_bad_scheme
// ---------------------------------------------------------------------------
#[test]
fn mcp_fetch_markdown_bad_scheme() {
    let mut client = McpClient::new();
    let (_id, resp) = client.call_tool(
        "fetch_markdown",
        json!({"url": "ftp://example.com/file.txt"}),
    );

    assert!(
        McpClient::is_error(&resp),
        "ftp:// scheme should produce isError: true: {resp:?}"
    );

    let text = McpClient::result_text(&resp);
    assert!(
        text.contains("ftp") || text.contains("scheme") || text.contains("unsupported"),
        "error text should mention the bad scheme: {text}"
    );
}

// ---------------------------------------------------------------------------
// 13. mcp_validate_timeout_zero
// ---------------------------------------------------------------------------
#[test]
fn mcp_validate_timeout_zero() {
    let mut client = McpClient::new();
    let (_id, resp) = client.call_tool(
        "fetch_markdown",
        json!({"url": "http://example.com", "timeout": 0}),
    );

    assert!(
        McpClient::is_error(&resp),
        "timeout=0 should produce isError: true: {resp:?}"
    );

    let text = McpClient::result_text(&resp);
    assert!(
        text.contains("timeout"),
        "error text should mention timeout: {text}"
    );
    assert!(
        text.contains("greater than 0") || text.contains('0'),
        "error text should describe constraint: {text}"
    );
}

// ---------------------------------------------------------------------------
// 14. mcp_validate_max_length_zero
// ---------------------------------------------------------------------------
#[test]
fn mcp_validate_max_length_zero() {
    let mut client = McpClient::new();
    let (_id, resp) = client.call_tool(
        "fetch_markdown",
        json!({"url": "http://example.com", "max_length": 0}),
    );

    assert!(
        McpClient::is_error(&resp),
        "max_length=0 should produce isError: true: {resp:?}"
    );

    let text = McpClient::result_text(&resp);
    assert!(
        text.contains("max_length") || text.contains("greater than 0"),
        "error text should describe max_length constraint: {text}"
    );
}

// ---------------------------------------------------------------------------
// 15. mcp_serve_help
// ---------------------------------------------------------------------------
#[test]
fn mcp_serve_help() {
    let output = Command::new(cargo_bin("mdget"))
        .args(["serve", "--help"])
        .output()
        .expect("spawn mdget serve --help");

    assert!(output.status.success(), "mdget serve --help should exit 0");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("MCP") || stdout.contains("mcp"),
        "help should mention MCP: {stdout}"
    );
    assert!(
        stdout.contains("mcpServers") || stdout.contains("settings.json"),
        "help should contain MCP setup instructions: {stdout}"
    );
    assert!(
        stdout.contains("stdio") || stdout.contains("fetch"),
        "help should describe what serve does: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// 16. mcp_fetch_markdown_connection_refused
// ---------------------------------------------------------------------------
#[test]
fn mcp_fetch_markdown_connection_refused() {
    if !port_1_refused_quickly() {
        // Skip: port 1 did not refuse fast enough; avoids hanging in unusual CI
        return;
    }

    let mut client = McpClient::new();
    let (_id, resp) = client.call_tool(
        "fetch_markdown",
        json!({"url": "http://127.0.0.1:1/nope", "timeout": 1, "retries": 0}),
    );

    assert!(
        McpClient::is_error(&resp),
        "connection refused should produce isError: true: {resp:?}"
    );
}
