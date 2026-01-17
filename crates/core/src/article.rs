use crate::metadata::Metadata;
use crate::parse::Document;
use serde::Serialize;

/// The complete result of reading an HTML document
///
/// Combines extracted content with metadata and provides additional
/// metrics like length, word count, and reading time.
#[derive(Debug, Clone, Serialize)]
pub struct Article {
    /// Extracted readable content as clean HTML
    pub content: String,

    /// Plain text version of content (all HTML tags stripped)
    pub text_content: String,

    /// Extracted metadata (title, author, date, etc.)
    pub metadata: Metadata,

    /// Length of content in characters
    pub length: usize,

    /// Word count of content
    pub word_count: usize,

    /// Estimated reading time in minutes (assuming 200 words per minute)
    pub reading_time: f64,

    /// Source URL if known
    pub source_url: Option<String>,
}

impl Article {
    /// Create a new Article from its components
    pub fn new(content: String, metadata: Metadata, source_url: Option<String>) -> Self {
        let text_content = html_to_text(&content);
        let length = content.chars().count();
        let word_count = count_words(&text_content);
        let reading_time = word_count as f64 / 200.0;

        Self { content, text_content, metadata, length, word_count, reading_time, source_url }
    }

    /// Create an Article from a Document and extracted content HTML
    pub fn from_document(doc: &Document, content_html: String, source_url: Option<String>) -> Self {
        let metadata = doc.extract_metadata();
        Self::new(content_html, metadata, source_url)
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
}
