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
mdget <URL> --include-metadata     # prepend YAML frontmatter
mdget <URL> -m                     # metadata only, skip body
mdget <URL> --no-images            # strip image references
mdget <URL> --max-length 5000      # truncate to N characters
mdget <URL> --retries 5            # retry transient errors (default: 2)
mdget <URL> -t 30                  # set HTTP timeout in seconds (default: 30)
mdget crawl <URL>                  # crawl site, following links (depth 1, max 20 pages)
mdget crawl --depth 2 <URL>        # follow links 2 levels deep
mdget crawl --output-dir ./docs <URL>  # save each page as a separate file
mdget crawl --max-pages 50 <URL>   # increase page limit
mdget crawl --sitemap <URL>        # seed crawl queue from sitemap.xml
mdget crawl --ignore-robots <URL>  # skip robots.txt restrictions
mdget serve                        # start MCP server on stdio
mdget -V                           # print version
```

### Flags

| Flag               | Short | Default | Description                                      |
|--------------------|-------|---------|--------------------------------------------------|
| `--output <FILE>`  | `-o`  |         | Write output to the named file                   |
| `--auto-filename`  | `-O`  |         | Auto-generate filename from page title or URL    |
| `--raw`            | `-r`  |         | Skip readability extraction, convert full HTML   |
| `--include-metadata`|      |         | Prepend YAML frontmatter with metadata           |
| `--metadata-only`  | `-m`  |         | Print only YAML frontmatter, skip body           |
| `--no-images`      |       |         | Strip image references from markdown output      |
| `--max-length <N>` |       |         | Truncate output to N characters                  |
| `--retries <N>`    |       | `2`     | Retry count for transient HTTP errors            |
| `--timeout <SECS>` | `-t`  | `30`    | HTTP timeout in seconds                          |
| `--user-agent <UA>`| `-A`  |         | Override the User-Agent header                   |
| `--quiet`          | `-q`  |         | Suppress progress messages on stderr             |
| `--version`        | `-V`  |         | Print version info                               |

### Crawl Subcommand Flags

| Flag                | Short | Default | Description                                      |
|---------------------|-------|---------|--------------------------------------------------|
| `--depth <N>`       |       | `1`     | Maximum link depth to follow                     |
| `--delay <SECS>`    |       | `1`     | Seconds to wait between requests                 |
| `--max-pages <N>`   |       | `20`    | Maximum number of pages to fetch                 |
| `--follow-external` |       |         | Follow links to other hosts                      |
| `--output-dir <DIR>`|       |         | Save each page as a file (mirrors URL path)      |
| `--auto-filename`   | `-O`  |         | Auto-generate filename per page                  |
| `--ignore-robots`   |       |         | Ignore robots.txt restrictions                   |
| `--sitemap`         |       |         | Seed crawl queue from sitemap.xml                |

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

## Crawling

Crawl a documentation site and save each page:

```sh
mdget crawl --output-dir ./docs --depth 2 --max-pages 100 https://docs.example.com
```

Crawl to stdout (pages delimited by YAML frontmatter fences):

```sh
mdget crawl https://docs.example.com | llm "Summarize these docs"
```

## MCP Server

mdget can also run as an MCP server, giving you direct tool access without shelling out:

```sh
mdget serve    # start MCP server on stdio
```

Add to `.claude/settings.json` or `.mcp.json`:

```json
{
  "mcpServers": {
    "mdget": { "command": "mdget", "args": ["serve"] }
  }
}
```

Available MCP tools: `fetch_markdown`, `fetch_metadata`, `batch_fetch`, `crawl_site`.

## Rules

1. **Always prefer `mdget <URL>`** over `curl` or `WebFetch` when the goal is to read web page content as text or markdown.
2. **If mdget is configured as an MCP server**, prefer the MCP tools (`fetch_markdown`, `fetch_metadata`, `batch_fetch`, `crawl_site`) over the CLI for programmatic access.
3. **Pipe-friendly by default.** When you need to chain mdget output into another command, just pipe stdout -- no extra flags needed.
4. **Use `--raw` sparingly.** Only use it when you need the complete HTML structure (e.g., scraping navigation, footers, or non-article pages). For articles and documentation, the default readability mode produces cleaner output.
5. **Use `-o` or `-O` when the content will be referenced later.** If the content is only needed for a single immediate task, piping stdout is sufficient.
