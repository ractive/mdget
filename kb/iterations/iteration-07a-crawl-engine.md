---
title: Crawl engine
type: iteration
date: 2026-05-03
tags:
  - iteration
  - crawling
status: completed
branch: iter-7a/crawl-engine
---

## Goal

Add a `crawl` subcommand that fetches multiple pages from a domain, following links breadth-first up to a configurable depth. Conservative defaults, same-host only, rate-limited. No external crate deps beyond what's already in the workspace.

## CLI Interface

```shell
mdget crawl https://docs.example.com              # crawl with defaults
mdget crawl --depth 2 https://docs.example.com    # follow links 2 levels deep
mdget crawl --delay 2 https://docs.example.com    # 2 seconds between requests
mdget crawl --max-pages 50 https://docs.example.com
mdget crawl -O https://docs.example.com           # auto-generate filenames, one per page
mdget crawl --output-dir ./docs https://docs.example.com  # output to directory
```

## Responsible Defaults

| Setting | Default | Flag |
|---------|---------|------|
| Depth | 1 | `--depth N` |
| Delay between requests | 1 second | `--delay N` |
| Max pages | 20 | `--max-pages N` |
| Stay on same host | yes | `--follow-external` to override |

Note: robots.txt and sitemap support deferred to [[iteration-07b-robots-sitemap]].

## Tasks

- [x] Add `crawl` subcommand to CLI (clap)
- [x] Implement link discovery from raw HTML (before readability — extract `<a href>` from full HTML, not after readability strips nav/sidebar)
- [x] URL normalization function in `mdget-core`: strip fragments, lowercase scheme/host, remove default ports, normalize trailing slashes, decode percent-encoding for dedup
- [x] Implement breadth-first crawl engine in `mdget-core` with depth limiting
- [x] Same-host filtering by default, `--follow-external` to allow cross-domain
- [x] Implement `--delay` rate limiting between requests (default 1s)
- [x] Implement `--max-pages` cap (default 20)
- [x] Implement `--output-dir` to write one markdown file per page (mirror URL path structure)
- [x] Implement `-O` flag (auto-generate filenames in current directory)
- [x] Stdout output for multi-page crawls: always include metadata frontmatter (source URL) per page, pages delimited by frontmatter fences
- [x] Progress reporting on stderr (pages fetched, queue size, errors)
- [x] Deduplication via normalized URL set
- [x] Add e2e tests with mock HTTP server
- [x] Run quality gates

## Design Decisions

- **Breadth-first**: more predictable than depth-first for bounded crawls. Top-level pages are usually more valuable.
- **Link discovery from raw HTML, not markdown**: readability strips nav/sidebar/footer links which are often the most useful for crawling (table of contents, next/prev page links). The crawl engine needs the full HTML to discover links, then passes each page through readability for the markdown output.
- **Crawl logic in `mdget-core`**: following the architecture pattern (core = logic, cli/mcp = presentation). The crawl engine API should be designed so that a future `crawl` MCP tool or 7b's robots.txt/sitemap features can layer on top.
- **URL normalization in core**: a small `normalize_url(url: &Url) -> String` function. No external crate — the steps are straightforward: strip fragment, lowercase scheme/host, remove default port (80/443), normalize path trailing slash, sort query params.
- **No robots.txt in 7a**: keeps the iteration focused on the crawl engine itself. Adding robots.txt requires fetching/caching an extra resource per domain and a new crate dep — cleaner as a separate step.

### Stdout output in crawl mode

Crawl mode **always emits metadata frontmatter** per page, regardless of `--include-metadata`. The frontmatter `---` fences naturally delimit pages, and each includes the `source:` URL so the origin is always recoverable. Without this, multi-page stdout is an ambiguous wall of markdown where `---` could be content or a separator.

### `--output-dir` path structure

Mirrors the URL path on disk:
```text
output-dir/
  path/to/page.md
  docs/getting-started.md
  docs/api/reference.md
```

Index pages (`/path/` or `/path`) become `path/index.md`.

**With `--follow-external`**, the hostname is prefixed to avoid path collisions across domains:
```text
output-dir/
  docs.example.com/getting-started.md
  blog.example.com/post/hello.md
```

Single-host crawls omit the hostname prefix since it's redundant.

### Same-host filtering

**Same host, not same registered domain.** Crawling `docs.example.com` will not follow links to `example.com` or `www.example.com` — those are treated as external. This is simple, predictable, and avoids needing a public suffix list dependency.

External links are **discovered but silently skipped** — not added to the crawl queue, no error emitted. With `--follow-external`, they are followed but still subject to `--max-pages` and `--depth` limits.
