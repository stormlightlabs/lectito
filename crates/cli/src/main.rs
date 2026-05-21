use anyhow::Context;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use lectito_core::{ReadabilityOptions, ReadableOptions};
use lectito_core::{extract_with_diagnostics, is_probably_readable};

use clap::{Args, Parser, Subcommand};

mod echo;
mod fetch;
mod fixtures;

#[derive(Debug, Parser)]
#[command(name = "lectito")]
#[command(about = "Extract and inspect readable article content")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Parse(ParseArgs),
    Readable(ReadableArgs),
    Fixture(fixtures::FixtureArgs),
}

#[derive(Debug, Args)]
struct ParseArgs {
    path: Option<PathBuf>,
    #[arg(short = 'i', long = "input", value_name = "PATH")]
    input: Option<PathBuf>,
    #[arg(long)]
    stdin: bool,
    #[arg(long)]
    url: Option<String>,
    #[arg(long, value_enum, default_value_t = echo::OutputFormat::Json)]
    format: echo::OutputFormat,
    #[arg(long)]
    pretty: bool,
    #[arg(long)]
    max_elems_to_parse: Option<usize>,
    #[arg(long, default_value_t = 500)]
    char_threshold: usize,
    #[arg(long, default_value_t = 5)]
    nb_top_candidates: usize,
    #[arg(long)]
    content_selector: Option<String>,
    #[arg(long)]
    mobile_viewport_width: Option<usize>,
    #[arg(long, value_enum)]
    diagnostic_format: Option<echo::DiagnosticFormat>,
    #[arg(long)]
    disable_json_ld: bool,
    #[arg(long)]
    keep_classes: bool,
    #[arg(long)]
    classes_to_preserve: Vec<String>,
}

#[derive(Debug, Args)]
struct ReadableArgs {
    path: Option<PathBuf>,
    #[arg(long)]
    stdin: bool,
    #[arg(long)]
    url: Option<String>,
    #[arg(long)]
    json: bool,
    #[arg(long)]
    pretty: bool,
    #[arg(long, default_value_t = 140)]
    min_content_length: usize,
    #[arg(long, default_value_t = 20.0)]
    min_score: f32,
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Parse(args) => {
            let input_path = input_path(args.path.as_deref(), args.input.as_deref())?;
            let input = fetch::InputDocument::read(input_path, args.stdin, args.url.as_deref())?;
            let options = ReadabilityOptions {
                max_elems_to_parse: args.max_elems_to_parse,
                nb_top_candidates: args.nb_top_candidates,
                char_threshold: args.char_threshold,
                content_selector: args.content_selector,
                mobile_viewport_width: args.mobile_viewport_width.or(Some(480)),
                classes_to_preserve: args.classes_to_preserve,
                keep_classes: args.keep_classes,
                disable_json_ld: args.disable_json_ld,
                link_density_modifier: 0.0,
            };
            let report = extract_with_diagnostics(&input.html(), input.base_url().as_deref(), &options)?;
            echo::parsed(
                report.article.as_ref(),
                args.format,
                args.pretty,
                input.base_url().as_deref(),
            )?;
            if let Some(format) = args.diagnostic_format {
                io::stdout().flush().context("failed to flush parse output")?;
                echo::diagnostics(&report.diagnostics, format)?;
            }
        }
        Command::Readable(args) => {
            let input = fetch::InputDocument::read(args.path.as_deref(), args.stdin, args.url.as_deref())?;
            let options = ReadableOptions { min_content_length: args.min_content_length, min_score: args.min_score };
            let readable = is_probably_readable(&input.html(), &options)?;
            echo::readable(readable, args.json, args.pretty)?;
        }
        Command::Fixture(args) => fixtures::run(&args)?,
    }

    Ok(())
}

fn input_path<'a>(path: Option<&'a Path>, input: Option<&'a Path>) -> anyhow::Result<Option<&'a Path>> {
    match (path, input) {
        (Some(_), Some(_)) => anyhow::bail!("cannot combine a file path with -i/--input"),
        (Some(path), None) | (None, Some(path)) => Ok(Some(path)),
        (None, None) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_short_input_path() {
        let cli = Cli::try_parse_from(["lectito", "parse", "-i", "article.html", "--format", "markdown"])
            .expect("parse args should accept -i input");

        let Command::Parse(args) = cli.command else {
            panic!("expected parse command");
        };

        assert_eq!(args.input.as_deref(), Some(Path::new("article.html")));
        assert!(matches!(args.format, echo::OutputFormat::Markdown));
    }

    #[test]
    fn input_path_rejects_positional_and_flag_paths() {
        let error = input_path(Some(Path::new("positional.html")), Some(Path::new("flag.html")))
            .expect_err("two input paths should be rejected");

        assert!(error.to_string().contains("cannot combine"));
    }
}
