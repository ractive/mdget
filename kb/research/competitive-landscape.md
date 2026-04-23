---
title: "Competitive Landscape: URL-to-Markdown Tools"
type: research
date: 2026-04-17
tags:
  - research
  - competitive-analysis
  - url-to-markdown
  - llm-tools
status: completed
---

# Competitive Landscape: URL-to-Markdown Tools

Research into existing CLI tools, libraries, and services that fetch a URL and return its content as Markdown, with focus on LLM/AI agent use cases. Conducted 2026-04-17.

See also: [[research/html-to-markdown-crates]] for detailed Rust library comparison.

---

## Category 1: Rust CLI Tools (Direct Competitors)

### 1. readable_url

- **Language:** Rust
- **URL:** https://github.com/masukomi/readable_url
- **Crate:** [readable_url](https://crates.io/crates/readable_url) v1.0.0 (2022-11-17)
- **Stars:** 2
- **Type:** CLI
- **What it does:** Fetch URL -> readability extract -> output as HTML or Markdown
- **Pipeline:** Downloads URL, applies arc90 readability algorithm, optionally converts to Markdown
- **Maintenance:** Single release in 2022. Author's own README (updated Feb 2026) says: "While this works, Rust's readability library does a notably worse job of extracting content when compared to the Golang implementation. I recommend you use the Golang reader utility instead."
- **Verdict:** Abandoned, author disowns it. Uses the old `readability` crate (kumabook). Exactly the concept mdget aims for, but abandoned and acknowledged as poor quality.

### 2. twars-url2md

- **Language:** Rust
- **URL:** https://github.com/twardoch/twars-url2md
- **Crate:** [twars-url2md](https://crates.io/crates/twars-url2md) v1.4.2 (2025-04-07)
- **Stars:** 5
- **Downloads:** 8,947
- **Type:** CLI + library
- **What it does:** Fetches multiple URLs, cleans HTML, converts to Markdown files
- **Pipeline:** libcurl fetch -> HTML cleaning/pruning -> htmd conversion -> save as .md files
- **Key features:** Batch processing, async concurrency, exponential backoff, browser-like headers, CDN compatibility, output as individual files or single packed file
- **No readability step:** Does NOT extract article content -- converts full page HTML, with only basic element pruning (removes script/style/nav/footer tags)
- **Maintenance:** Last release Apr 2025, 14 versions. Moderately active.
- **Verdict:** Closest Rust CLI competitor. Well-engineered for batch URL fetching. But lacks readability/article extraction -- it converts the full page HTML, which means output includes nav, ads, sidebars, etc. Not designed for LLM consumption.

### 3. llmtext

- **Language:** Rust
- **URL:** https://github.com/ayub-kokabi/llmtext
- **Crate:** [llmtext](https://crates.io/crates/llmtext) v1.0.0 (2025-08-03)
- **Stars:** 13
- **Downloads:** 1,816
- **Type:** CLI
- **What it does:** Scrapes a website and ALL its internal linked pages into a single Markdown file
- **Pipeline:** Fetches start URL -> discovers internal links -> scrapes all pages -> merges into one .md
- **Use case:** Creating full-site context dumps for LLMs (entire docs sites, etc.)
- **No readability step:** Basic HTML-to-Markdown conversion, no article extraction
- **Maintenance:** Single v1.0.0 release (Aug 2025). Dormant since Jan 2026.
- **Verdict:** Different scope -- site-wide crawling, not single-page extraction. No readability. Low adoption.

### 4. markdown-harvest

- **Language:** Rust
- **URL:** https://github.com/franciscotbjr/markdown-harvest
- **Crate:** [markdown-harvest](https://crates.io/crates/markdown-harvest) v0.1.6 (2026-01-04)
- **Stars:** ~5
- **Downloads:** 2,177
- **Type:** Library (not CLI)
- **What it does:** Extracts URLs from text messages, fetches them, converts to Markdown
- **Pipeline:** Text input -> URL extraction -> HTTP fetch -> HTML-to-Markdown -> chunking for RAG
- **Key features:** Built for RAG systems, smart content extraction (removes ads/nav), batch processing, semantic chunking with overlap
- **Maintenance:** 7 releases, last Jan 2026. Moderate activity.
- **Verdict:** Library only (no CLI). Designed for embedding in RAG pipelines. Has some content cleaning but unclear if it uses readability-style extraction or just tag-stripping. Low adoption.

### 5. web-capture

- **Language:** Rust + JavaScript (dual implementation)
- **URL:** https://github.com/link-assistant/web-capture
- **Crate:** [web-capture](https://crates.io/crates/web-capture) v0.3.1 (2026-04-14)
- **Stars:** 0
- **Downloads:** 64
- **Type:** CLI + microservice
- **What it does:** Fetches URL, outputs as Markdown, HTML, PNG, PDF, DOCX, or ZIP
- **Pipeline:** Headless browser rendering -> HTML-to-Markdown conversion (+ screenshot/PDF options)
- **Key features:** Multiple output formats, API server mode, image extraction, ZIP archives
- **Maintenance:** Brand new (April 2026), 5 releases in 4 days. Very early.
- **Verdict:** Interesting multi-format approach but requires headless browser (heavy). Zero adoption so far. Too new to evaluate.

### 6. readability-js-cli

- **Language:** Rust (wrapping Mozilla Readability.js via embedded JS runtime)
- **URL:** https://github.com/egemengol/readability-js
- **Crate:** [readability-js-cli](https://crates.io/crates/readability-js-cli) v0.1.5 (2025-10-03)
- **Stars:** 6
- **Downloads:** 1,252
- **Type:** CLI + library
- **What it does:** Extracts clean article content using actual Mozilla Readability.js, outputs Markdown
- **Pipeline:** HTML input -> Readability.js (via embedded JS runtime) -> clean HTML -> Markdown
- **Key features:** Uses the real Readability.js algorithm (same as Firefox Reader Mode), CLI outputs Markdown, library outputs clean HTML/text
- **Maintenance:** 5 releases, last Oct 2025.
- **Verdict:** Interesting approach -- gets 100% Readability.js fidelity by embedding the real JS. But: embeds a JavaScript runtime (boa or quickjs), adding ~30ms per init. Does NOT fetch URLs -- reads from stdin or file only. You'd need to pipe curl output into it.

### 7. rs-trafilatura

- **Language:** Rust
- **URL:** https://github.com/Murrough-Foley/rs-trafilatura
- **Stars:** 10
- **Type:** Library (not CLI, not on crates.io yet)
- **What it does:** Rust port of trafilatura -- ML-based web content extraction
- **Pipeline:** HTML input -> page-type classification (XGBoost) -> per-type content extraction -> Markdown/text output
- **Key features:** ML page-type classification (7 types), per-type extraction profiles, confidence scoring, GFM Markdown output, F1=0.966 on ScrapingHub benchmark
- **Maintenance:** Active (updated Apr 2026), but very new.
- **Verdict:** Most sophisticated content extraction in Rust. ML-based approach is more accurate than readability heuristics for diverse page types. Not a CLI, not on crates.io yet. Could be a powerful library dependency once published.

---

## Category 2: Non-Rust CLI / Desktop Tools

### 8. reader (Go)

- **Language:** Go
- **URL:** https://github.com/mrusme/reader (also on Codeberg)
- **Stars:** 400
- **Type:** CLI
- **What it does:** Fetches URL, applies readability, renders as highlighted terminal text or raw Markdown
- **Pipeline:** Fetch URL -> go-readability extraction -> terminal rendering (or raw Markdown with `-o` flag)
- **Key features:** Terminal rendering with syntax highlighting, image block-rendering in terminal, EML file support, stdin support
- **Maintenance:** Active (updated Apr 2026).
- **Verdict:** The Go tool that readable_url's author recommends instead. Well-maintained, good star count. Its primary focus is terminal rendering (like a TUI reader), but `-o` flag outputs raw Markdown. Uses go-readability which is a mature port. This is the closest existing tool to what mdget would be, but it's in Go and oriented toward human terminal reading, not LLM consumption.

### 9. Monolith (Rust)

- **Language:** Rust
- **URL:** https://github.com/Y2Z/monolith
- **Stars:** 15,008
- **Downloads (crates.io):** 128,603
- **Type:** CLI + library
- **What it does:** Saves a complete web page as a single HTML file (inlines CSS, JS, images as base64)
- **Maintenance:** Active, very popular.
- **Verdict:** Not a URL-to-Markdown tool. Saves full pages as self-contained HTML. No readability, no Markdown conversion. Used by twars-url2md as a dependency for HTML fetching. Relevant as a building block, not a competitor.

### 10. Trafilatura (Python)

- **Language:** Python
- **URL:** https://github.com/adbar/trafilatura
- **Stars:** 5,733
- **Type:** CLI + library
- **What it does:** Web content extraction with multiple output formats (Markdown, HTML, text, XML, CSV, JSON)
- **Pipeline:** Fetch URL -> boilerplate removal -> content extraction -> format conversion
- **Key features:** CLI and library, crawling support, metadata extraction, multi-format output, academic-grade extraction quality
- **Maintenance:** Very active (updated daily).
- **Verdict:** Gold standard for web content extraction. Python-only. Frequently cited in academic papers. Outputs Markdown. The tool mdget should aspire to match in quality. rs-trafilatura is the in-progress Rust port.

### 11. html2text (Python, by Aaron Swartz)

- **Language:** Python
- **URL:** https://github.com/Alir3z4/html2text (maintained fork)
- **Stars:** 2,143 (fork) / 2,838 (original)
- **Type:** CLI + library
- **What it does:** Converts HTML to Markdown-formatted plain text
- **Maintenance:** Maintained fork is active.
- **Verdict:** Pure converter, no fetching, no readability. Long-standing Python tool.

---

## Category 3: Hosted Services / APIs

### 12. Jina Reader (reader.jina.ai)

- **Language:** TypeScript (server-side)
- **URL:** https://github.com/jina-ai/reader
- **Stars:** 10,586
- **Type:** Hosted API service
- **What it does:** Prefix any URL with `https://r.jina.ai/` to get LLM-friendly Markdown
- **Pipeline:** Headless browser fetch -> content extraction -> Markdown output
- **Key features:** Zero setup, just prepend URL. Handles JS-rendered pages. Search mode (`s.jina.ai`). Free tier available.
- **Maintenance:** Very active (updated daily).
- **Verdict:** The most popular tool in this space. But: it's a hosted service (privacy/latency concerns), requires internet for the proxy, rate-limited free tier, paid for heavy use. Not a local CLI tool. Multiple MCP server wrappers exist for it (mcp-jina-reader, mcp-jinaai-reader).

### 13. Firecrawl

- **Language:** TypeScript
- **URL:** https://github.com/firecrawl/firecrawl
- **Stars:** 110,365
- **Type:** Hosted API + self-hostable service
- **What it does:** Web scraping API that returns clean Markdown. Handles JS rendering, crawling, structured extraction.
- **Key features:** LLM-optimized output, structured data extraction, crawl entire sites, JS rendering, self-hostable
- **Maintenance:** Extremely active, well-funded startup.
- **Verdict:** Most popular tool in the broader space. But: complex infrastructure (requires browser, queue, etc.), primarily a service not a CLI. Self-hosting requires Docker + Redis + browser. Overkill for "give me this one URL as Markdown." Has an official MCP server (6K stars).

### 14. Crawl4AI (Python)

- **Language:** Python
- **URL:** https://github.com/unclecode/crawl4ai
- **Stars:** 64,184
- **Type:** Library + API
- **What it does:** Open-source web crawler/scraper designed for LLMs and AI agents
- **Key features:** LLM-friendly Markdown output, JS rendering, structured extraction, session management
- **Maintenance:** Very active.
- **Verdict:** Python library, not a CLI tool. Focused on programmatic use in AI agent pipelines. Heavy (requires Playwright/browser).

---

## Category 4: MCP Servers / AI Agent Tools

### 15. Official MCP Fetch Server (modelcontextprotocol/servers)

- **Language:** Python
- **URL:** https://github.com/modelcontextprotocol/servers/tree/main/src/fetch
- **Stars:** 83,996 (parent repo)
- **Type:** MCP server
- **What it does:** Fetches URL content, converts HTML to Markdown, returns to LLM
- **Pipeline:** Fetch URL -> HTML-to-Markdown (using readabilipy or node-readability if available) -> truncated output
- **Key features:** Pagination support (start_index), max_length truncation, raw mode
- **Maintenance:** Actively maintained as part of official MCP servers.
- **Verdict:** The default way Claude/LLMs fetch web pages. But: Python, requires uv/pip install, basic extraction quality, truncates output. Not optimized for quality -- it's a quick-and-dirty fetch.

### 16. Firecrawl MCP Server

- **Language:** JavaScript
- **URL:** https://github.com/firecrawl/firecrawl-mcp-server
- **Stars:** 6,079
- **Type:** MCP server (wraps Firecrawl API)
- **Verdict:** Requires Firecrawl API key. Service dependency.

### 17. Jina Reader MCP Servers

- Multiple implementations: mcp-jina-reader (47 stars), mcp-jinaai-reader (31 stars)
- **Type:** MCP servers wrapping Jina Reader API
- **Verdict:** Service dependency on reader.jina.ai.

### 18. Claude Code Built-in WebFetch

- **Type:** Built-in tool in Claude Code
- **What it does:** Fetches URL, converts HTML to Markdown, processes with a small model
- **Limitations:** Summarizes large content, 15-min cache, cannot handle authenticated URLs, HTTPS only
- **Verdict:** Already available but limited -- processes content through an LLM (lossy), not raw Markdown output. Cannot be piped or used offline.

---

## Category 5: Rust Building-Block Libraries (Not Competitors, But Components)

### 19. spider-rs/spider

- **URL:** https://github.com/spider-rs/spider (2,431 stars)
- **Type:** Rust web crawler/scraper framework
- **Includes:** fast_html2md, llm_readability, spider_transformations
- **Verdict:** Ecosystem of crates for crawling + conversion. Could be used as dependencies. The llm_readability crate specifically targets LLM use cases.

### 20. dom_smoothie

- **URL:** https://github.com/niklak/dom_smoothie (202 stars)
- **Type:** Rust library (readability + Markdown output)
- **Verdict:** Best readability library in Rust ecosystem. Has built-in Markdown output mode. Key building block for mdget. See [[research/html-to-markdown-crates]] for details.

### 21. htmd

- **URL:** https://github.com/letmutex/htmd (437 stars)
- **Type:** Rust library (HTML-to-Markdown)
- **Verdict:** Best pure HTML-to-Markdown converter in Rust. See [[research/html-to-markdown-crates]] for details.

---

## Gap Analysis: What mdget Would Fill

### The key question answered

**No, there is no well-maintained Rust CLI that does exactly "fetch URL -> readability extract -> markdown output" for LLM consumption.**

The closest candidates and why they fall short:

| Tool | Fetch? | Readability? | Markdown? | CLI? | Maintained? | Gap |
|------|--------|-------------|-----------|------|-------------|-----|
| readable_url | Yes | Yes (poor) | Yes | Yes | Abandoned | Author disowns it, poor extraction quality |
| twars-url2md | Yes | No | Yes | Yes | Moderate | No readability -- outputs full page junk |
| llmtext | Yes | No | Yes | Yes | Dormant | No readability, different scope (site-wide) |
| readability-js-cli | No | Yes (excellent) | Yes | Yes | Low | No URL fetching, embeds JS runtime |
| reader (Go) | Yes | Yes | Yes | Yes | Active | Go, not Rust. Terminal-oriented, not LLM-focused |
| web-capture | Yes | No | Yes | Yes | Brand new | No readability, requires headless browser |
| markdown-harvest | Yes | Partial | Yes | No | Low | Library only, no CLI |

### Gaps mdget would fill

1. **No Rust CLI combines all three: fetch + readability + Markdown.** This is the core gap. Each existing tool is missing at least one step.

2. **LLM-optimized output.** Jina Reader and Firecrawl target this but are hosted services. No local CLI tool specifically optimizes Markdown output for LLM token efficiency.

3. **Lightweight and self-contained.** Jina, Firecrawl, Crawl4AI all require either a hosted service, Docker, or a browser runtime. A single Rust binary with no external dependencies would be unique.

4. **Privacy-preserving.** Jina Reader and Firecrawl route content through third-party servers. A local tool keeps content local.

5. **Speed.** Go's `reader` is decent but Rust can be faster. Python tools (trafilatura, crawl4ai) are significantly slower.

6. **MCP-ready.** Building as a CLI that could also expose an MCP interface would make it immediately useful for Claude Code, Cursor, and other AI coding tools without depending on external services.

### Competitive positioning

- vs. **Jina Reader:** Local, private, no rate limits, no API key, works offline, faster
- vs. **Firecrawl:** Lightweight single binary vs. complex infrastructure
- vs. **trafilatura:** Same quality goal but as a fast Rust binary, not Python
- vs. **reader (Go):** LLM-focused output (not terminal rendering), Rust performance
- vs. **readability-js-cli:** Includes fetching, pure Rust (no embedded JS runtime)
- vs. **twars-url2md:** Adds readability extraction for clean, focused content

### Recommended architecture using existing Rust crates

Based on this research and [[research/html-to-markdown-crates]]:

1. **HTTP fetch:** `reqwest` (async, handles redirects, compression, timeouts)
2. **Content extraction:** `dom_smoothie` (readability.js port with built-in Markdown mode)
3. **Alternative conversion:** `htmd` (if dom_smoothie's Markdown output needs more control)
4. **Fallback/advanced:** Consider `rs-trafilatura` once it's published to crates.io
