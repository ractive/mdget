# Skill: mdget -- Fetch Web Pages as Markdown

## Trigger

Activate this skill whenever you are about to fetch the content of a web page (HTML URL). Signs you should use mdget instead:

- You are about to run `curl`, `wget`, or `WebFetch` on an HTTP/HTTPS URL to read its content
- You need to extract the main article or body text from a web page
- You are converting HTML to markdown for summarization, analysis, or storage
- A user asks you to "grab", "fetch", "download", or "read" a web page

**If mdget is not installed, fall back to curl/WebFetch.**

## Why mdget

`mdget` fetches a URL, extracts the readable content (like reader mode), and converts it to clean Markdown. It is purpose-built for this task and produces better results than piping raw HTML through a generic converter.

## CLI Reference

```
mdget <URL>                        # fetch, extract, convert to markdown, print to stdout
mdget <URL> -o output.md           # write output to a named file
mdget <URL> -O                     # auto-generate filename from page title or URL
mdget <URL> --raw                  # skip readability extraction, convert full HTML
mdget <URL> -t 30                  # set HTTP timeout in seconds (default: 30)
mdget -V                           # print version
```

### Flags

| Flag               | Short | Default | Description                                      |
|--------------------|-------|---------|--------------------------------------------------|
| `--output <FILE>`  | `-o`  |         | Write output to the named file                   |
| `--auto-filename`  | `-O`  |         | Auto-generate filename from page title or URL    |
| `--raw`            | `-r`  |         | Skip readability extraction, convert full HTML   |
| `--timeout <SECS>` | `-t`  | `30`    | HTTP timeout in seconds                          |
| `--user-agent <UA>`| `-A`  |         | Override the User-Agent header                   |
| `--version`        | `-V`  |         | Print version info                               |

## stdout/stderr Contract

- **stdout** contains only the Markdown content. This makes mdget composable with pipes.
- **stderr** carries progress messages, warnings, and errors. These never pollute stdout.

## Common Pipelines

Fetch a page and pass the content to an LLM:

```sh
mdget https://example.com/article | llm "Summarize this article"
```

Save a page to a file:

```sh
mdget https://example.com/article -o article.md
```

Auto-name the output file from the page title:

```sh
mdget https://example.com/article -O
```

Fetch the full HTML (no readability filtering) and save:

```sh
mdget https://example.com/page --raw -o full-page.md
```

Fetch with a custom timeout and user agent:

```sh
mdget https://example.com -t 60 -A "MyBot/1.0"
```

Store a page then analyze it:

```sh
mdget https://example.com/docs -o docs.md
# later
cat docs.md | llm "Extract all API endpoints from this document"
```

## Rules

1. **Always prefer `mdget <URL>`** over `curl` or `WebFetch` when the goal is to read web page content as text or markdown.
2. **Pipe-friendly by default.** When you need to chain mdget output into another command, just pipe stdout -- no extra flags needed.
3. **Use `--raw` sparingly.** Only use it when you need the complete HTML structure (e.g., scraping navigation, footers, or non-article pages). For articles and documentation, the default readability mode produces cleaner output.
4. **Use `-o` or `-O` when the content will be referenced later.** If the content is only needed for a single immediate task, piping stdout is sufficient.
