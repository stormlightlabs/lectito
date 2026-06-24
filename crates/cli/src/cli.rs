use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use lectito::MediaRetention;

/// Extract readable article content from URLs, files, or stdin.
#[derive(Debug, Parser)]
#[command(name = "lectito")]
#[command(about = "Extract readable article content")]
#[command(long_about = "\
Extract readable article content from a URL, HTML file, or stdin. Markdown \
with TOML frontmatter is the default output.")]
pub struct Cli {
    #[command(flatten)]
    pub extract: ExtractArgs,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Check whether a document looks readable without extracting it.
    Readable(ReadableArgs),
    /// Print metadata, selected root, cleanup counts, and scoring details.
    Inspect(InspectArgs),
    /// Work with llms.txt files and LLM context bundles.
    Llms(LlmsArgs),
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OutputFormat {
    /// Print the full article structure as JSON.
    Json,
    /// Print cleaned article HTML.
    Html,
    /// Print Markdown. This is the default format.
    Markdown,
    /// Print extracted plain text.
    Text,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum DiagnosticFormat {
    /// Print diagnostics as JSON on stderr.
    Json,
    /// Print readable diagnostics on stderr.
    Pretty,
}

/// Extract article content. This is the default command.
#[derive(Debug, Args)]
pub struct ExtractArgs {
    /// URL, HTML file path, or '-' for stdin.
    pub input: Option<String>,

    /// Read HTML from stdin instead of an input argument.
    #[arg(long)]
    pub stdin: bool,

    /// Base URL for files or stdin, used to resolve relative links.
    #[arg(long)]
    pub base_url: Option<String>,

    /// Print Markdown output. This is the default.
    #[arg(long)]
    pub markdown: bool,

    /// Print cleaned article HTML.
    #[arg(long)]
    pub html: bool,

    /// Print extracted plain text.
    #[arg(long)]
    pub text: bool,

    /// Print the article structure as JSON.
    #[arg(long)]
    pub json: bool,

    /// Pretty-print JSON output.
    #[arg(long)]
    pub pretty: bool,

    /// Write article output to a file instead of stdout.
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Include TOML frontmatter in Markdown output. Enabled by default.
    #[arg(long)]
    pub frontmatter: bool,

    /// Omit TOML frontmatter from Markdown output.
    #[arg(long)]
    pub no_frontmatter: bool,

    /// Check readability and exit without extracting.
    #[arg(long)]
    pub readable: bool,

    /// Print extraction summary to stderr after article output.
    #[arg(long)]
    pub inspect: bool,

    /// Maximum seconds to spend on full extraction before exit code 3.
    #[arg(long, default_value_t = 30)]
    pub timeout: u64,

    /// Stop parsing after this many elements.
    #[arg(long)]
    pub max_elems_to_parse: Option<usize>,

    /// Minimum extracted text length required to accept an attempt.
    #[arg(long, default_value_t = 500)]
    pub char_threshold: usize,

    /// Number of top readability candidates to keep during scoring.
    #[arg(long, default_value_t = 5)]
    pub nb_top_candidates: usize,

    /// CSS selector for a known article root.
    #[arg(long)]
    pub content_selector: Option<String>,

    /// TOML site profile path. May be repeated.
    #[arg(long = "site-profile", value_name = "PATH")]
    pub profiles: Vec<PathBuf>,

    /// Viewport width used when applying mobile recovery rules.
    #[arg(long)]
    pub mobile_viewport_width: Option<usize>,

    /// Include full extraction diagnostics on stderr.
    #[arg(long, value_enum)]
    pub diagnostic_format: Option<DiagnosticFormat>,

    /// Disable JSON-LD metadata and article-body extraction.
    #[arg(long)]
    pub disable_json_ld: bool,

    /// Media retention mode: none, conservative, article, or all.
    #[arg(long = "media", default_value_t = MediaRetention::Article)]
    pub media: MediaRetention,

    /// Preserve class attributes in extracted HTML.
    #[arg(long = "keep-classes")]
    pub keep: bool,

    /// Class name to preserve in extracted HTML. May be repeated.
    #[arg(long = "preserve-class", value_name = "CLASS")]
    pub preserve: Vec<String>,
}

/// Check whether a document is probably readable.
///
/// This command only reports a boolean result. It does not extract or print
/// article content.
#[derive(Debug, Args)]
pub struct ReadableArgs {
    /// URL, HTML file path, or '-' for stdin.
    pub input: Option<String>,

    /// Read HTML from stdin instead of an input argument.
    #[arg(long)]
    pub stdin: bool,

    /// Base URL for files or stdin, used to resolve relative links.
    #[arg(long)]
    pub base_url: Option<String>,

    /// Print the readability result as JSON.
    #[arg(long)]
    pub json: bool,

    /// Pretty-print JSON output.
    #[arg(long)]
    pub pretty: bool,

    /// Minimum text length for a block to count toward readability.
    #[arg(long = "min-content-length", default_value_t = 140)]
    pub min_len: usize,

    /// Minimum accumulated score required for a readable result.
    #[arg(long, default_value_t = 20.0)]
    pub min_score: f32,
}

/// Print extraction metadata and scoring details.
#[derive(Debug, Args)]
pub struct InspectArgs {
    /// URL, HTML file path, or '-' for stdin.
    pub input: Option<String>,

    /// Read HTML from stdin instead of an input argument.
    #[arg(long)]
    pub stdin: bool,

    /// Base URL for files or stdin, used to resolve relative links.
    #[arg(long)]
    pub base_url: Option<String>,

    /// Print the article and diagnostics as JSON.
    #[arg(long)]
    pub json: bool,

    /// Pretty-print JSON output.
    #[arg(long)]
    pub pretty: bool,

    /// Maximum seconds to spend on full extraction before exit code 3.
    #[arg(long, default_value_t = 30)]
    pub timeout: u64,

    /// Stop parsing after this many elements.
    #[arg(long)]
    pub max_elems_to_parse: Option<usize>,

    /// Minimum extracted text length required to accept an attempt.
    #[arg(long, default_value_t = 500)]
    pub char_threshold: usize,

    /// Number of top readability candidates to keep during scoring.
    #[arg(long, default_value_t = 5)]
    pub nb_top_candidates: usize,

    /// CSS selector for a known article root.
    #[arg(long)]
    pub content_selector: Option<String>,

    /// TOML site profile path. May be repeated.
    #[arg(long = "site-profile", value_name = "PATH")]
    pub profiles: Vec<PathBuf>,

    /// Viewport width used when applying mobile recovery rules.
    #[arg(long)]
    pub mobile_viewport_width: Option<usize>,

    /// Disable JSON-LD metadata and article-body extraction.
    #[arg(long)]
    pub disable_json_ld: bool,

    /// Media retention mode: none, conservative, article, or all.
    #[arg(long = "media", default_value_t = MediaRetention::Article)]
    pub media: MediaRetention,

    /// Preserve class attributes in extracted HTML.
    #[arg(long = "keep-classes")]
    pub keep: bool,

    /// Class name to preserve in extracted HTML. May be repeated.
    #[arg(long = "preserve-class", value_name = "CLASS")]
    pub preserve: Vec<String>,
}

/// Work with llms.txt files and LLM context bundles.
#[derive(Debug, Args)]
pub struct LlmsArgs {
    #[command(subcommand)]
    pub command: LlmsCommands,
}

#[derive(Debug, Subcommand)]
pub enum LlmsCommands {
    /// Fetch a site's llms.txt file.
    Fetch(LlmsFetchArgs),
    /// Parse an llms.txt file into structured JSON.
    Parse(LlmsParseArgs),
    /// Expand linked resources into one Markdown context file.
    Expand(LlmsExpandArgs),
    /// Crawl pages and generate an llms.txt index.
    Generate(LlmsGenerateArgs),
}

#[derive(Debug, Args)]
pub struct LlmsFetchArgs {
    /// Site URL, llms.txt URL, local file path, or '-' for stdin.
    pub input: String,

    /// Write output to a file instead of stdout.
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct LlmsParseArgs {
    /// llms.txt URL, local file path, or '-' for stdin.
    pub input: String,

    /// Pretty-print JSON output.
    #[arg(long)]
    pub pretty: bool,
}

#[derive(Debug, Args)]
pub struct LlmsExpandArgs {
    /// llms.txt URL, local file path, or '-' for stdin.
    pub input: String,

    /// Include links from the special Optional section.
    #[arg(long)]
    pub include_optional: bool,

    /// Maximum linked resources to include.
    #[arg(long, default_value_t = 50)]
    pub max_links: usize,

    /// Maximum seconds to spend extracting each HTML resource.
    #[arg(long, default_value_t = 30)]
    pub timeout: u64,

    /// Write output to a file instead of stdout.
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct LlmsGenerateArgs {
    /// Seed URL or local HTML file to crawl.
    pub input: Option<String>,

    /// Sitemap URL or local sitemap XML file to read instead of crawling links.
    #[arg(long, value_name = "URL_OR_PATH")]
    pub sitemap: Option<String>,

    /// Title to use for the generated llms.txt file.
    #[arg(long)]
    pub title: Option<String>,

    /// Summary to include as the llms.txt blockquote.
    #[arg(long)]
    pub summary: Option<String>,

    /// H2 section name for crawled pages.
    #[arg(long, default_value = "Docs")]
    pub section: String,

    /// Maximum pages to fetch while crawling.
    #[arg(long, default_value_t = 25)]
    pub max_pages: usize,

    /// Filter candidate URLs. Prefix with '!' to exclude. May be repeated.
    #[arg(long = "filter", value_name = "PATTERN")]
    pub filters: Vec<String>,

    /// Delay between page fetches while generating, in milliseconds.
    #[arg(long = "delay", default_value_t = 0)]
    pub delay_ms: u64,

    /// User-agent token used when evaluating robots.txt.
    #[arg(long = "robots-agent", default_value = "Lectito")]
    pub robots_user_agent: String,

    /// Ignore robots.txt checks during remote generation.
    #[arg(long)]
    pub ignore_robots: bool,

    /// Discover sitemap URLs from robots.txt or /sitemap.xml.
    #[arg(long = "discover")]
    pub discover_sitemap: bool,

    /// Maximum sitemap files to read when a sitemap index is used.
    #[arg(long, default_value_t = 25)]
    pub max_sitemaps: usize,

    /// Maximum link depth from the seed page.
    #[arg(long, default_value_t = 2)]
    pub max_depth: usize,

    /// Maximum seconds to spend extracting each HTML page.
    #[arg(long, default_value_t = 30)]
    pub timeout: u64,

    /// Write output to a file instead of stdout.
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Also write expanded full-context Markdown for the generated links.
    #[arg(long = "full-output", visible_alias = "full", value_name = "PATH")]
    pub full_output: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_accepts_input_path() {
        let cli =
            Cli::try_parse_from(["lectito", "article.html", "--html"]).expect("root args should accept an input path");

        assert_eq!(cli.extract.input.as_deref(), Some("article.html"));
        assert!(cli.extract.html);
        assert!(cli.command.is_none());
    }

    #[test]
    fn llms_subcommand_parses() {
        let cli = Cli::try_parse_from(["lectito", "llms", "expand", "https://example.com", "--include-optional"])
            .expect("llms command should parse");

        match cli.command {
            Some(Commands::Llms(args)) => match args.command {
                LlmsCommands::Expand(args) => {
                    assert_eq!(args.input, "https://example.com");
                    assert!(args.include_optional);
                }
                other => panic!("unexpected llms subcommand: {other:?}"),
            },
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn llms_generate_subcommand_parses() {
        let cli = Cli::try_parse_from([
            "lectito",
            "llms",
            "generate",
            "https://example.com/docs/",
            "--max-pages",
            "5",
            "--max-depth",
            "1",
            "--full-output",
            "llms-full.txt",
        ])
        .expect("llms generate command should parse");

        match cli.command {
            Some(Commands::Llms(args)) => match args.command {
                LlmsCommands::Generate(args) => {
                    assert_eq!(args.input.as_deref(), Some("https://example.com/docs/"));
                    assert_eq!(args.max_pages, 5);
                    assert_eq!(args.max_depth, 1);
                    assert_eq!(args.full_output, Some(PathBuf::from("llms-full.txt")));
                }
                other => panic!("unexpected llms subcommand: {other:?}"),
            },
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn llms_generate_full_alias_parses() {
        let cli = Cli::try_parse_from([
            "lectito",
            "llms",
            "generate",
            "https://example.com/docs/",
            "--full",
            "full.md",
        ])
        .expect("llms generate --full alias should parse");

        match cli.command {
            Some(Commands::Llms(args)) => match args.command {
                LlmsCommands::Generate(args) => {
                    assert_eq!(args.full_output, Some(PathBuf::from("full.md")));
                }
                other => panic!("unexpected llms subcommand: {other:?}"),
            },
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn llms_generate_filters_and_delay_parse() {
        let cli = Cli::try_parse_from([
            "lectito",
            "llms",
            "generate",
            "https://example.com/docs/",
            "--filter",
            "/reference/",
            "--filter",
            "!/reference/archive/",
            "--delay",
            "100",
            "--robots-agent",
            "LectitoBot",
            "--ignore-robots",
            "--discover",
        ])
        .expect("llms generate filters should parse");

        match cli.command {
            Some(Commands::Llms(args)) => match args.command {
                LlmsCommands::Generate(args) => {
                    assert_eq!(args.filters, vec!["/reference/", "!/reference/archive/"]);
                    assert_eq!(args.delay_ms, 100);
                    assert_eq!(args.robots_user_agent, "LectitoBot");
                    assert!(args.ignore_robots);
                    assert!(args.discover_sitemap);
                }
                other => panic!("unexpected llms subcommand: {other:?}"),
            },
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn llms_generate_sitemap_subcommand_parses() {
        let cli = Cli::try_parse_from([
            "lectito",
            "llms",
            "generate",
            "--sitemap",
            "https://example.com/sitemap.xml",
            "--max-sitemaps",
            "3",
        ])
        .expect("llms generate --sitemap command should parse");

        match cli.command {
            Some(Commands::Llms(args)) => match args.command {
                LlmsCommands::Generate(args) => {
                    assert_eq!(args.input, None);
                    assert_eq!(args.sitemap.as_deref(), Some("https://example.com/sitemap.xml"));
                    assert_eq!(args.max_sitemaps, 3);
                }
                other => panic!("unexpected llms subcommand: {other:?}"),
            },
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn llms_generate_discover_sitemap_parses() {
        let cli = Cli::try_parse_from(["lectito", "llms", "generate", "https://example.com", "--discover"])
            .expect("llms generate --discover command should parse");

        match cli.command {
            Some(Commands::Llms(args)) => match args.command {
                LlmsCommands::Generate(args) => {
                    assert_eq!(args.input.as_deref(), Some("https://example.com"));
                    assert!(args.discover_sitemap);
                }
                other => panic!("unexpected llms subcommand: {other:?}"),
            },
            other => panic!("unexpected command: {other:?}"),
        }
    }
}
