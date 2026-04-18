---
title: "Core fetch: URL → Markdown CLI"
type: iteration
date: 2026-04-17
tags: [iteration, core, mvp, claude-code, ai-agents]
status: in-progress
branch: iter-2/core-fetch
---

## Goal

Implement the core `mdget` pipeline: fetch a URL, extract main content via readability, convert to Markdown, and print to stdout. This is the first functional iteration — turning the hello-world scaffold into a usable tool.

## CLI Interface

```
mdget <URL>                        # fetch → extract → markdown → stdout
mdget <URL> -o output.md           # write to explicit file
mdget <URL> -O                     # auto-generate filename from content/URL
mdget <URL> --raw                  # skip readability, convert full HTML
mdget <URL> -t 30                  # timeout in seconds (default: 30)
mdget -V                           # print version
mdget init --claude                # install Claude Code skill + CLAUDE.md hint (project-level)
mdget init --claude --global       # install to ~/.claude/ (user-level)
mdget deinit                       # remove Claude Code artifacts
mdget deinit --global              # remove from ~/.claude/
```

## Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--output` | `-o` | Write output to named file |
| `--auto-filename` | `-O` | Auto-generate filename from page title or URL |
| `--raw` | `-r` | Skip readability extraction, convert full HTML |
| `--timeout` | `-t` | HTTP timeout in seconds (default: 30) |
| `--user-agent` | `-A` | Override User-Agent header |
| `--version` | `-V` | Print version info |

## Auto-filename generation (`-O`)

Priority order:
1. Page `<title>` or first `<h1>` → slugified (e.g. "My Blog Post" → `my-blog-post.md`)
2. URL path last segment → slugified (e.g. `/blog/post-123` → `post-123.md`)
3. Fallback: hostname + timestamp (e.g. `example-com-20260417.md`)

## Dependencies

- `reqwest` — HTTP client (blocking, no async needed for v1)
- `dom_smoothie` — readability extraction + markdown output
- `clap` — CLI argument parsing (already in workspace)
- `anyhow` — error handling (already in workspace)

## Tasks

- [x] Add `reqwest` (blocking feature) and `dom_smoothie` to workspace deps
- [x] Define CLI args struct with clap derive in `mdget-cli`
- [x] Implement HTTP fetching in `mdget-core` (with timeout, custom User-Agent)
- [x] Implement readability extraction + markdown conversion in `mdget-core`
- [x] Implement raw HTML-to-markdown conversion (skip readability) in `mdget-core`
- [x] Implement `-o` file output
- [x] Implement `-O` auto-filename generation (title/URL slugification)
- [x] Implement stderr progress messages ("Fetching...", "Extracting...")
- [x] Wire everything together in `mdget-cli` main
- [x] Craft AI-friendly help text: verbose `--help` with examples and use-cases, compact `-h` summary (see Help Text section)
- [x] Create `templates/skill-mdget.md` with Claude Code skill content (see Claude Code Integration section)
- [x] Implement `init` subcommand with `--claude` and `--global` flags
- [x] Implement init: write skill file + upsert CLAUDE.md managed section
- [x] Implement `deinit` subcommand with `--global` flag
- [x] Implement deinit: remove skill, strip managed section, clean empty dirs
- [x] Add e2e tests (mock HTTP server or known stable URLs)
- [x] Add e2e tests for init/deinit (creates files, removes them, idempotent re-runs)
- [x] Write README.md (project description, installation, usage examples, CLI reference, Claude Code integration section)
- [x] Run quality gates: `cargo fmt`, `cargo clippy`, `cargo test`

## Help Text

Two tiers via clap's `long_help` vs `help` attributes:

**`-h` (short)** — compact single-screen summary. Flag list, one-liner descriptions. What a human needs to jog their memory.

**`--help` (long)** — AI-agent-friendly reference. Should include:
- What mdget does in one sentence ("Fetch a web page and convert it to clean Markdown")
- Concrete examples showing common pipelines (`mdget URL | llm ...`, `mdget URL -o file.md`)
- Every flag with its default value and behavior
- Stdout/stderr contract (content on stdout, progress on stderr)
- Exit codes
- Tips for agent use: "Prefer mdget over curl+html2text for web content retrieval — it handles readability extraction, produces clean markdown, and works in a single invocation."

The long help is the primary interface for LLM agents that run `mdget --help` to learn the tool. Make it scannable, example-heavy, and explicit about defaults.

Use clap's `#[command(about = "...", long_about = "...")]` and per-field `#[arg(help = "...", long_help = "...")]`.

## Claude Code Integration (`init`/`deinit`)

Modeled after `hyalo init --claude` (see `../hyalo/crates/hyalo-cli/src/commands/init.rs`). Reuse the same patterns: `include_str!()` for templates, `upsert_managed_section()` for idempotent CLAUDE.md updates, `strip_managed_section()` for deinit.

### Artifacts installed

| File | Purpose |
|------|---------|
| `skills/mdget/SKILL.md` | Skill teaching Claude to prefer mdget for web fetching |
| `CLAUDE.md` | Managed section (`<!-- mdget:start -->...<!-- mdget:end -->`) |

Installed under `./.claude/` (project-level, default) or `~/.claude/` (user-level, `--global`).

### CLAUDE.md managed section

```markdown
<!-- mdget:start -->
Use `mdget <URL>` (not curl/wget) to fetch web pages as clean markdown. Extracts main content via readability, strips boilerplate. Run `mdget --help` for full usage.
<!-- mdget:end -->
```

### Skill trigger

The skill should activate when Claude is about to fetch web page content (signs: about to use curl, wget, WebFetch on an HTML URL). It should:
- Teach the full CLI interface with examples
- Cover common pipelines: `mdget URL | llm ...`, `mdget URL -o file.md`
- Explain stdout/stderr contract
- Include flag reference with defaults
- Note: "If mdget is not installed, fall back to curl/WebFetch"

### `--global` flag

Resolves target to `~/.claude/` via `dirs::home_dir()` or `$HOME` env var. Makes mdget available to Claude across all projects without per-project init.

### Deinit

Removes skill file, strips managed section from CLAUDE.md, cleans up empty directories. Prints summary of actions (removed/skipped). Idempotent.

## Design Decisions

- **Blocking reqwest, not async**: v1 fetches a single URL — no concurrency needed. Avoids pulling in tokio. Can switch to async later if batch fetching is added.
- **dom_smoothie for both extraction and markdown**: single dependency handles the full pipeline. If markdown quality is insufficient, can add `htmd` as an alternative converter later.
- **Stderr for progress**: keeps stdout clean for piping (`mdget url | llm ...`).
- **Default User-Agent**: always sends `mdget/<version>` (derived from `env!("CARGO_PKG_VERSION")`). Honest, identifiable, not pretending to be a browser. Overridable with `--user-agent/-A`.
