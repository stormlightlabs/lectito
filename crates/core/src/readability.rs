use crate::article::Article;
use crate::extract::{ExtractConfig, extract_content_with_config};
use crate::parse::Document;
use crate::scoring::{ScoreConfig, calculate_score};
use crate::siteconfig::ConfigLoader;
use crate::{LectitoError, Result};
use url::Url;

/// Configuration for the Readability builder
///
/// Provides fine-grained control over the content extraction process.
#[derive(Debug, Clone)]
pub struct ReadabilityConfig {
    /// Minimum score threshold for extraction (default: 20.0)
    pub min_score: f64,

    /// Minimum character count for valid content (default: 500)
    pub char_threshold: usize,

    /// Number of top candidates to track (default: 5)
    pub nb_top_candidates: usize,

    /// Maximum elements to parse (0 = unlimited, default: 0)
    pub max_elems_to_parse: usize,

    /// Whether to remove unlikely candidates (default: true)
    pub remove_unlikely: bool,

    /// Whether to preserve class attributes in output HTML (default: false)
    pub keep_classes: bool,

    /// Whether to preserve images in output HTML (default: true)
    pub preserve_images: bool,
}

impl Default for ReadabilityConfig {
    fn default() -> Self {
        Self {
            min_score: 20.0,
            char_threshold: 500,
            nb_top_candidates: 5,
            max_elems_to_parse: 0,
            remove_unlikely: true,
            keep_classes: false,
            preserve_images: true,
        }
    }
}

impl ReadabilityConfig {
    /// Create a new builder for ReadabilityConfig
    pub fn builder() -> ReadabilityConfigBuilder {
        ReadabilityConfigBuilder::new()
    }
}

/// Builder for ReadabilityConfig
///
/// Provides a fluent API for configuring Readability.
pub struct ReadabilityConfigBuilder {
    config: ReadabilityConfig,
}

impl ReadabilityConfigBuilder {
    /// Create a new builder with default values
    pub fn new() -> Self {
        Self { config: ReadabilityConfig::default() }
    }

    /// Set the minimum score threshold
    pub fn min_score(mut self, value: f64) -> Self {
        self.config.min_score = value;
        self
    }

    /// Set the character threshold
    pub fn char_threshold(mut self, value: usize) -> Self {
        self.config.char_threshold = value;
        self
    }

    /// Set the number of top candidates
    pub fn nb_top_candidates(mut self, value: usize) -> Self {
        self.config.nb_top_candidates = value;
        self
    }

    /// Set the maximum elements to parse
    pub fn max_elems_to_parse(mut self, value: usize) -> Self {
        self.config.max_elems_to_parse = value;
        self
    }

    /// Set whether to remove unlikely candidates
    pub fn remove_unlikely(mut self, value: bool) -> Self {
        self.config.remove_unlikely = value;
        self
    }

    /// Set whether to preserve class attributes in output HTML
    pub fn keep_classes(mut self, value: bool) -> Self {
        self.config.keep_classes = value;
        self
    }

    /// Set whether to preserve images in output HTML
    pub fn preserve_images(mut self, value: bool) -> Self {
        self.config.preserve_images = value;
        self
    }

    /// Build the config
    pub fn build(self) -> ReadabilityConfig {
        self.config
    }
}

impl Default for ReadabilityConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for ReadabilityConfig
///
/// Provides an alternative name for the configuration struct.
pub type LectitoConfig = ReadabilityConfig;

/// Type alias for ReadabilityConfigBuilder
///
/// Provides an alternative name for the configuration builder.
pub type LectitoConfigBuilder = ReadabilityConfigBuilder;

/// Main entry point for content extraction
///
/// Provides a fluent API for extracting readable content from HTML.
pub struct Readability {
    config: ReadabilityConfig,
    config_loader: Option<ConfigLoader>,
}

impl Readability {
    /// Create a new Readability instance with default settings
    pub fn new() -> Self {
        Self { config: ReadabilityConfig::default(), config_loader: None }
    }

    /// Create a new Readability instance with a custom configuration
    pub fn with_config(config: ReadabilityConfig) -> Self {
        Self { config, config_loader: None }
    }

    /// Create a new Readability instance with a custom configuration and config loader
    pub fn with_config_and_loader(config: ReadabilityConfig, config_loader: ConfigLoader) -> Self {
        Self { config, config_loader: Some(config_loader) }
    }

    /// Parse HTML string and extract readable content
    pub fn parse(&self, html: &str) -> Result<Article> {
        let doc = Document::parse_with_preprocessing(html, None)?;
        self.extract_from_document(&doc, None)
    }

    /// Parse HTML with a known base URL (for relative link resolution)
    pub fn parse_with_url(&self, html: &str, url: &str) -> Result<Article> {
        let base_url = Url::parse(url).map_err(|e| LectitoError::InvalidUrl(e.to_string()))?;
        let doc = Document::parse_with_preprocessing(html, Some(base_url))?;
        self.extract_from_document(&doc, Some(url))
    }

    /// Extract article from a parsed document
    fn extract_from_document(&self, doc: &Document, url: Option<&str>) -> Result<Article> {
        let site_config = if let Some(mut loader) = self.config_loader.clone() {
            let html = doc.as_string();
            loader.load_for_html(&html).ok()
        } else {
            None
        };

        let extract_config = ExtractConfig {
            min_score_threshold: self.config.min_score,
            max_top_candidates: self.config.nb_top_candidates,
            char_threshold: self.config.char_threshold,
            max_elements: if self.config.max_elems_to_parse == 0 { 1000 } else { self.config.max_elems_to_parse },
            sibling_threshold: 0.2,
            postprocess: crate::postprocess::PostProcessConfig {
                strip_images: !self.config.preserve_images,
                keep_classes: self.config.keep_classes,
                ..Default::default()
            },
        };

        let extracted = extract_content_with_config(doc, &extract_config, site_config.as_ref())?;

        Ok(Article::from_document(
            doc,
            extracted.content,
            url.map(|u| u.to_string()),
        ))
    }

    /// Check if content appears readable without full extraction
    pub fn is_probably_readable(&self, html: &str) -> bool {
        self.is_probably_readable_with_threshold(html, 20.0)
    }

    /// Check if content appears readable with a custom threshold
    fn is_probably_readable_with_threshold(&self, html: &str, threshold: f64) -> bool {
        let doc = match Document::parse(html) {
            Ok(d) => d,
            Err(_) => return false,
        };

        let score_config = ScoreConfig::default();

        let mut max_score = 0.0;

        for tag in &["p", "div", "article", "section"] {
            if let Ok(elements) = doc.select(tag) {
                for element in elements {
                    let text = element.text();
                    if text.chars().count() < 25 {
                        continue;
                    }

                    let score_result = calculate_score(&element, &score_config);
                    if score_result.final_score > max_score {
                        max_score = score_result.final_score;
                    }
                }
            }
        }

        max_score >= threshold
    }
}

impl Default for Readability {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function: One-liner extraction with defaults
pub fn parse(html: &str) -> Result<Article> {
    Readability::new().parse(html)
}

/// Convenience function: One-liner with URL context
pub fn parse_with_url(html: &str, url: &str) -> Result<Article> {
    Readability::new().parse_with_url(html, url)
}

/// Convenience function: Quick readability check
pub fn is_probably_readable(html: &str) -> bool {
    Readability::new().is_probably_readable(html)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARTICLE_HTML: &str = r##"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <title>Test Article</title>
            <meta name="author" content="Test Author">
        </head>
        <body>
            <article class="main-content">
                <h1>Article Title</h1>
                <p>This is a long paragraph with lots of content to ensure it meets the character threshold.</p>
                <p>This is another long paragraph with plenty of content, text, commas, and meaningful sentences for scoring.</p>
                <p>A third paragraph with substantial content that should help boost the overall readability score significantly.</p>
            </article>
        </body>
        </html>
    "##;

    #[test]
    fn test_readability_config_default() {
        let config = ReadabilityConfig::default();
        assert_eq!(config.min_score, 20.0);
        assert_eq!(config.char_threshold, 500);
        assert_eq!(config.nb_top_candidates, 5);
        assert_eq!(config.max_elems_to_parse, 0);
        assert!(config.remove_unlikely);
        assert!(!config.keep_classes);
        assert!(config.preserve_images);
    }

    #[test]
    fn test_readability_config_builder() {
        let config = ReadabilityConfig::builder()
            .min_score(30.0)
            .char_threshold(1000)
            .nb_top_candidates(10)
            .max_elems_to_parse(500)
            .remove_unlikely(false)
            .keep_classes(true)
            .preserve_images(false)
            .build();

        assert_eq!(config.min_score, 30.0);
        assert_eq!(config.char_threshold, 1000);
        assert_eq!(config.nb_top_candidates, 10);
        assert_eq!(config.max_elems_to_parse, 500);
        assert!(!config.remove_unlikely);
        assert!(config.keep_classes);
        assert!(!config.preserve_images);
    }

    #[test]
    fn test_lectito_config_type_alias() {
        let config: LectitoConfig = LectitoConfig::default();
        assert_eq!(config.min_score, 20.0);
        assert!(!config.keep_classes);
        assert!(config.preserve_images);
    }

    #[test]
    fn test_lectito_config_builder() {
        let config = LectitoConfig::builder()
            .keep_classes(true)
            .preserve_images(true)
            .min_score(15.0)
            .build();

        assert!(config.keep_classes);
        assert!(config.preserve_images);
        assert_eq!(config.min_score, 15.0);
    }

    #[test]
    fn test_readability_default() {
        let reader = Readability::new();
        assert_eq!(reader.config.min_score, 20.0);
    }

    #[test]
    fn test_readability_with_config() {
        let config = ReadabilityConfig::builder().min_score(25.0).build();
        let reader = Readability::with_config(config);
        assert_eq!(reader.config.min_score, 25.0);
    }

    #[test]
    fn test_parse_article() {
        let reader = Readability::new();
        let result = reader.parse(ARTICLE_HTML);

        assert!(result.is_ok());
        let article = result.unwrap();
        assert!(!article.content.is_empty());
        assert!(article.word_count > 0);
        assert_eq!(article.metadata.title, Some("Test Article".to_string()));
        assert_eq!(article.metadata.author, Some("Test Author".to_string()));
    }

    #[test]
    fn test_parse_with_url() {
        let reader = Readability::new();
        let result = reader.parse_with_url(ARTICLE_HTML, "https://example.com/article");

        assert!(result.is_ok());
        let article = result.unwrap();
        assert_eq!(article.source_url, Some("https://example.com/article".to_string()));
    }

    #[test]
    fn test_is_probably_readable_true() {
        let reader = Readability::new();
        assert!(reader.is_probably_readable(ARTICLE_HTML));
    }

    #[test]
    fn test_is_probably_readable_false() {
        let html = r##"
            <html>
            <body>
                <nav>
                    <a href="#">Link 1</a>
                    <a href="#">Link 2</a>
                </nav>
            </body>
            </html>
        "##;

        let reader = Readability::new();
        assert!(!reader.is_probably_readable(html));
    }

    #[test]
    fn test_convenience_parse() {
        let result = parse(ARTICLE_HTML);
        assert!(result.is_ok());
        let article = result.unwrap();
        assert!(!article.content.is_empty());
    }

    #[test]
    fn test_convenience_parse_with_url() {
        let result = parse_with_url(ARTICLE_HTML, "https://example.com/test");
        assert!(result.is_ok());
        let article = result.unwrap();
        assert_eq!(article.source_url, Some("https://example.com/test".to_string()));
    }

    #[test]
    fn test_convenience_is_probably_readable() {
        assert!(is_probably_readable(ARTICLE_HTML));
    }
}
