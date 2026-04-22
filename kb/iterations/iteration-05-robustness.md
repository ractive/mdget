---
title: "Robustness & content type handling"
type: iteration
date: 2026-04-17
tags: [iteration, robustness, error-handling]
status: planned
branch: iter-5/robustness
---

## Goal

Improve reliability with retry logic, better error reporting, and support for non-HTML content types.

## Tasks

- [ ] Implement retry with exponential backoff for transient HTTP errors (5xx, timeouts)
- [ ] Add `--retries N` flag (default: 2)
- [ ] Report redirect chain in stderr (useful for debugging)
- [ ] Follow `<meta http-equiv="refresh">` redirects (HTML-level redirects not handled by reqwest). Parse the `content` attribute for the target URL, re-fetch with a max depth (e.g., 3). Use simple string matching, not a DOM parser. See: `https://blog.rust-lang.org/2025/02/20/Rust-1.85.0.html` as a real-world example.
- [ ] Detect content type from HTTP headers
- [ ] Handle plain text responses (pass through as-is or wrap in code block)
- [ ] Handle PDF responses (extract text if feasible, or error with clear message)
- [ ] Return clear error messages for unsupported content types
- [ ] Add e2e tests for retry behavior, content types, error cases
- [ ] Run quality gates

## Design Decisions

- **Retry is conservative**: only retries on 5xx and network timeouts, not on 4xx (client errors are not transient).
- **PDF support**: stretch goal. Rust PDF text extraction is possible (e.g. `pdf-extract`) but quality varies. May just return an informative error initially.
