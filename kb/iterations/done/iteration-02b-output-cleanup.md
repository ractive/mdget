---
title: "Post-process output cleanup: table quality & wiki artifacts"
type: iteration
date: 2026-04-20
tags:
  - iteration
  - quality
  - post-processing
  - dogfood
status: completed
branch: iter-2b/output-cleanup
---

## Goal

Improve markdown output quality by post-processing two common patterns that produce noisy, low-signal content: degenerate tables (especially Wikipedia infoboxes) and Wikipedia `[edit]` section links.

Discovered during dogfood review #2 (2026-04-18).

## Issues to fix

### 1. Noisy/degenerate tables (medium)

Wikipedia infoboxes and similar layout-abusing tables produce markdown tables with mostly empty cells, single-column structures, or broken colspan/rowspan layouts. These waste tokens and confuse readers.

**Examples:**
- Wikipedia "Markdown" article infobox renders as a table with 12+ columns where most cells are empty
- Tables with a single data column but many header columns from colspan

**Approach:** Add a post-processing pass that detects low-quality tables and converts them to a more compact format:
- Tables where >50% of cells are empty or whitespace-only -> collapse to key-value pairs (`**Key:** Value`) or remove
- Tables with only 1 data column -> convert to a simple list
- Tables where every row has a different column count (malformed) -> convert to plain text

The heuristic should be conservative — good data tables (benchmarks, comparison matrices) must pass through unchanged.

### 2. Wikipedia `[edit]` section links (low)

Readability extraction preserves `[edit]` links next to headings on Wikipedia. These are navigation artifacts, not content.

**Example:** `## Rise and divergence\n\n[edit]\n\n` or inline `\[[edit](https://en.wikipedia.org/w/index.php?title=...&action=edit&section=2 "Edit section: ...")\]`

**Approach:** Strip links whose visible text is `edit` and whose href contains `action=edit` from the markdown output. This is specific enough to avoid false positives on non-Wikipedia pages.

## Tasks

- [x] Implement table quality heuristic (empty-cell ratio, column consistency)
- [x] Convert degenerate tables to compact key-value or plain text format
- [x] Add unit tests for table post-processing with good and bad table examples
- [x] Implement `[edit]` link stripping for Wikipedia-style section edit links
- [x] Add unit tests for edit link stripping
- [x] e2e test: fetch HTML with `[edit]` links and degenerate table, verify cleanup
- [x] Run quality gates: `cargo fmt`, `cargo clippy`, `cargo test`

## Design Decisions

- **Post-process markdown, not HTML.** The table is already rendered to markdown by dom_query. We clean the markdown string rather than pre-processing the DOM, keeping our changes decoupled from the upstream library.
- **Conservative table heuristic.** Err on the side of keeping tables intact. A good data table rendered slightly ugly is better than a destroyed benchmark comparison.
- **Wikipedia edit-link pattern is narrow.** Only strip links matching `action=edit` in the href. Don't try to generalize to all "edit" links across all sites.
