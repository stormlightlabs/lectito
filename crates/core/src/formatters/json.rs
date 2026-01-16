use crate::formatters::markdown::LinkReference;
use crate::metadata::Metadata;
use crate::{LectitoError, Result};
use serde::Serialize;
use std::collections::HashMap;

/// Complete JSON output structure
#[derive(Debug, Clone, Serialize)]
pub struct JsonOutput {
    /// Extracted metadata
    pub metadata: Metadata,
    /// Content in multiple formats
    pub content: ContentFormats,
    /// Optional references array
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<Vec<JsonReference>>,
}

/// Content in multiple formats
#[derive(Debug, Clone, Serialize)]
pub struct ContentFormats {
    /// Content as Markdown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    /// Content as plain text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Content as HTML
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
}

/// A reference link for JSON output
#[derive(Debug, Clone, Serialize)]
pub struct JsonReference {
    /// Reference index
    pub index: usize,
    /// Link text
    pub text: String,
    /// Link URL
    pub url: String,
}

/// Configuration for JSON output
#[derive(Debug, Clone, Default)]
pub struct JsonConfig {
    /// Include Markdown in output
    pub include_markdown: bool,
    /// Include plain text in output
    pub include_text: bool,
    /// Include HTML in output
    pub include_html: bool,
    /// Include references array
    pub include_references: bool,
    /// Pretty print JSON output
    pub pretty: bool,
}

impl From<LinkReference> for JsonReference {
    fn from(link: LinkReference) -> Self {
        Self { text: link.text, url: link.url, index: 0 }
    }
}

/// Convert HTML to plain text by stripping tags
fn html_to_text(html: &str) -> String {
    let doc = scraper::Html::parse_document(html);
    doc.root_element().text().collect::<String>()
}

/// Extract links from HTML content
fn extract_links(html: &str) -> Result<Vec<JsonReference>> {
    let links: Vec<LinkReference> = crate::formatters::markdown::extract_links(html)?;
    let mut seen_urls = HashMap::new();

    let json_links: Vec<JsonReference> = links
        .into_iter()
        .filter_map(|link| {
            if seen_urls.contains_key(&link.url) {
                None
            } else {
                seen_urls.insert(link.url.clone(), seen_urls.len());
                Some(JsonReference::from(link))
            }
        })
        .collect();

    Ok(json_links)
}

/// Assign indices to references
fn assign_indices(mut references: Vec<JsonReference>) -> Vec<JsonReference> {
    for (index, ref mut link) in references.iter_mut().enumerate() {
        link.index = index + 1;
    }
    references
}

/// Convert content to JSON format
pub fn convert_to_json(
    html: &str, metadata: &Metadata, config: &JsonConfig, markdown_content: Option<&str>,
) -> Result<String> {
    let content = ContentFormats {
        markdown: if config.include_markdown { markdown_content.map(|s| s.to_string()) } else { None },
        text: if config.include_text { Some(html_to_text(html)) } else { None },
        html: if config.include_html { Some(html.to_string()) } else { None },
    };

    let references = if config.include_references { Some(assign_indices(extract_links(html)?)) } else { None };

    let output = JsonOutput { metadata: metadata.clone(), content, references };

    if config.pretty {
        Ok(serde_json::to_string_pretty(&output).map_err(|e| LectitoError::HtmlParseError(e.to_string()))?)
    } else {
        Ok(serde_json::to_string(&output).map_err(|e| LectitoError::HtmlParseError(e.to_string()))?)
    }
}

/// Convert metadata to JSON (for --metadata-only flag)
pub fn metadata_to_json(metadata: &Metadata, pretty: bool) -> Result<String> {
    if pretty {
        Ok(serde_json::to_string_pretty(metadata).map_err(|e| LectitoError::HtmlParseError(e.to_string()))?)
    } else {
        Ok(serde_json::to_string(metadata).map_err(|e| LectitoError::HtmlParseError(e.to_string()))?)
    }
}

/// JSON formatter with configurable options
pub struct JsonFormatter {
    config: JsonConfig,
}

impl JsonFormatter {
    pub fn new(config: JsonConfig) -> Self {
        Self { config }
    }

    pub fn convert(&self, html: &str, metadata: &Metadata, markdown_content: Option<&str>) -> Result<String> {
        convert_to_json(html, metadata, &self.config, markdown_content)
    }

    pub fn metadata_only(&self, metadata: &Metadata) -> Result<String> {
        metadata_to_json(metadata, self.config.pretty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_to_text_basic() {
        let html = r#"<h1>Title</h1><p>This is a paragraph.</p>"#;
        let text = html_to_text(html);
        assert!(text.contains("Title"));
        assert!(text.contains("This is a paragraph."));
    }

    #[test]
    fn test_html_to_text_strips_tags() {
        let html = r#"<p>Text with <strong>bold</strong> and <em>italic</em>.</p>"#;
        let text = html_to_text(html);
        assert!(!text.contains("<strong>"));
        assert!(!text.contains("<em>"));
        assert!(text.contains("bold"));
        assert!(text.contains("italic"));
    }

    #[test]
    fn test_extract_links_to_json_references() {
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
    }

    #[test]
    fn test_assign_indices() {
        let links = vec![
            JsonReference { text: "First".to_string(), url: "https://first.com".to_string(), index: 0 },
            JsonReference { text: "Second".to_string(), url: "https://second.com".to_string(), index: 0 },
        ];

        let indexed = assign_indices(links);
        assert_eq!(indexed[0].index, 1);
        assert_eq!(indexed[1].index, 2);
    }

    #[test]
    fn test_convert_to_json_with_all_formats() {
        let html = r#"<h1>Title</h1><p>Content here.</p>"#;
        let metadata = Metadata {
            title: Some("Test Title".to_string()),
            author: Some("Test Author".to_string()),
            ..Default::default()
        };

        let config = JsonConfig {
            include_markdown: true,
            include_text: true,
            include_html: true,
            include_references: false,
            pretty: true,
        };

        let markdown = "# Title\n\nContent here.";
        let result = convert_to_json(html, &metadata, &config, Some(markdown));

        assert!(result.is_ok());
        let json_str = result.unwrap();

        assert!(json_str.contains("metadata"));
        assert!(json_str.contains("title"));
        assert!(json_str.contains("Test Title"));
        assert!(json_str.contains("author"));
        assert!(json_str.contains("Test Author"));
        assert!(json_str.contains("content"));
        assert!(json_str.contains("markdown"));
        assert!(json_str.contains("html"));
        assert!(json_str.contains("text"));
        assert!(json_str.contains("Title"));
    }

    #[test]
    fn test_convert_to_json_with_references() {
        let html = r#"<p>Visit <a href="https://example.com">Example</a> for more.</p>"#;
        let metadata = Metadata::default();

        let config = JsonConfig {
            include_markdown: false,
            include_text: false,
            include_html: false,
            include_references: true,
            pretty: false,
        };

        let result = convert_to_json(html, &metadata, &config, None);
        assert!(result.is_ok());

        let json_str = result.unwrap();
        assert!(json_str.contains(r#""references":"#));
        assert!(json_str.contains("Example"));
        assert!(json_str.contains("https://example.com"));
    }

    #[test]
    fn test_metadata_to_json() {
        let metadata = Metadata {
            title: Some("Test Title".to_string()),
            author: Some("Test Author".to_string()),
            language: Some("en".to_string()),
            ..Default::default()
        };

        let json = metadata_to_json(&metadata, true).unwrap();
        assert!(json.contains("title"));
        assert!(json.contains("Test Title"));
        assert!(json.contains("author"));
        assert!(json.contains("Test Author"));
        assert!(json.contains("language"));
        assert!(json.contains("en"));
        assert!(json.starts_with("{"));
        assert!(json.ends_with("}"));
    }

    #[test]
    fn test_metadata_to_json_compact() {
        let metadata = Metadata { title: Some("Title".to_string()), ..Default::default() };

        let json = metadata_to_json(&metadata, false).unwrap();
        assert!(json.contains(r#""title":"Title""#));
    }

    #[test]
    fn test_json_formatter() {
        let html = r#"<p>Test content</p>"#;
        let metadata = Metadata::default();
        let config = JsonConfig::default();

        let formatter = JsonFormatter::new(config.clone());
        let result = formatter.convert(html, &metadata, None);

        assert!(result.is_ok());

        let direct_result = convert_to_json(html, &metadata, &config, None);
        assert!(direct_result.is_ok());

        assert_eq!(result.unwrap(), direct_result.unwrap());
    }

    #[test]
    fn test_json_formatter_metadata_only() {
        let metadata = Metadata { title: Some("Test".to_string()), ..Default::default() };
        let config = JsonConfig { pretty: true, ..Default::default() };

        let formatter = JsonFormatter::new(config);
        let result = formatter.metadata_only(&metadata);

        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("title"));
        assert!(json.contains("Test"));
    }

    #[test]
    fn test_json_reference_from_link_reference() {
        let link_ref = LinkReference { text: "Link".to_string(), url: "https://example.com".to_string() };
        let json_ref: JsonReference = link_ref.into();

        assert_eq!(json_ref.text, "Link");
        assert_eq!(json_ref.url, "https://example.com");
        assert_eq!(json_ref.index, 0);
    }

    #[test]
    fn test_content_formats_serialization() {
        let content = ContentFormats {
            markdown: Some("# Title".to_string()),
            text: Some("Title".to_string()),
            html: Some("<h1>Title</h1>".to_string()),
        };

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains(r#""markdown":"#));
        assert!(json.contains(r#"Title"#));
        assert!(json.contains(r#""text":"Title""#));
        assert!(json.contains(r#""html":"<h1>Title</h1>"#));
    }

    #[test]
    fn test_content_formats_skip_none() {
        let content = ContentFormats { markdown: Some("# Title".to_string()), text: None, html: None };

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("markdown"));
        assert!(!json.contains("text"));
        assert!(!json.contains("html"));
    }

    #[test]
    fn test_json_output_complete() {
        let metadata =
            Metadata { title: Some("Title".to_string()), language: Some("en".to_string()), ..Default::default() };

        let content = ContentFormats {
            markdown: Some("# Title".to_string()),
            text: Some("Title".to_string()),
            html: Some("<h1>Title</h1>".to_string()),
        };

        let references =
            vec![JsonReference { index: 1, text: "Link".to_string(), url: "https://example.com".to_string() }];

        let output = JsonOutput { metadata, content, references: Some(references) };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains(r#""metadata":"#));
        assert!(json.contains(r#""content":"#));
        assert!(json.contains(r#""references":"#));
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
