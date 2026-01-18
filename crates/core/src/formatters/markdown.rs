use crate::metadata::Metadata;
use crate::{LectitoError, Result};
use scraper::{Html, Selector};
use std::collections::HashMap;

/// Configuration for Markdown conversion
#[derive(Debug, Clone, Default)]
pub struct MarkdownConfig {
    /// Include TOML frontmatter with metadata
    pub include_frontmatter: bool,
    /// Generate reference table for all links
    pub include_references: bool,
    /// Strip images from output
    pub strip_images: bool,
    /// Include title as H1 heading at the start of content
    pub include_title_heading: bool,
}

/// A collected link reference
#[derive(Debug, Clone)]
pub struct LinkReference {
    /// The link text
    pub text: String,
    /// The link URL
    pub url: String,
}

/// Convert HTML content to Markdown with optional frontmatter and references
pub fn convert_to_markdown(html: &str, metadata: &Metadata, config: &MarkdownConfig) -> Result<String> {
    let mut output = String::new();

    if config.include_frontmatter {
        output.push_str(&generate_frontmatter(metadata)?);
        output.push('\n');
    }

    if config.include_title_heading
        && let Some(title) = &metadata.title
    {
        output.push_str(&format!("# {}\n\n", title));
    }

    let processed_html = if config.strip_images { strip_images(html)? } else { html.to_string() };

    let markdown = html_to_markdown(&processed_html);
    output.push_str(&markdown);

    if config.include_references {
        let links = extract_links(&processed_html)?;
        if !links.is_empty() {
            output.push_str("\n\n## References\n\n");
            output.push_str(&generate_reference_table(&links));
        }
    }

    Ok(output)
}

/// Generate TOML frontmatter from metadata
fn generate_frontmatter(metadata: &Metadata) -> Result<String> {
    let mut frontmatter = String::from("+++");

    if let Some(title) = &metadata.title {
        frontmatter.push_str(&format!("\ntitle = {}", toml_escape_string(title)));
    }

    if let Some(author) = &metadata.author {
        frontmatter.push_str(&format!("\nauthor = {}", toml_escape_string(author)));
    }

    if let Some(date) = &metadata.date {
        frontmatter.push_str(&format!("\ndate = {}", toml_escape_string(date)));
    }

    if let Some(site) = &metadata.site_name {
        frontmatter.push_str(&format!("\nsite = {}", toml_escape_string(site)));
    }

    if let Some(excerpt) = &metadata.excerpt {
        frontmatter.push_str(&format!("\nexcerpt = {}", toml_escape_string(excerpt)));
    }

    if let Some(word_count) = metadata.word_count {
        frontmatter.push_str(&format!("\nword_count = {}", word_count));
    }

    if let Some(reading_time) = metadata.reading_time_minutes {
        frontmatter.push_str(&format!("\nreading_time_minutes = {:.1}", reading_time));
    }

    frontmatter.push_str("\n+++\n");

    Ok(frontmatter)
}

/// Escape a string for TOML format
fn toml_escape_string(s: &str) -> String {
    let needs_escape = s.contains('"') || s.contains('\\') || s.contains('\n');
    if needs_escape {
        format!(
            "\"{}\"",
            s.replace('\\', "\\\\").replace('\"', "\\\"").replace('\n', "\\n")
        )
    } else {
        format!("\"{}\"", s)
    }
}

/// Convert HTML to Markdown using htmd crate
#[cfg(feature = "markdown")]
fn html_to_markdown(html: &str) -> String {
    htmd::convert(html).unwrap_or_default()
}

/// Fallback HTML to text conversion when markdown feature is disabled
#[cfg(not(feature = "markdown"))]
fn html_to_markdown(html: &str) -> String {
    let doc = scraper::Html::parse_document(html);
    doc.root_element().text().collect::<String>()
}

/// Strip all img tags from HTML
fn strip_images(html: &str) -> Result<String> {
    let mut output = Vec::new();
    let mut rewriter = lol_html::HtmlRewriter::new(
        lol_html::Settings {
            element_content_handlers: vec![lol_html::element!("img", |el| {
                el.remove_and_keep_content();
                Ok(())
            })],
            ..Default::default()
        },
        |c: &[u8]| output.extend_from_slice(c),
    );

    match rewriter.write(html.as_bytes()) {
        Ok(_) => {}
        Err(_) => return Ok(html.to_string()),
    }

    match rewriter.end() {
        Ok(_) => {
            if output.is_empty() {
                Ok(html.to_string())
            } else {
                String::from_utf8(output).map_err(|e| LectitoError::HtmlParseError(e.to_string()))
            }
        }
        Err(_) => Ok(html.to_string()),
    }
}

/// Extract all links from HTML content
pub fn extract_links(html: &str) -> Result<Vec<LinkReference>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href]").unwrap();
    let mut links = Vec::new();
    let mut seen_urls = HashMap::new();

    for element in document.select(&selector) {
        let text = element.text().collect::<String>().trim().to_string();
        let url = match element.value().attr("href") {
            Some(u) => u.to_string(),
            None => continue,
        };

        if text.is_empty() || url.is_empty() {
            continue;
        }

        if !seen_urls.contains_key(&url) {
            seen_urls.insert(url.clone(), links.len());
            links.push(LinkReference { text, url });
        }
    }

    Ok(links)
}

/// Generate a reference table from collected links
fn generate_reference_table(links: &[LinkReference]) -> String {
    let mut table = String::from("| # | Text | URL |\n");
    table.push_str("|---|------|-----|\n");

    for (i, link) in links.iter().enumerate() {
        let escaped_text = escape_pipe(&link.text);
        let escaped_url = escape_pipe(&link.url);
        table.push_str(&format!("| {} | {} | {} |\n", i + 1, escaped_text, escaped_url));
    }

    table
}

/// Escape pipe characters for Markdown tables
fn escape_pipe(s: &str) -> String {
    s.replace('|', "\\|")
}

/// Markdown formatter with configurable options
pub struct MarkdownFormatter {
    config: MarkdownConfig,
}

impl MarkdownFormatter {
    pub fn new(config: MarkdownConfig) -> Self {
        Self { config }
    }

    pub fn convert(&self, html: &str, metadata: &Metadata) -> Result<String> {
        convert_to_markdown(html, metadata, &self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_to_markdown_basic() {
        let html = r#"<h1>Title</h1><p>This is a paragraph.</p>"#;
        let metadata = Metadata::default();
        let config = MarkdownConfig::default();

        let result = convert_to_markdown(html, &metadata, &config);
        assert!(result.is_ok());
        let markdown = result.unwrap();
        assert!(markdown.contains("# Title"));
        assert!(markdown.contains("This is a paragraph."));
    }

    #[test]
    fn test_html_to_markdown_with_links() {
        let html = r#"<p>Check out <a href="https://example.com">this link</a>.</p>"#;
        let metadata = Metadata::default();
        let config = MarkdownConfig::default();

        let result = convert_to_markdown(html, &metadata, &config);
        assert!(result.is_ok());
        let markdown = result.unwrap();
        assert!(markdown.contains("[this link](https://example.com)"));
    }

    #[test]
    fn test_html_to_markdown_with_images() {
        let html = r#"<p>An image: <img src="photo.jpg" alt="A photo"></p>"#;
        let metadata = Metadata::default();
        let config = MarkdownConfig::default();

        let result = convert_to_markdown(html, &metadata, &config);
        assert!(result.is_ok());
        let markdown = result.unwrap();
        assert!(markdown.contains("![A photo](photo.jpg)"));
    }

    #[test]
    fn test_strip_images() {
        let html = r#"<p>Text before <img src="photo.jpg"> text after.</p>"#;
        let metadata = Metadata::default();
        let config = MarkdownConfig { strip_images: true, ..Default::default() };

        let result = convert_to_markdown(html, &metadata, &config);
        assert!(result.is_ok());
        let markdown = result.unwrap();
        assert!(!markdown.contains("photo.jpg"));
    }

    #[test]
    fn test_frontmatter_generation() {
        let metadata = Metadata {
            title: Some("Test Title".to_string()),
            author: Some("Test Author".to_string()),
            date: Some("2024-01-15".to_string()),
            site_name: Some("Test Site".to_string()),
            excerpt: Some("Test excerpt".to_string()),
            word_count: Some(500),
            reading_time_minutes: Some(2.5),
            language: None,
        };

        let frontmatter = generate_frontmatter(&metadata).unwrap();
        assert!(frontmatter.contains("title = \"Test Title\""));
        assert!(frontmatter.contains("author = \"Test Author\""));
        assert!(frontmatter.contains("date = \"2024-01-15\""));
        assert!(frontmatter.contains("site = \"Test Site\""));
        assert!(frontmatter.contains("word_count = 500"));
        assert!(frontmatter.contains("reading_time_minutes = 2.5"));
    }

    #[test]
    fn test_extract_links() {
        let html = r#"
            <p>
                <a href="https://example.com">Example</a>
                <a href="/relative">Relative</a>
            </p>
        "#;

        let links = extract_links(html).unwrap();
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].text, "Example");
        assert_eq!(links[0].url, "https://example.com");
        assert_eq!(links[1].text, "Relative");
        assert_eq!(links[1].url, "/relative");
    }

    #[test]
    fn test_reference_table_generation() {
        let links = vec![
            LinkReference { text: "Example Site".to_string(), url: "https://example.com".to_string() },
            LinkReference { text: "Test Link".to_string(), url: "https://test.com".to_string() },
        ];

        let table = generate_reference_table(&links);
        assert!(table.contains("| # | Text | URL |"));
        assert!(table.contains("|---|"));
        assert!(table.contains("| 1 | Example Site | https://example.com |"));
        assert!(table.contains("| 2 | Test Link | https://test.com |"));
    }

    #[test]
    fn test_convert_with_references() {
        let html = r#"<p>Visit <a href="https://example.com">Example</a> for more info.</p>"#;
        let metadata = Metadata::default();
        let config = MarkdownConfig { include_references: true, ..Default::default() };

        let result = convert_to_markdown(html, &metadata, &config);
        assert!(result.is_ok());
        let markdown = result.unwrap();
        assert!(markdown.contains("## References"));
        assert!(markdown.contains("| # | Text | URL |"));
    }

    #[test]
    fn test_escape_pipe() {
        assert_eq!(escape_pipe("foo|bar"), r"foo\|bar");
        assert_eq!(escape_pipe("no pipes"), "no pipes");
        assert_eq!(escape_pipe("a|b|c"), r"a\|b\|c");
    }

    #[test]
    fn test_markdown_formatter() {
        let html = r#"<h1>Test</h1><p>Content</p>"#;
        let metadata = Metadata::default();
        let config = MarkdownConfig::default();
        let formatter = MarkdownFormatter::new(config.clone());

        let result = formatter.convert(html, &metadata);
        assert!(result.is_ok());

        let direct_result = convert_to_markdown(html, &metadata, &config);
        assert!(direct_result.is_ok());

        assert_eq!(result.unwrap(), direct_result.unwrap());
    }

    #[test]
    fn test_html_to_markdown_with_tables() {
        let html = r#"
            <table>
                <thead>
                    <tr><th>Column 1</th><th>Column 2</th></tr>
                </thead>
                <tbody>
                    <tr><td>Data 1</td><td>Data 2</td></tr>
                </tbody>
            </table>
        "#;

        let metadata = Metadata::default();
        let config = MarkdownConfig::default();

        let result = convert_to_markdown(html, &metadata, &config);
        assert!(result.is_ok());
        let markdown = result.unwrap();
        assert!(markdown.contains("|"));
        assert!(markdown.contains("Column 1"));
        assert!(markdown.contains("Data 1"));
    }

    #[test]
    fn test_html_to_markdown_with_code_blocks() {
        let html = r#"<pre><code>fn main() { println!("Hello"); }</code></pre>"#;
        let metadata = Metadata::default();
        let config = MarkdownConfig::default();

        let result = convert_to_markdown(html, &metadata, &config);
        assert!(result.is_ok());
        let markdown = result.unwrap();
        assert!(markdown.contains("```"));
    }

    #[test]
    fn test_html_to_markdown_with_blockquotes() {
        let html = r#"<blockquote>This is a quote</blockquote>"#;
        let metadata = Metadata::default();
        let config = MarkdownConfig::default();

        let result = convert_to_markdown(html, &metadata, &config);
        assert!(result.is_ok());
        let markdown = result.unwrap();
        assert!(markdown.contains(">"));
    }

    #[test]
    fn test_toml_escape_with_quotes() {
        let escaped = toml_escape_string("My \"Title\" here");
        assert_eq!(escaped, r#""My \"Title\" here""#);
    }

    #[test]
    fn test_toml_escape_with_newlines() {
        let escaped = toml_escape_string("Line 1\nLine 2");
        assert_eq!(escaped, r#""Line 1\nLine 2""#);
    }

    #[test]
    fn test_extract_links_deduplication() {
        let html = r#"
            <p>
                <a href="https://example.com">First</a>
                <a href="https://example.com">Second</a>
            </p>
        "#;

        let links = extract_links(html).unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "First");
    }
}
