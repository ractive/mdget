---
title: "Future direction: does mdget justify its existence?"
type: research
date: 2026-05-03
tags:
  - strategy
  - positioning
  - ai-agents
status: planned
---

## Context

After 12 iterations, 4 dogfood reviews, and a 4-session benchmark, we honestly assessed whether mdget brings enough value to justify continued development. See [[benchmark-results-2026-05-03]] and [[positioning-cross-agent]] for the data.

## The uncomfortable findings

1. **For Claude Code agents, mdget is marginal.** WebFetch does server-side summarization, putting fewer tokens in context. Benchmarks showed comparable or better wall-clock for targeted research tasks.
2. **Features Claude Code pushed for turned out unnecessary.** Crawling, MCP server, batch fetching, metadata extraction — all were suggested by Claude during development but agents naturally chose simple individual fetches when not prompted toward them.
3. **WebFetch's server-side summarization was never flagged as a competitor.** Claude didn't know its own tool well enough to warn that WebFetch compresses content before it hits context.
4. **The skill file didn't help.** Session D (cold-start, just `--help`) outperformed Session C (with skill file). The `--help` text alone was sufficient.

## What mdget does well (genuinely)

- Readability extraction quality is excellent on docs and articles
- Sub-second execution (50-100x faster per call than WebFetch)
- Clean markdown with code blocks, tables, links preserved
- Crawling with robots.txt, sitemap, path-prefix support
- Non-English content (French, Japanese) works perfectly
- stdout/stderr separation for Unix composability
- No runtime dependencies (pure Rust binary)

## Brainstormed directions

### 1. Cross-agent web fetching tool
Most AI coding agents (Codex CLI, Aider, Cursor, Windsurf) have no built-in page fetching. mdget as an MCP server or CLI tool fills a real gap for non-Claude agents. See [[positioning-cross-agent]].

### 2. Documentation archiver
Download entire doc sites as clean markdown for offline use, RAG ingestion, or version control. "Archive these docs before the vendor changes them." WebFetch can't do this. The crawl engine is already built.

### 3. RAG ingestion pipeline
Add chunking by heading/section and structured output (JSONL) for feeding web content into vector databases. `mdget ingest https://docs.example.com --output chunks.jsonl`

### 4. Content differ
Fetch a page, store it, fetch later, show what changed. Useful for monitoring docs, tracking API changes. `mdget diff https://example.com/api --baseline previous.md`

### 5. Unix tool for humans (not agents)
Drop the "for AI agents" pitch. Position as: `mdget` is to web pages what `cat` is to files. Fast, pipe-friendly, no dependencies. Audience: developers who want readable web content in their terminal.

### 6. Library, not CLI
The real value might be `mdget-core` as a Rust crate for embedding in other tools. Fast, zero-dependency HTML→markdown for any Rust project.

### 7. Kill it
The readability extraction is dom_smoothie's work. mdget is a thin wrapper. If the wrapper doesn't provide enough unique value, contributing upstream might be more impactful.

## Meta-lesson: AI-driven scope creep

This project is a case study in how AI coding assistants push toward complexity:
- More features (crawl, MCP, batch, metadata) instead of questioning whether the core is valuable
- Favoring Anthropic-adjacent tech (MCP) without evaluating alternatives
- Not knowing the competitive landscape (WebFetch internals)
- Treating "build more" as progress when "stop and evaluate" would have been better

Worth keeping in mind for future projects.

## Decision needed

Which direction to pursue (or whether to stop). Factors:
- Personal interest and motivation
- Size of the audience for each direction
- How much of the existing code is reusable
- Whether it's worth maintaining long-term
