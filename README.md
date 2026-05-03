# mdget

Fetch a web page, extract the main content using a readability algorithm (like browser reader mode), and convert it to clean Markdown. Content goes to stdout, progress to stderr -- pipe-friendly for LLM workflows.

## Installation

```
cargo install mdget-cli
```

## Usage

```sh
# Fetch a page and print markdown to stdout
mdget https://example.com/article

# Save to a specific file
mdget https://example.com/article -o page.md

# Auto-generate filename from page title
mdget https://example.com/article -O

# Skip readability, convert full HTML
mdget https://example.com/article --raw

# Control retries for flaky servers
mdget https://example.com --retries 5

# Set timeout and custom user agent
mdget https://example.com -t 60 -A "MyBot/1.0"

# Pipe to an LLM
mdget https://example.com/article | llm "summarize this"

# Triage URLs: print only metadata (title, word count, excerpt)
mdget -m url1 url2 url3

# LLM-optimized output: metadata + no images
mdget --include-metadata --no-images https://example.com/article

# Truncate long pages
mdget --max-length 5000 https://example.com/article

# Crawl a documentation site (follow links breadth-first)
mdget crawl https://docs.example.com

# Crawl deeper and save each page as a file
mdget crawl --depth 2 --output-dir ./docs https://docs.example.com

# Crawl with higher page limit
mdget crawl --max-pages 100 https://docs.example.com

# Crawl using sitemap.xml to discover pages
mdget crawl --sitemap --depth 0 https://docs.example.com

# Ignore robots.txt restrictions
mdget crawl --ignore-robots https://docs.example.com
```

## CLI Reference

```
mdget <URL>                        # fetch -> extract -> markdown -> stdout
mdget <URL> -o output.md           # write to explicit file
mdget <URL> -O                     # auto-generate filename
mdget <URL> --raw                  # skip readability, convert full HTML
mdget <URL> --include-metadata     # prepend YAML frontmatter
mdget <URL> -m                     # metadata only, skip body
mdget <URL> --no-images            # strip image references
mdget <URL> --max-length 5000      # truncate to N characters
mdget <URL> --retries 5             # retry transient errors (default: 2)
mdget <URL> -t 30                  # timeout in seconds (default: 30)
mdget crawl <URL>                  # crawl site following links (depth 1, max 20)
mdget crawl --depth 2 <URL>        # follow links 2 levels deep
mdget crawl --output-dir ./docs <URL>  # save each page as a file
mdget crawl --sitemap --depth 0 <URL>  # discover pages via sitemap.xml
mdget crawl --ignore-robots <URL>      # bypass robots.txt restrictions
mdget -V                           # print version
```

### Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--output` | `-o` | Write output to named file |
| `--auto-filename` | `-O` | Auto-generate filename from page title or URL |
| `--raw` | `-r` | Skip readability extraction, convert full HTML |
| `--include-metadata` | | Prepend YAML frontmatter with title, URL, date, word count |
| `--metadata-only` | `-m` | Print only YAML frontmatter, skip body |
| `--no-images` | | Strip image references from markdown output |
| `--max-length` | | Truncate output to N characters |
| `--retries` | | Number of retries for transient HTTP errors (default: 2) |
| `--timeout` | `-t` | HTTP timeout in seconds (default: 30) |
| `--user-agent` | `-A` | Override User-Agent header |
| `--quiet` | `-q` | Suppress progress messages on stderr |
| `--version` | `-V` | Print version info |

### Crawl Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--depth` | `1` | Maximum link depth to follow (0 = start page only) |
| `--delay` | `1` | Seconds to wait between requests |
| `--max-pages` | `20` | Maximum number of pages to fetch |
| `--follow-external` | | Follow links to other hosts |
| `--output-dir` | | Save each page as a file (mirrors URL path) |
| `--auto-filename` / `-O` | | Auto-generate filename per page |
| `--ignore-robots` | | Bypass robots.txt restrictions |
| `--sitemap` | | Discover pages via sitemap.xml and add to crawl queue |

## MCP Server

mdget can run as an MCP (Model Context Protocol) server, allowing AI agents to fetch web content as markdown without shelling out to a CLI.

```sh
mdget serve    # start MCP server on stdio
```

### Setup (Claude Code)

Add to your `.claude/settings.json` or `.mcp.json`:

```json
{
  "mcpServers": {
    "mdget": { "command": "mdget", "args": ["serve"] }
  }
}
```

### Setup (Claude Desktop)

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "mdget": { "command": "mdget", "args": ["serve"] }
  }
}
```

### Available Tools

| Tool | Description |
|------|-------------|
| `fetch_markdown` | Fetch a URL and return clean markdown. Supports `raw`, `include_metadata`, `no_images`, `max_length`, `timeout`, `retries` options. |
| `fetch_metadata` | Fetch a URL and return only YAML metadata (title, word count, excerpt). Useful for triaging URLs before a full fetch. |
| `batch_fetch` | Fetch multiple URLs in parallel. Returns an array of results, each with `url`, `title`, and `content` or `error`. |

## Claude Code Integration

mdget includes built-in Claude Code integration so Claude learns to use mdget for web fetching:

```sh
# Install skill + CLAUDE.md hint (project-level)
mdget init --claude

# Install globally (~/.claude/)
mdget init --claude --global

# Remove integration
mdget deinit
mdget deinit --global
```

## Design

- Content on stdout, progress on stderr -- pipe-friendly
- Uses readability algorithm for content extraction (like browser reader mode)
- Automatic retry with exponential backoff for transient HTTP errors
- Follows HTTP redirects and `<meta http-equiv="refresh">` HTML redirects
- Single binary, no runtime dependencies
- Default user-agent: `mdget/<version>`

## License

MIT
