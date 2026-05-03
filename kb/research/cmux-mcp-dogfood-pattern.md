---
title: "Pattern: Launch Claude Code with MCP server via cmux"
type: research
date: 2026-05-03
tags: [cmux, mcp, dogfood, pattern]
---

## Problem

When you add an MCP server mid-session (`claude mcp add`), the tools don't appear in the current session's deferred tools list. You need a fresh Claude Code session to pick up the `.mcp.json`.

## Solution

Install the MCP server, then spawn a child Claude Code session in a cmux pane.

### Step-by-step

```bash
# 1. Build the binary
cargo build --release

# 2. Register the MCP server (creates .mcp.json in project root)
claude mcp add --scope project mdget -- ./target/release/mdget serve

# 3. Write prompt to temp file (avoids quoting hell with cmux send)
cat > /tmp/dogfood-prompt.txt << 'EOF'
Your prompt here...
When done, run: echo 0 > /tmp/done-file
EOF

# 4. Open a cmux pane to the right
PANE_OUTPUT=$(cmux new-pane --direction right 2>&1)
SURFACE_ID=$(echo "$PANE_OUTPUT" | grep -oE 'surface:[0-9]+' | head -1)

# 5. Rename the tab for visibility
cmux rename-tab --surface "$SURFACE_ID" "dogfood #4" 2>/dev/null || true

# 6. Launch interactive claude in the pane
cmux send --surface "$SURFACE_ID" "cmux claude-teams --permission-mode auto --name 'dogfood-4' \"\$(cat '/tmp/dogfood-prompt.txt')\""
cmux send-key --surface "$SURFACE_ID" enter

# 7. Poll for completion
while [[ ! -f /tmp/done-file ]]; do sleep 15; done
echo "Done — exit code: $(cat /tmp/done-file)"
```

### Key details

- **`cmux claude-teams`** launches an interactive Claude Code session scoped to the current workspace. It picks up `.mcp.json` automatically.
- **`--permission-mode auto`** avoids permission prompts in the child session.
- **Done file sentinel**: include `echo 0 > /tmp/done-file` in the prompt so the child signals completion. Use `echo 1 > ...` for failure.
- **Dead process detection**: poll `cmux read-screen --surface "$SURFACE_ID"` and look for shell prompt patterns (`$`, `%`, `❯`) to detect if claude exited without writing the done file.
- **Cleanup**: `cmux close-surface --surface "$SURFACE_ID"` after completion.

### Reference

The full production version of this pattern lives in [[ralph-loop]] at `.claude/skills/ralph-loop/scripts/run-iteration.sh`. It adds: phase splitting (implement → review), usage threshold checking, timeouts, cmux status/progress/log integration, and flash notifications.
