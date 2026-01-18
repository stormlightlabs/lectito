//! Main content extraction API.
//!
//! This module provides the primary API for extracting readable content
//! from HTML pages. The main entry point is the [`Readability`] struct,
//! along with convenience functions like [`parse`] and [`fetch_and_parse`].
//!
//! # Example
//!
//! ```rust
//! use lectito_core::readability::{parse, fetch_and_parse};
//!
//! // Parse HTML from a string
//! let html = reqwest::get("https://example.com/article")?.text()?;
//! let article = parse(&html)?;
//! println!("Title: {:?}", article.metadata.title);
//!
//! // Or fetch and parse in one step
//! # #[tokio::main]
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let article = fetch_and_parse("https://example.com/article").await?;
//! # Ok(())
//! # }
//! ```

use crate::article::Article;
use crate::extract::{ExtractConfig, extract_content_with_config};
use crate::fetch::{FetchConfig, fetch_url};
use crate::parse::Document;
use crate::scoring::{ScoreConfig, calculate_score};
use crate::siteconfig::ConfigLoader;
use crate::{LectitoError, Result};
use url::Url;

/// Configuration for the Readability builder.
///
/// Provides fine-grained control over the content extraction process.
///
/// # Example
///
/// ```rust
/// use lectito_core::ReadabilityConfig;
///
/// let config = ReadabilityConfig::builder()
///     .min_score(25.0)
///     .char_threshold(500)
///     .preserve_images(true)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct ReadabilityConfig {
    /// Minimum score threshold for extraction (default: 20.0).
    pub min_score: f64,

    /// Minimum character count for valid content (default: 500).
    pub char_threshold: usize,

    /// Number of top candidates to track (default: 5).
    pub nb_top_candidates: usize,

    /// Maximum elements to parse (0 = unlimited, default: 0).
    pub max_elems_to_parse: usize,

    /// Whether to remove unlikely candidates (default: true).
    pub remove_unlikely: bool,

    /// Whether to preserve class attributes in output HTML (default: false).
    pub keep_classes: bool,

    /// Whether to preserve images in output HTML (default: true).
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
    /// Creates a new builder for ReadabilityConfig.
    ///
    /// # Example
    ///
    /// ```rust
    /// use lectito_core::ReadabilityConfig;
    ///
    /// let builder = ReadabilityConfig::builder();
    /// let config = builder.min_score(30.0).build();
    /// ```
    pub fn builder() -> ReadabilityConfigBuilder {
        ReadabilityConfigBuilder::new()
    }
}

/// Builder for ReadabilityConfig.
///
/// Provides a fluent API for configuring Readability.
///
/// # Example
///
/// ```rust
/// use lectito_core::ReadabilityConfig;
///
/// let config = ReadabilityConfig::builder()
///     .min_score(30.0)
///     .char_threshold(1000)
///     .keep_classes(true)
///     .build();
/// ```
pub struct ReadabilityConfigBuilder {
    config: ReadabilityConfig,
}

impl ReadabilityConfigBuilder {
    /// Creates a new builder with default values.
    pub fn new() -> Self {
        Self { config: ReadabilityConfig::default() }
    }

    /// Sets the minimum score threshold.
    pub fn min_score(mut self, value: f64) -> Self {
        self.config.min_score = value;
        self
    }

    /// Sets the character threshold.
    pub fn char_threshold(mut self, value: usize) -> Self {
        self.config.char_threshold = value;
        self
    }

    /// Sets the number of top candidates.
    pub fn nb_top_candidates(mut self, value: usize) -> Self {
        self.config.nb_top_candidates = value;
        self
    }

    /// Sets the maximum elements to parse.
    pub fn max_elems_to_parse(mut self, value: usize) -> Self {
        self.config.max_elems_to_parse = value;
        self
    }

    /// Sets whether to remove unlikely candidates.
    pub fn remove_unlikely(mut self, value: bool) -> Self {
        self.config.remove_unlikely = value;
        self
    }

    /// Sets whether to preserve class attributes in output HTML.
    pub fn keep_classes(mut self, value: bool) -> Self {
        self.config.keep_classes = value;
        self
    }

    /// Sets whether to preserve images in output HTML.
    pub fn preserve_images(mut self, value: bool) -> Self {
        self.config.preserve_images = value;
        self
    }

    /// Builds the config.
    pub fn build(self) -> ReadabilityConfig {
        self.config
    }
}

impl Default for ReadabilityConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for ReadabilityConfig.
///
/// Provides an alternative name for the configuration struct.
pub type LectitoConfig = ReadabilityConfig;

/// Type alias for ReadabilityConfigBuilder.
///
/// Provides an alternative name for the configuration builder.
pub type LectitoConfigBuilder = ReadabilityConfigBuilder;

/// Main entry point for content extraction.
///
/// Readability provides a fluent API for extracting readable content from HTML.
/// Use its methods to parse HTML strings or fetch and parse from URLs.
///
/// # Example
///
/// ```rust
/// use lectito_core::Readability;
///
/// let reader = Readability::new();
/// let html = "<html><body><article><p>Content here</p></article></body></html>";
/// let article = reader.parse(html).unwrap();
/// println!("Extracted: {}", article.text_content);
/// ```
pub struct Readability {
    config: ReadabilityConfig,
    config_loader: Option<ConfigLoader>,
}

impl Readability {
    /// Creates a new Readability instance with default settings.
    ///
    /// # Example
    ///
    /// ```rust
    /// use lectito_core::Readability;
    ///
    /// let reader = Readability::new();
    /// ```
    pub fn new() -> Self {
        Self { config: ReadabilityConfig::default(), config_loader: None }
    }

    /// Creates a new Readability instance with a custom configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration options for extraction
    ///
    /// # Example
    ///
    /// ```rust
    /// use lectito_core::{Readability, ReadabilityConfig};
    ///
    /// let config = ReadabilityConfig::builder()
    ///     .min_score(30.0)
    ///     .build();
    /// let reader = Readability::with_config(config);
    /// ```
    pub fn with_config(config: ReadabilityConfig) -> Self {
        Self { config, config_loader: None }
    }

    /// Creates a new Readability instance with a custom configuration and config loader.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration options for extraction
    /// * `config_loader` - Site configuration loader for XPath-based extraction
    pub fn with_config_and_loader(config: ReadabilityConfig, config_loader: ConfigLoader) -> Self {
        Self { config, config_loader: Some(config_loader) }
    }

    /// Parses HTML string and extracts readable content.
    ///
    /// # Arguments
    ///
    /// * `html` - The HTML content to parse
    ///
    /// # Errors
    ///
    /// Returns various errors if parsing or extraction fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use lectito_core::Readability;
    ///
    /// let reader = Readability::new();
    /// let html = "<html><body><article><p>Content</p></article></body></html>";
    /// let article = reader.parse(html).unwrap();
    /// ```
    pub fn parse(&self, html: &str) -> Result<Article> {
        let doc = Document::parse_with_preprocessing(html, None)?;
        self.extract_from_document(&doc, None)
    }

    /// Parses HTML with a known base URL (for relative link resolution).
    ///
    /// # Arguments
    ///
    /// * `html` - The HTML content to parse
    /// * `url` - The base URL for resolving relative links
    ///
    /// # Errors
    ///
    /// Returns [`LectitoError::InvalidUrl`] if the URL is invalid.
    ///
    /// # Example
    ///
    /// ```rust
    /// use lectito_core::Readability;
    ///
    /// let reader = Readability::new();
    /// let html = "<html><body><article><p>Content</p></article></body></html>";
    /// let article = reader.parse_with_url(html, "https://example.com").unwrap();
    /// assert_eq!(article.source_url, Some("https://example.com".to_string()));
    /// ```
    pub fn parse_with_url(&self, html: &str, url: &str) -> Result<Article> {
        let base_url = Url::parse(url).map_err(|e| LectitoError::InvalidUrl(e.to_string()))?;
        let doc = Document::parse_with_preprocessing(html, Some(base_url))?;
        self.extract_from_document(&doc, Some(url))
    }

    /// Fetch HTML from URL and extract readable content using default fetch config
    ///
    /// This async method fetches HTML from the given URL and extracts
    /// readable content using default Fetch configuration.
    pub async fn fetch_and_parse(&self, url: &str) -> Result<Article> {
        let fetch_config = FetchConfig::default();
        self.fetch_and_parse_with_config(url, &fetch_config).await
    }

    /// Fetch HTML from URL and extract readable content with custom fetch config
    ///
    /// This async method fetches HTML from the given URL and extracts
    /// readable content using the provided Fetch configuration.
    pub async fn fetch_and_parse_with_config(&self, url: &str, fetch_config: &FetchConfig) -> Result<Article> {
        let html = fetch_url(url, fetch_config).await?;
        self.parse_with_url(&html, url)
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

    /// Checks if content appears readable without full extraction.
    ///
    /// This is a quick heuristic that checks if the page likely contains
    /// readable content based on element scores.
    ///
    /// # Arguments
    ///
    /// * `html` - The HTML content to check
    ///
    /// Returns `true` if content appears readable, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use lectito_core::Readability;
    ///
    /// let reader = Readability::new();
    /// let html_article = "<html><body><article><p>Long content here...</p></article></body></html>";
    /// let html_nav = "<html><body><nav><a href=\"#\">Link</a></nav></body></html>";
    ///
    /// assert!(reader.is_probably_readable(html_article));
    /// assert!(!reader.is_probably_readable(html_nav));
    /// ```
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

/// Convenience function for one-liner extraction with defaults.
///
/// This is the simplest way to extract content from HTML.
///
/// # Arguments
///
/// * `html` - The HTML content to parse
///
/// # Errors
///
/// Returns various errors if parsing or extraction fails.
///
/// # Example
///
/// ```rust
/// use lectito_core::readability::parse;
///
/// let html = "<html><body><article><p>Content here</p></article></body></html>";
/// let article = parse(html).unwrap();
/// ```
pub fn parse(html: &str) -> Result<Article> {
    Readability::new().parse(html)
}

/// Convenience function for one-liner with URL context.
///
/// # Arguments
///
/// * `html` - The HTML content to parse
/// * `url` - The base URL for resolving relative links
///
/// # Errors
///
/// Returns [`LectitoError::InvalidUrl`] if the URL is invalid.
pub fn parse_with_url(html: &str, url: &str) -> Result<Article> {
    Readability::new().parse_with_url(html, url)
}

/// Convenience function for quick readability check.
///
/// Returns `true` if content appears readable, `false` otherwise.
pub fn is_probably_readable(html: &str) -> bool {
    Readability::new().is_probably_readable(html)
}

/// Convenience function: Fetch and parse from URL with defaults
///
/// This async function fetches HTML from the given URL and extracts
/// readable content using default Readability and Fetch configurations.
///
/// # Example
///
/// ```no_run
/// use lectito_core::fetch_and_parse;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let article = fetch_and_parse("https://example.com/article").await?;
///     println!("Title: {:?}", article.metadata.title);
///     Ok(())
/// }
/// ```
pub async fn fetch_and_parse(url: &str) -> Result<Article> {
    let reader = Readability::new();
    reader.fetch_and_parse(url).await
}

/// Convenience function: Fetch and parse with custom configurations
///
/// This async function fetches HTML from the given URL and extracts
/// readable content using the provided Readability and Fetch configurations.
///
/// # Example
///
/// ```no_run
/// use lectito_core::{fetch_and_parse_with_config, ReadabilityConfig, FetchConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let readability_config = ReadabilityConfig::builder()
///         .min_score(30.0)
///         .build();
///     let fetch_config = FetchConfig {
///         timeout: 60,
///         ..Default::default()
///     };
///
///     let article = fetch_and_parse_with_config(
///         "https://example.com/article",
///         &readability_config,
///         &fetch_config
///     ).await?;
///
///     println!("Title: {:?}", article.metadata.title);
///     Ok(())
/// }
/// ```
pub async fn fetch_and_parse_with_config(
    url: &str, readability_config: &ReadabilityConfig, fetch_config: &FetchConfig,
) -> Result<Article> {
    let reader = Readability::with_config(readability_config.clone());
    reader.fetch_and_parse_with_config(url, fetch_config).await
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

    #[test]
    fn test_readability_fetch_and_parse_invalid_url() {
        let reader = Readability::new();
        let result = std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async { reader.fetch_and_parse("not-a-url").await })
        })
        .join()
        .unwrap();

        assert!(matches!(result, Err(LectitoError::InvalidUrl(_))));
    }

    #[test]
    fn test_convenience_fetch_and_parse_invalid_url() {
        let result = std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async { fetch_and_parse("not-a-url").await })
        })
        .join()
        .unwrap();

        assert!(matches!(result, Err(LectitoError::InvalidUrl(_))));
    }

    #[test]
    fn test_readability_fetch_and_parse_with_config_custom_timeout() {
        let reader = Readability::new();
        let fetch_config = FetchConfig { timeout: 1, ..Default::default() };

        // TODO: integration test with mock server
        let result = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                reader
                    .fetch_and_parse_with_config("https://httpbin.org/delay/5", &fetch_config)
                    .await
            })
        })
        .join()
        .unwrap();

        assert!(matches!(result, Err(LectitoError::Timeout { .. })));
    }

    #[test]
    fn test_convenience_fetch_and_parse_with_config() {
        let readability_config = ReadabilityConfig::builder().min_score(50.0).build();
        let fetch_config = FetchConfig { timeout: 1, ..Default::default() };

        let result = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                fetch_and_parse_with_config("https://httpbin.org/delay/5", &readability_config, &fetch_config).await
            })
        })
        .join()
        .unwrap();

        assert!(matches!(result, Err(LectitoError::Timeout { .. })));
    }
}
