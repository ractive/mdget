---
title: mdget Roadmap
type: project
date: 2026-04-17
tags: [roadmap, planning]
status: in-progress
---

## Roadmap

High-level plan for mdget iterations beyond the initial project setup.

### Iteration 2 — Core fetch: URL → Markdown CLI

See [[iterations/iteration-02-core-fetch]].

Single URL fetch with readability extraction, markdown output, file saving, auto-filename generation. Blocking HTTP, no concurrency.

### Iteration 3 — Batch fetching & local files

- Multiple URLs: `mdget url1 url2 url3`
- Stdin input: `cat urls.txt | mdget -`
- Local HTML files: `mdget ./page.html` or `mdget file:///path/to/page.html`
- Parallel fetching with `std::thread` or rayon (no tokio/async)
- Configurable parallelism: `--jobs/-j N`

### Iteration 4 — Output control & metadata

- `--include-metadata` — prepend YAML frontmatter (title, source URL, date fetched, word count)
- `--no-images` — strip `![alt](url)` image references (noise for LLMs since they can't see images)
- `--max-length N` — truncate output to N characters
- Links always included by default (no option to strip)

### Iteration 5 — Robustness

- Retry logic with backoff for transient failures
- Redirect chain reporting
- Handle non-HTML content types gracefully (PDF, plain text)
- Optional JS rendering via headless browser (`--js` flag, opt-in, only if browser is installed)

### Iteration 6 — MCP server

- `mdget serve` subcommand exposing mdget as an MCP tool (stdio transport)
- Tool surface: `fetch_markdown(url, raw?, timeout?)` — straightforward mapping of CLI flags
- Enables any AI agent to use mdget for web fetching

### Iteration 7 — Site crawling

- `mdget crawl --depth 2 https://docs.example.com`
- Sitemap.xml support
- Output to directory structure mirroring site paths
- Good citizen by default:
  - Respect `robots.txt`
  - Rate limiting with `--delay` (default: 1s between requests)
  - Default max pages cap
  - Identifiable User-Agent
