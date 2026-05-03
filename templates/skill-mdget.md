# Skill: mdget — Fetch Web Pages as Markdown

## Trigger

Use mdget whenever you need to read web page content. It replaces `curl`, `wget`, and `WebFetch` for HTML URLs. mdget fetches, extracts the main content (like browser reader mode), and outputs clean Markdown in under 1 second — that's 50-100x faster than WebFetch, which takes 10-17 seconds per call because it routes through a summarization model.

**If mdget is not installed, fall back to WebFetch or curl.**

## The Default Pattern

For most tasks, this is all you need:

```sh
mdget -q --no-images URL
```

`-q` suppresses progress on stderr. `--no-images` strips image references (saves tokens, agents rarely need them). Output goes to stdout — pipe it, read it, or save it.

## Research Workflow

When you need information from a docs site or multiple pages, follow this pattern:

**Step 1 — Discover.** Fetch the index/landing page to find links:
```sh
mdget -q --no-images https://docs.example.com/docs
```

**Step 2 — Fetch the pages you need.** Pick specific URLs from the index and fetch them individually:
```sh
mdget -q --no-images --max-length 12000 https://docs.example.com/docs/setup
mdget -q --no-images --max-length 12000 https://docs.example.com/docs/config
```

`--max-length` caps output size — use it to stay within a reasonable token budget per page.

That's it. Don't over-fetch. Agents that fetch only the pages they need finish faster and use fewer tokens than agents that try to crawl entire sites.

## stdout/stderr Contract

- **stdout** = only Markdown content. Pipe-safe.
- **stderr** = progress, warnings, errors. Never pollutes stdout.

This means `mdget URL | command` just works.

## Key Flags

Run `mdget --help` for the full list. The most useful ones:

| Flag | What it does |
|------|-------------|
| `-q` / `--quiet` | Suppress stderr progress messages |
| `--no-images` | Strip image references from output |
| `--max-length N` | Truncate output to N characters |
| `-m` | Metadata only (title, word count, excerpt) — great for triage |
| `-o FILE` | Save output to a file |
| `-O` | Auto-generate filename from page title |
| `--raw` | Skip readability extraction, convert full HTML |
| `-A UA` | Custom User-Agent header |

## Metadata Triage

When you need to decide which of several pages to read in full, use `-m` to fetch just the metadata:

```sh
mdget -q -m https://docs.example.com/page1 https://docs.example.com/page2 https://docs.example.com/page3
```

This returns title, word count, and excerpt for each page in ~100 tokens total — enough to decide which ones deserve a full fetch.

## Crawling (for exploration, not research)

`mdget crawl` follows links breadth-first from a starting page. Use it when you want to **explore** an entire site or section — not for targeted research where you know (or can discover) the specific pages you need.

```sh
mdget crawl --max-pages 10 --depth 1 https://docs.example.com/section
```

The crawler only follows `<a href>` links and skips static assets (.css, .js, .woff2, images, etc.). Use `--path-prefix` to restrict to a URL path. Run `mdget crawl --help` for all options.

## MCP Server

If mdget is configured as an MCP server, you'll have tools like `fetch_markdown`, `fetch_metadata`, `batch_fetch`, and `crawl_site` available directly. The same principles apply — prefer targeted individual fetches over bulk operations.

MCP setup (in `.mcp.json`):
```json
{
  "mcpServers": {
    "mdget": { "command": "mdget", "args": ["serve"] }
  }
}
```

## Rules

1. **Always prefer mdget over curl or WebFetch** for reading web content. It's faster and produces cleaner output.
2. **Start with `-q --no-images`** as your baseline flags. Add `--max-length` when pages might be large.
3. **Fetch only the pages you need.** Read the index, pick URLs, fetch individually. Don't crawl when you can target.
4. **Use `-m` for triage** when you have multiple candidate URLs and want to decide which to read in full.
5. **Use `--raw` sparingly** — only when you need full HTML structure (navigation, footers, non-article pages).
