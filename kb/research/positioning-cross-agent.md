---
title: "Positioning: mdget as cross-agent web fetching"
type: research
date: 2026-05-03
tags:
  - positioning
  - strategy
  - mcp
  - ai-agents
status: completed
---

## Insight

Most AI coding agents lack built-in web page fetching:

| Agent | Web fetch capability |
|-------|---------------------|
| Claude Code | WebFetch (server-side summarization) + WebSearch |
| OpenAI Codex CLI | Web search from pre-cached index. No raw page fetch by default |
| OpenCode | `websearch` (Exa AI) + `webfetch` (raw retrieval, no summarization) |
| Cursor / Windsurf | Basic web search only. No page fetching |
| Aider | No built-in web fetching |

Claude Code's WebFetch with server-side summarization is the exception, not the norm. Most agents have no way to read a web page at all.

## Reframed positioning

mdget isn't competing with WebFetch. It's filling a gap that most agents don't cover.

**Old pitch:** "Use this instead of WebFetch" — benchmarks showed this is marginal at best.

**New pitch:** "Install this so your agent can read the web" — for Codex CLI, Aider, Cursor, and any agent without built-in page fetching, mdget (as a CLI tool or MCP server) is the only option for clean web content.

## Potential directions

- **MCP server for any agent:** Most agents support MCP now. `mdget serve` could be the standard "web reading" MCP server across the ecosystem.
- **Deeper integration:** Explore what it takes to be a first-class tool in Codex CLI, OpenCode, Aider — e.g. tool definitions, plugin formats, configuration docs.
- **Positioning in README/docs:** Shift from "better than curl" to "give your AI agent web access."
- **Benchmark against no-tool baseline:** The real comparison isn't mdget vs WebFetch — it's mdget vs nothing (which is what most agents have).

## Open questions

- How do Codex CLI and OpenCode handle MCP servers? Can mdget slot in directly?
- What does Aider's tool extension model look like?
- Would a lightweight "just fetch one page" mode (no crawl, no metadata, minimal binary) be more attractive for cross-agent adoption?
- Should the README lead with the cross-agent story instead of the Claude Code story?
