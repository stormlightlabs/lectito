use anyhow::Context;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use cli::{Cli, Commands};
use lectito::{ReadabilityOptions, ReadableOptions};
use lectito::{extract_with_diagnostics, is_probably_readable};

use clap::Parser;

mod cli;
mod echo;
mod fetch;
mod fixtures;

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Commands::Parse(args) => {
            let input_path = input_path(args.path.as_deref(), args.input.as_deref())?;
            let input = fetch::InputDocument::read(input_path, args.stdin, args.url.as_deref())?;
            let options = ReadabilityOptions {
                max_elems_to_parse: args.max_elems_to_parse,
                nb_top_candidates: args.nb_top_candidates,
                char_threshold: args.char_threshold,
                content_selector: args.content_selector,
                site_profiles: read_site_profiles(&args.profiles)?,
                mobile_viewport_width: args.mobile_viewport_width.or(Some(480)),
                classes_to_preserve: args.preserve,
                keep_classes: args.keep,
                disable_json_ld: args.disable_json_ld,
                link_density_modifier: 0.0,
                media_retention: args.media,
            };
            let report = extract_with_diagnostics(input.html(), input.base_url(), &options)?;
            echo::parsed(report.article.as_ref(), args.format, args.pretty, input.base_url())?;
            if let Some(format) = args.diagnostic_format {
                io::stdout().flush().context("failed to flush parse output")?;
                echo::diagnostics(&report.diagnostics, format)?;
            }
        }
        Commands::Readable(args) => {
            let input = fetch::InputDocument::read(args.path.as_deref(), args.stdin, args.url.as_deref())?;

            let options = ReadableOptions { min_content_length: args.min_len, min_score: args.min_score };

            let readable = is_probably_readable(input.html(), &options)?;
            echo::readable(readable, args.json, args.pretty)?;
        }
        Commands::Fixture(args) => fixtures::run(&args)?,
    }

    Ok(())
}

fn read_site_profiles(paths: &[PathBuf]) -> anyhow::Result<Vec<String>> {
    paths
        .iter()
        .map(|path| fs::read_to_string(path).with_context(|| format!("failed to read site profile {}", path.display())))
        .collect()
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
    fn input_path_rejects_positional_and_flag_paths() {
        let error = input_path(Some(Path::new("positional.html")), Some(Path::new("flag.html")))
            .expect_err("two input paths should be rejected");

        assert!(error.to_string().contains("cannot combine"));
    }
}
