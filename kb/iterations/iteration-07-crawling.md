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
- [ ] Implement link discovery from fetched markdown/HTML
- [ ] Implement breadth-first crawl with depth limiting
- [ ] Parse and respect `robots.txt` (find or build a Rust parser)
- [ ] Implement `--delay` rate limiting between requests
- [ ] Implement `--max-pages` cap
- [ ] Implement `--sitemap` for sitemap.xml-based discovery
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
