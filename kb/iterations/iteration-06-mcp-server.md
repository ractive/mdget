---
title: "MCP server"
type: iteration
date: 2026-04-17
tags: [iteration, mcp, ai-agents]
status: planned
branch: iter-6/mcp-server
---

## Goal

Expose mdget as an MCP (Model Context Protocol) tool so AI agents can fetch web content as markdown without shelling out to a CLI.

## CLI Interface

```
mdget serve                        # start MCP server (stdio transport)
```

## MCP Tool Surface

### `fetch_markdown`

Fetches a URL and returns its content as markdown.

**Parameters:**
- `url` (string, required) — URL to fetch
- `raw` (boolean, optional) — skip readability extraction
- `timeout` (integer, optional) — timeout in seconds
- `include_metadata` (boolean, optional) — prepend YAML frontmatter
- `no_images` (boolean, optional) — strip image references
- `max_length` (integer, optional) — truncate output

**Returns:** markdown string

## Tasks

- [ ] Add MCP SDK dependency (research best Rust MCP SDK)
- [ ] Implement `mdget serve` subcommand
- [ ] Define `fetch_markdown` tool with parameter schema
- [ ] Wire tool handler to existing core pipeline
- [ ] Implement stdio transport
- [ ] Handle errors as MCP tool errors (not process crashes)
- [ ] Add SSRF protection: reject requests to private/loopback IPs (127.0.0.0/8, 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16, ::1, link-local) — agents can be tricked into fetching internal URLs
- [ ] Add integration tests (send JSON-RPC requests, verify responses)
- [ ] Document MCP server setup in README
- [ ] Run quality gates

## Design Decisions

- **Stdio transport only for v1**: simplest, works with all MCP clients (Claude Code, Claude Desktop, etc.). HTTP/SSE transport can come later if needed.
- **Single tool**: one `fetch_markdown` tool maps cleanly to the CLI's core function. Batch fetching could be a second tool later.
- **Reuse core pipeline**: the MCP handler calls the same `mdget-core` functions as the CLI. No duplication.
- **SSRF protection required**: unlike CLI usage (where the user controls inputs), MCP exposes fetch to AI agents that could be tricked into requesting internal IPs. Block private/loopback ranges before connecting. (Flagged in iter-5b review as "revisit when MCP server is added".)
