---
title: "Dogfood review #3 — CLI + MCP server"
type: iteration
date: 2026-05-03
tags:
  - dogfood
  - review
  - mcp
  - crawl
status: completed
---

## Test Setup

Tested mdget v0.1.0 as both CLI tool and MCP server. Pages fetched: turborepo docs (Docker, run reference, CI, configuration), Hacker News, MDN Promise docs, Wikipedia Rust article, docs.rs crate pages, Vercel landing page, httpbin HTML/JSON, various error cases. MCP server tested via child claude-code session with all three tools exercised.

## What Works Well

### Readability extraction is excellent on docs/articles
Turborepo docs pages came through perfectly: headings, code blocks with language tags, tables, links all preserved. The Docker guide's multi-stage Dockerfile examples were flawless. MDN's Promise docs with nested code blocks and inline formatting — clean and usable.

### Metadata-only triage is a killer workflow
`mdget -m url1 url2 url3` is exactly what an agent needs for deciding which pages to fetch in full. The YAML frontmatter with title, word_count, excerpt gives enough signal to prioritize. Tested on 4 turborepo docs + 3 docs.rs crate pages — fast and useful.

### stdout/stderr separation works perfectly
Progress on stderr, content on stdout. `grep -c '^#' <<(mdget -q url)` just works. Quiet mode correctly suppresses all progress. This is the kind of Unix composability agents need.

### JSON handling is smart
`httpbin.org/json` returns JSON wrapped in a fenced code block with `json` language tag. No readability confusion, no errors — just sensible behavior.

### Max-length truncation is clean
`--max-length 500` truncates at a paragraph boundary and appends `[Truncated]`. No mid-sentence cuts.

### Error messages are clear (for real HTTP errors)
Real 404s: `HTTP 404 Not Found fetching URL: <url>` — no retries, immediate, machine-parseable. PDF: tells you to use `pdftotext`. No args: concise usage hint.

### MCP server works well
All three tools (`fetch_markdown`, `fetch_metadata`, `batch_fetch`) function correctly. `batch_fetch` with `max_length` truncation for multi-page research is a strong productivity win.

### Auto-filename is sensible
`-O` on the Docker guide created `docker.md` — clean slug from page title.

### Init skill file is comprehensive
The generated `.claude/skills/mdget/SKILL.md` is one of the best skill files I've seen: trigger conditions, CLI reference table, pipeline examples, MCP setup instructions. An agent encountering this would know exactly when and how to use mdget.

## What's Awkward or Broken

### BUG: Crawler follows static assets (fonts, CSS, JS) — CRITICAL
`mdget crawl --depth 1 --max-pages 5 https://turborepo.dev/docs/guides/tools/docker` spent 4 of its 5 page budget on `.woff2` font files and `.css` stylesheets. The crawler extracts ALL URLs from the HTML source (including `<link rel="stylesheet">`, `<script src>`, `<link rel="preload">`, resource hints) rather than only `<a href>` links.

**Impact:** With depth > 0, the crawl budget is wasted on binary assets, producing 300KB of garbage in stdout. The actual doc pages the user wanted are skipped because `max_pages` is exhausted on fonts.

**Fix:** Only follow `<a href>` links. Optionally filter by content-type on response (skip non-HTML). This is the most important bug found.

### Wikipedia infobox renders as flat text
The Rust programming language infobox becomes a list of key-value pairs without any table structure — just `ParadigmsConcurrent, functional, ...` run together. The article body below is fine, but the infobox at the top is noisy.

### Landing pages produce mediocre output
Vercel.com's landing page extraction is readable but the structure is lost — marketing copy, feature lists, and CTAs all flatten into a stream of text. This is inherent to readability extraction on non-article pages. Not really a bug, but `--raw` wasn't much better.

### Multi-URL output separation is fragile
Multiple URLs output their markdown separated by `---` lines. If the output includes `--include-metadata`, each page starts with YAML frontmatter (which also uses `---`), making it hard to programmatically split pages. Consider a more explicit separator (e.g., `===PAGE_BREAK===` or structured JSON envelope).

### MCP batch_fetch embeds markdown in JSON strings
The `batch_fetch` tool returns `{url, title, content}` where `content` includes YAML frontmatter inside a JSON string field. Agents have to parse YAML-within-JSON. Consider promoting structured metadata to JSON-level fields.

### Init modifies CLAUDE.md without asking
`mdget init --claude` appends to CLAUDE.md with HTML comments as delimiters. This is fine for user projects but surprising if CLAUDE.md is version-controlled. Should warn or ask.

### `--max-length 200` on a long page can produce near-empty output
On Wikipedia's Rust article, `--max-length 200` returns just the infobox header `Rust` plus `[Truncated]`. The truncation target is too small to be useful for pages with large preambles.

## Output Quality Report

| Page | Quality | Notes |
|------|---------|-------|
| Turborepo Docker docs | Excellent | Code blocks, links, structure all preserved perfectly |
| Turborepo run reference | Excellent | Tables, flags, examples all clean |
| Turborepo CI docs | Excellent | Clean extraction, good signal-to-noise |
| MDN Promise docs | Excellent | Nested code blocks, inline formatting preserved |
| Hacker News front page | Good | All 30 items with links, but escaped brackets in URLs add noise |
| Wikipedia Rust article | Good | Body is clean, infobox is messy flat text |
| docs.rs crate pages | Good | Content extracted, but excerpt metadata is weak ("github crates-io docs-rs") |
| Vercel landing page | Mediocre | Marketing copy flattened, structure lost — inherent limitation |
| httpbin/html | Good | Clean Moby-Dick passage extraction |
| httpbin/json | Excellent | Smart JSON-in-fenced-code-block handling |

**Pattern:** Works great on articles and docs, struggles with marketing/landing pages and complex infoboxes.

## Feature Ideas

### 1. Crawl: filter by content-type or URL pattern (HIGH)
Allow `--include-pattern '*.html'` or `--content-type text/html` to prevent crawling binary assets. This would make depth > 0 usable on real sites.

### 2. Crawl: `--same-path-prefix` or `--path-prefix` (HIGH)
When crawling `https://docs.example.com/docs/foo`, only follow links under `/docs/`. Currently, the crawler follows all same-host links, which for docs sites includes blog posts, changelogs, and marketing pages.

### 3. `--format json` for structured multi-URL output (MEDIUM)
Output each result as a JSON object with `{url, title, metadata, content}` fields on separate lines (JSONL). Agents can parse this reliably vs. the current `---`-delimited approach.

### 4. MCP `crawl_site` tool (MEDIUM)
The MCP server exposes single/batch fetch but not crawling. A `crawl_site` tool with depth/max-pages would enable agent workflows like "explore these docs".

### 5. `--follow-redirects-report` or redirect info in metadata (LOW)
When a URL redirects, report the final URL in the output metadata. Useful for link-checking workflows.

### 6. `--selector` CSS selector extraction (LOW)
For non-article pages, allow extracting specific DOM elements. E.g., `--selector "main"` or `--selector ".docs-content"`. Useful when readability fails on SPAs or complex layouts.

## Verdict

mdget is genuinely better than `curl | html2text` or WebFetch for 90% of what an AI agent needs. The readability extraction on docs and articles is excellent — clean, well-structured markdown with code blocks, tables, and links preserved. The metadata-only triage workflow, quiet piping, and auto-filename are all agent-friendly design choices that show thoughtful ergonomics.

The **critical issue is the crawler following static assets** — this makes `mdget crawl` with depth > 0 nearly unusable on modern sites (Next.js, etc.) that embed many resource URLs in their HTML. Fix that and the crawl story goes from broken to strong. The MCP server works well and having it available as a tool rather than shelling out is a better experience.

I would use mdget as my primary web fetching tool for single pages and metadata triage today. For crawling, I'd hold off until the asset-following bug is fixed.
