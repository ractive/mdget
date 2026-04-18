---
title: "Dogfood fixes: escaping, content-type, quiet mode"
type: iteration
date: 2026-04-18
tags: [iteration, bugfix, quality, dogfood]
status: in-progress
branch: iter-2a/dogfood-fixes
---

## Goal

Fix issues discovered during the `/dogfood` ergonomic review of iteration 2. These are quality-of-life and correctness fixes that should land before adding new features.

See the full review in the conversation that produced this iteration plan.

## Issues to fix

### 1. Over-aggressive backslash escaping (critical)

Every `.` becomes `\.`, every `(` becomes `\(`, etc. throughout all output. This is the most visible quality issue — it affects every page fetched and produces technically incorrect markdown.

**Example:** `"Memory is managed through a system of ownership\."` — that `\.` is wrong.

**Root cause:** Likely dom_smoothie's markdown serializer escaping characters that don't need escaping in standard CommonMark. Periods only need escaping when they'd trigger an ordered list (`1.`), and parentheses only in specific link/image contexts.

**Fix options (investigate in order):**
1. dom_smoothie configuration — check if there's an option to control escaping behavior
2. Post-processing pass — strip unnecessary backslash escapes from the output
3. If dom_smoothie is the bottleneck, evaluate alternative HTML-to-markdown converters (`htmd`, `html2md`)

### 2. Binary content dumped to stdout (high)

`mdget https://httpbin.org/image/png` dumps raw PNG bytes to stdout with exit code 0. An agent piping this into an LLM gets garbage.

**Fix:** Check the `Content-Type` response header before processing:
- `text/html`, `application/xhtml+xml` → normal pipeline (readability + markdown)
- `text/plain` → pass through as-is (or wrap in code block)
- `application/json` → wrap in a ```json code block
- Binary types (`image/*`, `application/pdf`, `application/octet-stream`, etc.) → error with clear message: `"Error: URL returned binary content (image/png). mdget only processes HTML pages."`
- Unknown → warn on stderr, attempt HTML pipeline, fail gracefully

### 3. No `--quiet` / `-q` flag (medium)

Progress messages on stderr ("Fetching...", "Extracting...") are helpful for humans but noisy for agent pipelines and scripts.

**Fix:** Add `-q` / `--quiet` flag that suppresses all stderr progress messages. Errors should still go to stderr even with `--quiet`.

### 4. No trailing newline (minor)

Output doesn't consistently end with a newline before the shell prompt. This is a minor cosmetic issue but visible when piping.

**Fix:** Ensure output always ends with `\n`.

## Additional feature: `--metadata-only` / `-m`

Add to [[iteration-04-output-control]]:

```
mdget <URL> --metadata-only        # print only YAML frontmatter, skip body
```

Fetches the page, extracts title/byline/date/word_count, and prints just the YAML frontmatter block. Skips the body entirely. Useful when an agent needs to:
- Triage a list of URLs (what's this page about? how long is it?)
- Build a bibliography or link index
- Decide whether to fetch the full body based on metadata

Could potentially use a HEAD request + partial body fetch to save bandwidth, but the readability algorithm needs the full DOM. A simpler approach: fetch full body, extract metadata, discard the markdown output.

## Tasks

- [x] Investigate dom_smoothie escaping behavior — find root cause of over-escaping
- [x] Fix or work around the backslash escaping issue
- [x] Implement Content-Type detection from HTTP response headers
- [x] Handle plain text responses (pass through or code block)
- [x] Handle JSON responses (wrap in ```json code block)
- [x] Reject binary content with a clear error message
- [x] Add `--quiet` / `-q` flag to suppress stderr progress
- [x] Ensure output always ends with a trailing newline
- [x] Add e2e tests for content-type handling (text, JSON, binary)
- [x] Add e2e test for `--quiet` flag
- [x] Run quality gates: `cargo fmt`, `cargo clippy`, `cargo test`

## Design Decisions

- **Content-type detection uses HTTP headers, not sniffing.** Trust `Content-Type` from the server. Don't try to detect binary by inspecting bytes — that's fragile and slow.
- **JSON gets a code block, not the HTML pipeline.** Wrapping JSON in ` ```json ` is more useful than trying to "convert" it. Agents fetching API endpoints get usable output.
- **`--quiet` suppresses progress, not errors.** Errors always go to stderr regardless of `-q`. An agent needs to know when something fails.
