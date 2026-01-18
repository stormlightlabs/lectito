//! Article output type with content, metadata, and format conversion.
//!
//! This module defines the [`Article`] struct which represents the complete
//! result of content extraction, including the extracted HTML, plain text,
//! metadata, and calculated metrics.

use crate::formatters::markdown::MarkdownConfig;
use crate::formatters::markdown::convert_to_markdown;
use crate::{Document, Metadata};
use crate::{LectitoError, Result};
use serde::Serialize;

/// Output format options for Article content.
///
/// Specifies the desired output format when converting article content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// HTML format (original extracted content).
    Html,
    /// Markdown format with TOML frontmatter.
    Markdown,
    /// Plain text format (stripped HTML tags).
    PlainText,
    /// JSON format (structured data).
    Json,
}

/// The complete result of reading an HTML document.
///
/// Article combines extracted content with metadata and provides additional
/// metrics like length, word count, and estimated reading time.
#[derive(Debug, Clone, Serialize)]
pub struct Article {
    /// Extracted readable content as clean HTML.
    pub content: String,

    /// Plain text version of content (all HTML tags stripped).
    pub text_content: String,

    /// Extracted metadata (title, author, date, etc.).
    pub metadata: Metadata,

    /// Length of content in characters.
    pub length: usize,

    /// Word count of content.
    pub word_count: usize,

    /// Estimated reading time in minutes (assuming 200 words per minute).
    pub reading_time: f64,

    /// Source URL if known.
    pub source_url: Option<String>,
}

impl Article {
    /// Creates a new Article from its components.
    ///
    /// This constructor automatically calculates derived metrics including
    /// plain text content, character length, word count, and reading time.
    pub fn new(content: String, metadata: Metadata, source_url: Option<String>) -> Self {
        let text_content = html_to_text(&content);
        let length = content.chars().count();
        let word_count = count_words(&text_content);
        let reading_time = word_count as f64 / 200.0;

        Self { content, text_content, metadata, length, word_count, reading_time, source_url }
    }

    /// Creates an Article from a Document and extracted content HTML.
    ///
    /// This is a convenience method that extracts metadata from the document
    /// and creates an Article with the provided content HTML.
    pub fn from_document(doc: &Document, content_html: String, source_url: Option<String>) -> Self {
        let metadata = doc.extract_metadata();
        Self::new(content_html, metadata, source_url)
    }

    /// Converts content to the specified format.
    pub fn to_format(&self, format: OutputFormat) -> Result<String> {
        match format {
            OutputFormat::Html => Ok(self.content.clone()),
            OutputFormat::Markdown => self.to_markdown(),
            OutputFormat::PlainText => Ok(self.text_content.clone()),
            OutputFormat::Json => self.to_json().map(|v| v.to_string()),
        }
    }

    /// Gets content as Markdown with TOML frontmatter.
    pub fn to_markdown(&self) -> Result<String> {
        let config = MarkdownConfig::default();
        convert_to_markdown(&self.content, &self.metadata, &config)
    }

    /// Gets content as Markdown with custom configuration.
    pub fn to_markdown_with_config(&self, config: &MarkdownConfig) -> Result<String> {
        convert_to_markdown(&self.content, &self.metadata, config)
    }

    /// Gets content as structured JSON.
    ///
    /// Returns a `serde_json::Value` representing the complete article
    /// including content, metadata, and metrics.
    pub fn to_json(&self) -> Result<serde_json::Value> {
        serde_json::to_value(self).map_err(|e| LectitoError::HtmlParseError(e.to_string()))
    }

    /// Gets content as plain text.
    ///
    /// This is an alias for the `text_content` field.
    pub fn to_text(&self) -> String {
        self.text_content.clone()
    }
}

/// Convert HTML to plain text by removing tags
fn html_to_text(html: &str) -> String {
    let doc = Document::parse(html).unwrap_or_else(|_| Document::parse("<html></html>").unwrap());
    doc.text_content()
}

/// Count words in text using a simple regex pattern
fn count_words(text: &str) -> usize {
    use regex::Regex;
    let word_regex = Regex::new(r"\b[\w'-]+\b").unwrap();
    word_regex.find_iter(text).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_article_creation() {
        let content = "<p>This is a test article with some content.</p>".to_string();
        let metadata = Metadata {
            title: Some("Test Article".to_string()),
            author: None,
            date: None,
            excerpt: None,
            site_name: None,
            word_count: None,
            reading_time_minutes: None,
            language: None,
        };

        let article = Article::new(content.clone(), metadata, Some("https://example.com".to_string()));

        assert_eq!(article.content, content);
        assert_eq!(article.text_content, "This is a test article with some content.");
        assert_eq!(article.metadata.title, Some("Test Article".to_string()));
        assert_eq!(article.source_url, Some("https://example.com".to_string()));
        assert_eq!(article.word_count, 8);
        assert!(article.reading_time > 0.0);
    }

    #[test]
    fn test_article_from_document() {
        let html = r#"
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <title>Test Page</title>
                <meta name="author" content="Test Author">
            </head>
            <body>
                <article>
                    <p>This is article content with multiple words.</p>
                </article>
            </body>
            </html>
        "#;

        let doc = Document::parse(html).unwrap();
        let content_html = "<p>This is article content with multiple words.</p>".to_string();

        let article = Article::from_document(&doc, content_html, Some("https://example.com/article".to_string()));

        assert_eq!(article.metadata.title, Some("Test Page".to_string()));
        assert_eq!(article.metadata.author, Some("Test Author".to_string()));
        assert_eq!(article.metadata.language, Some("en".to_string()));
        assert_eq!(article.source_url, Some("https://example.com/article".to_string()));
        assert!(article.word_count > 0);
    }

    #[test]
    fn test_html_to_text() {
        let html = "<p>Hello world</p><p>Second paragraph</p>";
        let text = html_to_text(html);
        assert_eq!(text, "Hello worldSecond paragraph");
    }

    #[test]
    fn test_count_words() {
        assert_eq!(count_words("hello world"), 2);
        assert_eq!(count_words("one"), 1);
        assert_eq!(count_words(""), 0);
        assert_eq!(count_words("a b c d e"), 5);
    }

    #[test]
    fn test_article_reading_time_calculation() {
        let content = "word ".repeat(200);
        let html = format!("<p>{}</p>", content);

        let metadata = Metadata::default();
        let article = Article::new(html, metadata, None);
        assert!((article.reading_time - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_article_serialization() {
        let content = "<p>Test content</p>".to_string();
        let metadata = Metadata {
            title: Some("Test".to_string()),
            author: Some("Author".to_string()),
            date: Some("2024-01-01".to_string()),
            excerpt: Some("Excerpt".to_string()),
            site_name: Some("Site".to_string()),
            word_count: Some(2),
            reading_time_minutes: Some(0.01),
            language: Some("en".to_string()),
        };

        let article = Article::new(content, metadata, Some("https://example.com".to_string()));

        let json = serde_json::to_string(&article).unwrap();
        assert!(json.contains(r#""content":"<p>Test content</p>""#));
        assert!(json.contains(r#""title":"Test""#));
        assert!(json.contains(r#""author":"Author""#));
        assert!(json.contains(r#""source_url":"https://example.com""#));
    }

    #[test]
    fn test_to_format_html() {
        let content = "<p>Test content</p>".to_string();
        let metadata = Metadata::default();
        let article = Article::new(content, metadata, None);

        let result = article.to_format(OutputFormat::Html);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "<p>Test content</p>");
    }

    #[test]
    fn test_to_format_markdown() {
        let content = "<h1>Test</h1><p>Content</p>".to_string();
        let metadata = Metadata { title: Some("Test".to_string()), ..Default::default() };
        let article = Article::new(content, metadata, None);

        let result = article.to_format(OutputFormat::Markdown);
        assert!(result.is_ok());
        let markdown = result.unwrap();
        assert!(markdown.contains("# Test") || markdown.contains("Test"));
    }

    #[test]
    fn test_to_format_plain_text() {
        let content = "<p>Test content</p>".to_string();
        let metadata = Metadata::default();
        let article = Article::new(content.clone(), metadata, None);

        let result = article.to_format(OutputFormat::PlainText);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Test content");
    }

    #[test]
    fn test_to_format_json() {
        let content = "<p>Test</p>".to_string();
        let metadata = Metadata { title: Some("Test".to_string()), ..Default::default() };
        let article = Article::new(content, metadata, None);

        let result = article.to_format(OutputFormat::Json);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains(r#"{"#));
        assert!(json.contains("content"));
    }

    #[test]
    fn test_to_markdown_default() {
        let content = "<h1>Title</h1><p>Content</p>".to_string();
        let metadata = Metadata { title: Some("Title".to_string()), ..Default::default() };
        let article = Article::new(content, metadata, None);

        let result = article.to_markdown();
        assert!(result.is_ok());
        let markdown = result.unwrap();
        assert!(markdown.contains("Title"));
    }

    #[test]
    fn test_to_markdown_with_config() {
        let content = "<h1>Title</h1><p>Content</p>".to_string();
        let metadata =
            Metadata { title: Some("Title".to_string()), author: Some("Author".to_string()), ..Default::default() };
        let article = Article::new(content, metadata, None);

        let config = MarkdownConfig { include_frontmatter: true, ..Default::default() };
        let result = article.to_markdown_with_config(&config);
        assert!(result.is_ok());
        let markdown = result.unwrap();
        assert!(markdown.contains("+++"));
    }

    #[test]
    fn test_to_json() {
        let content = "<p>Test</p>".to_string();
        let metadata = Metadata {
            title: Some("Test Title".to_string()),
            author: Some("Test Author".to_string()),
            ..Default::default()
        };
        let article = Article::new(content, metadata, Some("https://example.com".to_string()));

        let result = article.to_json();
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.is_object());
        assert!(json.get("content").is_some());
        assert!(json.get("metadata").is_some());
    }

    #[test]
    fn test_to_text() {
        let content = "<p>Test content</p>".to_string();
        let metadata = Metadata::default();
        let article = Article::new(content, metadata, None);

        let text = article.to_text();
        assert_eq!(text, "Test content");
    }
}
