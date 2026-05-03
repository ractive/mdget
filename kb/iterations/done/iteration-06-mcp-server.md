---
title: MCP server
type: iteration
date: 2026-04-17
tags:
  - iteration
  - mcp
  - ai-agents
status: completed
branch: iter-6/mcp-server
---

## Goal

Expose mdget as an MCP (Model Context Protocol) server so AI agents can fetch web content as markdown without shelling out to a CLI.

## Architecture

New crate: `crates/mdget-mcp`. Follows the same pattern as `mdget-cli` — thin presentation layer that calls into `mdget-core` for all logic. The CLI crate does NOT depend on the MCP crate or vice versa.

The MCP crate adds `tokio` and `rmcp` as dependencies. Core stays synchronous — MCP handlers use `tokio::task::spawn_blocking` to call core functions.

```
crates/
├── mdget-core/    # business logic (sync, no tokio)
├── mdget-cli/     # CLI presentation (sync, clap)
└── mdget-mcp/     # MCP presentation (async, rmcp + tokio)
```

## CLI Interface

```
mdget serve                        # start MCP server (stdio transport)
```

The `serve` subcommand is added to `mdget-cli` but delegates immediately to `mdget-mcp`. This avoids requiring users to install a separate binary.

## MCP Tool Surface

### `fetch_markdown`

Fetches a URL and returns its content as markdown.

**Parameters:**
- `url` (string, required) — URL to fetch
- `raw` (boolean, optional, default false) — skip readability extraction
- `include_metadata` (boolean, optional, default false) — prepend YAML frontmatter
- `no_images` (boolean, optional, default false) — strip image references
- `max_length` (integer, optional) — truncate output to N characters
- `timeout` (integer, optional, default 30) — timeout in seconds
- `retries` (integer, optional, default 2) — retry count for transient errors

**Returns:** markdown string (content type `text`)

### `fetch_metadata`

Fetches a URL and returns only the YAML metadata frontmatter (title, word count, excerpt, etc.). Equivalent to `mdget -m <URL>`. Useful for triaging URLs before committing to a full fetch.

**Parameters:**
- `url` (string, required) — URL to fetch
- `timeout` (integer, optional, default 30) — timeout in seconds
- `retries` (integer, optional, default 2) — retry count for transient errors

**Returns:** YAML frontmatter string (content type `text`)

### `batch_fetch`

Fetches multiple URLs in parallel and returns all results. Equivalent to `mdget -j <N> url1 url2 ...`.

**Parameters:**
- `urls` (array of strings, required) — URLs to fetch
- `raw` (boolean, optional, default false) — skip readability extraction
- `include_metadata` (boolean, optional, default false) — prepend YAML frontmatter
- `no_images` (boolean, optional, default false) — strip image references
- `max_length` (integer, optional) — truncate each output to N characters
- `timeout` (integer, optional, default 30) — timeout in seconds per URL
- `retries` (integer, optional, default 2) — retry count for transient errors

**Returns:** array of results, each with `url`, `content` (or `error`), and `title`

## SDK

Use `rmcp` 1.6 — the official Anthropic-backed Rust MCP SDK.

```toml
rmcp = { version = "1.6", features = ["server", "macros", "transport-io"] }
tokio = { version = "1", features = ["full"] }
```

Tool definitions use `#[tool]` derive macros. Stdio transport via `rmcp::transport::stdio()`.

## Tasks

- [x] Create `crates/mdget-mcp` crate with `rmcp` + `tokio` dependencies
- [x] Implement MCP server struct with `#[tool_router]` / `#[tool_handler]`
- [x] Implement `fetch_markdown` tool — call `mdget_core::fetch` + `extract` via `spawn_blocking`
- [x] Implement `fetch_metadata` tool — same pipeline but return only frontmatter
- [x] Implement `batch_fetch` tool — parallel fetch via `spawn_blocking` tasks
- [x] Add `serve` subcommand to `mdget-cli` that starts the MCP server
- [x] Input validation: URL format, timeout > 0, max_length > 0, retries in reasonable range
- [x] Return clean MCP error responses (no stack traces, no internal paths)
- [x] Add integration tests (send JSON-RPC over stdin, verify responses)
- [x] Document MCP server setup in README (Claude Code config, Claude Desktop config)
- [x] Update `mdget init --claude` skill template with MCP server instructions
- [x] Run quality gates

## Design Decisions

- **Separate crate (`mdget-mcp`)**: keeps tokio/rmcp deps out of the CLI binary. Users who only want the CLI don't pay the compile-time or binary-size cost. Follows project convention: core logic in `mdget-core`, presentation layers in separate crates.
- **Stdio transport only for v1**: simplest, works with all MCP clients (Claude Code, Claude Desktop, etc.). HTTP/SSE transport can come later if needed.
- **Three tools, not one**: `fetch_metadata` enables cheap triage (inspect title/word count before full fetch). `batch_fetch` saves round-trips for multi-URL workflows. Each maps to an existing CLI workflow.
- **Reuse core pipeline**: the MCP handlers call the same `mdget-core` functions as the CLI. No logic duplication. `spawn_blocking` bridges async MCP ↔ sync core.
- **`serve` subcommand in CLI crate**: users install one binary (`mdget`), not two. The CLI crate gains an optional dependency on `mdget-mcp`, feature-gated if binary size becomes a concern.

## Security Considerations

### SSRF protection — deferred (not needed for stdio)

The original plan included SSRF filtering (blocking private/loopback IPs). After review, this is **not warranted for a stdio-only MCP server**:

- **Same trust level as CLI**: the user starts the MCP server locally and controls which MCP client connects. An AI agent sending URLs through MCP has the same access as the user typing `mdget http://127.0.0.1` in a terminal. Blocking private IPs in MCP but allowing them in CLI would be inconsistent and surprising.
- **No untrusted callers**: stdio transport means only the locally-connected client can send requests. There is no network-facing attack surface.
- **SSRF matters for multi-tenant / HTTP-exposed servers**: if mdget ever adds HTTP/SSE transport or runs as a shared service, SSRF filtering should be added at that point. Until then, it's security theatre.

**Revisit when**: HTTP/SSE transport is added, or mdget is deployed as a shared service.

### What IS implemented

- **Input validation**: URL format validation (scheme must be http/https), timeout/retries bounds checking, max_length > 0. Rejects malformed inputs before they reach core.
- **Max response size**: enforced in `mdget-core` (50 MB limit from iter-5b). MCP inherits this protection.
- **Redirect limits**: combined 15-hop limit across HTTP redirects + meta-refresh (from iter-5b). MCP inherits this.
- **Clean error messages**: MCP error responses never expose stack traces, file paths, or internal state. Errors are mapped to user-facing messages.
- **Request timeout**: per-request timeout (default 30s) prevents hanging on unresponsive servers.

### DNS rebinding — not applicable

DNS rebinding attacks require an attacker to control both a DNS server and the target service. In mdget's threat model (user-controlled local process), the user IS the caller. No mitigation needed for stdio transport.
