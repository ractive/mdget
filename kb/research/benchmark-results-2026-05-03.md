---
title: "Benchmark results: mdget vs WebFetch (2026-05-03)"
type: research
date: 2026-05-03
tags: [benchmark, results, token-usage, mcp, cli]
---

## Overview

Three benchmark runs comparing four web content retrieval approaches for a multi-page research task (Turborepo docs). See [[token-usage-benchmark]] for the test setup.

## Task

Research Turborepo documentation to answer four questions (project structure, Docker, remote caching, GitHub Actions CI). Run 3 skipped guide generation — sessions just confirmed they had the answers.

## Sessions

| Session | Tool | MCP | Skill file | Description |
|---------|------|-----|------------|-------------|
| A | WebFetch | no | no | Vanilla Claude Code baseline |
| B | mdget MCP | yes | yes | MCP server tools |
| C | mdget CLI | no | yes | CLI with skill file pre-loaded |
| D | mdget CLI | no | no | CLI cold-start (learned from --help) |

## Run 3 Results (final, research-only)

| Metric | A (WebFetch) | B (MCP) | C (CLI+skill) | D (CLI cold) |
|--------|-------------|---------|---------------|-------------|
| **Wall-clock** | 129s | 99s | 99s | **79s** |
| **Fetches** | 10 (1 fail) | **5** | 10 (2 fail) | **6** |
| **Chars fetched** | ~18K | ~28K | ~61K | ~41K |
| **Pages confirmed** | 9 | 5 | 8 | 6 |
| **404 errors** | 1 | 0 | 2 | 0 |

### Per-fetch timing

| Session | Avg per fetch | Reason |
|---------|--------------|--------|
| A | ~14s | WebFetch runs summarization model per call |
| B | ~3.5s | MCP tool call overhead + HTTP fetch |
| C | <0.3s | CLI subprocess, near-instant |
| D | <0.1s | CLI subprocess with --quiet, fastest |

### Strategy each session chose

**A (WebFetch):** Fetched docs index first to discover structure, then 9 targeted pages with extraction prompts. 3 parallel batches. Each call returned pre-summarized text (~1-3K chars). Hit 1 wrong URL (404).

**B (MCP):** 5 individual `fetch_markdown` calls. No crawl_site, no batch_fetch — the agent chose targeted fetches over bulk retrieval without prompting. Used `no_images=true, max_length=20000`. Fetches 2-5 in parallel. Zero errors.

**C (CLI+skill):** 10 individual `mdget` calls saving to files (`-o`). Did NOT use `--no-images` or `--max-length` despite having the skill file. Hit 2 wrong URLs (404). More verbose strategy than needed.

**D (CLI cold-start):** Read `mdget --help` first, then made 6 calls with `--quiet --no-images --max-length 12000`. Most efficient mdget strategy — learned the right flags from help text alone. Zero errors.

## Run 2 Results (with guide generation, for comparison)

| Metric | A (WebFetch) | B (MCP) | C (CLI+skill) | D (CLI cold) |
|--------|-------------|---------|---------------|-------------|
| **Wall-clock** | 188s | 233s | 173s | 173s |
| **Fetches** | 9 | 4 | 12 | 12 |
| **Chars fetched** | ~22K | ~183K | ~64K | ~93K |
| **Guide words** | 1958 | 2013 | 1822 | 1715 |

## Run 1 Results (biased prompts — discarded)

Prompts for B and C suggested using crawl/batch strategies, causing massive over-fetching (B: 478K chars, C: 601K chars). Not representative — demonstrates prompt sensitivity.

## Analysis

### WebFetch latency tax

WebFetch takes 10-17s per call because it fetches the page, runs it through a summarization model, then returns compressed output. mdget CLI calls complete in <1s. Even with fewer output tokens, the wall-clock cost is 50-100x per call. Over 10 calls this adds ~2 minutes of pure waiting.

### Summarization tradeoff

WebFetch returns ~1-3K chars per page (pre-summarized). mdget returns ~5-10K chars (full readability-extracted markdown). WebFetch uses fewer context tokens but:
- The agent can't control what the summarizer keeps or discards
- If important details are lost, the agent must re-fetch (compounding the latency)
- The summarization prompt becomes a second variable to tune

mdget gives the agent full content and lets it decide what matters.

### Skill file provides minimal advantage

Session C (with skill file) performed worse than Session D (cold-start from --help):
- C made more fetches (10 vs 6)
- C had more 404 errors (2 vs 0)
- C didn't use `--no-images` or `--max-length`
- C was 20s slower (99s vs 79s)

The `--help` text was sufficient for an agent to discover the right flags. The skill file didn't lead to better strategy choices in these runs.

### MCP vs CLI

MCP (Session B) and CLI (Session D) achieved similar results:
- B: 5 calls, 28K chars, 99s
- D: 6 calls, 41K chars, 79s

CLI was faster in wall-clock (subprocess overhead < MCP tool call overhead) but MCP required fewer calls. Both found the right pages with zero errors.

### Nobody used crawl or batch (unprompted)

When not nudged toward crawl/batch strategies, all sessions independently chose individual page fetches. This suggests:
- For targeted research tasks, individual fetches are the natural agent strategy
- crawl/batch shine for exploratory tasks ("read these entire docs") not targeted questions
- The benchmark task may not be the right shape to exercise crawl/batch

### URL guessing accuracy

| Session | Guessed correctly | 404s |
|---------|-------------------|------|
| A | 8/9 (89%) | 1 |
| B | 5/5 (100%) | 0 |
| C | 8/10 (80%) | 2 |
| D | 6/6 (100%) | 0 |

B and D had perfect URL accuracy. The docs index page contains enough links to discover the right URLs rather than guessing.

## Token usage

Not directly measured — `/usage` only shows percentage bars, and `claude -p --output-format json` failed with auth errors in the cmux environment. Estimated from chars fetched at ~4 chars/token:

| Session | Est. content tokens | Notes |
|---------|-------------------|-------|
| A | ~4.5K | Summarized output |
| B | ~7K | Full markdown |
| C | ~15K | Full markdown, no truncation |
| D | ~10K | Full markdown, max-length capped |

These are content tokens only — they don't include system prompt, tool descriptions, or agent reasoning overhead.

## Conclusions

1. **mdget is faster** — the WebFetch summarization round-trip is the dominant cost. mdget CLI fetches complete in <1s vs 10-17s for WebFetch.

2. **mdget trades context size for fidelity** — agents see 2-3x more content tokens but get the full page. Whether this is worth it depends on the task.

3. **The --help text works** — agents learn mdget from `--help` alone without needing a skill file. This validates the "agent-friendly help text" design investment.

4. **Crawl/batch are situational** — for targeted research, individual fetches win. Crawl/batch are for exploration tasks. Don't push agents toward them.

5. **Prompt sensitivity is high** — suggesting strategies in the prompt caused 5-10x over-fetching. Keep tool instructions minimal.

## Limitations

- Single task (Turborepo docs) — results may not generalise to other domains
- No exact token counts — estimated from character counts
- 1 run each — no variance data (would need 3-5 runs per session)
- Shared rate limits across sessions may have affected parallel runs
- WebFetch summarization quality is a black box — we can't inspect what was lost
