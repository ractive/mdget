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
mdget <URL> -t 30                  # timeout in seconds (default: 30)
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
| `--timeout` | `-t` | HTTP timeout in seconds (default: 30) |
| `--user-agent` | `-A` | Override User-Agent header |
| `--version` | `-V` | Print version info |

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
- Single binary, no runtime dependencies
- Default user-agent: `mdget/<version>`

## License

MIT
