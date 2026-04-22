---
title: "dom_query markdown escaping: root cause & options"
type: research
date: 2026-04-20
tags: [dom_query, dom_smoothie, escaping, markdown, quality]
---

## Problem

dom_query's markdown serializer (`MDSerializer`) escapes all characters in a hardcoded `ESCAPE_CHARS` list, regardless of context:

```rust
// dom_query/src/serializing/md/constants.rs
const ESCAPE_CHARS: &[char] = &[
    '`', '*', '_', '{', '}', '[', ']', '<', '>', '(', ')', '#', '+', '.', '!', '|', '"',
];
```

This produces `Hello\!`, `foo\.bar`, `\(example\)` etc. in all text nodes.

## Why it escapes

When converting HTML text content to markdown, characters that are *literal* in HTML can be *syntactic* in markdown. For example, a literal `*` in HTML would trigger emphasis in markdown. So escaping is fundamentally correct — the problem is the **character set is too broad**.

Characters like `!`, `(`, `)`, `{`, `}`, `.`, `"` only have special meaning in very specific contexts (e.g., `!` only before `[` for images, `.` only after digits at line start for ordered lists). dom_query takes the "escape everything that *could* matter" approach — valid but ugly.

## Architecture

- **dom_smoothie** (readability extraction) depends on **dom_query** (DOM manipulation + markdown serializer)
- dom_smoothie calls `root_node.md(None)` which invokes `MDSerializer`
- dom_smoothie's `Config` has no markdown-related options — only readability heuristics
- dom_query's `MDSerializer` has an internal `FormatOpts::skip_escape` flag, but it's `pub(super)` — only used for `<code>` elements
- The public `md()` API accepts only `skip_tags: Option<&[&str]>` — no escape configuration

## Characters that genuinely need context-free escaping

| Character | Why |
|-----------|-----|
| `` ` `` | Code spans |
| `*` | Emphasis |
| `_` | Emphasis |
| `[`, `]` | Links/images |

Characters that need **context-sensitive** escaping only:
- `#` — at line start (headings)
- `+`, `-` — at line start (unordered lists)
- `>` — at line start (blockquotes)
- `.` — after digits at line start (`1.` ordered lists)
- `!` — before `[` (image syntax)
- `|` — inside tables
- `<` — before letters (HTML tags)

Characters that **never** need escaping in CommonMark: `(`, `)`, `{`, `}`, `"`.

## Options evaluated

1. **Post-process** (chosen): Strip unnecessary backslash escapes from dom_query's output. Our `clean_markdown_escapes` function handles `!`, `.`, `(`, `)`, `{`, `}`, `"` with correct context awareness. ~60 lines, single-pass, zero dependencies.

2. **Markdown roundtrip** (rejected): Parse with pulldown-cmark, serialize with pulldown-cmark-to-cmark. Normalizes escapes but risks reformatting tables/lists, adds dependencies, slower. See [[decision-log]].

3. **Fork/patch dom_query** (deferred): The escaping logic is in 2 small files. Could patch `ESCAPE_CHARS` or make it configurable. But maintaining a fork is a burden.

4. **Upstream issue** (recommended): File on [github.com/niklak/dom_query](https://github.com/niklak/dom_query) requesting configurable escape set or context-sensitive escaping. The internal `skip_escape` concept already exists.

## dom_smoothie Config options worth exposing

Investigated all `dom_smoothie::Config` fields. Most are irrelevant (HTML-only, unused code paths, or internal tuning knobs). Three are worth exposing as CLI flags:

| Config field | Flag | What it does |
|---|---|---|
| `candidate_select_mode` | `--candidate-mode` | `Readability` (default, conservative) vs `DomSmoothie` (more aggressive at finding content split across sibling divs) |
| `char_threshold` | `--char-threshold` | Controls the "sieve" — if extracted text is shorter than this (default 500), retries with looser heuristics. Lower for short pages. |
| `max_elements_to_parse` | `--max-elements` | Safety limit on DOM size. Default 0 (unlimited). |

See [[iteration-04-output-control]] for implementation plan.
