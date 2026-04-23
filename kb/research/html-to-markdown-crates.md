---
title: "Rust HTML-to-Markdown Crate Research"
type: research
date: 2026-04-17
tags:
  - rust
  - html-to-markdown
  - dependencies
  - research
status: completed
---

# Rust HTML-to-Markdown Crate Research

Research into actively maintained Rust libraries for converting HTML to Markdown, conducted 2026-04-17.

## Summary Table

| Crate | Version | Last Published | Downloads (total) | License | Maintained? |
|---|---|---|---|---|---|
| htmd | 0.5.4 | 2026-04-04 | 452K | Apache-2.0 | Yes |
| html-to-markdown-rs | 3.2.4 | 2026-04-17 | 205K | MIT | Yes (very active) |
| html2md | 0.2.15 | 2025-01-12 | 541K | GPL-3.0+ | Low activity |
| fast_html2md | 0.0.61 | 2026-03-29 | 174K | MIT | Yes |
| mdka | 2.0.3 | 2026-04-16 | 102K | Apache-2.0 | Yes |
| dom_smoothie | 0.17.0 | 2026-03-28 | 49K | MIT | Yes |
| readabilityrs | 0.1.3 | 2026-04-05 | 71K | Apache-2.0 | Yes (new) |
| html_to_markdown (Zed) | 0.1.0 | 2024-07-01 | 70K | Apache-2.0 | Stale (single release) |
| llm_readability | 0.0.13 | 2026-02-03 | 42K | MIT | Low activity |
| readability-rust | 0.1.0 | 2025-07-22 | 27K | Apache-2.0 | Low activity |
| readable-readability | 0.4.0 | 2022-12-17 | 27K | MIT | Abandoned |

---

## Tier 1: Actively Maintained HTML-to-Markdown Converters

### 1. htmd

- **Crate:** [htmd](https://crates.io/crates/htmd)
- **GitHub:** https://github.com/letmutex/htmd (437 stars)
- **Version:** 0.5.4 (published 2026-04-04)
- **Downloads:** 452K total, 279K recent
- **License:** Apache-2.0
- **Content extraction:** No -- pure HTML-to-Markdown conversion only
- **Key features:**
  - Inspired by turndown.js; passes all turndown.js test cases
  - HTML table to Markdown table conversion
  - Custom tag handlers (pluggable per-element conversion)
  - Skip tags option (e.g., strip `<script>`, `<style>`)
  - Heading style options (ATX vs Setex)
  - Faithful mode (preserves HTML for unsupported tags)
  - Fast: ~16ms for a 1.37MB Wikipedia page on Apple M4
  - CLI companion: `htmd-cli`
- **Runtime dependencies:** html5ever, markup5ever_rcdom, phf
- **Async runtime needed:** No
- **Assessment:** Best pure HTML-to-Markdown converter. Minimal deps, well-tested against turndown.js suite, actively maintained, good API design. The clear front-runner for raw conversion.

### 2. html-to-markdown-rs (Kreuzberg)

- **Crate:** [html-to-markdown-rs](https://crates.io/crates/html-to-markdown-rs)
- **GitHub:** https://github.com/kreuzberg-dev/html-to-markdown (654 stars)
- **Version:** 3.2.4 (published 2026-04-17 -- today)
- **Downloads:** 205K total, 165K recent
- **License:** MIT
- **Content extraction:** Partial -- has metadata extraction (title, headers, links, images, JSON-LD, Microdata, RDFa) but not readability-style article extraction
- **Key features:**
  - 150-280 MB/s throughput (claims 10-80x faster than Python alternatives)
  - 12 language bindings (Rust, Python, Node, Ruby, PHP, Go, Java, C#, Elixir, R, C FFI, WASM)
  - Structured result: returns content, metadata, tables, images, warnings
  - Visitor pattern for custom callbacks, URL rewriting
  - Table extraction (structured cell data)
  - HTML sanitization via ammonia
  - Inline images feature (base64 embedding)
  - CommonMark compliant output
- **Runtime dependencies:** ahash, astral-tl, base64, html-escape, html5ever, lru, memchr, once_cell, regex, thiserror. Optional: image, serde, serde_json
- **Async runtime needed:** No
- **Feature flags:** `default`, `full`, `inline-images`, `metadata`, `serde`, `visitor`
- **Assessment:** Most feature-rich option. Very actively maintained (109 releases). Heavier dependency tree than htmd. The astral-tl parser is a newer/less battle-tested HTML parser vs html5ever. Good choice if you need metadata extraction or structured output alongside markdown.

### 3. mdka

- **Crate:** [mdka](https://crates.io/crates/mdka)
- **GitHub:** https://github.com/nabbisen/mdka-rs (50 stars)
- **Version:** 2.0.3 (published 2026-04-16)
- **Downloads:** 102K total, 12K recent
- **License:** Apache-2.0
- **Content extraction:** No -- pure conversion
- **Key features:**
  - HTML to Markdown converter
  - Actively maintained (75 versions, frequent updates)
- **Runtime dependencies:** ego-tree, scraper, thiserror. Optional: rayon
- **Async runtime needed:** No
- **Assessment:** Actively maintained but lower adoption and fewer stars than htmd. Uses scraper (which wraps html5ever + selectors). Less feature documentation available. Benchmarks in its dev-deps against htmd, html2md, fast_html2md, and html-to-markdown-rs -- suggesting the author is performance-conscious.

---

## Tier 2: Readability / Content Extraction Libraries

These extract the main article content from a web page (removing nav, ads, sidebars, etc.), similar to Mozilla's Readability.js or Safari Reader.

### 4. dom_smoothie

- **Crate:** [dom_smoothie](https://crates.io/crates/dom_smoothie)
- **GitHub:** https://github.com/niklak/dom_smoothie (202 stars)
- **Version:** 0.17.0 (published 2026-03-28)
- **Downloads:** 49K total, 26K recent
- **License:** MIT
- **Content extraction:** Yes -- faithful port of Mozilla's readability.js
- **Key features:**
  - Closely follows readability.js implementation
  - Outputs: HTML content, plain text content, and Markdown (`TextMode::Markdown`)
  - Metadata parsing: title, byline, excerpt, site name, published/modified time, image, URL
  - JSON-LD metadata support
  - Configurable (max elements, char thresholds, etc.)
  - WASM compatible (wasm-bindgen-test in dev-deps)
  - Optional aho-corasick and serde features
- **Runtime dependencies:** dom_query, flagset, foldhash, gjson, html-escape, once_cell, phf, tendril, thiserror, unicode-segmentation
- **Async runtime needed:** No
- **Assessment:** The most promising readability library. Active development, good star count, faithful readability.js port with Markdown output mode built in. This is the closest thing to "readability + markdown conversion" in one crate.

### 5. readabilityrs

- **Crate:** [readabilityrs](https://crates.io/crates/readabilityrs)
- **GitHub:** https://github.com/theiskaa/readabilityrs (74 stars)
- **Version:** 0.1.3 (published 2026-04-05)
- **Downloads:** 71K total, 71K recent (all recent -- very new)
- **License:** Apache-2.0
- **Content extraction:** Yes -- port of Mozilla's Readability
- **Key features:**
  - Mozilla Readability port
  - Outputs HTML article content
  - Does NOT output Markdown (would need a separate converter)
- **Runtime dependencies:** bitflags, kuchikikiki, once_cell, regex, scraper, serde, serde_json, thiserror, url, v_htmlescape
- **Async runtime needed:** No
- **Assessment:** New but gaining traction fast. Heavier dependency set (kuchikikiki, scraper). Outputs HTML only -- you'd still need htmd or similar to convert to Markdown.

### 6. llm_readability

- **Crate:** [llm_readability](https://crates.io/crates/llm_readability)
- **GitHub:** https://github.com/spider-rs/llm-readability (17 stars)
- **Version:** 0.0.13 (published 2026-02-03)
- **Downloads:** 42K total, 12K recent
- **License:** MIT
- **Content extraction:** Yes -- readability focused on LLM consumption
- **Key features:**
  - From the spider-rs ecosystem (same author as fast_html2md and spider_transformations)
  - Designed to produce clean text suitable for LLM input
- **Runtime dependencies:** auto_encoder, html5ever, markup5ever, markup5ever_rcdom, regex, url
- **Async runtime needed:** No
- **Assessment:** Niche -- focused on preparing text for LLM consumption rather than general readability. Part of the spider-rs ecosystem. Low star count, less documented.

---

## Tier 3: Usable But Less Ideal

### 7. html2md

- **Crate:** [html2md](https://crates.io/crates/html2md)
- **GitHub (GitLab):** https://gitlab.com/Kanedias/html2md
- **Version:** 0.2.15 (published 2025-01-12)
- **Downloads:** 541K total, 120K recent
- **License:** GPL-3.0+ (viral license -- significant consideration)
- **Content extraction:** No -- pure conversion
- **Key features:**
  - Oldest and most downloaded HTML-to-MD crate in the ecosystem
  - Handles basic HTML elements, links, images, code blocks
  - Android JNI support
- **Runtime dependencies:** html5ever, lazy_static, markup5ever_rcdom, percent-encoding, regex
- **Async runtime needed:** No
- **Assessment:** High download count due to being the first mover, but GPL-3.0+ license is a deal-breaker for many projects. Uses an older html5ever (0.27). The previous release before 0.2.15 was in 2022 -- effectively low-maintenance. The GPL license alone makes this a non-starter for most commercial or Apache/MIT-licensed projects.

### 8. fast_html2md

- **Crate:** [fast_html2md](https://crates.io/crates/fast_html2md)
- **GitHub:** https://github.com/spider-rs/html2md (69 stars)
- **Version:** 0.0.61 (published 2026-03-29)
- **Downloads:** 174K total, 63K recent
- **License:** MIT
- **Content extraction:** No -- pure conversion
- **Key features:**
  - Fork/rewrite of html2md with MIT license
  - Feature flags: `rewriter` (lol_html), `scraper` (html5ever), `stream` (futures-util)
  - Streaming support via futures-util (optional)
  - Part of spider-rs ecosystem
- **Runtime dependencies (default):** auto_encoder, lazy_static, lol_html, percent-encoding, regex, url, futures-util. Optional: html5ever, markup5ever_rcdom
- **Async runtime needed:** No for core; optional stream feature needs futures-util
- **Assessment:** Active but version 0.0.x suggests API instability. The default feature set pulls in lol_html (Cloudflare's HTML rewriter) which is a heavier dep. The `auto_encoder` dep (character encoding detection) adds weight. 57 releases in ~2 years indicates churn. Uses older html5ever 0.27 in the scraper feature.

### 9. html_to_markdown (Zed)

- **Crate:** [html_to_markdown](https://crates.io/crates/html_to_markdown)
- **GitHub:** https://github.com/zed-industries/zed (monorepo)
- **Version:** 0.1.0 (published 2024-07-01)
- **Downloads:** 70K total, 7K recent
- **License:** Apache-2.0
- **Content extraction:** No
- **Runtime dependencies:** anyhow, html5ever, markup5ever_rcdom, regex
- **Assessment:** Published from the Zed editor monorepo. Single release, never updated. Likely for internal use only. Not recommended as a standalone dependency -- it could break or be yanked if Zed reorganizes.

---

## Tier 4: Abandoned or Not Recommended

### 10. readability-rust

- **Crate:** [readability-rust](https://crates.io/crates/readability-rust)
- **GitHub:** https://github.com/dreampuf/readability-rust (20 stars)
- **Version:** 0.1.0 (published 2025-07-22)
- **Downloads:** 27K total
- **License:** Apache-2.0
- **Assessment:** Single release, last GitHub push 2025-11. Includes clap as a runtime dependency (appears to be a binary, not a pure library). 5MB crate size suggests bundled test fixtures. Not recommended.

### 11. readable-readability

- **Crate:** [readable-readability](https://crates.io/crates/readable-readability)
- **GitHub:** https://github.com/readable-app/readability.rs (19 stars)
- **Version:** 0.4.0 (published 2022-12-17)
- **License:** MIT
- **Assessment:** Last update December 2022. Last GitHub push April 2024. Effectively abandoned.

### 12. readable-rs

- **Crate:** [readable-rs](https://crates.io/crates/readable-rs)
- **GitHub:** https://github.com/Ahmed-Ali/readable-rs
- **Version:** 0.1.2 (published 2026-02-05)
- **Downloads:** 92
- **Assessment:** Brand new, near-zero adoption. Too early to evaluate.

---

## Recommendations

### For pure HTML-to-Markdown conversion: **htmd**

- Lightest dependency footprint (html5ever + phf only)
- Battle-tested against turndown.js test suite
- Good API: builder pattern, custom handlers, skip tags
- Table support built in
- Apache-2.0 license
- 437 GitHub stars, active maintenance

### For conversion + metadata extraction: **html-to-markdown-rs**

- If you need structured metadata (title, links, images, JSON-LD) alongside markdown
- Heaviest option but most feature-rich
- MIT license, very actively maintained
- 654 GitHub stars

### For readability (article extraction) + markdown: **dom_smoothie**

- Best readability library in the Rust ecosystem
- Has built-in Markdown output mode (`TextMode::Markdown`)
- Could potentially replace both a readability step and a conversion step
- MIT license, actively maintained
- 202 GitHub stars

### Recommended combination for mdget

If the goal is to fetch a web page and produce clean markdown of its main content:

1. **dom_smoothie** for content extraction (readability) with Markdown output -- single crate does both
2. OR: **dom_smoothie** (readability, HTML output) + **htmd** (HTML-to-Markdown) for more control over the markdown conversion
3. OR: **htmd** alone if readability/content extraction is not needed

All three options are sync-only (no async runtime required), MIT or Apache-2.0 licensed, and actively maintained.

### Crates to avoid

- **html2md** -- GPL-3.0+ license
- **html_to_markdown** (Zed) -- stale single release from monorepo
- **readable-readability** -- abandoned
- **readability-rust** -- single release, questionable packaging
