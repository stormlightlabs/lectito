use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use lectito::ReadabilityOptions;
use reqwest::Url;
use serde::Serialize;

use crate::cli::{LlmsArgs, LlmsCommands, LlmsExpandArgs, LlmsFetchArgs, LlmsParseArgs};
use crate::{echo, fetch, read_site_profiles};

#[derive(Debug, Serialize)]
pub struct LlmsDocument {
    pub title: String,
    pub summary: Option<String>,
    pub details: Option<String>,
    pub sections: Vec<LlmsSection>,
}

#[derive(Debug, Serialize)]
pub struct LlmsSection {
    pub name: String,
    pub optional: bool,
    pub links: Vec<LlmsLink>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LlmsLink {
    pub title: String,
    pub url: String,
    pub notes: Option<String>,
}

struct LlmsSource {
    text: String,
    base: Option<String>,
}

pub fn run(args: LlmsArgs) -> Result<ExitCode> {
    match args.command {
        LlmsCommands::Fetch(args) => run_fetch(args),
        LlmsCommands::Parse(args) => run_parse(args),
        LlmsCommands::Expand(args) => run_expand(args),
    }
}

fn run_fetch(args: LlmsFetchArgs) -> Result<ExitCode> {
    let source = read_llms_source(&args.input)?;
    write_output(args.output.as_ref(), &source.text)?;
    Ok(ExitCode::SUCCESS)
}

fn run_parse(args: LlmsParseArgs) -> Result<ExitCode> {
    let source = read_llms_source(&args.input)?;
    let document = parse_llms_txt(&source.text)?;
    let output = if args.pretty {
        serde_json::to_string_pretty(&document).context("failed to serialize llms.txt JSON")?
    } else {
        serde_json::to_string(&document).context("failed to serialize llms.txt JSON")?
    };
    println!("{output}");
    Ok(ExitCode::SUCCESS)
}

fn run_expand(args: LlmsExpandArgs) -> Result<ExitCode> {
    let source = read_llms_source(&args.input)?;
    let document = parse_llms_txt(&source.text)?;
    let links = selected_links(&document, args.include_optional, args.max_links);
    let mut output = String::new();

    output.push_str("# ");
    output.push_str(&document.title);
    output.push_str("\n\n");
    if let Some(summary) = document.summary.as_deref() {
        output.push_str("> ");
        output.push_str(summary);
        output.push_str("\n\n");
    }
    if let Some(details) = document.details.as_deref() {
        output.push_str(details);
        output.push_str("\n\n");
    }

    for link in links {
        let resolved = resolve_link(&link.url, source.base.as_deref())?;
        let content = read_resource_markdown(&resolved, args.timeout)
            .with_context(|| format!("failed to expand {}", link.url))?;

        output.push_str("---\n\n");
        output.push_str("# Source: ");
        output.push_str(&link.title);
        output.push('\n');
        output.push_str("URL: ");
        output.push_str(&resolved);
        if let Some(notes) = link.notes.as_deref() {
            output.push('\n');
            output.push_str("Notes: ");
            output.push_str(notes);
        }
        output.push_str("\n\n");
        output.push_str(content.trim());
        output.push_str("\n\n");
    }

    write_output(args.output.as_ref(), &output)?;
    Ok(ExitCode::SUCCESS)
}

fn selected_links(document: &LlmsDocument, include_optional: bool, max_links: usize) -> Vec<LlmsLink> {
    document
        .sections
        .iter()
        .filter(|section| include_optional || !section.optional)
        .flat_map(|section| section.links.iter().cloned())
        .take(max_links)
        .collect()
}

fn read_llms_source(input: &str) -> Result<LlmsSource> {
    if input == "-" {
        let mut text = String::new();
        io::stdin().read_to_string(&mut text).context("failed to read stdin")?;
        return Ok(LlmsSource { text, base: None });
    }

    if input.starts_with("http://") || input.starts_with("https://") {
        let url = llms_url(input)?;
        let document = fetch::InputDocument::read_source(Some(url.as_str()), false, None)?;
        return Ok(LlmsSource { text: document.html().to_string(), base: Some(url.to_string()) });
    }

    let path = Path::new(input);
    let text = fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let base = path.parent().map(|path| path.to_string_lossy().to_string());
    Ok(LlmsSource { text, base })
}

fn llms_url(input: &str) -> Result<Url> {
    let mut url = Url::parse(input).with_context(|| format!("invalid URL: {input}"))?;
    let path = url.path();
    if path == "/" || path.is_empty() {
        url.set_path("/llms.txt");
    }
    Ok(url)
}

fn read_resource_markdown(input: &str, timeout: u64) -> Result<String> {
    if input.starts_with("http://") || input.starts_with("https://") {
        let document = fetch::InputDocument::read_source(Some(input), false, None)?;
        if looks_like_markdown(input, document.html()) {
            return Ok(document.html().to_string());
        }
        return extract_resource_markdown(document.html(), document.base_url(), timeout);
    }

    let text = fs::read_to_string(input).with_context(|| format!("failed to read {input}"))?;
    if looks_like_markdown(input, &text) {
        return Ok(text);
    }
    extract_resource_markdown(&text, None, timeout)
}

fn extract_resource_markdown(html: &str, base_url: Option<&str>, timeout: u64) -> Result<String> {
    let options = ReadabilityOptions {
        max_elems_to_parse: None,
        nb_top_candidates: 5,
        char_threshold: 500,
        content_selector: None,
        site_profiles: read_site_profiles(&[])?,
        mobile_viewport_width: Some(480),
        classes_to_preserve: Vec::new(),
        keep_classes: false,
        disable_json_ld: false,
        link_density_modifier: 0.0,
        media_retention: lectito::MediaRetention::Article,
    };
    let Some(report) = super::extract_with_timeout(html, base_url, options, timeout)? else {
        anyhow::bail!("extraction timed out after {timeout}s");
    };
    echo::render_article(
        report.article.as_ref(),
        echo::RenderOptions::new(crate::cli::OutputFormat::Markdown, false, base_url, false),
    )
}

fn looks_like_markdown(input: &str, text: &str) -> bool {
    let lower = input.to_ascii_lowercase();
    lower.ends_with(".md")
        || lower.ends_with(".mdx")
        || lower.ends_with(".txt")
        || text.trim_start().starts_with("# ")
        || text.trim_start().starts_with("---\n")
}

fn resolve_link(value: &str, base: Option<&str>) -> Result<String> {
    if value.starts_with("http://") || value.starts_with("https://") {
        return Ok(value.to_string());
    }

    if let Some(base) = base {
        if base.starts_with("http://") || base.starts_with("https://") {
            return Url::parse(base)
                .and_then(|url| url.join(value))
                .map(|url| url.to_string())
                .with_context(|| format!("failed to resolve {value} against {base}"));
        }

        let path = PathBuf::from(base).join(value);
        return Ok(path.to_string_lossy().to_string());
    }

    Ok(value.to_string())
}

fn write_output(path: Option<&PathBuf>, output: &str) -> Result<()> {
    match path {
        Some(path) => fs::write(path, output).with_context(|| format!("failed to write {}", path.display())),
        None => {
            print!("{output}");
            if !output.ends_with('\n') {
                println!();
            }
            Ok(())
        }
    }
}

pub fn parse_llms_txt(text: &str) -> Result<LlmsDocument> {
    let normalized = normalize_llms_markdown(text);
    let mut title = None;
    let mut summary = Vec::new();
    let mut details = Vec::new();
    let mut sections = Vec::new();
    let mut current: Option<LlmsSection> = None;
    let mut before_sections = true;

    for raw_line in normalized.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(value) = line.strip_prefix("# ").filter(|_| title.is_none()) {
            title = Some(value.trim().to_string());
            continue;
        }

        if let Some(value) = line.strip_prefix("## ") {
            if let Some(section) = current.take() {
                sections.push(section);
            }
            let name = value.trim().to_string();
            current = Some(LlmsSection { optional: name.eq_ignore_ascii_case("optional"), name, links: Vec::new() });
            before_sections = false;
            continue;
        }

        if let Some(section) = current.as_mut() {
            if let Some(link) = parse_link_line(line) {
                section.links.push(link);
            }
            continue;
        }

        if before_sections && line.starts_with('>') {
            summary.push(line.trim_start_matches('>').trim().to_string());
        } else if before_sections {
            details.push(line.to_string());
        }
    }

    if let Some(section) = current {
        sections.push(section);
    }

    let title = title
        .filter(|title| !title.is_empty())
        .ok_or_else(|| anyhow::anyhow!("llms.txt is missing an H1 title"))?;
    Ok(LlmsDocument { title, summary: non_empty_join(summary), details: non_empty_join(details), sections })
}

fn normalize_llms_markdown(text: &str) -> String {
    text.trim_start_matches('\u{feff}')
        .replace(" ## ", "\n## ")
        .replace(" # ", "\n# ")
        .replace(" > ", "\n> ")
        .replace(" - [", "\n- [")
}

fn parse_link_line(line: &str) -> Option<LlmsLink> {
    let line = line.strip_prefix("- ").or_else(|| line.strip_prefix("* "))?.trim();
    let start = line.find('[')?;
    let after_start = &line[start + 1..];
    let title_end = after_start.find("](")?;
    let title = after_start[..title_end].trim();
    let after_title = &after_start[title_end + 2..];
    let url_end = after_title.find(')')?;
    let url = after_title[..url_end].trim();
    let notes = after_title[url_end + 1..]
        .trim()
        .strip_prefix(':')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    if title.is_empty() || url.is_empty() {
        return None;
    }

    Some(LlmsLink { title: title.to_string(), url: url.to_string(), notes })
}

fn non_empty_join(values: Vec<String>) -> Option<String> {
    let joined = values
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    (!joined.is_empty()).then_some(joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sections_and_optional_links() {
        let document = parse_llms_txt(
            r#"# Lectito

> Readability extraction tools.

Use Markdown outputs for agent context.

## Docs

- [Quick start](https://example.com/quick.md): First steps

## Optional

- [Archive](https://example.com/archive.md)
"#,
        )
        .expect("llms.txt should parse");

        assert_eq!(document.title, "Lectito");
        assert_eq!(document.summary.as_deref(), Some("Readability extraction tools."));
        assert_eq!(document.sections.len(), 2);
        assert!(!document.sections[0].optional);
        assert!(document.sections[1].optional);
        assert_eq!(document.sections[0].links[0].notes.as_deref(), Some("First steps"));
    }

    #[test]
    fn parses_minified_common_shape() {
        let document = parse_llms_txt(
            "# Docs > Links point to Markdown. ## Guides - [Start](https://example.com/start.md): Begin ## Optional - [API](https://example.com/api.md)",
        )
        .expect("minified llms.txt should parse");

        assert_eq!(document.title, "Docs");
        assert_eq!(document.sections.len(), 2);
        assert_eq!(document.sections[0].links[0].title, "Start");
        assert!(document.sections[1].optional);
    }

    #[test]
    fn selected_links_skip_optional_by_default() {
        let document = parse_llms_txt("# Docs\n\n## Main\n- [A](a.md)\n\n## Optional\n- [B](b.md)\n")
            .expect("llms.txt should parse");

        let links = selected_links(&document, false, 10);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].title, "A");
    }
}
