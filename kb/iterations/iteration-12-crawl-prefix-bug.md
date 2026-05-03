---
title: "Fix crawl path-prefix auto-inference for single-segment paths"
type: iteration
date: 2026-05-03
tags:
  - iteration
  - bugfix
  - crawl
status: in-progress
branch: iter-12/crawl-prefix-bug
---

## Goal

Fix `infer_path_prefix()` so that single-segment paths like `/docs` infer `/docs/` as the crawl prefix. Currently returns `None`, causing the crawler to follow links to `/blog`, `/terms`, `/en/showcase` etc. — wasting the page budget on irrelevant pages.

## Bug

```
mdget crawl --depth 1 --max-pages 30 https://turborepo.dev/docs
```

Expected: only fetches pages under `/docs/`.
Actual: fetches `/en`, `/en/blog`, `/en/showcase`, `/blog`, `/terms`, `/governance`, `/` — 8+ pages outside `/docs/`.

Root cause: `infer_path_prefix()` in `crates/mdget-core/src/crawl.rs:395` returns `None` for single-segment paths like `/docs` because it only considers the parent directory (`/` in this case, which it treats as "no restriction").

## Additional issues found

- **Duplicate fetches via locale redirects**: `/en/docs/core-concepts/remote-caching` redirects to `/docs/core-concepts/remote-caching` but both are fetched. The URL dedup set should include the final (post-redirect) URL.
- **Duplicate file writes**: `docs.md`, `index.md`, `blog.md` are saved twice (second write overwrites first silently).

## Tasks

- [x] Fix `infer_path_prefix()`: for single-segment paths like `/docs`, return `Some("/docs/")` instead of `None`. The heuristic: if the path has exactly one segment (no trailing slash, one `/` at the start), append `/` and use it as the prefix.
- [x] Update the `infer_path_prefix_single_segment` test — the expected value changes from `None` to `Some("/docs/")`.
- [x] Add test case for `https://example.com/docs` → `Some("/docs/")`.
- [x] Fix post-redirect URL dedup: when a URL redirects (301/302), add the final URL to the visited set, not just the original. This prevents fetching `/en/docs/...` and `/docs/...` as separate pages.
- [x] Fix duplicate file saves in `--output-dir`: resolved by redirect dedup — duplicate results are no longer generated, preventing duplicate file writes.
- [x] Add e2e test: crawl with auto-inferred prefix on a single-segment path, verify no pages outside the prefix are fetched.
- [x] Add e2e test: crawl redirect dedup, verify redirected pages don't produce duplicate results.
- [x] Run quality gates (fmt, clippy, test).

## Notes

- This bug was found during benchmark run 4 — Session D used `mdget crawl --depth 1 --max-pages 30 https://turborepo.dev/docs` and wasted 8 of 23 fetches on non-docs pages.
- The redirect dedup issue is separate but related — fixing prefix inference alone would prevent most of the `/en/*` pages from being fetched, but the dedup fix prevents duplicate work when URLs redirect within the allowed prefix.
