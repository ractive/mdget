---
title: Batch fetching & local HTML files
type: iteration
date: 2026-04-17
tags:
  - iteration
  - batch
  - local-files
status: completed
branch: iter-3/batch-and-local
---

## Goal

Support multiple URLs in a single invocation, reading URLs from a file, and converting local HTML files. Add parallel fetching using OS threads (no async runtime).

## CLI Interface

```
mdget url1 url2 url3               # fetch multiple URLs
mdget ./page.html                  # convert local HTML file
mdget file:///path/to/page.html    # explicit file URI
mdget -j 4 url1 url2 url3         # parallel fetching with 4 threads
mdget -i urls.txt                  # read URLs from file (one per line)
```

## Tasks

- [x] Accept multiple positional URL arguments
- [x] Detect local file paths vs URLs (path exists on disk, or `file://` scheme)
- [x] Read local HTML files and pass through the same extract → markdown pipeline
- [x] Add `-i/--input-file` flag to read URLs from a file (one per line, skip blank lines and `#` comments)
- [x] Implement parallel fetching with `std::thread::scope` or rayon
- [x] Add `--jobs/-j` flag for parallelism (default: 4, or 1 if single URL)
- [x] Handle mixed inputs (some URLs, some local files)
- [x] Output ordering: preserve input order regardless of completion order
- [x] Separator between outputs (e.g. `---` or filename header) when multiple inputs
- [x] Error handling: report failures per-URL, don't abort the whole batch
- [x] Replace hand-rolled `spawn_http_server` with `mockito` crate (dev-dependency) in existing e2e tests
- [x] Add e2e tests for batch and local file modes
- [x] Run quality gates

## Design Decisions

- **`std::thread::scope` over tokio**: HTTP fetching is I/O-bound but `reqwest::blocking` handles it fine per-thread. No need for an async runtime for this workload. Rayon is an option if work-stealing improves throughput noticeably.
- **Local file detection**: if the argument is an existing file path or starts with `file://`, treat as local. Otherwise treat as URL.
- **Output ordering**: collect results and print in input order, even if fetches complete out of order. Predictable output matters for scripting.
