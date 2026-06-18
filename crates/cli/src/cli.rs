use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use lectito::MediaRetention;

/// Extract, inspect, and debug readable article content from HTML.
#[derive(Debug, Parser)]
#[command(name = "lectito")]
#[command(about = "Extract and inspect readable article content")]
pub struct Cli {
    #[command(flatten)]
    pub extract: ExtractArgs,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Check whether a document appears to contain readable article content.
    Readable(ReadableArgs),
    /// Print extraction metadata and scoring details.
    Inspect(InspectArgs),
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OutputFormat {
    /// Print the full article structure as JSON.
    Json,
    /// Print the extracted article HTML.
    Html,
    /// Print Markdown with TOML frontmatter.
    Markdown,
    /// Print only the extracted text content.
    Text,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum DiagnosticFormat {
    /// Print diagnostics as JSON to stderr.
    Json,
    /// Print human-readable diagnostics to stderr.
    Pretty,
}

/// Extract article content.
#[derive(Debug, Args)]
pub struct ExtractArgs {
    /// URL, HTML file path, or '-' for stdin.
    pub input: Option<String>,

    /// Read HTML from stdin.
    #[arg(long)]
    pub stdin: bool,

    /// Base URL for files or stdin, used to resolve relative links.
    #[arg(long)]
    pub base_url: Option<String>,

    /// Print Markdown output.
    #[arg(long)]
    pub markdown: bool,

    /// Print extracted article HTML.
    #[arg(long)]
    pub html: bool,

    /// Print only extracted text.
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

    /// Include TOML frontmatter in Markdown output.
    #[arg(long)]
    pub frontmatter: bool,

    /// Omit TOML frontmatter from Markdown output.
    #[arg(long)]
    pub no_frontmatter: bool,

    /// Check readability and exit without extracting.
    #[arg(long)]
    pub readable: bool,

    /// Print extraction summary to stderr.
    #[arg(long)]
    pub inspect: bool,

    /// Maximum seconds to spend on full extraction.
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

    /// CSS selector for a known article container.
    #[arg(long)]
    pub content_selector: Option<String>,

    /// TOML site profile path. May be repeated.
    #[arg(long = "site-profile", value_name = "PATH")]
    pub profiles: Vec<PathBuf>,

    /// Viewport width used when applying mobile recovery rules.
    #[arg(long)]
    pub mobile_viewport_width: Option<usize>,

    /// Include extraction diagnostics on stderr.
    #[arg(long, value_enum)]
    pub diagnostic_format: Option<DiagnosticFormat>,

    /// Disable JSON-LD metadata extraction.
    #[arg(long)]
    pub disable_json_ld: bool,

    /// Media retention mode for extracted content.
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

    /// Read HTML from stdin.
    #[arg(long)]
    pub stdin: bool,

    /// Base URL for files or stdin, used to resolve relative links.
    #[arg(long)]
    pub base_url: Option<String>,

    /// Print the result as JSON.
    #[arg(long)]
    pub json: bool,

    /// Pretty-print JSON output.
    #[arg(long)]
    pub pretty: bool,

    /// Minimum text length required for readability.
    #[arg(long = "min-content-length", default_value_t = 140)]
    pub min_len: usize,

    /// Minimum readability score required.
    #[arg(long, default_value_t = 20.0)]
    pub min_score: f32,
}

/// Print extraction metadata and scoring details.
#[derive(Debug, Args)]
pub struct InspectArgs {
    /// URL, HTML file path, or '-' for stdin.
    pub input: Option<String>,

    /// Read HTML from stdin.
    #[arg(long)]
    pub stdin: bool,

    /// Base URL for files or stdin, used to resolve relative links.
    #[arg(long)]
    pub base_url: Option<String>,

    /// Print the full article structure as JSON.
    #[arg(long)]
    pub json: bool,

    /// Pretty-print JSON output.
    #[arg(long)]
    pub pretty: bool,

    /// Maximum seconds to spend on full extraction.
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

    /// CSS selector for a known article container.
    #[arg(long)]
    pub content_selector: Option<String>,

    /// TOML site profile path. May be repeated.
    #[arg(long = "site-profile", value_name = "PATH")]
    pub profiles: Vec<PathBuf>,

    /// Viewport width used when applying mobile recovery rules.
    #[arg(long)]
    pub mobile_viewport_width: Option<usize>,

    /// Disable JSON-LD metadata extraction.
    #[arg(long)]
    pub disable_json_ld: bool,

    /// Media retention mode for extracted content.
    #[arg(long = "media", default_value_t = MediaRetention::Article)]
    pub media: MediaRetention,

    /// Preserve class attributes in extracted HTML.
    #[arg(long = "keep-classes")]
    pub keep: bool,

    /// Class name to preserve in extracted HTML. May be repeated.
    #[arg(long = "preserve-class", value_name = "CLASS")]
    pub preserve: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_accepts_input_path() {
        let cli = Cli::try_parse_from(["lectito", "article.html", "--html"])
            .expect("root args should accept an input path");

        assert_eq!(cli.extract.input.as_deref(), Some("article.html"));
        assert!(cli.extract.html);
        assert!(cli.command.is_none());
    }
}
