use anyhow::Context;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, ExtractArgs, InspectArgs, OutputFormat, ReadableArgs};
use lectito::ExtractionReport;
use lectito::{ReadabilityOptions, ReadableOptions};
use lectito::{extract_with_diagnostics, is_probably_readable};

mod cli;
mod echo;
mod fetch;

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("lectito: {error:#}");
            if error.downcast_ref::<lectito::Error>().is_some() { ExitCode::from(3) } else { ExitCode::from(2) }
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    match cli.command {
        Some(Commands::Readable(args)) => run_readable(args),
        Some(Commands::Inspect(args)) => run_inspect(args),
        None => run_extract(cli.extract),
    }
}

fn run_extract(args: ExtractArgs) -> Result<ExitCode> {
    if args.readable {
        let input = fetch::InputDocument::read_source(args.input.as_deref(), args.stdin, args.base_url.as_deref())?;
        let readable = is_probably_readable(input.html(), &ReadableOptions::default())?;
        echo::readable(readable, args.json, args.pretty)?;
        return Ok(if readable { ExitCode::SUCCESS } else { ExitCode::from(1) });
    }

    let format = output_format(args.markdown, args.html, args.text, args.json)?;
    let frontmatter = markdown_frontmatter(args.frontmatter, args.no_frontmatter)?;
    let input = fetch::InputDocument::read_source(args.input.as_deref(), args.stdin, args.base_url.as_deref())?;
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
    let Some(report) = extract_with_timeout(input.html(), input.base_url(), options, args.timeout)? else {
        eprintln!("lectito: extraction timed out after {}s", args.timeout);
        return Ok(ExitCode::from(3));
    };
    let output = echo::render_article(
        report.article.as_ref(),
        format,
        args.pretty,
        input.base_url(),
        frontmatter,
    )?;

    match args.output {
        Some(path) => {
            fs::write(&path, output).with_context(|| format!("failed to write {}", path.display()))?;
        }
        None => {
            if !output.is_empty() {
                println!("{output}");
            }
        }
    }

    if args.inspect {
        io::stdout().flush().context("failed to flush article output")?;
        eprintln!("{}", echo::inspect(&report, input.base_url(), false, false)?);
    }
    if let Some(format) = args.diagnostic_format {
        io::stdout().flush().context("failed to flush article output")?;
        echo::diagnostics(&report.diagnostics, format)?;
    }

    Ok(if report.article.is_some() { ExitCode::SUCCESS } else { ExitCode::from(1) })
}

fn run_readable(args: ReadableArgs) -> Result<ExitCode> {
    let input = fetch::InputDocument::read_source(args.input.as_deref(), args.stdin, args.base_url.as_deref())?;
    let options = ReadableOptions { min_content_length: args.min_len, min_score: args.min_score };
    let readable = is_probably_readable(input.html(), &options)?;
    echo::readable(readable, args.json, args.pretty)?;
    Ok(if readable { ExitCode::SUCCESS } else { ExitCode::from(1) })
}

fn run_inspect(args: InspectArgs) -> Result<ExitCode> {
    let input = fetch::InputDocument::read_source(args.input.as_deref(), args.stdin, args.base_url.as_deref())?;
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
    let Some(report) = extract_with_timeout(input.html(), input.base_url(), options, args.timeout)? else {
        eprintln!("lectito: extraction timed out after {}s", args.timeout);
        return Ok(ExitCode::from(3));
    };
    println!("{}", echo::inspect(&report, input.base_url(), args.json, args.pretty)?);
    Ok(if report.article.is_some() { ExitCode::SUCCESS } else { ExitCode::from(1) })
}

fn extract_with_timeout(
    html: &str, base_url: Option<&str>, options: ReadabilityOptions, timeout_secs: u64,
) -> Result<Option<ExtractionReport>> {
    let html = html.to_string();
    let base_url = base_url.map(str::to_string);
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        let result = extract_with_diagnostics(&html, base_url.as_deref(), &options);
        let _ = sender.send(result);
    });

    match receiver.recv_timeout(Duration::from_secs(timeout_secs)) {
        Ok(result) => result.map(Some).map_err(Into::into),
        Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
        Err(mpsc::RecvTimeoutError::Disconnected) => anyhow::bail!("extraction worker disconnected"),
    }
}

fn read_site_profiles(paths: &[PathBuf]) -> Result<Vec<String>> {
    paths
        .iter()
        .map(|path| fs::read_to_string(path).with_context(|| format!("failed to read site profile {}", path.display())))
        .collect()
}

fn output_format(markdown: bool, html: bool, text: bool, json: bool) -> Result<OutputFormat> {
    let selected = [markdown, html, text, json]
        .into_iter()
        .filter(|selected| *selected)
        .count();
    if selected > 1 {
        anyhow::bail!("choose only one output format");
    }

    if html {
        Ok(OutputFormat::Html)
    } else if text {
        Ok(OutputFormat::Text)
    } else if json {
        Ok(OutputFormat::Json)
    } else {
        Ok(OutputFormat::Markdown)
    }
}

fn markdown_frontmatter(frontmatter: bool, no_frontmatter: bool) -> Result<bool> {
    if frontmatter && no_frontmatter {
        anyhow::bail!("cannot combine --frontmatter and --no-frontmatter");
    }
    Ok(!no_frontmatter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_format_rejects_multiple_formats() {
        let error = output_format(false, true, true, false).expect_err("two formats should be rejected");

        assert!(error.to_string().contains("choose only one"));
    }
}
