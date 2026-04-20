---
title: "Output control & metadata"
type: iteration
date: 2026-04-17
tags: [iteration, output, metadata]
status: planned
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

- [ ] Implement `--metadata-only` / `-m` flag — fetch page, extract metadata, print only YAML frontmatter, skip body
- [ ] Implement `--include-metadata` flag
- [ ] Extract metadata from dom_smoothie readability output (title, byline, excerpt, published, language, site_name)
- [ ] Generate YAML frontmatter block — always: title, source, fetched, word_count; optional: byline, excerpt, published, language, site_name
- [ ] Implement `--no-images` flag — strip `![alt](url)` patterns from markdown output
- [ ] Implement `--max-length N` flag — truncate output to N characters (clean break at paragraph/sentence boundary if possible)
- [ ] Ensure flags compose correctly (e.g. `--include-metadata --no-images --max-length 3000`)
- [ ] Metadata word count should reflect post-processing (after image stripping, before truncation)
- [ ] Add e2e tests for each flag and combinations
- [ ] Run quality gates

## Design Decisions

- **`--metadata-only` for triage**: Agents often need to triage a list of URLs before deciding which to read fully. Printing just the frontmatter (title, word count, excerpt) lets them decide cheaply. Still requires a full fetch (readability needs the DOM), but skips markdown serialization and saves output tokens. Idea surfaced in [[iteration-02a-dogfood-fixes]] dogfood review.
- **Links always included**: hyperlinks are valuable context for LLMs (source attribution, navigation). No flag to strip them.
- **Images are noise for LLMs**: LLMs can't see images, so `![alt](url)` references waste tokens. `--no-images` strips them cleanly.
- **Truncation is character-based**: token counting is model-specific and adds complexity. Characters are universal and predictable. Users can estimate tokens from character count.

## Readability tuning flags (advanced)

Expose select dom_smoothie `Config` options as advanced CLI flags. These let users fix bad extractions without falling back to `--raw`. Show only in `--help` (not `-h`) to keep the simple interface clean.

```
mdget --char-threshold 200 <URL>         # lower = accept shorter articles (default 500)
mdget --candidate-mode dom-smoothie <URL> # alternative content detection algorithm
mdget --max-elements 50000 <URL>         # safety limit on DOM size (default unlimited)
```

See [[dom-query-escaping]] for research on these options.

### Tasks (additional)

- [ ] Add `--char-threshold` flag (maps to `dom_smoothie::Config::char_threshold`)
- [ ] Add `--candidate-mode` flag (maps to `dom_smoothie::Config::candidate_select_mode`, values: `readability`, `dom-smoothie`)
- [ ] Add `--max-elements` flag (maps to `dom_smoothie::Config::max_elements_to_parse`)
- [ ] Hide these from `-h` short help, show only in `--help`
- [ ] Add e2e tests for each flag
