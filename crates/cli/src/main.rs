use anyhow::Context;
use clap::{CommandFactory, Parser, ValueEnum};
use clap_complete::{generate, shells::Bash, shells::Fish, shells::PowerShell, shells::Zsh};
use lectito_cli::echo;
use lectito_core::formatters::{JsonConfig, convert_to_json, metadata_to_json, metadata_to_toml};
use lectito_core::siteconfig::SiteConfigProcessing;
use lectito_core::{
    Document, ExtractConfig, FetchConfig, MarkdownConfig, PostProcessConfig, convert_to_markdown, extract_content,
    extract_content_with_config, fetch_url,
};
use owo_colors::OwoColorize;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;
use url::Url;

/// Output format for extracted content
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Markdown,
    Html,
    Text,
    Json,
}

/// Shell type for completion generation
#[derive(Clone, Debug, ValueEnum)]
enum Shell {
    Bash,
    Zsh,
    Fish,
    Powershell,
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "markdown" | "md" => Ok(Self::Markdown),
            "html" => Ok(Self::Html),
            "text" | "txt" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            _ => Err(format!(
                "Invalid format: {}. Valid options: markdown, html, text, json",
                s
            )),
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
    input: Option<String>,

    /// Output file (default: stdout)
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Output format (markdown, html, text, json)
    #[arg(short, long, default_value = "markdown", value_name = "FORMAT")]
    format: OutputFormat,

    /// Include reference table with all links (Markdown/JSON only)
    #[arg(long)]
    references: bool,

    /// Include TOML frontmatter (Markdown only)
    #[arg(long)]
    frontmatter: bool,

    /// Output as JSON with metadata and content
    #[arg(short = 'j', long)]
    json: bool,

    /// Output only metadata (TOML or JSON format)
    #[arg(short = 'm', long)]
    metadata_only: bool,

    /// Metadata output format for --metadata-only (toml, json)
    #[arg(long, default_value = "toml", value_name = "FORMAT")]
    metadata_format: String,

    /// HTTP timeout in seconds
    #[arg(long, default_value = "30", value_name = "SECS")]
    timeout: u64,

    /// Custom User-Agent for HTTP requests
    #[arg(long, value_name = "UA")]
    user_agent: Option<String>,

    /// Custom site config directory
    #[arg(short = 'c', long, value_name = "DIR")]
    config_dir: Option<PathBuf>,

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

    /// Generate shell completion script
    #[arg(long, value_name = "SHELL", exclusive = true)]
    completions: Option<Shell>,
}

fn is_url(input: &str) -> bool {
    input.starts_with("http://") || input.starts_with("https://")
}

fn build_config_loader(args: &Args) -> lectito_core::ConfigLoader {
    if let Some(config_dir) = &args.config_dir {
        lectito_core::ConfigLoaderBuilder::new().custom_dir(config_dir).build()
    } else {
        lectito_core::ConfigLoader::default()
    }
}

fn load_site_config(args: &Args, input: &str) -> Option<lectito_core::siteconfig::SiteConfig> {
    let mut config_loader = build_config_loader(args);
    config_loader.load_for_url(input).ok()
}

fn resolve_user_agent(args: &Args, site_config: Option<&lectito_core::siteconfig::SiteConfig>) -> String {
    let mut user_agent = args
        .user_agent
        .clone()
        .unwrap_or_else(|| "Mozilla/5.0 (compatible; Lectito/1.0)".to_string());

    if let Some(site_config) = site_config {
        for (name, value) in &site_config.http_headers {
            if name.to_lowercase() == "user-agent" {
                user_agent = value.clone();
            }
        }
    }

    user_agent
}

async fn read_input(
    args: &Args, input: &str, site_config: Option<&lectito_core::siteconfig::SiteConfig>,
) -> anyhow::Result<(String, usize)> {
    if input == "-" {
        if args.verbose {
            echo::print_step(1, 4, "Reading from stdin");
        }
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("Failed to read from stdin")?;
        let len = buffer.len();
        Ok((buffer, len))
    } else if is_url(input) {
        if args.verbose {
            echo::print_step(1, 4, &format!("Fetching from {}", input.bright_white().underline()));
        }

        let user_agent = resolve_user_agent(args, site_config);
        let config = FetchConfig { timeout: args.timeout, user_agent };

        let content = fetch_url(input, &config).await.context("Failed to fetch URL")?;
        let len = content.len();
        Ok((content, len))
    } else {
        if args.verbose {
            echo::print_step(1, 4, &format!("Reading from file {}", input.bright_white()));
        }
        let content = fs::read_to_string(input).with_context(|| format!("Failed to read file: {}", input))?;
        let len = content.len();
        Ok((content, len))
    }
}

fn parse_document(
    args: &Args, input: &str, html: String, site_config: Option<&lectito_core::siteconfig::SiteConfig>,
    base_url: Option<Url>,
) -> anyhow::Result<Document> {
    if is_url(input) && site_config.is_some_and(|cfg| cfg.has_extraction_config()) {
        if args.verbose {
            echo::print_info("Applying site configuration");
        }
        let processed_html = site_config
            .map(|cfg| cfg.apply_text_replacements(&html))
            .unwrap_or(html);
        return Document::parse_with_base_url(&processed_html, base_url).context("Failed to parse HTML");
    }

    let preprocess_config = lectito_core::PreprocessConfig { base_url, ..Default::default() };
    let doc = Document::parse_with_preprocessing_config(&html, &preprocess_config).context("Failed to parse HTML")?;
    Ok(doc)
}

fn build_extract_config(args: &Args) -> ExtractConfig {
    ExtractConfig {
        char_threshold: args.char_threshold,
        max_top_candidates: args.max_elements,
        postprocess: PostProcessConfig { strip_images: args.no_images, ..Default::default() },
        ..Default::default()
    }
}

fn extract_article(
    args: &Args, input: &str, doc: &Document, extract_config: &ExtractConfig,
    site_config: Option<&lectito_core::siteconfig::SiteConfig>,
) -> anyhow::Result<lectito_core::ExtractedContent> {
    if is_url(input)
        && let Some(site_config) = site_config
        && site_config.has_extraction_config()
    {
        if args.verbose {
            echo::print_info("Using site configuration for extraction");
        }
        match extract_content_with_config(doc, extract_config, Some(site_config)) {
            Ok(extracted) => {
                if args.verbose {
                    echo::print_success("Successfully extracted content using site configuration");
                }
                return Ok(extracted);
            }
            Err(_) => {
                if args.verbose {
                    echo::print_warning("Site config extraction failed, falling back to heuristics");
                }
                return extract_content(doc, extract_config).context("Failed to extract content");
            }
        }
    }

    if args.verbose && is_url(input) {
        echo::print_info("No site configuration found, using heuristics");
    }

    extract_content(doc, extract_config).context("Failed to extract content")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if let Some(shell) = args.completions {
        let mut cmd = Args::command();
        let name = cmd.get_name().to_string();

        match shell {
            Shell::Bash => generate(Bash, &mut cmd, name, &mut std::io::stdout()),
            Shell::Zsh => generate(Zsh, &mut cmd, name, &mut std::io::stdout()),
            Shell::Fish => generate(Fish, &mut cmd, name, &mut std::io::stdout()),
            Shell::Powershell => generate(PowerShell, &mut cmd, name, &mut std::io::stdout()),
        }
        return Ok(());
    }

    let input = args
        .input
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Input argument required"))?;

    let total_start = Instant::now();
    let mut timings = Vec::new();

    if args.verbose {
        echo::print_banner();
        echo::print_info("Debug logging enabled");
    }

    let fetch_start = Instant::now();
    let site_config = if is_url(&input) { load_site_config(&args, &input) } else { None };
    let (html, size) = read_input(&args, &input, site_config.as_ref()).await?;

    timings.push(("Fetch/Input".to_string(), fetch_start.elapsed()));

    if args.verbose {
        eprintln!("  {} {}\n", "Size:".dimmed(), echo::format_size(size).bright_white());
    }

    if args.verbose {
        echo::print_step(2, 4, "Parsing HTML document");
    }

    let parse_start = Instant::now();

    let base_url = if is_url(&input) { Url::parse(&input).ok() } else { None };
    let doc = parse_document(&args, &input, html, site_config.as_ref(), base_url)?;

    timings.push(("Parse".to_string(), parse_start.elapsed()));

    if args.verbose
        && let Some(title) = doc.title()
    {
        eprintln!("  {} {}\n", "Title:".dimmed(), title.bright_white());
    }

    if args.verbose {
        echo::print_step(3, 4, "Extracting main content");
    }

    let extract_start = Instant::now();

    let extract_config = build_extract_config(&args);
    let extracted = extract_article(&args, &input, &doc, &extract_config, site_config.as_ref())?;

    timings.push(("Extract".to_string(), extract_start.elapsed()));

    if args.verbose {
        eprintln!(
            "  {} {}\n",
            "Score:".dimmed(),
            format!("{:.1}", extracted.top_score).bright_white()
        );
        eprintln!(
            "  {} {}\n",
            "Elements:".dimmed(),
            extracted.element_count.to_string().bright_white()
        );
    }

    let metadata = doc.extract_metadata();

    if args.verbose {
        echo::print_extraction_details(&extracted);
    }

    if args.metadata_only {
        let output = if args.metadata_format.to_lowercase() == "json" {
            metadata_to_json(&metadata, true).context("Failed to convert metadata to JSON")?
        } else {
            metadata_to_toml(&metadata).context("Failed to convert metadata to TOML")?
        };

        if args.verbose {
            echo::print_step(4, 4, "Writing output");
            eprintln!(
                "  {} {}\n",
                "Format:".dimmed(),
                args.metadata_format.to_uppercase().bright_white()
            );
            eprintln!("  {} {}\n", "Mode:".dimmed(), "Metadata Only".bright_white());
        }

        match args.output {
            Some(path) => {
                fs::write(&path, output).with_context(|| format!("Failed to write to file: {}", path.display()))?;
                echo::print_success(&format!("Output written to {}", path.display().bright_white()))
            }
            None => print!("{}", output),
        }
        return Ok(());
    }

    let format_start = Instant::now();

    let output = match args.format {
        OutputFormat::Markdown => {
            let config = MarkdownConfig {
                include_frontmatter: args.frontmatter,
                include_references: args.references,
                strip_images: args.no_images,
                include_title_heading: true, // Always include title as H1
            };
            convert_to_markdown(&extracted.content, &metadata, &config).context("Failed to convert to Markdown")?
        }
        OutputFormat::Html => extracted.content.clone(),
        OutputFormat::Text => {
            let doc = Document::parse(&extracted.content).context("Failed to parse extracted HTML")?;
            doc.text_content()
        }
        OutputFormat::Json => {
            let config = JsonConfig {
                include_markdown: true,
                include_text: true,
                include_html: true,
                include_references: args.references,
                pretty: true,
            };
            convert_to_json(&extracted.content, &metadata, &config, None).context("Failed to convert to JSON")?
        }
    };

    let output = if args.json {
        let config = JsonConfig {
            include_markdown: true,
            include_text: true,
            include_html: true,
            include_references: args.references,
            pretty: true,
        };
        convert_to_json(&extracted.content, &metadata, &config, None).context("Failed to convert to JSON")?
    } else {
        output
    };

    timings.push(("Format".to_string(), format_start.elapsed()));

    if args.verbose {
        echo::print_step(4, 4, "Writing output");
        let format_display = if args.json { "JSON".to_string() } else { format!("{:?}", args.format) };

        if args.json || args.format == OutputFormat::Json {
            if args.references {
                eprintln!("  {} {}", "References:".dimmed(), "Yes".bright_white());
            }
        } else if args.format == OutputFormat::Markdown {
            if args.frontmatter {
                eprintln!("  {} {}", "Frontmatter:".dimmed(), "Yes".bright_white());
            }
            if args.references {
                eprintln!("  {} {}", "References:".dimmed(), "Yes".bright_white());
            }
        }
        eprintln!("  {} {}\n", "Format:".dimmed(), format_display.bright_white());
    }

    match args.output {
        Some(path) => {
            fs::write(&path, output).with_context(|| format!("Failed to write to file: {}", path.display()))?;
            echo::print_success(&format!("Output written to {}", path.display().bright_white()));
        }
        None => {
            print!("{}", output);
        }
    }

    if args.verbose {
        echo::print_timing_summary(total_start.elapsed(), &timings);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lectito_core::siteconfig::SiteConfig;

    fn base_args() -> Args {
        Args {
            input: None,
            output: None,
            format: OutputFormat::Markdown,
            references: false,
            frontmatter: false,
            json: false,
            metadata_only: false,
            metadata_format: "toml".to_string(),
            timeout: 30,
            user_agent: None,
            config_dir: None,
            char_threshold: 500,
            max_elements: 5,
            no_images: false,
            verbose: false,
            completions: None,
        }
    }

    #[test]
    fn test_build_extract_config() {
        let mut args = base_args();
        args.char_threshold = 123;
        args.max_elements = 9;
        args.no_images = true;

        let config = build_extract_config(&args);

        assert_eq!(config.char_threshold, 123);
        assert_eq!(config.max_top_candidates, 9);
        assert!(config.postprocess.strip_images);
    }

    #[test]
    fn test_resolve_user_agent_prefers_site_config() {
        let mut args = base_args();
        args.user_agent = Some("CLI-UA".to_string());

        let mut site_config = SiteConfig::new();
        site_config
            .http_headers
            .insert("User-Agent".to_string(), "Site-UA".to_string());

        let resolved = resolve_user_agent(&args, Some(&site_config));
        assert_eq!(resolved, "Site-UA");
    }
}
