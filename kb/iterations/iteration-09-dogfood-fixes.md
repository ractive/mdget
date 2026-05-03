---
title: "Dogfood #3 fixes — crawl filtering, output quality, MCP improvements"
type: iteration
date: 2026-05-03
tags:
  - iteration
  - dogfood
  - crawl
  - mcp
  - quality
status: in-progress
branch: iter-9/dogfood-fixes
---

## Goal

Fix all actionable issues found in [[iteration-08-dogfood-review]] (dogfood review #3). No new features — only fixes for bugs and rough edges discovered during hands-on testing of CLI and MCP server.

## Tasks

### Crawl improvements

- [x] Crawl: fix link extraction to only follow `<a href>` tags, not `<link>`, `<script>`, etc. Add `is_static_asset_url()` extension blocklist as defense-in-depth to skip `.woff2`, `.css`, `.js`, images, archives, etc. before downloading. Apply filter both pre-queue and pre-download in the crawl loop.
- [x] Crawl: add `--path-prefix` flag to restrict crawl to URLs under a given path prefix (e.g., crawling `https://docs.example.com/docs/foo` with `--path-prefix /docs/` only follows links under `/docs/`). Auto-infer from start URL path when not explicitly set — if the start URL is `https://example.com/docs/getting-started`, default the prefix to `/docs/`.

### Output quality

- [x] Multi-URL to stdout warning: when multiple URLs are written to stdout (no `-O`, `-o`, or `--output-dir`), emit a stderr warning that clearly lists the alternatives: `warning: multiple URLs to stdout — output is concatenated and hard to split. Use -O (auto-named files), -o FILE (single file), --output-dir DIR (one file per URL), or the MCP batch_fetch tool instead.`
- [x] Wikipedia-style infobox: investigate whether dom_smoothie / readability can be tuned to either render infoboxes as tables or strip them entirely. The flat-text rendering of key-value pairs without structure is noisy. (May be a dom_smoothie upstream limitation — if so, document and skip.)

### MCP server improvements

- [x] MCP `batch_fetch`: promote structured metadata fields (`title`, `word_count`, `excerpt`, `language`, `byline`) to top-level JSON fields in the response alongside `content`, so agents don't have to parse YAML-within-JSON.
- [x] MCP: add `crawl_site` tool exposing the crawl engine (parameters: `url`, `depth`, `max_pages`, `delay`, `path_prefix`, `include_metadata`). Returns array of `{url, title, content}` results. This enables agent workflows like "explore these docs" without shelling out.
- [x] MCP: add `user_agent` parameter to `fetch_markdown` and `batch_fetch` tools, matching CLI's `-A` flag.

### CLI polish

- [x] `init --claude`: warn (on stderr) when modifying an existing CLAUDE.md that is tracked by git, so users aren't surprised by uncommitted changes to a versioned file.
- [x] Metadata `excerpt` quality: for pages where readability can't extract an excerpt, fall back to `<meta name="description">` or `<meta property="og:description">` before leaving it blank. Fixes the docs.rs "github crates-io docs-rs" junk excerpt.

## Out of scope

- `--format jsonl` / multi-URL output redesign — JSONL is unreadable (encoded markdown on one line), and multi-URL-to-stdout isn't a workflow anyone should use. Agents use MCP `batch_fetch`, humans use `-O`. A stderr warning is sufficient.
- `--selector` CSS selector extraction — useful but adds complexity, defer to a future iteration
- `--follow-redirects-report` / redirect info in metadata — nice-to-have, low priority
- Landing page extraction quality — inherent readability limitation, not fixable without a different approach
- `--max-length` producing near-empty output on pages with large preambles — by design (user asked for 200 chars)

## Notes

The crawl static-asset fix was prototyped on main during the dogfood session and is included in this iteration's branch.
