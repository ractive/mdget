---
title: Decision Log
type: reference
date: 2026-04-20
tags:
  - decisions
  - architecture
status: completed
---

## 2026-04-20: Keep hand-rolled escape cleanup (no markdown roundtrip library)

**Context:** dom_query over-escapes markdown output (`\!`, `\.`, `\(`, etc.). We investigated using a markdown parser to normalize escapes via parse→AST→serialize roundtrip.

**Options:**
1. **pulldown-cmark + pulldown-cmark-to-cmark** — works (unnecessary escapes vanish at parse time) but risks reformatting tables/lists, adds 2 deps, slower than string scan
2. **comrak** — explicitly produces *more* escaping than necessary (disqualified)
3. **markdown-rs / mdast_util_to_markdown** — alpha quality (v0.0.2), too immature
4. **Hand-rolled `clean_markdown_escapes`** — ~60 lines, single-pass, zero deps, context-aware

**Decision:** Keep option 4. The escape set from dom_query is known and stable. Our function handles each character with correct context rules (preserve `\!` before `[`, `\.` after line-start digits). Adding a full markdown parser for 6-7 known unnecessary escapes is overkill.

**Revisit if:** dom_query changes its escape behavior significantly, or we need general-purpose markdown normalization beyond escape cleanup.

See [[research/dom-query-escaping]] for full research.
