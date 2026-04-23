---
title: "Code review fixes: security, help texts, and quality"
type: iteration
date: 2026-04-23
tags:
  - iteration
  - security
  - documentation
  - quality
status: in-progress
branch: iter-5b/review-fixes
---

## Goal

Address findings from the comprehensive code review: fix security gaps, improve help text for AI agents, add a cookbook section to `--help`, and fix minor code quality issues.

## Tasks

### Security fixes

- [x] Add `MAX_RESPONSE_SIZE` limit (e.g. 50 MB) enforced before `.text()` in `fetch.rs` — prevents memory exhaustion from malicious servers
- [x] Combine HTTP redirect (10) and meta-refresh (3) limits into a single `MAX_TOTAL_HOPS` — currently independent, allowing 13+ total hops
- [x] Fix `rustls-webpki` vulnerability (RUSTSEC-2026-0104) — update transitive dep via `cargo update`

### Bug fix

- [x] Thread panic in batch processing doesn't set `had_error = true` (`main.rs:395-400`) — process exits 0 despite missing results

### Help text improvements

- [x] Add COOKBOOK section to `long_about` with real-world recipes: LLM-optimized fetch, triage workflow, bulk+parallel, resilient fetch, local HTML conversion, capped output
- [x] Add BEHAVIOR NOTES section to `long_about` documenting: redirect following (HTTP 3xx + meta-refresh), retry semantics (5xx yes, 4xx no), content-type handling (HTML/JSON/plain/PDF/binary), multi-input error behavior
- [x] Expand EXIT CODES to clarify partial failure ("one or more inputs failed")
- [x] Expand AGENT TIPS with: `-m` for triage, `--no-images` for token savings, 4xx not retried
- [x] Add `long_help` to `--retries` explaining that 4xx client errors are NOT retried (only 5xx and timeouts)

### Minor code quality

- [x] Replace string concatenation with `+` in table processing (`extract.rs:570,612,682`) with `push('\n')` on pre-allocated String
- [x] Add e2e test verifying `--help` long output contains COOKBOOK section
- [x] Run quality gates (fmt, clippy, test)

## Out of scope

- **SSRF filtering for private IPs** — acceptable risk for a CLI tool; revisit if/when MCP server is added (iteration 6)
- **Output path validation for `-o`** — users explicitly pass this flag; path traversal is user intent, not an attack vector for CLI usage
- **Splitting `extract.rs` into sub-modules** — not worth the churn at 1195 lines; revisit if it grows past 2000
- **reqwest 0.13 upgrade** — not urgent, 0.12 is current for its major version
