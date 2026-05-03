---
title: "Benchmark: mdget vs WebFetch token usage & speed"
type: research
date: 2026-05-03
tags: [benchmark, dogfood, token-usage, mcp, cli]
status: planned
---

## Goal

Measure whether mdget saves tokens and wall-clock time compared to the built-in WebFetch tool for a realistic multi-page research task. Compare four setups to isolate the value of MCP vs CLI, and the cost of tool discovery.

## Test Matrix

| Session | Web tool | MCP server | Skill file | What it measures |
|---------|----------|------------|------------|------------------|
| **A** | WebFetch only | no | no | Baseline — vanilla Claude Code agent |
| **B** | mdget MCP | yes | yes | Best-case mdget (MCP tools + skill pre-loaded) |
| **C** | mdget CLI | no | yes | CLI with skill file (knows flags, no discovery cost) |
| **D** | mdget CLI | no | no | Cold-start — must read `mdget --help` to learn the tool |

## Task Prompt

All four sessions receive the same core task. Only the tooling instructions differ.

### Core task (shared)

```
I'm setting up a Turborepo monorepo with Docker for CI. Research the
Turborepo documentation at https://turborepo.dev/docs until you can
confidently answer these questions:

1. How should the project be structured? (workspaces, packages)
2. How do I configure Docker? (multi-stage builds, layer caching)
3. How do I set up remote caching?
4. How do I integrate with GitHub Actions CI?

Do NOT write a guide. Once you have enough information to answer all
four questions with specific configuration examples, just write a short
confirmation to confirm.md in the output directory listing which pages
you read and a one-line summary of what each covered.

IMPORTANT: Log EVERY request that retrieves web content to fetch-log.md
in the output directory — this includes WebFetch, WebSearch, curl, wget,
mdget, MCP tool calls, or any other method. Use this markdown table format:

| # | Tool | Command or URL | Parameters | Response size (chars) | Wall-clock (s) |
```

### Session-specific preambles

**Session A (baseline):**
```
Use whatever tools you naturally would to retrieve web content (WebFetch,
WebSearch, curl, etc.). Do NOT use mdget.
Output directory: /tmp/mdget-benchmark/A
```

**Session B (mdget MCP):**
```
Use the mdget MCP server tools to retrieve web content. Do NOT use
WebFetch.
Output directory: /tmp/mdget-benchmark/B
```

**Session C (mdget CLI + skill):**
```
Use the mdget CLI tool (globally installed, run `mdget`) to retrieve
web content. Do NOT use WebFetch. You have a skill file with mdget
documentation available in this project.
Output directory: /tmp/mdget-benchmark/C
```

**Session D (mdget CLI, no skill):**
```
Use the mdget CLI tool (globally installed, run `mdget`) to retrieve
web content. Do NOT use WebFetch. You have NOT used mdget before — run
`mdget --help` and `mdget crawl --help` to learn how it works before
starting the task. Do NOT read any skill files or documentation about
mdget other than its own --help output.
Output directory: /tmp/mdget-benchmark/D
```

## Setup Procedure

### Prerequisites

```bash
# Install mdget globally
cargo install --path crates/mdget-cli

# Verify
which mdget && mdget --version

# Ensure .mcp.json uses global binary (for Session B)
cat .mcp.json
# Should show: "command": "mdget", "args": ["serve"]

# Create output directories
mkdir -p /tmp/mdget-benchmark/{A,B,C,D}
```

### Launching sessions

Use `cmux claude-teams -p --output-format json` for non-interactive execution
with structured token usage in the output. Run all 4 in parallel.

```bash
# Record start time
date +%s > /tmp/mdget-benchmark/start_time

for s in A B C D; do
  PROMPT=$(cat /tmp/mdget-benchmark/$s/prompt.txt)

  # Launch in background, capture JSON output with token usage
  cmux claude-teams -p --output-format json --permission-mode auto "$PROMPT" \
    > /tmp/mdget-benchmark/$s/result.json 2>/dev/null &

  echo "$s launched (PID $!)"
done

echo "All sessions launched. Waiting..."
wait
echo "All sessions complete."
```

Token usage is extracted from the JSON output:

```bash
for s in A B C D; do
  echo "=== Session $s ==="
  python3 -c "
import json, sys
d = json.load(open('/tmp/mdget-benchmark/$s/result.json'))
u = d.get('usage', {})
print(f'  Input tokens:  {u.get(\"input_tokens\", \"?\")}')
print(f'  Output tokens: {u.get(\"output_tokens\", \"?\")}')
print(f'  Cache create:  {u.get(\"cache_creation_input_tokens\", \"?\")}')
print(f'  Cache read:    {u.get(\"cache_read_input_tokens\", \"?\")}')
print(f'  Duration:      {d.get(\"duration_ms\", \"?\")}ms')
print(f'  Cost:          \${d.get(\"total_cost_usd\", \"?\"):.4f}')
"
done
```

**Session-specific setup:**
- **A**: Remove `.mcp.json` temporarily (or run from a directory without it). Remove mdget skill file if present.
- **B**: `.mcp.json` must be present with mdget server. Skill file present.
- **C**: No `.mcp.json` (no MCP server). Skill file present.
- **D**: No `.mcp.json`. No skill file. Agent discovers mdget from `--help` only.

**Parallelism:** Sessions A-D are independent and can run in parallel. This also tests under identical network conditions.

### Skill file management

The mdget skill file lives at `.claude/skills/mdget/SKILL.md` (created by `mdget init --claude`). For sessions that should NOT have the skill:

```bash
# Temporarily hide it
mv .claude/skills/mdget .claude/skills/mdget.bak

# Restore after benchmark
mv .claude/skills/mdget.bak .claude/skills/mdget
```

## Measurements

### From result.json (per session) — primary metrics

- `input_tokens` — total input tokens (system prompt + tools + content)
- `output_tokens` — total output tokens (reasoning + tool calls + responses)
- `cache_creation_input_tokens` — tokens written to prompt cache
- `cache_read_input_tokens` — tokens read from prompt cache
- `total_cost_usd` — total API cost
- `duration_ms` — total wall-clock time

### From fetch-log.md (per session) — strategy analysis

- Total number of fetches
- Total response chars consumed
- Which URLs were fetched (did the session find the right pages?)
- Tool usage pattern (did it use triage, batch, crawl, or individual fetches?)

### From confirm.md (quality check)

- Did the session find pages covering all 4 topics?
- Are the right pages listed? (structuring, docker, remote-caching, github-actions)

### Analysis script

```bash
for s in A B C D; do
  echo "=== Session $s ==="

  # Token usage from JSON
  python3 -c "
import json
d = json.load(open('/tmp/mdget-benchmark/$s/result.json'))
u = d.get('usage', {})
print(f'  Input:    {u.get(\"input_tokens\", \"?\"):>8}')
print(f'  Output:   {u.get(\"output_tokens\", \"?\"):>8}')
print(f'  Cache R:  {u.get(\"cache_read_input_tokens\", \"?\"):>8}')
print(f'  Cache W:  {u.get(\"cache_creation_input_tokens\", \"?\"):>8}')
print(f'  Cost:     \${d.get(\"total_cost_usd\", 0):.4f}')
print(f'  Time:     {d.get(\"duration_ms\", 0)/1000:.1f}s')
"

  # Fetch count from log
  if [[ -f "/tmp/mdget-benchmark/$s/fetch-log.md" ]]; then
    fetches=$(grep -c '^|[^-]' /tmp/mdget-benchmark/$s/fetch-log.md 2>/dev/null || echo 0)
    echo "  Fetches:  $((fetches - 1))"  # subtract header row
  fi

  # Confirm file
  if [[ -f "/tmp/mdget-benchmark/$s/confirm.md" ]]; then
    echo "  Confirm:  $(wc -l < /tmp/mdget-benchmark/$s/confirm.md) lines"
  else
    echo "  Confirm:  MISSING"
  fi
  echo ""
done
```

## Expected Hypotheses

1. **A (WebFetch) uses fewer input tokens** because WebFetch summarises before hitting context — but may need more round-trips if summaries miss details.
2. **B (MCP) uses fewest tool calls** — batch_fetch/crawl_site can fetch multiple pages per call.
3. **C (CLI + skill) and D (CLI cold) converge on similar strategies** — individual fetches are the natural CLI pattern.
4. **D has slightly higher output tokens** due to reading `--help` and reasoning about flags.
5. **All sessions find the same core pages** — the task is well-defined enough that URL discovery converges.

## Notes

- The task deliberately skips guide generation to isolate fetch cost from synthesis cost.
- WebFetch uses a small summarisation model internally — input tokens reflect the compressed output, not the raw page content.
- Network latency affects wall-clock time but not token counts — tokens are the primary metric.
- Run multiple times to check variance if results are close.

## Previous runs

### Run 1 (biased prompts — strategy hints for B/C)

B and C prompts suggested using crawl/batch, causing over-fetching. B pulled 478K chars, C pulled 601K. Not representative.

### Run 2 (unbiased prompts, with guide generation)

| Metric | A (WebFetch) | B (MCP) | C (CLI+skill) | D (CLI cold) |
|--------|-------------|---------|---------------|-------------|
| Wall-clock | 188s | 233s | 173s | 173s |
| Tool calls | 9 | 4 | 12 | 12 |
| Chars fetched | ~22K | ~183K | ~64K | ~93K |
| Guide words | 1958 | 2013 | 1822 | 1715 |

Key finding: without strategy hints, CLI sessions (C/D) naturally chose individual fetches over crawl/batch. D (cold-start) matched C (skill file) in speed and strategy. Token counts not captured.
