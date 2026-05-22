use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Extract, inspect, and debug readable article content from HTML.
#[derive(Debug, Parser)]
#[command(name = "lectito")]
#[command(about = "Extract and inspect readable article content")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Extract article content from a file, stdin, or URL.
    Parse(ParseArgs),
    /// Check whether a document appears to contain readable article content.
    Readable(ReadableArgs),
    /// Compare extraction behavior against a bundled or local fixture.
    Fixture(FixtureArgs),
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
///
/// Provide exactly one input source: a positional path, `--input`, `--stdin`,
/// or `--url`. Diagnostics, when requested, are written to stderr after the
/// main output so stdout remains usable for piping.
#[derive(Debug, Args)]
pub struct ParseArgs {
    /// HTML file to read.
    pub path: Option<PathBuf>,

    /// HTML file to read.
    #[arg(short = 'i', long = "input", value_name = "PATH")]
    pub input: Option<PathBuf>,

    /// Read HTML from stdin.
    #[arg(long)]
    pub stdin: bool,

    /// Fetch HTML from a URL.
    #[arg(long)]
    pub url: Option<String>,

    /// Output format for the extracted article.
    #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
    pub format: OutputFormat,

    /// Pretty-print JSON output.
    #[arg(long)]
    pub pretty: bool,

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
    /// HTML file to read.
    pub path: Option<PathBuf>,

    /// Read HTML from stdin.
    #[arg(long)]
    pub stdin: bool,

    /// Fetch HTML from a URL.
    #[arg(long)]
    pub url: Option<String>,

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

/// Inspect fixture extraction behavior.
#[derive(Debug, Args)]
pub struct FixtureArgs {
    /// Fixture name or path to a fixture directory.
    pub path: PathBuf,

    /// Base URL to use while extracting the fixture.
    #[arg(long)]
    pub url: Option<String>,

    /// Directory where expected and actual fixture output should be written.
    #[arg(long)]
    pub diff_dir: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_short_input_path() {
        let cli = Cli::try_parse_from(["lectito", "parse", "-i", "article.html", "--format", "markdown"])
            .expect("parse args should accept -i input");

        let Commands::Parse(args) = cli.command else {
            panic!("expected parse command");
        };

        assert_eq!(args.input.as_deref(), Some(std::path::Path::new("article.html")));
        assert!(matches!(args.format, OutputFormat::Markdown));
    }
}
