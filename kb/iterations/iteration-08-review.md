---
title: Comprehensive review
type: iteration
date: 2026-05-03
tags:
  - iteration
  - review
  - security
  - quality
status: in-progress
branch: iter-8/review
---

## Goal

Thorough review of the entire codebase after the rapid iteration 1–7b build phase. Fix issues found — this is not a read-only audit but a fix-it iteration. No new features.

## Review Areas

### 1. Dependency audit

- [x] Run `cargo update --dry-run` — identify outdated deps
- [x] Update all dependencies to latest compatible versions
- [x] Check for unused dependencies (`cargo +nightly udeps` or manual review of `Cargo.toml` vs actual imports)
- [x] Audit feature flags — are we pulling in more than needed from any crate? (e.g., tokio `full` vs specific features)
- [x] Check for known vulnerabilities via `cargo audit`

### 2. Rust best practices

- [x] No `.unwrap()` / `.expect()` outside tests — use `anyhow::Context` with descriptive messages
- [x] No unnecessary `.clone()` — prefer references where possible
- [x] No unnecessary `pub` on struct fields or functions
- [x] Error handling consistency — are all `?` chains using `Context` with actionable messages?
- [x] Any silent error swallows (e.g., `let _ = ...` on Results that matter)?
- [x] Clippy with all warnings enabled (`-D warnings`) — already gated, but verify nothing is suppressed with `#[allow]`

### 3. Architecture & module boundaries

- [x] Review crate boundaries: `mdget-core` (logic), `mdget-cli` (presentation), `mdget-mcp` (presentation) — is the separation clean?
- [x] Does `mdget-cli` or `mdget-mcp` contain any business logic that belongs in `mdget-core`?
- [x] Does `mdget-core` expose a clean public API, or is it leaking implementation details?
- [x] Coupling: do crates depend on each other's internals? Are there any circular or unnecessary dependencies?
- [x] Cohesion: are related functions grouped together? Are modules too large or too small?

### 4. Security review

- [x] **Input validation**: are all external inputs validated at system boundaries? (URLs, CLI args, HTTP responses, HTML content, robots.txt, sitemap XML)
- [x] **Malicious content**: what happens when mdget processes a page with XSS payloads, script injection, excessively nested HTML, huge attribute values, or embedded data URIs?
- [x] **Resource exhaustion**: can a malicious server cause unbounded memory use? Check: gzip bombs, chunked transfer with no size limit, extremely large HTML documents, deep DOM nesting
- [x] **Path traversal**: does `--output-dir` sanitise filenames derived from URLs? Can a crafted URL write outside the output directory? (e.g., `https://evil.com/../../etc/passwd`)
- [ ] **SSRF via crawl**: with `--follow-external`, can crawled links lead to internal/private IPs? (deferred — requires DNS resolution interception, tracked for future iteration)
- [x] **Redirect-based attacks**: can a chain of redirects bypass same-host filtering in the crawler?
- [x] **Credential leakage**: are URLs with embedded credentials (userinfo) logged or exposed in error messages?
- [x] **Dependency supply chain**: are all deps from well-known, maintained sources?

### 5. Privacy exposure

- [x] What data does mdget send in HTTP requests? Review User-Agent, Referer, cookies, etc.
- [x] Does mdget follow tracking redirects or load tracking pixels?
- [x] Are any user inputs (file paths, URLs) exposed in error messages that could leak to logs?

### 6. Testing gaps

- [x] Identify modules/paths with no test coverage
- [x] Are error paths tested, not just happy paths?
- [x] Are e2e tests testing real behavior or just smoke tests?
- [x] Add tests for security-relevant scenarios (path traversal, resource exhaustion, malicious HTML)

### 7. CLI & help text ergonomics (AI-agent focus)

- [x] Review `--help` output for all subcommands — is it structured for LLM consumption? (clear parameter names, explicit defaults, one-line descriptions)
- [x] Are examples in `--help` copy-pasteable and covering common agent workflows?
- [x] Is the output format documented clearly enough that an agent can parse it reliably?
- [x] Are error messages machine-parseable? (consistent format, actionable, no ambiguous natural language)
- [x] Is the MCP tool surface well-described? Do tool descriptions include parameter constraints, defaults, and example usage?
- [x] Check `mdget --help`, `mdget crawl --help`, `mdget serve --help` for consistency and completeness

### 8. Performance

- [x] Any unnecessary allocations in hot paths (crawl loop, HTML link extraction, markdown conversion)?
- [x] Is streaming actually streaming? Check that HTML processing doesn't buffer entire responses unnecessarily
- [x] URL normalization — called per-link, should be allocation-efficient

### 9. Documentation & stale content

- [x] Is README accurate after iterations 6–7b?
- [x] Are there stale TODOs, FIXMEs, or dead comments?
- [x] Are iteration plan files accurately marked as completed/superseded?
- [x] Is `mdget init --claude` template up to date with MCP server and crawl capabilities?

## Approach

This iteration should be implemented by a thorough agent that:
1. Reads through every source file systematically
2. Logs findings as it goes
3. Fixes issues directly (not just reports them)
4. Runs quality gates after all fixes
5. Creates a single PR with all fixes

The review is the work — there's no separate "implement" step.
