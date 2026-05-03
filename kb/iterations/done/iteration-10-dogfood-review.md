---
title: "Dogfood review #4 — iter-9 features, edge cases, MCP deep test"
type: iteration
date: 2026-05-03
tags:
  - dogfood
  - review
  - mcp
  - crawl
  - edge-cases
status: completed
---

## Test Setup

Tested mdget after iteration 9 shipped. Focus areas: new iter-9 features (--path-prefix, static asset filtering, MCP crawl_site, batch_fetch metadata, user_agent param, excerpt fallback, multi-URL warning), edge cases not previously tested (non-English content, large tables, GitHub READMEs, RSS/XML, SPAs, redirects, code-heavy pages), MCP server deep test (all 4 tools + error handling), crawl engine verification, and composability/piping.

Pages fetched: Ruff docs (rules, tutorial, configuration), Rust Book (ownership, borrowing), React docs, Le Monde (French), Japanese Wikipedia, docs.rs/anyhow, GitHub/astral-sh/ruff, Wikipedia country population list, httpbin endpoints, blog.rust-lang.org/feed.xml, example.com, crates.io/serde.

## What Works Well

### Iter-9: Path-prefix auto-inference works correctly
`mdget crawl https://docs.astral.sh/ruff/rules/ --depth 1 --max-pages 5` auto-inferred prefix to `/ruff/` and all 5 fetched pages were under `/ruff/rules/*`. Explicit `--path-prefix /ruff/rules/` also works. No pages outside the prefix were fetched.

### Iter-9: Static asset filtering is fixed
Crawling the Rust Book chapter with `--depth 1 --max-pages 3` only followed `<a href>` links — fetched `print.html`, `ch03-02-data-types.html`, no `.woff2`, `.css`, or `.js` files appeared. The critical bug from dogfood #3 is resolved.

### Iter-9: Multi-URL stdout warning works
`mdget url1 url2` without output flags emits on stderr: `warning: multiple URLs to stdout — output is concatenated and hard to split. Use -O (auto-named files), -o FILE (single file), or the MCP batch_fetch tool instead.` Clear and actionable.

### Iter-9: MCP batch_fetch has structured metadata
`batch_fetch` returns top-level JSON fields: `url`, `title`, `content`, `word_count`, `excerpt`, `language`, `byline`. No more YAML-within-JSON parsing needed. Confirmed with 3-URL batch fetch — all fields present at JSON level.

### Iter-9: MCP user_agent parameter works
`batch_fetch` with `user_agent: "DogfoodTestBot/1.0"` confirmed via httpbin.org/user-agent — response shows the custom UA string. CLI `-A 'DogfoodBot/1.0'` also works.

### Iter-9: MCP crawl_site tool works
`crawl_site` returns structured JSON array with `url`, `title`, `content`, `word_count`, `depth` per page. `include_metadata` flag embeds YAML frontmatter in content. `path_prefix` parameter restricts crawl scope.

### Non-English content handles perfectly
- **French (Le Monde):** Accented characters (é, è, ê, ç, à) render correctly. Article text, image references, and structure all preserved.
- **Japanese (Wikipedia):** Kanji, hiragana, katakana all render correctly. Disambiguation note table formatted properly. Full Unicode support confirmed.

### GitHub README extraction is excellent
`mdget https://github.com/astral-sh/ruff` extracts the full README with emoji bullets, badges, links, lists, and code references. Title includes the repo description. Byline correctly set to "astral-sh". Excerpt from og:description is clean.

### Code-heavy pages render cleanly
Rust Book ownership chapter: code blocks are properly fenced, explanatory text preserved, figure references kept. The code examples are complete and correctly indented. Very usable for LLM context.

### SPA content extraction works (SSR)
React.dev `/learn` page — despite being a React SPA, server-side rendering means mdget gets full content. Code blocks, headings, explanatory text all clean. Tested with `--max-length 3000`.

### Composability and piping are excellent
- `mdget -q -m url1 url2 url3 | grep "^title:"` correctly extracts three title lines from metadata-only output
- `mdget -q --max-length 2000 --no-images url` produces exactly 1981 chars — respects limit precisely
- stderr/stdout separation confirmed: `2>/tmp/stderr` captures only the multi-URL warning, stdout has only content
- `grep`, `wc`, `head` all work naturally in pipeline

### Error handling is clean and informative
- Invalid URL: `invalid URL 'not-a-valid-url': relative URL without a base`
- Unreachable host: DNS error with full detail
- HTTP 404: `HTTP 404 Not Found fetching URL: <url>` — no confusing retries
- Unexpected content-type: warning on stderr, still attempts extraction

### Crawl --output-dir mirrors URL structure
`crawl --output-dir /tmp/test` creates nested directories: `ruff/tutorial/index.md`. Files include YAML frontmatter. Clean filesystem layout.

## What's Awkward or Broken

### BUG: docs.rs excerpt is still garbage
`mdget -m https://docs.rs/anyhow/latest/anyhow/` returns `excerpt: "github crates-io docs-rs"`. Investigation: docs.rs itself has `<meta name="description" content="github crates-io docs-rs">` — the meta tag fallback IS working, but the source data is junk. The fix from iter 9 correctly falls back to meta tags, but docs.rs is the problem.

**Possible improvement:** Fall back further to the first paragraph of extracted content if the meta description looks like a tag list (e.g., no spaces between words, or fewer than N words).

### ISSUE: MCP crawl_site can blow up context windows
Crawling `ch04-01-what-is-ownership.html` with `--depth 1 --max-pages 3` produced a 1.4M character result because it followed a link to `print.html` (the entire Rust Book). The MCP `crawl_site` tool has no `max_length` parameter to cap per-page content size.

**Impact:** An agent using `crawl_site` on a docs site that links to a "print all" page gets a context-window-busting response. This is a significant usability issue.

**Fix:** Add `max_length` parameter to `crawl_site` that truncates each page's content, matching the CLI and other MCP tools.

### AWKWARD: RSS/XML feed produces escaped XML soup
`mdget https://blog.rust-lang.org/feed.xml` shows warning `unexpected Content-Type 'application/xml'` then dumps ~96KB of escaped XML with backslash-escaped angle brackets. Unreadable and unusable.

**Fix options:** (a) Detect Atom/RSS feeds and extract entries as structured markdown (title, date, summary per entry), or (b) return a clear error like "XML/RSS feeds are not supported — use a feed parser".

### AWKWARD: Wikipedia large tables stripped by readability
`mdget --max-length 5000 https://en.wikipedia.org/wiki/List_of_countries_by_population_(United_Nations)` — the actual population data table is completely missing. Only "See also" links are shown. Readability strips the data table as non-article content.

This is an inherent readability limitation. The `--raw` flag would include the table but also all the navigation chrome.

### AWKWARD: crates.io returns 404 (JS-only SPA)
`mdget https://crates.io/crates/serde` returns `HTTP 404 Not Found`. crates.io is a pure client-side SPA that returns no content to non-JS user agents. Not mdget's bug, but worth noting as a common URL agents encounter. A friendlier error like "this page may require JavaScript rendering" would help.

### AWKWARD: Crawl auto-inference can be too narrow
Crawling `https://docs.astral.sh/ruff/tutorial/` auto-infers prefix to `/ruff/` but only 1 page is fetched because the tutorial page has no sub-pages under `/ruff/tutorial/`. Meanwhile, pages like `/ruff/configuration/` that the tutorial links to are within the `/ruff/` prefix but are separate sections. The auto-inference works correctly, but users may expect more pages.

### MINOR: --max-length 0 produces only `[Truncated]`
Edge case: `--max-length 0` outputs just the truncation marker with no content. Should probably be treated as "no limit" or produce an error.

### MINOR: -q flag position with crawl subcommand
`mdget crawl ... -q` fails with "unexpected argument '-q'". The flag must go before the subcommand: `mdget -q crawl ...`. This is standard clap behavior but surprising for users who type the subcommand first and add flags at the end.

## Output Quality Report

| Page | Quality | Notes |
|------|---------|-------|
| Ruff rules listing | Excellent | 900+ rules rendered as clean markdown tables with links. 21K words. |
| Ruff tutorial | Excellent | Code blocks, config examples, CLI output all preserved perfectly |
| Ruff individual rule pages | Excellent | Each rule page clean with code examples and explanation |
| Rust Book (ownership) | Excellent | Code blocks, concepts, figures referenced cleanly. ~4500 words. |
| Rust Book (borrowing) | Excellent | Full content with metadata. Excerpt auto-extracted from first paragraph. |
| React docs (/learn) | Good | Code blocks present but some JSX examples lose formatting (inline) |
| React useState reference | Good | API docs extracted well, code examples clean |
| Le Monde (French) | Good | Text and accents perfect, images referenced, structure reasonable for news |
| Japanese Wikipedia | Good | CJK characters perfect, tables formatted, some disambiguation table noise |
| GitHub README (ruff) | Excellent | Full README with emoji, badges, links, lists. Title includes description. |
| docs.rs/anyhow | Good | Content clean, but excerpt is junk (upstream issue) |
| Wikipedia population list | Poor | Data table completely missing. Only nav links shown. Readability limitation. |
| httpbin/html | Excellent | Clean Moby-Dick text extraction |
| httpbin/user-agent | Fair | JSON wrapped in code fence. Usable but title is "Untitled". |
| blog.rust-lang.org/feed.xml | Poor | Escaped XML soup. Not useful. |
| crates.io/serde | Fail | HTTP 404. JS-only SPA returns nothing. |
| example.com | Excellent | Perfect extraction of simple page |
| GitHub landing page | Good | Marketing copy extracted, excerpt from og:description. Reasonable. |

## Feature Ideas

### P1: `max_length` on MCP `crawl_site`
Without per-page content truncation, `crawl_site` is a footgun for agents. A single "print all" page can blow up the response. This is the highest-priority fix.

### P2: RSS/Atom feed detection
Either extract entries as markdown (title + date + summary per entry) or return a clear "not supported" error. Current escaped XML output is worse than an error.

### P2: Smarter excerpt fallback
When `<meta name="description">` looks like a tag list (no spaces, very short, or matches known junk patterns like "github crates-io docs-rs"), fall back to the first paragraph of extracted content instead.

### P3: `--quiet` on crawl subcommand directly
Allow `mdget crawl -q ...` in addition to `mdget -q crawl ...`. Users naturally type flags after the subcommand name.

### P3: JS-SPA detection hint
When a page returns 404 or empty content but has `<script>` tags and a `<noscript>` hint, suggest that the page may require JavaScript rendering.

### P3: `--max-length 0` should mean "no limit"
Treat 0 as "unlimited" instead of truncating to empty content. Or reject it with a clear error.

## Verdict

**Iteration 9 fixes land well.** The critical crawl bugs from dogfood #3 are resolved: static asset filtering works, path-prefix auto-inference works, MCP batch_fetch has clean structured metadata, crawl_site MCP tool is functional. The multi-URL warning and user_agent parameter work as specified.

**Core strengths remain strong.** Readability extraction on docs and articles is excellent. Non-English content (French, Japanese) works perfectly. GitHub README extraction is surprisingly good. Composability with Unix pipes is natural and clean. Error messages are clear and machine-parseable.

**One new P1 emerged:** MCP `crawl_site` needs a `max_length` parameter — without it, agents risk context window blowouts from large pages (1.4M chars from a single Rust Book crawl). This is the most important fix before agents can safely use `crawl_site`.

**Quality trajectory:** Output quality has improved steadily across dogfood sessions. The tool is production-ready for docs, articles, and GitHub pages. Weak spots remain on data-heavy pages (Wikipedia tables), pure SPAs (crates.io), and non-HTML content (RSS feeds), but these are inherent limitations of the readability approach, not regressions.
