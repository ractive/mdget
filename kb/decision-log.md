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

## 2026-05-03: Project parked — mdget does not justify continued development (recorded 2026-08-30)

**Context:** After 12 iterations, 4 dogfood reviews and the 4-session benchmark ([[research/benchmark-results-2026-05-03]]), the question "does mdget have any reason to exist?" was asked directly. This entry reconstructs the answer from the final session's prompts (`~/.claude/history.jsonl`, session `ec7e2421`, 2026-05-03) and a later remark in the ff-rdp project (2026-08-30), since the session transcripts themselves are gone. See [[research/future-direction]] for the full brainstorm.

**Findings that led to the decision:**
1. **Marginal for the target audience.** For Claude Code agents, the built-in WebFetch summarizes server-side and therefore puts *fewer* tokens into context. mdget is 50–100x faster per call, but agents see 2–3x more content tokens, and wall-clock for targeted research tasks was comparable or better with WebFetch.
2. **Most harnesses already have a WebFetch-equivalent.** The remaining gap (agents without server-side fetching: Codex, Aider, Cursor, OpenCode) was judged too small to pursue now; it was parked as an idea in [[research/positioning-cross-agent]].
3. **The scope was AI-driven.** Crawling, MCP server, batch fetching and metadata extraction were all suggested by Claude Code during development. Unprompted agents used none of them — they issued simple single fetches. MCP in particular was pushed repeatedly without evaluating alternatives.
4. **Claude never flagged the competitor.** WebFetch's server-side summarization was never mentioned as a reason mdget might be redundant until the benchmark exposed it.
5. **The skill file added nothing.** The cold-start session using only `--help` outperformed the session with the skill file.
6. **The valuable part isn't ours.** Readability extraction quality comes from dom_smoothie; mdget is a thin wrapper around it.

**Decision:** Stop active development. No formal "kill" — the repo, KB and the seven brainstormed directions in [[research/future-direction]] are kept so the work can be picked up later. Cross-agent positioning is the most plausible revival path.

**User's own summary (ff-rdp session, 2026-08-30):** "This was a project idea that got dumped because it turned out that most agent harnesses have kind of a 'webfetch' tool that already does that. But for what we try to achieve here, the work done in mdget could be very valuable!" — i.e. the reader-view / HTML→markdown extraction is a candidate for reuse in ff-rdp.

**Meta-lesson:** Treat "stop and evaluate" as a valid iteration. Question the core value before accepting feature suggestions from the coding assistant, and check what the harness already provides natively.

**Revisit if:** a concrete consumer appears (ff-rdp reader view, a non-Claude agent harness lacking fetch, a docs-archiving/RAG need), or WebFetch-style tools regress in quality/availability.
