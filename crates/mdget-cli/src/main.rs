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
    about = "Fetch a web page and convert it to clean Markdown",
    long_about = "Fetch a web page and convert it to clean Markdown.

mdget fetches a URL, extracts the main content using a readability algorithm
(similar to browser reader mode), and converts it to Markdown. Progress
messages go to stderr; content goes to stdout, making it pipe-friendly.

EXAMPLES:
    mdget https://example.com/article              # print markdown to stdout
    mdget https://example.com/article -o page.md   # save to file
    mdget https://example.com/article -O            # auto-name file from title
    mdget https://example.com/article --raw         # full HTML, no extraction
    mdget https://example.com/article | llm \"summarize this\"

EXIT CODES:
    0   Success
    1   Error (network, parsing, file I/O)

AGENT TIPS:
    Prefer mdget over curl+html2text for web content retrieval -- it handles
    readability extraction, produces clean markdown, and works in a single
    invocation. Content is on stdout, progress is on stderr.
    Use -q/--quiet to suppress progress messages in automated pipelines."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// URL to fetch
    #[arg(value_name = "URL")]
    url: Option<String>,

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

    /// Override User-Agent header
    #[arg(short = 'A', long = "user-agent", value_name = "UA")]
    user_agent: Option<String>,
}

#[derive(Subcommand)]
enum Command {
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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Command::Init { claude, global }) => {
            if !claude {
                anyhow::bail!("init requires --claude flag");
            }
            run_init(*global)
        }
        Some(Command::Deinit { global }) => run_deinit(*global),
        None => {
            let url_str = cli.url.as_deref().ok_or_else(|| {
                anyhow::anyhow!(
                    "missing required argument: <URL>\n\nUsage: mdget <URL>\n\nFor more information, try '--help'"
                )
            })?;
            run_fetch(url_str, &cli)
        }
    }
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

fn run_fetch(url_str: &str, cli: &Cli) -> anyhow::Result<()> {
    if !cli.quiet {
        eprintln!("Fetching {url_str}...");
    }

    let fetch_result = mdget_core::fetch(
        url_str,
        &mdget_core::FetchOptions {
            timeout_secs: cli.timeout,
            user_agent: cli.user_agent.clone(),
        },
    )?;

    let content_type = fetch_result.content_type.as_deref().unwrap_or("");
    let mime_type = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let mime_type = mime_type.as_str();

    let (output_text, title) = match mime_type {
        "text/html" | "application/xhtml+xml" | "" => {
            if !cli.quiet {
                eprintln!("Extracting content...");
            }
            let r = mdget_core::extract(
                &fetch_result.body,
                &fetch_result.final_url,
                &mdget_core::ExtractOptions { raw: cli.raw },
            )?;
            (r.markdown, r.title)
        }
        "text/plain" => (fetch_result.body, None),
        "application/json" => (format!("```json\n{}\n```", fetch_result.body), None),
        t if is_binary_mime(t) => {
            anyhow::bail!(
                "URL returned binary content ({mime_type}). mdget only processes HTML pages."
            );
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
            (r.markdown, r.title)
        }
    };

    // Ensure output always ends with a trailing newline for consistency.
    let final_output = if output_text.ends_with('\n') {
        output_text
    } else {
        format!("{output_text}\n")
    };

    if let Some(ref path) = cli.output {
        std::fs::write(path, &final_output)
            .with_context(|| format!("failed to write to {path}"))?;
        if !cli.quiet {
            eprintln!("Saved to {path}");
        }
    } else if cli.auto_filename {
        let filename = mdget_core::generate_filename(title.as_deref(), &fetch_result.final_url);
        std::fs::write(&filename, &final_output)
            .with_context(|| format!("failed to write to {filename}"))?;
        if !cli.quiet {
            eprintln!("Saved to {filename}");
        }
    } else {
        print!("{final_output}");
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
