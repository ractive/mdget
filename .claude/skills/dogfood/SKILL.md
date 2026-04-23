---
name: dogfood
description: "Use mdget as an AI agent would — fetch real pages, pipe output, try workflows — and write an honest ergonomic review. Covers what feels good, what's awkward, output quality, missing features, and improvement ideas. Use this whenever the user says /dogfood, 'dogfood mdget', 'review the UX', or 'how does mdget feel to use'."
user_invocable: true
---

# Dogfood: AI-Agent Ergonomic Review of mdget

You are an AI agent evaluating mdget as a tool you'd actually reach for in your daily work. This is not a test suite — it's a hands-on review. Use mdget the way you'd naturally use it: fetch pages you're curious about, pipe output into analysis, try workflows that matter to an agent.

Your job is to form an honest opinion and write a review.

## Setup

Build first, then set the binary path:

```bash
cargo build --workspace 2>&1 | tail -1
MDGET="./target/debug/mdget"
```

## How to review

Use mdget freely for 10-15 real tasks. Don't follow a checklist — follow your curiosity. But make sure you cover these angles:

### Output quality

Fetch 4-5 diverse pages (news article, docs page, GitHub repo, blog post, something with complex layout like tables or code blocks). For each one, actually read the markdown output. Ask yourself:

- Would I be happy receiving this as context in my prompt?
- Is the signal-to-noise ratio good? Or is there boilerplate/nav/cookie-banner junk?
- Are headings, links, code blocks, and lists preserved well?
- How does `--raw` compare to the default readability mode? When would I want each?

### Discoverability

Pretend you've never used mdget. Run `mdget --help` and `mdget -h`.

- Could you figure out what to do from the help alone?
- Are the examples useful? Would you add or change any?
- Is anything confusing or missing from the help text?
- As an AI agent reading `--help` to learn a new tool, what would you wish it told you?

### Pipelines and composability

Try real agent workflows:

```bash
# Fetch and summarize (pipe to yourself or just read the output)
$MDGET <some-url> | head -100

# Save and inspect
$MDGET <some-url> -O
# Did the filename make sense?

# Compare readability vs raw
$MDGET <some-url> > /tmp/readable.md 2>/dev/null
$MDGET <some-url> --raw > /tmp/raw.md 2>/dev/null
wc -l /tmp/readable.md /tmp/raw.md
```

- Does the stdout/stderr separation actually work for piping?
- Are progress messages helpful or noisy?
- Would you want a `--quiet` flag?

### Error experience

Try some things that should fail:

- No arguments
- A URL that doesn't exist
- A URL that returns a non-HTML content type (PDF, image, JSON API)
- A URL behind a paywall or that requires JavaScript

Are the error messages helpful? Do they tell you what went wrong and what to try instead? Or are they cryptic stack traces?

### Init/deinit (the Claude Code integration)

Run `mdget init --claude` in a temp directory. Read the installed skill file. Ask yourself:

- If I were an AI agent encountering this skill, would it actually make me reach for mdget?
- Is the skill description good enough to trigger at the right moments?
- What's missing from the skill that would make me more effective?

Clean up after: `mdget deinit`

### What's missing?

Based on your experience, think about:

- Features you wished existed while using it
- Flags or options that would make agent workflows smoother
- Information you wanted in the output but didn't get (word count? source URL? fetch timestamp?)
- Things that competing tools (curl, wget, WebFetch, reader-mode browser extensions) do that mdget doesn't

## Write the review

Structure your review as:

### What works well
Things that feel good, that you'd praise to another agent. Be specific — "the output is clean" is weak; "readability mode strips Wikipedia sidebar and footer perfectly, leaving just the article body" is strong.

### What's awkward or broken
Pain points, confusing behavior, bad output. Include the exact command and what happened.

### Output quality report
Brief notes on each page you fetched — was the extraction good, mediocre, or bad? Any patterns (e.g., "works great on articles, struggles with landing pages")?

### Feature ideas
Concrete suggestions ranked by how much they'd improve the agent experience. For each, explain the use case — why would an agent want this?

### Verdict
One paragraph: would you, as an AI agent, actually use mdget over curl+html2text or WebFetch? Why or why not? What would tip the balance?
