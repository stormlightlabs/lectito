use std::collections::{HashSet, VecDeque};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use lectito::ReadabilityOptions;
use reqwest::Url;
use scraper::{Html, Selector};
use serde::Serialize;
use sitemap::reader::{SiteMapEntity, SiteMapReader};

use crate::cli::{LlmsArgs, LlmsCommands, LlmsExpandArgs, LlmsFetchArgs, LlmsGenerateArgs, LlmsParseArgs};
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

struct CrawlItem {
    target: String,
    depth: usize,
}

struct CrawlPage {
    id: String,
    html: String,
    base_url: Option<String>,
}

struct CrawledEntry {
    title: String,
    url: String,
    notes: Option<String>,
}

pub fn run(args: LlmsArgs) -> Result<ExitCode> {
    match args.command {
        LlmsCommands::Fetch(args) => run_fetch(args),
        LlmsCommands::Parse(args) => run_parse(args),
        LlmsCommands::Expand(args) => run_expand(args),
        LlmsCommands::Generate(args) => run_generate(args),
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

fn run_generate(args: LlmsGenerateArgs) -> Result<ExitCode> {
    if args.max_pages == 0 {
        anyhow::bail!("--max-pages must be greater than zero");
    }
    if args.max_sitemaps == 0 {
        anyhow::bail!("--max-sitemaps must be greater than zero");
    }

    let entries = generate_entries(&args)?;
    let title = args
        .title
        .clone()
        .or_else(|| entries.first().map(|entry| entry.title.clone()))
        .unwrap_or_else(|| "Site documentation".to_string());
    let source = args.sitemap.as_deref().or(args.input.as_deref()).unwrap_or("sitemap");
    let summary = args
        .summary
        .clone()
        .unwrap_or_else(|| format!("Readable pages discovered from {}.", display_seed(source)));
    let output = render_generated_llms_txt(&title, &summary, &args.section, &entries);

    write_output(args.output.as_ref(), &output)?;
    Ok(if entries.is_empty() { ExitCode::from(1) } else { ExitCode::SUCCESS })
}

fn generate_entries(args: &LlmsGenerateArgs) -> Result<Vec<CrawledEntry>> {
    match (args.input.as_deref(), args.sitemap.as_deref()) {
        (Some(_), Some(_)) => anyhow::bail!("pass either a crawl seed or --sitemap, not both"),
        (Some(_), None) => crawl_entries(args),
        (None, Some(sitemap)) => sitemap_entries(sitemap, args),
        (None, None) => anyhow::bail!("pass a seed URL/path or --sitemap"),
    }
}

fn crawl_entries(args: &LlmsGenerateArgs) -> Result<Vec<CrawledEntry>> {
    let input = args.input.as_deref().context("missing crawl seed")?;
    let seed = normalize_seed(input)?;
    let mut queue = VecDeque::from([CrawlItem { target: seed.clone(), depth: 0 }]);
    let mut seen = HashSet::new();
    let mut entries = Vec::new();

    while let Some(item) = queue.pop_front() {
        if seen.len() >= args.max_pages {
            break;
        }
        if !seen.insert(item.target.clone()) {
            continue;
        }

        let page = match read_crawl_page(&item.target) {
            Ok(page) => page,
            Err(error) => {
                eprintln!("lectito: skipping {}: {error:#}", item.target);
                continue;
            }
        };

        if let Some(entry) = crawled_entry(&page, args.timeout)? {
            entries.push(entry);
        }

        if item.depth >= args.max_depth {
            continue;
        }

        for link in discover_links(&page, &seed) {
            if !seen.contains(&link) {
                queue.push_back(CrawlItem { target: link, depth: item.depth + 1 });
            }
        }
    }

    Ok(entries)
}

fn sitemap_entries(input: &str, args: &LlmsGenerateArgs) -> Result<Vec<CrawledEntry>> {
    let urls = sitemap_urls(input, args.max_sitemaps, args.max_pages)?;
    let mut entries = Vec::new();

    for url in urls.into_iter().take(args.max_pages) {
        if !Url::parse(&url)
            .ok()
            .is_none_or(|parsed| crawlable_url_path(parsed.path()))
        {
            continue;
        }

        let page = match read_crawl_page(&url) {
            Ok(page) => page,
            Err(error) => {
                eprintln!("lectito: skipping {url}: {error:#}");
                continue;
            }
        };

        if let Some(entry) = crawled_entry(&page, args.timeout)? {
            entries.push(entry);
        }
    }

    Ok(entries)
}

fn sitemap_urls(input: &str, max_sitemaps: usize, max_urls: usize) -> Result<Vec<String>> {
    let origin = sitemap_origin(input);
    let mut sitemap_queue = VecDeque::from([input.to_string()]);
    let mut seen_sitemaps = HashSet::new();
    let mut seen_urls = HashSet::new();
    let mut urls = Vec::new();

    while let Some(sitemap) = sitemap_queue.pop_front() {
        if seen_sitemaps.len() >= max_sitemaps || urls.len() >= max_urls {
            break;
        }
        if !seen_sitemaps.insert(sitemap.clone()) {
            continue;
        }

        let source = read_text_source(&sitemap).with_context(|| format!("failed to read sitemap {sitemap}"))?;
        for entity in SiteMapReader::new(source.text.as_bytes()) {
            match entity {
                SiteMapEntity::Url(entry) => {
                    let Some(url) = entry.loc.get_url().map(|url| url.to_string()) else {
                        continue;
                    };
                    if origin.as_deref().is_some_and(|origin| !same_origin(origin, &url)) {
                        continue;
                    }
                    if seen_urls.insert(url.clone()) {
                        urls.push(url);
                        if urls.len() >= max_urls {
                            break;
                        }
                    }
                }
                SiteMapEntity::SiteMap(entry) => {
                    let Some(url) = entry.loc.get_url().map(|url| url.to_string()) else {
                        continue;
                    };
                    if origin.as_deref().is_some_and(|origin| !same_origin(origin, &url)) {
                        continue;
                    }
                    if seen_sitemaps.len() + sitemap_queue.len() < max_sitemaps {
                        sitemap_queue.push_back(url);
                    }
                }
                SiteMapEntity::Err(error) => {
                    eprintln!("lectito: sitemap parse error in {sitemap}: {error}");
                }
            }
        }
    }

    Ok(urls)
}

fn read_text_source(input: &str) -> Result<LlmsSource> {
    if input.starts_with("http://") || input.starts_with("https://") {
        let document = fetch::InputDocument::read_source(Some(input), false, None)?;
        return Ok(LlmsSource { text: document.html().to_string(), base: document.base_url().map(str::to_string) });
    }

    let path = Path::new(input);
    let text = fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let base = path.parent().map(|path| path.to_string_lossy().to_string());
    Ok(LlmsSource { text, base })
}

fn sitemap_origin(input: &str) -> Option<String> {
    let url = Url::parse(input).ok()?;
    Some(format!(
        "{}://{}{}",
        url.scheme(),
        url.host_str()?,
        url.port().map(|port| format!(":{port}")).unwrap_or_default()
    ))
}

fn normalize_seed(input: &str) -> Result<String> {
    if input.starts_with("http://") || input.starts_with("https://") {
        return normalize_url(input);
    }

    Ok(PathBuf::from(input).to_string_lossy().to_string())
}

fn read_crawl_page(input: &str) -> Result<CrawlPage> {
    if input.starts_with("http://") || input.starts_with("https://") {
        let document = fetch::InputDocument::read_source(Some(input), false, None)?;
        let base_url = document.base_url().map(str::to_string);
        let id = base_url.clone().unwrap_or_else(|| input.to_string());
        return Ok(CrawlPage { id, html: document.html().to_string(), base_url });
    }

    if input.starts_with("file://") {
        let url = Url::parse(input).with_context(|| format!("invalid file URL: {input}"))?;
        let path = url
            .to_file_path()
            .map_err(|_| anyhow::anyhow!("file URL cannot be converted to a local path: {input}"))?;
        let html = fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
        return Ok(CrawlPage { id: input.to_string(), html, base_url: None });
    }

    let path = Path::new(input);
    let html = fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(CrawlPage { id: path.to_string_lossy().to_string(), html, base_url: None })
}

fn crawled_entry(page: &CrawlPage, timeout: u64) -> Result<Option<CrawledEntry>> {
    let options = default_readability_options()?;
    let Some(report) = super::extract_with_timeout(&page.html, page.base_url.as_deref(), options, timeout)? else {
        eprintln!("lectito: extraction timed out for {}", page.id);
        return Ok(None);
    };
    let Some(article) = report.article else {
        return Ok(None);
    };

    let title = article
        .title
        .as_deref()
        .filter(|title| !title.trim().is_empty())
        .unwrap_or(page.id.as_str())
        .trim()
        .to_string();
    let notes = article
        .excerpt
        .as_deref()
        .map(clean_note)
        .filter(|note| !note.is_empty());

    Ok(Some(CrawledEntry { title, url: page.id.clone(), notes }))
}

fn discover_links(page: &CrawlPage, seed: &str) -> Vec<String> {
    let document = Html::parse_document(&page.html);
    let selector = Selector::parse("a[href]").expect("valid link selector");
    let mut links = Vec::new();
    let mut seen = HashSet::new();

    for element in document.select(&selector) {
        let Some(href) = element.value().attr("href") else {
            continue;
        };
        let Some(link) = resolve_crawl_link(href, page, seed) else {
            continue;
        };
        if seen.insert(link.clone()) {
            links.push(link);
        }
    }

    links
}

fn resolve_crawl_link(href: &str, page: &CrawlPage, seed: &str) -> Option<String> {
    let href = href.trim();
    if href.is_empty()
        || href.starts_with('#')
        || href.starts_with("mailto:")
        || href.starts_with("tel:")
        || href.starts_with("javascript:")
    {
        return None;
    }

    if seed.starts_with("http://") || seed.starts_with("https://") {
        let base = page.base_url.as_deref().unwrap_or(page.id.as_str());
        let resolved = Url::parse(base).ok()?.join(href).ok()?;
        if !same_origin(seed, resolved.as_str()) || !crawlable_url_path(resolved.path()) {
            return None;
        }
        let mut resolved = resolved;
        resolved.set_fragment(None);
        resolved.set_query(None);
        return Some(resolved.to_string());
    }

    if href.starts_with("http://") || href.starts_with("https://") {
        return None;
    }

    let href = href.split('#').next().unwrap_or(href);
    let href = href.split('?').next().unwrap_or(href);
    if href.is_empty() || !crawlable_url_path(href) {
        return None;
    }
    let base = Path::new(&page.id).parent().unwrap_or_else(|| Path::new("."));
    Some(base.join(href).to_string_lossy().to_string())
}

fn same_origin(seed: &str, candidate: &str) -> bool {
    let Ok(seed) = Url::parse(seed) else {
        return false;
    };
    let Ok(candidate) = Url::parse(candidate) else {
        return false;
    };
    seed.scheme() == candidate.scheme()
        && seed.host_str() == candidate.host_str()
        && seed.port_or_known_default() == candidate.port_or_known_default()
}

fn crawlable_url_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    !matches!(
        Path::new(&lower).extension().and_then(|ext| ext.to_str()),
        Some(
            "avif"
                | "css"
                | "gif"
                | "ico"
                | "jpeg"
                | "jpg"
                | "js"
                | "json"
                | "pdf"
                | "png"
                | "svg"
                | "webp"
                | "woff"
                | "woff2"
                | "xml"
                | "zip"
        )
    )
}

fn normalize_url(input: &str) -> Result<String> {
    let mut url = Url::parse(input).with_context(|| format!("invalid URL: {input}"))?;
    url.set_fragment(None);
    Ok(url.to_string())
}

fn render_generated_llms_txt(title: &str, summary: &str, section: &str, entries: &[CrawledEntry]) -> String {
    let mut output = String::new();
    output.push_str("# ");
    output.push_str(&clean_heading(title));
    output.push_str("\n\n> ");
    output.push_str(&clean_note(summary));
    output.push_str("\n\nGenerated by Lectito from crawled readable pages.\n\n## ");
    output.push_str(&clean_heading(section));
    output.push_str("\n\n");

    for entry in entries {
        output.push_str("- [");
        output.push_str(&escape_link_label(&entry.title));
        output.push_str("](");
        output.push_str(&escape_link_destination(&entry.url));
        output.push(')');
        if let Some(notes) = entry.notes.as_deref() {
            output.push_str(": ");
            output.push_str(notes);
        }
        output.push('\n');
    }

    output
}

fn display_seed(input: &str) -> String {
    input.trim_end_matches('/').to_string()
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
        if looks_like_markdown(input, document.content_type(), document.html()) {
            return Ok(document.html().to_string());
        }
        return extract_resource_markdown(document.html(), document.base_url(), timeout);
    }

    let text = fs::read_to_string(input).with_context(|| format!("failed to read {input}"))?;
    if looks_like_markdown(input, None, &text) {
        return Ok(text);
    }
    extract_resource_markdown(&text, None, timeout)
}

fn extract_resource_markdown(html: &str, base_url: Option<&str>, timeout: u64) -> Result<String> {
    let options = default_readability_options()?;
    let Some(report) = super::extract_with_timeout(html, base_url, options, timeout)? else {
        anyhow::bail!("extraction timed out after {timeout}s");
    };
    echo::render_article(
        report.article.as_ref(),
        echo::RenderOptions::new(crate::cli::OutputFormat::Markdown, false, base_url, false),
    )
}

fn default_readability_options() -> Result<ReadabilityOptions> {
    Ok(ReadabilityOptions {
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
    })
}

fn looks_like_markdown(input: &str, content_type: Option<&str>, text: &str) -> bool {
    if let Some(content_type) = content_type.and_then(|value| value.split(';').next()) {
        let content_type = content_type.trim().to_ascii_lowercase();
        if matches!(content_type.as_str(), "text/markdown" | "text/plain") {
            return true;
        }
        if matches!(content_type.as_str(), "text/html" | "application/xhtml+xml") {
            return false;
        }
    }

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

fn clean_heading(value: &str) -> String {
    let value = value.replace(['\n', '\r'], " ");
    let value = value.trim().trim_start_matches('#').trim();
    if value.is_empty() { "Untitled".to_string() } else { value.to_string() }
}

fn clean_note(value: &str) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let limit = 220;
    if collapsed.chars().count() <= limit {
        return collapsed;
    }

    let mut clipped = collapsed.chars().take(limit).collect::<String>();
    if let Some((prefix, _)) = clipped.rsplit_once(' ') {
        clipped = prefix.to_string();
    }
    clipped.push_str("...");
    clipped
}

fn escape_link_label(value: &str) -> String {
    value.replace('\\', "\\\\").replace('[', "\\[").replace(']', "\\]")
}

fn escape_link_destination(value: &str) -> String {
    value.replace(' ', "%20").replace(')', "%29")
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

    #[test]
    fn discovers_same_origin_html_links_only() {
        let page = CrawlPage {
            id: "https://example.com/docs/index.html".to_string(),
            base_url: Some("https://example.com/docs/index.html".to_string()),
            html: r##"
                <a href="/docs/guide.html#intro">Guide</a>
                <a href="https://other.example/docs/offsite.html">Offsite</a>
                <a href="/assets/site.css">CSS</a>
                <a href="mailto:test@example.com">Mail</a>
            "##
            .to_string(),
        };

        let links = discover_links(&page, "https://example.com/docs/");

        assert_eq!(links, vec!["https://example.com/docs/guide.html"]);
    }

    #[test]
    fn renders_generated_llms_txt() {
        let output = render_generated_llms_txt(
            "Example",
            "Readable pages.",
            "Guides",
            &[CrawledEntry {
                title: "A [guide]".to_string(),
                url: "https://example.com/a guide.html".to_string(),
                notes: Some("Short note.".to_string()),
            }],
        );

        assert!(output.contains("# Example"));
        assert!(output.contains("## Guides"));
        assert!(output.contains("- [A \\[guide\\]](https://example.com/a%20guide.html): Short note."));
    }

    #[test]
    fn parses_sitemap_urls_from_file() {
        let dir = std::env::temp_dir().join(format!("lectito-sitemap-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let sitemap = dir.join("sitemap.xml");
        std::fs::write(
            &sitemap,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>https://example.com/docs/a.html</loc></url>
  <url><loc>https://example.com/docs/b.html</loc></url>
</urlset>
"#,
        )
        .expect("write sitemap");

        let urls = sitemap_urls(sitemap.to_str().expect("utf-8 path"), 5, 10).expect("parse sitemap");

        assert_eq!(
            urls,
            vec!["https://example.com/docs/a.html", "https://example.com/docs/b.html"]
        );
    }

    #[test]
    fn content_type_controls_markdown_detection() {
        assert!(looks_like_markdown(
            "https://example.com/page",
            Some("text/markdown; charset=utf-8"),
            "<p>No</p>"
        ));
        assert!(!looks_like_markdown(
            "https://example.com/page.md",
            Some("text/html"),
            "# No"
        ));
    }
}
