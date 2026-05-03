use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use clap::{Parser, Subcommand};

const SKILL_TEMPLATE: &str = include_str!("../../../templates/skill-mdget.md");
const MANAGED_SECTION_START: &str = "<!-- mdget:start -->";
const MANAGED_SECTION_END: &str = "<!-- mdget:end -->";
const MANAGED_SECTION_CONTENT: &str = "<!-- mdget:start -->\nUse `mdget <URL>` (not curl/wget) to fetch web pages as clean markdown. Extracts main content via readability, strips boilerplate. Run `mdget --help` for full usage.\n<!-- mdget:end -->";

#[derive(Parser)]
#[command(
    name = "mdget",
    version,
    about = "Fetch web pages and convert them to clean Markdown",
    long_about = "Fetch web pages and convert them to clean Markdown.

mdget fetches URLs or reads local HTML files, extracts the main content using
a readability algorithm (similar to browser reader mode), and converts it to
Markdown. Progress messages go to stderr; content goes to stdout, making it
pipe-friendly.

EXAMPLES:
    mdget https://example.com/article              # print markdown to stdout
    mdget https://example.com/article -o page.md   # save to file
    mdget https://example.com/article -O            # auto-name file from title
    mdget https://example.com/article --raw         # full HTML, no extraction
    mdget url1 url2 url3                            # fetch multiple URLs
    mdget url1 url2 -j 8                            # parallel fetching (8 threads)
    mdget ./page.html                               # convert local HTML file
    mdget -i urls.txt                               # read URLs from file

COOKBOOK:
    # LLM-optimized fetch (metadata + no images = fewer tokens)
    mdget --include-metadata --no-images https://example.com/article

    # Triage multiple URLs (metadata only, no body)
    mdget -m url1 url2 url3

    # Bulk fetch with high parallelism
    mdget -i urls.txt -j 16 -O

    # Resilient fetch (retries + longer timeout)
    mdget --retries 3 -t 60 https://flaky-site.example.com

    # Convert a saved HTML file to markdown
    mdget ./saved-page.html -o article.md

    # Cap output size for context-window-limited models
    mdget --max-length 5000 https://example.com/long-article

    # Crawl a documentation site (follow links 2 levels deep)
    mdget crawl --depth 2 --max-pages 50 https://docs.example.com

    # Crawl and save each page as a separate file
    mdget crawl --output-dir ./docs https://docs.example.com

BEHAVIOR NOTES:
    Redirects: mdget follows HTTP 3xx redirects and <meta http-equiv=\"refresh\">
    tags, up to 10 total hops (combined). The redirect chain is reported on
    stderr unless --quiet is set.

    Retries: transient failures (5xx status codes, network errors, timeouts)
    are retried with exponential backoff (1s, 2s, 4s, ...). Client errors
    (4xx) are never retried -- they indicate a permanent problem.

    Content types: HTML and XHTML are extracted via readability. JSON is
    wrapped in a fenced code block. Plain text is passed through. PDFs and
    binary types (images, audio, video) are rejected with a descriptive error.

    Multi-input errors: when processing multiple URLs, failures are reported
    per-input on stderr. Successful results are still printed. The process
    exits 1 if any input failed.

EXIT CODES:
    0   All inputs processed successfully
    1   One or more inputs failed (partial failure in batch mode, or
        a single-input error)

AGENT TIPS:
    Prefer mdget over curl+html2text for web content retrieval -- it handles
    readability extraction, produces clean markdown, and works in a single
    invocation. Content is on stdout, progress is on stderr.
    Use -q/--quiet to suppress progress messages in automated pipelines.
    Use -m/--metadata-only to triage URLs before full fetch.
    Use --no-images to save tokens -- LLMs cannot see images anyway.
    Use 'mdget crawl' to fetch entire doc sites -- follows links breadth-first.
    Note: 4xx errors are not retried; check the URL before retrying manually."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// URLs or local file paths to process
    #[arg(value_name = "INPUT")]
    inputs: Vec<String>,

    /// Read inputs from a file (one per line, skip blank lines and #comments)
    #[arg(short = 'i', long = "input-file", value_name = "FILE")]
    input_file: Option<String>,

    /// Number of parallel fetch threads (minimum 1)
    #[arg(short = 'j', long = "jobs", value_name = "N", value_parser = clap::value_parser!(u64).range(1..))]
    jobs: Option<u64>,

    /// Write output to named file
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    output: Option<String>,

    /// Auto-generate filename from page title or URL
    #[arg(
        short = 'O',
        long = "auto-filename",
        long_help = "Auto-generate filename from page title or URL.\n\nPriority: page <title> → URL path segment → hostname-YYYYMMDD.\nThe filename is slugified (lowercase, hyphens, .md extension)."
    )]
    auto_filename: bool,

    /// Skip readability extraction, convert full HTML
    #[arg(
        short = 'r',
        long = "raw",
        long_help = "Skip readability extraction, convert full HTML.\n\nBy default, mdget uses a readability algorithm to extract the main content\n(article body) from the page. With --raw, the entire HTML document is\nconverted to Markdown without filtering."
    )]
    raw: bool,

    /// Prepend YAML frontmatter with title, URL, date, word count, and other metadata
    #[arg(
        long = "include-metadata",
        long_help = "Prepend YAML frontmatter with page metadata.\n\nAlways includes: title, source URL, fetch timestamp, word count.\nOptionally includes (when available): byline, excerpt, published date,\nlanguage, site name."
    )]
    include_metadata: bool,

    /// Print only YAML frontmatter metadata, skip body
    #[arg(
        short = 'm',
        long = "metadata-only",
        long_help = "Print only YAML frontmatter metadata, skip the article body.\n\nUseful for triaging URLs: inspect title, word count, and excerpt before\ndeciding which pages to fetch in full. Still requires a full fetch\n(readability needs the DOM), but saves output tokens."
    )]
    metadata_only: bool,

    /// Strip image references from markdown output
    #[arg(
        long = "no-images",
        long_help = "Strip ![alt](url) image references from markdown output.\n\nLLMs cannot see images, so image references waste tokens. This flag\nremoves all markdown image syntax cleanly."
    )]
    no_images: bool,

    /// Truncate output to N characters (0 = no limit)
    #[arg(
        long = "max-length",
        value_name = "N",
        long_help = "Truncate output to approximately N characters.\n\nBreaks at the nearest paragraph, sentence, or word boundary before N.\nAppends '[Truncated]' when truncation occurs. Character-based (not tokens)\nfor predictability across models.\n\nUse 0 for no limit (no truncation applied)."
    )]
    max_length: Option<usize>,

    /// Suppress progress messages on stderr (errors still shown)
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,

    /// HTTP timeout in seconds
    #[arg(
        short = 't',
        long = "timeout",
        default_value = "30",
        value_name = "SECS"
    )]
    timeout: u64,

    /// Number of retries for transient HTTP errors (5xx, timeouts)
    #[arg(
        long = "retries",
        default_value = "2",
        value_name = "N",
        long_help = "Number of retries for transient HTTP errors.\n\nRetries on 5xx status codes and network/timeout errors with exponential\nbackoff (1s, 2s, 4s). Does NOT retry on 4xx client errors."
    )]
    retries: u32,

    /// Override User-Agent header
    #[arg(short = 'A', long = "user-agent", value_name = "UA")]
    user_agent: Option<String>,
}

#[derive(Subcommand)]
enum Command {
    /// Crawl a website following links breadth-first
    #[command(
        long_about = "Crawl a website breadth-first, following links up to a configurable depth.\n\n\
            Each discovered page is fetched, converted to clean markdown, and output\n\
            to stdout or saved to individual files.\n\n\
            ROBOTS.TXT:\n    \
            By default, mdget respects robots.txt. The crawl delay from robots.txt is\n    \
            used if it is higher than the configured --delay. Use --ignore-robots to\n    \
            bypass robots.txt restrictions entirely.\n\n\
            SITEMAP:\n    \
            With --sitemap, mdget fetches sitemap.xml from the start URL domain and\n    \
            seeds the crawl queue with all discovered URLs. Supports both <urlset>\n    \
            and nested <sitemapindex> formats. Combine with --depth 0 to fetch only\n    \
            sitemap URLs without following any additional links.\n\n\
            EXAMPLES:\n    \
            mdget crawl https://docs.example.com              # crawl with defaults\n    \
            mdget crawl --depth 2 https://docs.example.com    # follow links 2 levels deep\n    \
            mdget crawl --delay 2 https://docs.example.com    # 2 seconds between requests\n    \
            mdget crawl --max-pages 50 https://docs.example.com\n    \
            mdget crawl -O https://docs.example.com           # auto-generate filenames\n    \
            mdget crawl --output-dir ./docs https://docs.example.com  # save to directory\n    \
            mdget crawl --sitemap --depth 0 https://docs.example.com  # sitemap URLs + start page\n    \
            mdget crawl --ignore-robots https://docs.example.com      # skip robots.txt\n    \
            mdget crawl --path-prefix /docs/ https://example.com/docs/intro  # stay under /docs/"
    )]
    Crawl {
        /// Starting URL to crawl from
        #[arg(value_name = "URL")]
        url: String,

        /// Maximum link depth to follow (0 = start page only)
        #[arg(long, default_value = "1", value_name = "N")]
        depth: u32,

        /// Seconds to wait between requests
        #[arg(long, default_value = "1", value_name = "SECS")]
        delay: u64,

        /// Maximum number of pages to fetch
        #[arg(long = "max-pages", default_value = "20", value_name = "N")]
        max_pages: usize,

        /// Follow links to other hosts
        #[arg(long = "follow-external")]
        follow_external: bool,

        /// Write one markdown file per page in this directory (mirrors URL path)
        #[arg(long = "output-dir", value_name = "DIR")]
        output_dir: Option<String>,

        /// Auto-generate filename per page in current directory
        #[arg(short = 'O', long = "auto-filename", conflicts_with = "output_dir")]
        auto_filename: bool,

        /// Ignore robots.txt restrictions
        #[arg(long = "ignore-robots")]
        ignore_robots: bool,

        /// Discover pages via sitemap.xml and add to crawl queue
        #[arg(long)]
        sitemap: bool,

        /// Only follow links whose URL path starts with this prefix (e.g. /docs/).
        /// Auto-inferred from start URL path when not set.
        #[arg(long = "path-prefix", value_name = "PREFIX")]
        path_prefix: Option<String>,

        /// Suppress progress messages (errors still shown)
        #[arg(short = 'q', long = "quiet")]
        quiet: bool,
    },
    /// Start MCP (Model Context Protocol) server on stdio
    #[command(long_about = "Start an MCP server on stdio transport.\n\n\
        This exposes mdget as an MCP tool server so AI agents can fetch web\n\
        content as markdown without shelling out to a CLI.\n\n\
        Available tools:\n  \
        - fetch_markdown: Fetch a URL and return clean markdown\n  \
        - fetch_metadata: Fetch a URL and return only YAML metadata\n  \
        - batch_fetch: Fetch multiple URLs in parallel\n  \
        - crawl_site: Crawl a website breadth-first and return all pages\n\n\
        SETUP (Claude Code):\n  \
        Add to your .claude/settings.json or .mcp.json:\n  \
        {\n    \
          \"mcpServers\": {\n      \
            \"mdget\": { \"command\": \"mdget\", \"args\": [\"serve\"] }\n    \
          }\n  \
        }\n\n\
        SETUP (Claude Desktop):\n  \
        Add to your claude_desktop_config.json:\n  \
        {\n    \
          \"mcpServers\": {\n      \
            \"mdget\": { \"command\": \"mdget\", \"args\": [\"serve\"] }\n    \
          }\n  \
        }")]
    Serve,
    /// Install Claude Code integration (skill + CLAUDE.md hint)
    Init {
        /// Install Claude Code skill and CLAUDE.md hint
        #[arg(long)]
        claude: bool,
        /// Install to ~/.claude/ instead of ./.claude/
        #[arg(long)]
        global: bool,
    },
    /// Remove Claude Code integration artifacts
    Deinit {
        /// Remove from ~/.claude/ instead of ./.claude/
        #[arg(long)]
        global: bool,
    },
}

struct CrawlArgs<'a> {
    url: &'a str,
    depth: u32,
    delay: u64,
    max_pages: usize,
    follow_external: bool,
    output_dir: Option<&'a str>,
    auto_filename: bool,
    ignore_robots: bool,
    use_sitemap: bool,
    path_prefix: Option<&'a str>,
}

struct ProcessedInput {
    content: String,
    title: Option<String>,
    final_url: url::Url,
}

fn main() -> anyhow::Result<()> {
    let mut cli = Cli::parse();

    match &cli.command {
        Some(Command::Crawl {
            url,
            depth,
            delay,
            max_pages,
            follow_external,
            output_dir,
            auto_filename,
            ignore_robots,
            sitemap,
            path_prefix,
            quiet,
        }) => {
            cli.quiet |= quiet;
            run_crawl(
                &CrawlArgs {
                    url,
                    depth: *depth,
                    delay: *delay,
                    max_pages: *max_pages,
                    follow_external: *follow_external,
                    output_dir: output_dir.as_deref(),
                    auto_filename: *auto_filename,
                    ignore_robots: *ignore_robots,
                    use_sitemap: *sitemap,
                    path_prefix: path_prefix.as_deref(),
                },
                &cli,
            )
        }
        Some(Command::Serve) => run_serve(),
        Some(Command::Init { claude, global }) => {
            if !claude {
                anyhow::bail!("init requires --claude flag");
            }
            run_init(*global)
        }
        Some(Command::Deinit { global }) => run_deinit(*global),
        None => {
            let all_inputs = collect_inputs(&cli)?;
            if all_inputs.is_empty() {
                anyhow::bail!(
                    "no inputs provided\n\nUsage: mdget <INPUT>...\n\nFor more information, try '--help'"
                );
            }

            // -o conflicts with multiple inputs
            if all_inputs.len() > 1 && cli.output.is_some() {
                anyhow::bail!(
                    "cannot use -o/--output with multiple inputs; use -O for per-input auto-naming"
                );
            }

            run_batch(&all_inputs, &cli)
        }
    }
}

fn run_serve() -> anyhow::Result<()> {
    mdget_mcp::run_server()
}

/// Compute the output file path for a crawled URL inside `output_dir`.
///
/// When `include_host` is true (e.g. `--follow-external` crawls), the hostname
/// is prepended so that pages from different origins don't collide.
fn url_to_output_path(url: &url::Url, output_dir: &str, include_host: bool) -> PathBuf {
    let mut path = PathBuf::from(output_dir);

    if include_host && let Some(host) = url.host_str() {
        path.push(host);
    }

    // Build the path from URL path segments.
    let url_path = url.path();
    if url_path == "/" || url_path.is_empty() {
        path.push("index.md");
        return path;
    }

    // Split on '/' and push each non-empty, non-traversal segment.
    // Filter '.' and '..' to prevent path traversal outside the output directory.
    let segments: Vec<&str> = url_path
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .collect();

    if segments.is_empty() {
        path.push("index.md");
        return path;
    }

    // All segments except the last are directories.
    for segment in &segments[..segments.len() - 1] {
        path.push(segment);
    }

    let last = segments[segments.len() - 1];
    // If the original path ends with '/', treat the last segment as a directory
    // and use index.md inside it.
    if url_path.ends_with('/') {
        path.push(last);
        path.push("index.md");
    } else {
        // Append .md extension (replacing any existing extension to avoid double-ext).
        let stem = std::path::Path::new(last)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(last);
        path.push(format!("{stem}.md"));
    }

    path
}

fn run_crawl(args: &CrawlArgs, cli: &Cli) -> anyhow::Result<()> {
    // Compute effective path prefix: explicit flag wins; otherwise auto-infer from start URL.
    let effective_path_prefix: Option<String> = if let Some(explicit) = args.path_prefix {
        Some(explicit.to_string())
    } else {
        let start = url::Url::parse(args.url)
            .with_context(|| format!("invalid start URL: {}", args.url))?;
        mdget_core::infer_path_prefix(&start)
    };

    let options = mdget_core::CrawlOptions {
        fetch_options: mdget_core::FetchOptions {
            timeout_secs: cli.timeout,
            user_agent: cli.user_agent.clone(),
            retries: cli.retries,
            quiet: cli.quiet,
        },
        extract_options: mdget_core::ExtractOptions { raw: cli.raw },
        max_depth: args.depth,
        max_pages: args.max_pages,
        delay: Duration::from_secs(args.delay),
        follow_external: args.follow_external,
        no_images: cli.no_images,
        ignore_robots: args.ignore_robots,
        use_sitemap: args.use_sitemap,
        path_prefix: effective_path_prefix,
    };

    let quiet = cli.quiet;
    let max_pages = args.max_pages;
    let results = mdget_core::crawl(args.url, &options, |progress| {
        match progress {
            mdget_core::CrawlProgress::Error { url, error } => {
                // Always show errors, even in quiet mode.
                eprintln!("  \u{2717} Error: {url} ({error})");
            }
            _ if quiet => {}
            mdget_core::CrawlProgress::Fetching {
                url,
                depth: d,
                queue_size,
                fetched,
            } => {
                eprintln!(
                    "[{}/{}] Fetching {} (depth {d}, queued: {queue_size})",
                    fetched + 1,
                    max_pages,
                    url
                );
            }
            mdget_core::CrawlProgress::Fetched { url: _, title } => {
                if let Some(t) = title {
                    eprintln!("  \u{2192} {t}");
                }
            }
            mdget_core::CrawlProgress::Skipped { url, reason } => {
                eprintln!("  \u{26a0} Skipped: {url} ({reason})");
            }
            mdget_core::CrawlProgress::Done { total } => {
                eprintln!("Crawl complete: {total} pages fetched");
            }
            mdget_core::CrawlProgress::RobotsLoaded {
                domain,
                delay,
                found,
            } => {
                if *found {
                    if let Some(d) = delay {
                        eprintln!("  robots.txt loaded for {domain} (crawl-delay: {d}s)");
                    } else {
                        eprintln!("  robots.txt loaded for {domain}");
                    }
                } else {
                    eprintln!("  robots.txt not found for {domain} (allowing all)");
                }
            }
            mdget_core::CrawlProgress::SitemapLoaded { url_count } => {
                eprintln!("  Sitemap: {url_count} URLs discovered");
            }
        }
    })?;

    // Output phase
    for (page_index, result) in results.iter().enumerate() {
        let frontmatter = mdget_core::format_metadata_frontmatter(
            &result.metadata,
            result.url.as_str(),
            result.word_count,
        );
        let newline = if result.markdown.ends_with('\n') {
            ""
        } else {
            "\n"
        };
        let page_content = format!("{frontmatter}\n{}{newline}", result.markdown);

        if let Some(dir) = args.output_dir {
            let out_path = url_to_output_path(&result.url, dir, args.follow_external);
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create directory: {}", parent.display()))?;
            }
            std::fs::write(&out_path, &page_content)
                .with_context(|| format!("failed to write to {}", out_path.display()))?;
            if !quiet {
                eprintln!("Saved {}", out_path.display());
            }
        } else if args.auto_filename {
            let filename = mdget_core::generate_filename(result.title.as_deref(), &result.url);
            std::fs::write(&filename, &page_content)
                .with_context(|| format!("failed to write to {filename}"))?;
            if !quiet {
                eprintln!("Saved {filename}");
            }
        } else {
            // Stdout: separate pages with a blank line between them.
            // Each page begins with its own YAML frontmatter --- block.
            if page_index > 0 {
                println!();
            }
            print!("{page_content}");
        }
    }

    Ok(())
}

fn is_binary_mime(mime: &str) -> bool {
    mime.starts_with("image/")
        || mime.starts_with("audio/")
        || mime.starts_with("video/")
        || matches!(
            mime,
            "application/pdf" | "application/octet-stream" | "application/zip" | "application/gzip"
        )
}

/// Collect all inputs from positional args and input file.
fn collect_inputs(cli: &Cli) -> anyhow::Result<Vec<String>> {
    // clone needed: positional inputs are owned Strings in the CLI struct
    let mut inputs: Vec<String> = cli.inputs.clone();

    if let Some(ref file) = cli.input_file {
        let content = std::fs::read_to_string(file)
            .with_context(|| format!("failed to read input file: {file}"))?;
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                inputs.push(trimmed.to_string());
            }
        }
    }

    Ok(inputs)
}

enum InputKind {
    Url(String),
    LocalFile(std::path::PathBuf),
}

fn classify_input(input: &str) -> InputKind {
    if input.starts_with("file://") {
        match url::Url::parse(input) {
            Ok(url) => match url.to_file_path() {
                Ok(path) => InputKind::LocalFile(path),
                Err(()) => InputKind::Url(input.to_string()),
            },
            Err(_) => InputKind::Url(input.to_string()),
        }
    } else if std::path::Path::new(input).exists() {
        InputKind::LocalFile(std::path::PathBuf::from(input))
    } else {
        InputKind::Url(input.to_string())
    }
}

/// Process a single input (URL or local file) and return the processed content.
fn process_single(input: &str, cli: &Cli) -> anyhow::Result<ProcessedInput> {
    let fetch_result = match classify_input(input) {
        InputKind::Url(ref url) => {
            if !cli.quiet {
                eprintln!("Fetching {url}...");
            }
            mdget_core::fetch(
                url,
                &mdget_core::FetchOptions {
                    timeout_secs: cli.timeout,
                    user_agent: cli.user_agent.clone(),
                    retries: cli.retries,
                    quiet: cli.quiet,
                },
            )?
        }
        InputKind::LocalFile(ref path) => {
            if !cli.quiet {
                eprintln!("Reading {}...", path.display());
            }
            mdget_core::read_local(path)?
        }
    };

    let content_type = fetch_result.content_type.as_deref().unwrap_or("");
    let mime_type = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let mime_type = mime_type.as_str();

    let (output_text, title, metadata) = match mime_type {
        "text/html" | "application/xhtml+xml" | "" => {
            if !cli.quiet {
                eprintln!("Extracting content...");
            }
            let r = mdget_core::extract(
                &fetch_result.body,
                &fetch_result.final_url,
                &mdget_core::ExtractOptions { raw: cli.raw },
            )?;
            (r.markdown, r.title, r.metadata)
        }
        "text/plain" => (fetch_result.body, None, mdget_core::Metadata::default()),
        "application/json" => (
            format!("```json\n{}\n```", fetch_result.body),
            None,
            mdget_core::Metadata::default(),
        ),
        "application/pdf" => {
            anyhow::bail!(
                "PDF content detected; mdget cannot extract text from PDFs.\n\
                 Try a dedicated tool like pdftotext, or convert the PDF to HTML first."
            );
        }
        "application/rss+xml" | "application/atom+xml" => {
            anyhow::bail!("RSS/Atom feed detected; use a feed parser instead of mdget");
        }
        "application/xml" | "text/xml" => {
            let trimmed = fetch_result.body.trim_start();
            if trimmed.starts_with("<rss")
                || trimmed.starts_with("<feed")
                || (trimmed.starts_with("<?xml")
                    && (trimmed.contains("<rss") || trimmed.contains("<feed")))
            {
                anyhow::bail!("RSS/Atom feed detected; use a feed parser instead of mdget");
            }
            // Not a feed — warn and attempt HTML extraction
            eprintln!("Warning: unexpected Content-Type '{mime_type}', attempting HTML extraction");
            if !cli.quiet {
                eprintln!("Extracting content...");
            }
            let r = mdget_core::extract(
                &fetch_result.body,
                &fetch_result.final_url,
                &mdget_core::ExtractOptions { raw: cli.raw },
            )?;
            (r.markdown, r.title, r.metadata)
        }
        t if is_binary_mime(t) => {
            anyhow::bail!("binary content ({mime_type}); mdget only processes HTML pages");
        }
        _ => {
            eprintln!("Warning: unexpected Content-Type '{mime_type}', attempting HTML extraction");
            if !cli.quiet {
                eprintln!("Extracting content...");
            }
            let r = mdget_core::extract(
                &fetch_result.body,
                &fetch_result.final_url,
                &mdget_core::ExtractOptions { raw: cli.raw },
            )?;
            (r.markdown, r.title, r.metadata)
        }
    };

    // Post-processing pipeline:
    // 1. Strip images (if requested)
    let output_text = if cli.no_images {
        mdget_core::strip_images(&output_text)
    } else {
        output_text
    };

    // 2. Compute word count (after image stripping, before truncation)
    let wc = mdget_core::word_count(&output_text);

    // 3. Truncate (if requested; 0 means no limit)
    let output_text = if let Some(max) = cli.max_length {
        if max > 0 {
            mdget_core::truncate_output(&output_text, max)
        } else {
            output_text
        }
    } else {
        output_text
    };

    // 4. Build final output with optional metadata
    let final_output = if cli.metadata_only {
        mdget_core::format_metadata_frontmatter(&metadata, fetch_result.final_url.as_str(), wc)
    } else if cli.include_metadata {
        let frontmatter =
            mdget_core::format_metadata_frontmatter(&metadata, fetch_result.final_url.as_str(), wc);
        // Ensure trailing newline on body before prepending
        let body = if output_text.ends_with('\n') {
            output_text
        } else {
            format!("{output_text}\n")
        };
        format!("{frontmatter}\n{body}")
    } else {
        // Ensure trailing newline
        if output_text.ends_with('\n') {
            output_text
        } else {
            format!("{output_text}\n")
        }
    };

    Ok(ProcessedInput {
        content: final_output,
        title,
        final_url: fetch_result.final_url,
    })
}

fn run_batch(inputs: &[String], cli: &Cli) -> anyhow::Result<()> {
    let multi = inputs.len() > 1;
    let jobs = usize::try_from(cli.jobs.unwrap_or(if multi { 4 } else { 1 })).unwrap_or(4);

    // Process all inputs, collecting results in input order.
    let results: Vec<(&str, Result<ProcessedInput, String>)> = if jobs <= 1 || inputs.len() <= 1 {
        // Sequential processing
        inputs
            .iter()
            .map(|input| {
                let result = process_single(input, cli).map_err(|e| format!("{e:#}"));
                (input.as_str(), result)
            })
            .collect()
    } else {
        // Parallel processing with std::thread::scope
        let chunk_size = inputs.len().div_ceil(jobs);

        std::thread::scope(|s| {
            let chunks: Vec<_> = inputs.chunks(chunk_size).collect();
            let handles: Vec<_> = chunks
                .iter()
                .map(|chunk| {
                    s.spawn(|| {
                        chunk
                            .iter()
                            .map(|input| {
                                let result =
                                    process_single(input, cli).map_err(|e| format!("{e:#}"));
                                (input.as_str(), result)
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();

            let mut results = Vec::with_capacity(inputs.len());
            for (handle, chunk) in handles.into_iter().zip(chunks.iter()) {
                if let Ok(chunk_results) = handle.join() {
                    results.extend(chunk_results);
                } else {
                    eprintln!("Error: a processing thread panicked unexpectedly");
                    for input in *chunk {
                        results.push((
                            input.as_str(),
                            Err("processing thread panicked".to_string()),
                        ));
                    }
                }
            }
            results
        })
    };

    // Warn when multiple URLs are going to stdout without file output
    if inputs.len() > 1 && cli.output.is_none() && !cli.auto_filename {
        eprintln!(
            "warning: multiple URLs to stdout — output is concatenated and hard to split. \
             Use -O (auto-named files), -o FILE (single file), or the MCP batch_fetch tool instead."
        );
    }

    // Output phase: emit results in input order
    let mut had_error = false;

    for (i, (input, result)) in results.iter().enumerate() {
        match result {
            Ok(ProcessedInput {
                content,
                title,
                final_url,
            }) => {
                if multi && i > 0 {
                    println!("\n---\n");
                }
                if let Some(ref path) = cli.output {
                    // Single input only (validated above)
                    std::fs::write(path, content)
                        .with_context(|| format!("failed to write to {path}"))?;
                    if !cli.quiet {
                        eprintln!("Saved to {path}");
                    }
                } else if cli.auto_filename {
                    let filename = mdget_core::generate_filename(title.as_deref(), final_url);
                    std::fs::write(&filename, content)
                        .with_context(|| format!("failed to write to {filename}"))?;
                    if !cli.quiet {
                        eprintln!("Saved to {filename}");
                    }
                } else {
                    print!("{content}");
                }
            }
            Err(e) => {
                had_error = true;
                eprintln!("Error: {input}: {e}");
            }
        }
    }

    if had_error {
        anyhow::bail!("one or more inputs failed");
    }

    Ok(())
}

fn home_dir() -> anyhow::Result<std::path::PathBuf> {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .or_else(|_| std::env::var("USERPROFILE").map(std::path::PathBuf::from))
        .context("could not determine home directory")
}

fn run_init(global: bool) -> anyhow::Result<()> {
    let (base_dir, claude_md_path) = resolve_paths(global)?;

    let skill_dir = base_dir.join("skills").join("mdget");
    std::fs::create_dir_all(&skill_dir)
        .with_context(|| format!("failed to create skill directory: {}", skill_dir.display()))?;

    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(&skill_path, SKILL_TEMPLATE)
        .with_context(|| format!("failed to write skill file: {}", skill_path.display()))?;
    eprintln!("Installed skill to {}", skill_path.display());

    if claude_md_path.exists() && is_git_tracked(&claude_md_path) {
        eprintln!(
            "warning: {} is tracked by git — this will create uncommitted changes",
            claude_md_path.display()
        );
    }

    upsert_managed_section(&claude_md_path)?;
    eprintln!("Updated CLAUDE.md");

    Ok(())
}

fn run_deinit(global: bool) -> anyhow::Result<()> {
    let (base_dir, claude_md_path) = resolve_paths(global)?;

    // Remove skill file
    let skill_file = base_dir.join("skills").join("mdget").join("SKILL.md");
    if skill_file.exists() {
        std::fs::remove_file(&skill_file)
            .with_context(|| format!("failed to remove {}", skill_file.display()))?;
        eprintln!("Removed {}", skill_file.display());
    } else {
        eprintln!("Skipped (not found): {}", skill_file.display());
    }

    // Remove mdget dir if empty
    let mdget_dir = base_dir.join("skills").join("mdget");
    remove_dir_if_empty(&mdget_dir)?;

    // Remove skills dir if empty
    let skills_dir = base_dir.join("skills");
    remove_dir_if_empty(&skills_dir)?;

    // Strip managed section from CLAUDE.md
    strip_managed_section(&claude_md_path)?;

    // Remove the base .claude/ dir itself if now empty
    remove_dir_if_empty(&base_dir)?;

    Ok(())
}

fn is_git_tracked(path: &std::path::Path) -> bool {
    // Convert to string for git command; if it can't be converted, assume not tracked
    let Some(path_str) = path.to_str() else {
        return false;
    };
    std::process::Command::new("git")
        .args(["ls-files", "--error-unmatch", path_str])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Returns (base_dir, claude_md_path).
/// - global: base_dir = ~/.claude/, claude_md_path = ~/.claude/CLAUDE.md
/// - project: base_dir = ./.claude/, claude_md_path = ./CLAUDE.md
fn resolve_paths(global: bool) -> anyhow::Result<(std::path::PathBuf, std::path::PathBuf)> {
    if global {
        let home = home_dir()?;
        let base_dir = home.join(".claude");
        let claude_md = base_dir.join("CLAUDE.md");
        Ok((base_dir, claude_md))
    } else {
        let base_dir = std::path::PathBuf::from(".claude");
        let claude_md = std::path::PathBuf::from("CLAUDE.md");
        Ok((base_dir, claude_md))
    }
}

fn upsert_managed_section(claude_md_path: &std::path::Path) -> anyhow::Result<()> {
    let existing = if claude_md_path.exists() {
        std::fs::read_to_string(claude_md_path)
            .with_context(|| format!("failed to read {}", claude_md_path.display()))?
    } else {
        String::new()
    };

    let new_content = if existing.contains(MANAGED_SECTION_START) {
        // Replace existing managed section
        replace_managed_section(&existing)
    } else {
        // Append at end, with a leading newline if the file is non-empty and doesn't end with one
        if existing.is_empty() {
            MANAGED_SECTION_CONTENT.to_string()
        } else if existing.ends_with('\n') {
            format!("{existing}{MANAGED_SECTION_CONTENT}\n")
        } else {
            format!("{existing}\n\n{MANAGED_SECTION_CONTENT}\n")
        }
    };

    // Ensure parent directory exists for the CLAUDE.md file
    if let Some(parent) = claude_md_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory: {}", parent.display()))?;
    }

    std::fs::write(claude_md_path, &new_content)
        .with_context(|| format!("failed to write {}", claude_md_path.display()))?;

    Ok(())
}

/// Returns the content with the managed section replaced, or the original content unchanged
/// if the start marker is present but the end marker is missing (malformed section).
fn replace_managed_section(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut inside = false;
    let mut replaced = false;

    for line in content.lines() {
        if line.trim() == MANAGED_SECTION_START {
            if !replaced {
                result.push_str(MANAGED_SECTION_CONTENT);
                result.push('\n');
                replaced = true;
            }
            inside = true;
            continue;
        }
        if inside {
            if line.trim() == MANAGED_SECTION_END {
                inside = false;
            }
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }

    // If we never found the end marker the section is malformed — return original unchanged.
    if inside {
        return content.to_string();
    }

    result
}

fn strip_managed_section(claude_md_path: &std::path::Path) -> anyhow::Result<()> {
    if !claude_md_path.exists() {
        eprintln!("Skipped (not found): {}", claude_md_path.display());
        return Ok(());
    }

    let existing = std::fs::read_to_string(claude_md_path)
        .with_context(|| format!("failed to read {}", claude_md_path.display()))?;

    if !existing.contains(MANAGED_SECTION_START) {
        eprintln!("No managed section found in {}", claude_md_path.display());
        return Ok(());
    }

    let mut result = String::with_capacity(existing.len());
    let mut inside = false;

    for line in existing.lines() {
        if line.trim() == MANAGED_SECTION_START {
            inside = true;
            continue;
        }
        if inside {
            if line.trim() == MANAGED_SECTION_END {
                inside = false;
            }
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }

    // If the end marker was never found the section is malformed — leave file unchanged.
    if inside {
        eprintln!(
            "Warning: malformed managed section in {} (missing end marker) — leaving file unchanged",
            claude_md_path.display()
        );
        return Ok(());
    }

    // Trim trailing blank lines but keep a final newline if there's content
    let trimmed = result.trim_end().to_string();

    if trimmed.is_empty() {
        std::fs::remove_file(claude_md_path)
            .with_context(|| format!("failed to remove {}", claude_md_path.display()))?;
        eprintln!("Removed {} (now empty)", claude_md_path.display());
    } else {
        let final_content = format!("{trimmed}\n");
        std::fs::write(claude_md_path, &final_content)
            .with_context(|| format!("failed to write {}", claude_md_path.display()))?;
        eprintln!("Updated {}", claude_md_path.display());
    }

    Ok(())
}

fn remove_dir_if_empty(dir: &std::path::Path) -> anyhow::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    let is_empty = dir
        .read_dir()
        .with_context(|| format!("failed to read directory: {}", dir.display()))?
        .next()
        .is_none();
    if is_empty {
        std::fs::remove_dir(dir)
            .with_context(|| format!("failed to remove directory: {}", dir.display()))?;
        eprintln!("Removed directory {}", dir.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_path_root_url() {
        let url = url::Url::parse("https://example.com/").unwrap();
        let path = url_to_output_path(&url, "/out", false);
        assert_eq!(path, PathBuf::from("/out").join("index.md"));
    }

    #[test]
    fn test_output_path_simple_page() {
        let url = url::Url::parse("https://example.com/about").unwrap();
        let path = url_to_output_path(&url, "/out", false);
        assert_eq!(path, PathBuf::from("/out").join("about.md"));
    }

    #[test]
    fn test_output_path_nested() {
        let url = url::Url::parse("https://example.com/docs/api/reference").unwrap();
        let path = url_to_output_path(&url, "/out", false);
        assert_eq!(
            path,
            PathBuf::from("/out")
                .join("docs")
                .join("api")
                .join("reference.md")
        );
    }

    #[test]
    fn test_output_path_trailing_slash() {
        let url = url::Url::parse("https://example.com/docs/").unwrap();
        let path = url_to_output_path(&url, "/out", false);
        assert_eq!(path, PathBuf::from("/out").join("docs").join("index.md"));
    }

    #[test]
    fn test_output_path_with_host() {
        let url = url::Url::parse("https://example.com/page").unwrap();
        let path = url_to_output_path(&url, "/out", true);
        assert_eq!(
            path,
            PathBuf::from("/out").join("example.com").join("page.md")
        );
    }

    #[test]
    fn test_output_path_filters_dotdot() {
        let url = url::Url::parse("https://example.com/../../etc/passwd").unwrap();
        let path = url_to_output_path(&url, "/out", false);
        // ".." segments should be filtered out
        assert!(path.starts_with(std::path::Path::new("/out")));
        assert!(!path.to_string_lossy().contains(".."));
    }

    #[test]
    fn test_output_path_filters_dot() {
        let url = url::Url::parse("https://example.com/./page").unwrap();
        let path = url_to_output_path(&url, "/out", false);
        assert!(path.starts_with(std::path::Path::new("/out")));
        assert_eq!(path, PathBuf::from("/out").join("page.md"));
    }

    #[test]
    fn test_output_path_replaces_extension() {
        let url = url::Url::parse("https://example.com/page.html").unwrap();
        let path = url_to_output_path(&url, "/out", false);
        assert_eq!(path, PathBuf::from("/out").join("page.md"));
    }

    #[test]
    fn test_output_path_empty_path() {
        let url = url::Url::parse("https://example.com").unwrap();
        let path = url_to_output_path(&url, "/out", false);
        assert_eq!(path, PathBuf::from("/out").join("index.md"));
    }

    #[test]
    fn test_output_path_encoded_traversal() {
        // %2e%2e is decoded to ".." by the url crate, so should also be filtered
        let url = url::Url::parse("https://example.com/%2e%2e/etc/passwd").unwrap();
        let path = url_to_output_path(&url, "/out", false);
        assert!(path.starts_with(std::path::Path::new("/out")));
        assert!(!path.to_string_lossy().contains(".."));
    }
}
