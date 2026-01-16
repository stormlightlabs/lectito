use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Context;
use clap::Parser;
use lectito_core::{
    Document, ExtractConfig, FetchConfig, MarkdownConfig, PostProcessConfig, convert_to_markdown, extract_content,
    fetch_url,
};
use owo_colors::OwoColorize;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Output format for extracted content
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Markdown,
    Html,
    Text,
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "markdown" | "md" => Ok(Self::Markdown),
            "html" => Ok(Self::Html),
            "text" | "txt" => Ok(Self::Text),
            _ => Err(format!("Invalid format: {}. Valid options: markdown, html, text", s)),
        }
    }
}

/// Extract article content from web pages and convert to clean Markdown
#[derive(Parser, Debug)]
#[command(name = "lectito")]
#[command(author = "Lectito Contributors")]
#[command(version = "0.1.0")]
#[command(about = "Extract article content from web pages", long_about = None)]
struct Args {
    /// URL to fetch, local HTML file, or "-" for stdin
    #[arg(value_name = "INPUT")]
    input: String,

    /// Output file (default: stdout)
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Output format (markdown, html, text)
    #[arg(short, long, default_value = "markdown", value_name = "FORMAT")]
    format: OutputFormat,

    /// Include reference table with all links (Markdown only)
    #[arg(long)]
    references: bool,

    /// Include TOML frontmatter (Markdown only)
    #[arg(long)]
    frontmatter: bool,

    /// HTTP timeout in seconds
    #[arg(long, default_value = "30", value_name = "SECS")]
    timeout: u64,

    /// Custom User-Agent for HTTP requests
    #[arg(long, value_name = "UA")]
    user_agent: Option<String>,

    /// Minimum character threshold for content candidates
    #[arg(long, default_value = "500", value_name = "NUM")]
    char_threshold: usize,

    /// Maximum number of top candidates to track
    #[arg(long, default_value = "5", value_name = "NUM")]
    max_elements: usize,

    /// Strip images from output
    #[arg(long)]
    no_images: bool,

    /// Enable debug logging
    #[arg(short, long)]
    verbose: bool,
}

/// Print a styled banner for verbose mode
fn print_banner() {
    eprintln!(
        "\n{} {} {}",
        "Lectito".bold().bright_blue(),
        "v".dimmed(),
        VERSION.dimmed()
    );
    eprintln!("{}", "Extract article content from web pages".dimmed());
    eprintln!();
}

/// Print a styled step message
fn print_step(step: usize, total: usize, message: &str) {
    eprintln!("{} {}", format!("[{}/{}]", step, total).dimmed(), message.bright_cyan());
}

/// Print a success message
fn print_success(message: &str) {
    eprintln!("{} {}", "✓".green(), message.bright_green());
}

/// Print an info message
fn print_info(message: &str) {
    eprintln!("{} {}", "ℹ".blue(), message.bright_blue());
}

/// Print a warning message
#[allow(dead_code)]
fn print_warning(message: &str) {
    eprintln!("{} {}", "⚠".yellow(), message.bright_yellow());
}

/// Print an error message
#[allow(dead_code)]
fn print_error(message: &str) {
    eprintln!("{} {}", "✗".red(), message.bright_red());
}

/// Format file size for display
fn format_size(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = 1024 * KB;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.verbose {
        print_banner();
        print_info("Debug logging enabled");
        eprintln!();
    }

    let (html, size) = if args.input == "-" {
        if args.verbose {
            print_step(1, 4, "Reading from stdin");
        }
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("Failed to read from stdin")?;
        let len = buffer.len();
        (buffer, len)
    } else if args.input.starts_with("http://") || args.input.starts_with("https://") {
        if args.verbose {
            print_step(
                1,
                4,
                &format!("Fetching from {}", args.input.bright_white().underline()),
            );
        }

        let config = FetchConfig {
            timeout: args.timeout,
            user_agent: args
                .user_agent
                .unwrap_or_else(|| "Mozilla/5.0 (compatible; Lectito/1.0)".to_string()),
        };

        let content = fetch_url(&args.input, &config).await.context("Failed to fetch URL")?;
        let len = content.len();
        (content, len)
    } else {
        if args.verbose {
            print_step(1, 4, &format!("Reading from file {}", args.input.bright_white()));
        }
        let content =
            fs::read_to_string(&args.input).with_context(|| format!("Failed to read file: {}", args.input))?;
        let len = content.len();
        (content, len)
    };

    if args.verbose {
        eprintln!("  {} {}", "Size:".dimmed(), format_size(size).bright_white());
        eprintln!();
    }

    if args.verbose {
        print_step(2, 4, "Parsing HTML document");
    }

    let doc = Document::parse(&html).context("Failed to parse HTML")?;

    if args.verbose {
        if let Some(title) = doc.title() {
            eprintln!("  {} {}", "Title:".dimmed(), title.bright_white());
        }
        eprintln!();
    }

    if args.verbose {
        print_step(3, 4, "Extracting main content");
    }

    let extract_config = ExtractConfig {
        char_threshold: args.char_threshold,
        max_top_candidates: args.max_elements,
        postprocess: PostProcessConfig { strip_images: args.no_images, ..Default::default() },
        ..Default::default()
    };

    let extracted = extract_content(&doc, &extract_config).context("Failed to extract content")?;

    if args.verbose {
        eprintln!(
            "  {} {}",
            "Score:".dimmed(),
            format!("{:.1}", extracted.top_score).bright_white()
        );
        eprintln!(
            "  {} {}",
            "Elements:".dimmed(),
            extracted.element_count.to_string().bright_white()
        );
        eprintln!();
    }

    let metadata = doc.extract_metadata();

    let output = match args.format {
        OutputFormat::Markdown => {
            let config = MarkdownConfig {
                include_frontmatter: args.frontmatter,
                include_references: args.references,
                strip_images: args.no_images,
            };
            convert_to_markdown(&extracted.content, &metadata, &config).context("Failed to convert to Markdown")?
        }
        OutputFormat::Html => extracted.content,
        OutputFormat::Text => {
            let doc = Document::parse(&extracted.content).context("Failed to parse extracted HTML")?;
            doc.text_content()
        }
    };

    if args.verbose {
        print_step(4, 4, "Writing output");
        if args.format == OutputFormat::Markdown {
            if args.frontmatter {
                eprintln!("  {} {}", "Frontmatter:".dimmed(), "Yes".bright_white());
            }
            if args.references {
                eprintln!("  {} {}", "References:".dimmed(), "Yes".bright_white());
            }
        }
        eprintln!(
            "  {} {}",
            "Format:".dimmed(),
            format!("{:?}", args.format).bright_white()
        );
        eprintln!();
    }

    match args.output {
        Some(path) => {
            fs::write(&path, output).with_context(|| format!("Failed to write to file: {}", path.display()))?;
            print_success(&format!("Output written to {}", path.display().bright_white()));
        }
        None => {
            print!("{}", output);
        }
    }

    Ok(())
}
