---
title: "Site crawling"
type: iteration
date: 2026-04-17
tags: [iteration, crawling]
status: planned
branch: iter-7/crawling
---

## Goal

Add site crawling to fetch multiple pages from a domain, following links up to a configurable depth. Must be a responsible crawler by default — respect robots.txt, rate limit, and cap page counts.

## CLI Interface

```
mdget crawl https://docs.example.com              # crawl with defaults
mdget crawl --depth 2 https://docs.example.com    # follow links 2 levels deep
mdget crawl --delay 2 https://docs.example.com    # 2 seconds between requests
mdget crawl --max-pages 50 https://docs.example.com
mdget crawl --sitemap https://docs.example.com    # use sitemap.xml for discovery
mdget crawl -O https://docs.example.com           # auto-generate filenames, one per page
mdget crawl --output-dir ./docs https://docs.example.com  # output to directory
```

## Responsible Defaults

| Setting | Default | Flag |
|---------|---------|------|
| Depth | 1 | `--depth N` |
| Delay between requests | 1 second | `--delay N` |
| Max pages | 20 | `--max-pages N` |
| Respect robots.txt | yes | `--ignore-robots` to override |
| Stay on same domain | yes | `--follow-external` to override |

## Tasks

- [ ] Add `crawl` subcommand to CLI
- [ ] Implement link discovery from raw HTML (before readability — see design decisions)
- [ ] Implement breadth-first crawl with depth limiting
- [ ] Parse and respect `robots.txt` (evaluate `robotstxt` vs `texting_robots` crates)
- [ ] Implement `--delay` rate limiting between requests
- [ ] Implement `--max-pages` cap
- [ ] Implement `--sitemap` for sitemap.xml-based discovery (requires XML dep, e.g. `quick-xml`)
- [ ] Implement `--output-dir` to write one markdown file per page
- [ ] Stay on same domain by default, `--follow-external` to allow cross-domain
- [ ] Progress reporting on stderr (pages fetched, queue size, errors)
- [ ] Deduplication: don't fetch the same URL twice (normalize URLs)
- [ ] Add e2e tests with mock HTTP server
- [ ] Run quality gates

## Design Decisions

- **Responsible by default**: conservative limits, robots.txt respected, identifiable User-Agent. The tool should not be easily misusable as an aggressive bot.
- **Same-domain only by default**: prevents accidentally crawling the entire internet. `--follow-external` is an explicit opt-in.
- **Breadth-first**: more predictable than depth-first for bounded crawls. Top-level pages are usually more valuable.
- **URL normalization**: strip fragments, normalize trailing slashes, decode percent-encoding to avoid duplicate fetches.
- **Link discovery from raw HTML, not markdown**: readability strips nav/sidebar/footer links which are often the most useful for crawling (table of contents, next/prev page links). The crawl engine needs the full HTML to discover links, then passes each page through readability for the markdown output.
- **Crawl logic in `mdget-core`**: following the architecture pattern (core = logic, cli/mcp = presentation). The crawl engine API should be designed so that a future `crawl` MCP tool can reuse it.
- **Stdout output for multi-page crawls**: pages separated by `---` (consistent with existing multi-URL behavior). `--include-metadata` pairs naturally with crawling to identify each page's source URL in the output.

## Open Questions

### Consider splitting into 7a/7b

This iteration is significantly larger than previous ones. Consider splitting:
- **7a**: Core crawl engine — BFS, depth limit, same-domain, dedup, rate limiting, `--output-dir`, progress reporting
- **7b**: robots.txt parsing, sitemap.xml discovery (adds external crate deps)

### MCP crawl tool

Crawling is long-running — MCP's request/response model isn't a natural fit. Options:
- Use MCP progress notifications (supported by `rmcp`) to stream status updates
- Return a summary with page list + errors rather than streaming content
- Defer MCP crawl tool entirely and let agents use the CLI via shell

Decision deferred until after [[iteration-06-mcp-server|iteration 6 (MCP server)]] is implemented and we understand the MCP integration patterns better.

### robots.txt crate selection

Main candidates:
- `robotstxt` — Google's C++ parser ported to Rust, well-tested against real-world edge cases
- `texting_robots` — pure Rust, more idiomatic API

Evaluate both during implementation for correctness, maintenance status, and dependency footprint.

### Sitemap XML dependency

`--sitemap` requires an XML parser (e.g. `quick-xml`). If splitting into 7a/7b, sitemap support is a natural fit for 7b to keep 7a leaner.
