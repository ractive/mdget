---
title: "Dogfood #4 fixes — crawl_site safety, content detection, CLI polish"
type: iteration
date: 2026-05-03
tags:
  - iteration
  - dogfood
  - mcp
  - crawl
  - quality
status: completed
branch: iter-11/dogfood-4-fixes
---

## Goal

Address issues found in [[iterations/done/iteration-10-dogfood-review|dogfood review #4]]. The P1 is adding `max_length` to the MCP `crawl_site` tool to prevent context window blowouts. P2s improve content detection (RSS feeds, smarter excerpt fallback). P3s are CLI polish.

## Tasks

### MCP server improvements

- [x] MCP `crawl_site`: add `max_length` parameter (per-page content truncation). Without this, a single "print all" page (e.g. Rust Book `print.html` at 1.4M chars) can blow up an agent's context window. Apply the same truncation logic as CLI's `--max-length`. Default to a sensible limit (e.g. 50000 chars per page).

### Content detection

- [x] RSS/Atom feed detection: when Content-Type is `application/rss+xml`, `application/atom+xml`, or `application/xml` and the body starts with `<rss` or `<feed`, return a clear error message: `"RSS/Atom feeds are not supported — use a feed parser instead"`. Currently produces escaped XML soup that is unreadable and wastes tokens.
- [x] Smarter excerpt fallback: when `<meta name="description">` looks like a tag list (no spaces between words, fewer than 3 words, or matches known junk patterns like "github crates-io docs-rs"), fall back to the first ~200 chars of extracted body content instead. The current meta tag fallback from iter-9 works but inherits upstream junk.

### CLI polish

- [x] Allow `-q`/`--quiet` as a crawl subcommand flag: users naturally type `mdget crawl -q ...` but clap only accepts `mdget -q crawl ...`. Add `-q` to the crawl subcommand's arg struct (forwarding to the global flag).
- [x] `--max-length 0` semantics: treat 0 as "no limit" instead of producing only `[Truncated]` with no content. Alternatively, reject 0 with a clear validation error.

## Out of scope

- **Wikipedia data table extraction**: readability strips these by design. Would need `--selector` CSS extraction (tracked separately).
- **JS-only SPA rendering**: crates.io returns 404 because it's a pure client-side SPA. Solving this requires headless browser integration — out of scope for mdget.
- **Crawl auto-inference tuning**: the prefix auto-inference is technically correct; users who want broader crawls can set `--path-prefix /` explicitly.

## Notes

- The `crawl_site` `max_length` fix is the highest priority — it's a safety issue for agent workflows.
- RSS detection should be conservative: only trigger on known feed content-types + body sniffing, not all XML.
- The `-q` flag forwarding is a UX improvement; the workaround (`mdget -q crawl ...`) works fine.
