---
title: Output control & metadata
type: iteration
date: 2026-04-17
tags:
  - iteration
  - output
  - metadata
status: completed
branch: iter-4/output-control
---

## Goal

Add flags to control markdown output: prepend YAML metadata frontmatter, strip image references, and truncate output length. Tailor output for LLM consumption.

## CLI Interface

```
mdget --include-metadata <URL>     # prepend YAML frontmatter with title, URL, date, word count
mdget --metadata-only <URL>        # print only YAML frontmatter, skip body (saves context/tokens)
mdget --no-images <URL>            # strip ![alt](url) image references
mdget --max-length 5000 <URL>      # truncate output to N characters
```

## Metadata Frontmatter Format

```yaml
---
title: "Page Title"
source: "https://example.com/article"
fetched: 2026-04-17T12:00:00Z
word_count: 1523
byline: "Author Name"
excerpt: "Short description of the article content..."
published: 2026-04-15
language: en
site_name: "Example News"
---
```

Fields are included only when available from the page. `title`, `source`, `fetched`, and `word_count` are always present. The rest (`byline`, `excerpt`, `published`, `language`, `site_name`) come from dom_smoothie's readability extraction and are omitted when the page doesn't provide them.

## Tasks

- [x] Implement `--metadata-only` / `-m` flag — fetch page, extract metadata, print only YAML frontmatter, skip body
- [x] Implement `--include-metadata` flag
- [x] Extract metadata from dom_smoothie readability output (title, byline, excerpt, published, language, site_name)
- [x] Generate YAML frontmatter block — always: title, source, fetched, word_count; optional: byline, excerpt, published, language, site_name
- [x] Implement `--no-images` flag — strip `![alt](url)` patterns from markdown output
- [x] Implement `--max-length N` flag — truncate output to N characters (clean break at paragraph/sentence boundary if possible)
- [x] Ensure flags compose correctly (e.g. `--include-metadata --no-images --max-length 3000`)
- [x] Metadata word count should reflect post-processing (after image stripping, before truncation)
- [x] Add e2e tests for each flag and combinations
- [x] Run quality gates

## Design Decisions

- **`--metadata-only` for triage**: Agents often need to triage a list of URLs before deciding which to read fully. Printing just the frontmatter (title, word count, excerpt) lets them decide cheaply. Still requires a full fetch (readability needs the DOM), but skips markdown serialization and saves output tokens. Idea surfaced in [[iterations/done/iteration-02a-dogfood-fixes]] dogfood review.
- **Links always included**: hyperlinks are valuable context for LLMs (source attribution, navigation). No flag to strip them.
- **Images are noise for LLMs**: LLMs can't see images, so `![alt](url)` references waste tokens. `--no-images` strips them cleanly.
- **Truncation is character-based**: token counting is model-specific and adds complexity. Characters are universal and predictable. Users can estimate tokens from character count.

## Deferred: readability tuning flags

dom_smoothie exposes config options (`char_threshold`, `candidate_select_mode`, `max_elements_to_parse`) that could theoretically improve extraction on edge-case pages. However, these are deep engine internals — if extraction fails, `--raw` is the pragmatic escape hatch. Adding niche flags would clutter the CLI for negligible benefit. Revisit only if users report specific extraction failures where these knobs would help. See [[research/dom-query-escaping]] for full research.
