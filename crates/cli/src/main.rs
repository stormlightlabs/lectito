use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Commands, ExtractArgs, InspectArgs, ReadableArgs};

use lectito::ExtractionReport;
use lectito::{ReadabilityOptions, ReadableOptions};
use lectito::{extract_with_diagnostics, is_probably_readable};

use crate::echo::InspectOptions;

mod atproto;
mod cli;
mod echo;
mod fetch;
mod llms;
mod utils;

fn main() -> ExitCode {
    let parsed = Cli::parse();
    let color = color_enabled();

    let res = match parsed.command {
        Some(Commands::Readable(args)) => run_readable(args),
        Some(Commands::Inspect(args)) => run_inspect(args),
        Some(Commands::Llms(args)) => llms::run(args),
        None => run_extract(parsed.extract, color),
    };

    match res {
        Ok(code) => code,
        Err(error) => {
            eprintln!("lectito: {error:#}");
            if error.downcast_ref::<lectito::Error>().is_some() { ExitCode::from(3) } else { ExitCode::from(2) }
        }
    }
}

fn run_extract(args: ExtractArgs, color: bool) -> Result<ExitCode> {
    let input = fetch::InputDocument::read_src(args.input.as_deref(), args.stdin, args.base_url.as_deref())?;
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
        echo::RenderOptions::new(args.format, args.pretty, input.base_url(), args.frontmatter),
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
        eprintln!(
            "{}",
            echo::inspect(
                &report,
                InspectOptions::new(false, input.base_url(), false).with_atproto_warnings(input.atproto_warnings())
            )?
        );
    }
    if let Some(format) = args.diagnostic_format {
        io::stdout().flush().context("failed to flush article output")?;
        echo::diagnostics_with_atproto_warnings(&report.diagnostics, format, color, input.atproto_warnings())?;
    }

    Ok(if report.article.is_some() { ExitCode::SUCCESS } else { ExitCode::from(1) })
}

fn run_readable(args: ReadableArgs) -> Result<ExitCode> {
    let input = fetch::InputDocument::read_src(args.input.as_deref(), args.stdin, args.base_url.as_deref())?;
    let options = ReadableOptions { min_content_length: args.min_len, min_score: args.min_score };
    let Some(readable) = readable_with_timeout(input.html(), options, args.timeout)? else {
        eprintln!("lectito: readability check timed out after {}s", args.timeout);
        return Ok(ExitCode::from(3));
    };
    echo::readable(readable, args.json, args.pretty)?;
    Ok(if readable { ExitCode::SUCCESS } else { ExitCode::from(1) })
}

fn run_inspect(args: InspectArgs) -> Result<ExitCode> {
    let input = fetch::InputDocument::read_src(args.input.as_deref(), args.stdin, args.base_url.as_deref())?;
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
    println!(
        "{}",
        echo::inspect(
            &report,
            InspectOptions::new(args.pretty, input.base_url(), args.json)
                .with_atproto_warnings(input.atproto_warnings())
        )?
    );
    Ok(if report.article.is_some() { ExitCode::SUCCESS } else { ExitCode::from(1) })
}

fn extract_with_timeout(
    html: &str, base_url: Option<&str>, opts: ReadabilityOptions, timeout: u64,
) -> Result<Option<ExtractionReport>> {
    let html = html.to_string();
    let base_url = base_url.map(str::to_string);
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        let result = extract_with_diagnostics(&html, base_url.as_deref(), &opts);
        let _ = sender.send(result);
    });

    match receiver.recv_timeout(Duration::from_secs(timeout)) {
        Ok(result) => result.map(Some).map_err(Into::into),
        Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
        Err(mpsc::RecvTimeoutError::Disconnected) => anyhow::bail!("extraction worker disconnected"),
    }
}

fn readable_with_timeout(html: &str, opts: ReadableOptions, timeout: u64) -> Result<Option<bool>> {
    let html = html.to_string();
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        let result = is_probably_readable(&html, &opts);
        let _ = sender.send(result);
    });

    match receiver.recv_timeout(Duration::from_secs(timeout)) {
        Ok(result) => result.map(Some).map_err(Into::into),
        Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
        Err(mpsc::RecvTimeoutError::Disconnected) => anyhow::bail!("readability worker disconnected"),
    }
}

fn read_site_profiles(paths: &[PathBuf]) -> Result<Vec<String>> {
    paths
        .iter()
        .map(|path| fs::read_to_string(path).with_context(|| format!("failed to read site profile {}", path.display())))
        .collect()
}

fn color_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_color_env_disables_color() {
        let key = "NO_COLOR";
        let previous = std::env::var_os(key);

        unsafe {
            std::env::remove_var(key);
        }
        assert!(color_enabled());

        unsafe {
            std::env::set_var(key, "");
        }
        assert!(!color_enabled());

        if let Some(value) = previous {
            unsafe {
                std::env::set_var(key, value);
            }
        } else {
            unsafe {
                std::env::remove_var(key);
            }
        }
    }
}
