//! Main content extraction API.
//!
//! This module provides the primary API for extracting readable content from HTML pages.
//! The main entry point is the [`Readability`] struct, along with convenience functions
//! like [`parse`] and [`fetch_and_parse`].

use crate::article::Article;
use crate::extract::{
    ExtractConfig, extract_content_with_config, extract_largest_hidden_subtree, extract_schema_org_article,
};
use crate::fetch::{FetchConfig, fetch_url};
use crate::parse::Document;
use crate::preprocess::PreprocessConfig;
use crate::scoring::{ScoreConfig, calculate_score};
use crate::siteconfig::{ConfigLoader, SiteConfig, SiteConfigXPath};
use crate::siteextractors::{ExtractorOutcome, ExtractorRegistry};
use crate::{LectitoError, Result};
use std::collections::HashMap;
use url::Url;

/// Minimum word count that satisfies content extraction — below this threshold
/// the retry strategy kicks in with progressively relaxed settings.
const RETRY_WORD_THRESHOLD: usize = 200;

/// Configuration for the Readability builder.
///
/// Provides fine-grained control over the content extraction process.
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
    pub fn builder() -> ReadabilityConfigBuilder {
        ReadabilityConfigBuilder::new()
    }
}

/// Builder for ReadabilityConfig.
///
/// Provides a fluent API for configuring Readability.
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
pub struct Readability {
    config: ReadabilityConfig,
    config_loader: Option<ConfigLoader>,
}

impl Readability {
    /// Creates a new Readability instance with default settings.
    pub fn new() -> Self {
        Self { config: ReadabilityConfig::default(), config_loader: None }
    }

    /// Creates a new Readability instance with a custom configuration.
    pub fn with_config(config: ReadabilityConfig) -> Self {
        Self { config, config_loader: None }
    }

    /// Creates a new Readability instance with a custom configuration and config loader.
    pub fn with_config_and_loader(config: ReadabilityConfig, config_loader: ConfigLoader) -> Self {
        Self { config, config_loader: Some(config_loader) }
    }

    /// Parses HTML string and extracts readable content.
    ///
    /// Automatically retries with progressively relaxed settings if the initial
    /// extraction yields fewer than 200 words (see [`RETRY_WORD_THRESHOLD`]).
    pub fn parse(&self, html: &str) -> Result<Article> {
        self.parse_with_retry(html, None)
    }

    /// Parses HTML with a known base URL (for relative link resolution).
    ///
    /// Applies the same multi-pass retry strategy as [`Readability::parse`].
    pub fn parse_with_url(&self, html: &str, url: &str) -> Result<Article> {
        Url::parse(url).map_err(|e| LectitoError::InvalidUrl(e.to_string()))?;
        self.parse_with_retry(html, Some(url))
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
        let parsed_url = Url::parse(url).map_err(|e| LectitoError::InvalidUrl(e.to_string()))?;
        let prefetch_site_config = self.resolve_site_config(Some(url), None);
        let request_config = self.apply_site_config_headers(fetch_config, prefetch_site_config.as_ref());
        let html = fetch_url(url, &request_config).await?;
        let merged_site_config = self.resolve_site_config(Some(url), Some(&html));

        let doc = Document::parse_with_base_url(&html, Some(parsed_url.clone()))?;

        if let Some(article) =
            self.try_site_extractor_async(&doc, &parsed_url, &request_config, merged_site_config.as_ref()).await?
        {
            return Ok(article);
        }

        self.parse_with_retry_and_site_config(&html, Some(url), merged_site_config)
    }

    /// Multi-pass retry extraction strategy.
    ///
    /// Tries up to six extraction passes with progressively relaxed settings,
    /// returning as soon as a pass yields ≥ [`RETRY_WORD_THRESHOLD`] words.
    /// If no pass reaches the threshold the best (highest word-count) result is
    /// returned; if all passes fail with errors a [`LectitoError::NotReadable`]
    /// is returned.
    ///
    /// | Pass | What changes |
    /// |------|--------------|
    /// | 0    | Default settings |
    /// | 1    | Partial-selector removal disabled (`remove_unlikely = false`) |
    /// | 2    | Hidden-element removal also disabled (`remove_hidden = false`) |
    /// | 3    | Target the largest hidden subtree directly |
    /// | 4    | Scoring threshold removed (`min_score = 0.0`) |
    /// | 5    | schema.org `articleBody` / `text` fallback |
    fn parse_with_retry(&self, html: &str, url: Option<&str>) -> Result<Article> {
        let site_config = self.resolve_site_config(url, Some(html));
        self.parse_with_retry_and_site_config(html, url, site_config)
    }

    fn parse_with_retry_and_site_config(
        &self, html: &str, url: Option<&str>, site_config: Option<SiteConfig>,
    ) -> Result<Article> {
        let base_url = url
            .map(|u| Url::parse(u).map_err(|e| LectitoError::InvalidUrl(e.to_string())))
            .transpose()?;

        let mut best: Option<Article> = None;

        let preprocess0 = PreprocessConfig {
            base_url: base_url.clone(),
            remove_unlikely: self.config.remove_unlikely,
            ..Default::default()
        };
        if let Ok(raw_doc) = Document::parse_with_base_url(html, base_url.clone())
            && let Some(url) = url
            && let Ok(parsed_url) = Url::parse(url)
            && let Some(article) = self.try_site_extractor(&raw_doc, &parsed_url, site_config.as_ref())?
        {
            return Ok(article);
        }
        match self.try_extract_pass(html, url, &preprocess0, self.config.min_score, site_config.as_ref()) {
            Ok(article) if article.word_count >= RETRY_WORD_THRESHOLD => return Ok(article),
            Ok(article) => update_best(&mut best, article),
            Err(_) => {}
        }

        let preprocess1 = PreprocessConfig { base_url: base_url.clone(), remove_unlikely: false, ..Default::default() };
        match self.try_extract_pass(html, url, &preprocess1, self.config.min_score, site_config.as_ref()) {
            Ok(article) if article.word_count >= RETRY_WORD_THRESHOLD => return Ok(article),
            Ok(article) => update_best(&mut best, article),
            Err(_) => {}
        }

        let preprocess2 = PreprocessConfig {
            base_url: base_url.clone(),
            remove_unlikely: false,
            remove_hidden: false,
            ..Default::default()
        };
        if let Ok(pass2_doc) = Document::parse_with_preprocessing_config(html, &preprocess2) {
            match self.extract_from_doc(&pass2_doc, url, self.config.min_score, site_config.as_ref()) {
                Ok(article) if article.word_count >= RETRY_WORD_THRESHOLD => return Ok(article),
                Ok(article) => update_best(&mut best, article),
                Err(_) => {}
            }

            if let Some(extracted) = extract_largest_hidden_subtree(&pass2_doc) {
                let article = Article::from_document(&pass2_doc, extracted.content, url.map(|u| u.to_string()));
                if article.word_count >= RETRY_WORD_THRESHOLD {
                    return Ok(article);
                }
                update_best(&mut best, article);
            }

            match self.extract_from_doc(&pass2_doc, url, 0.0, site_config.as_ref()) {
                Ok(article) if article.word_count >= RETRY_WORD_THRESHOLD => return Ok(article),
                Ok(article) => update_best(&mut best, article),
                Err(_) => {}
            }
        }

        let schema_preprocess = PreprocessConfig {
            base_url: base_url.clone(),
            remove_scripts: false,
            remove_unlikely: false,
            remove_hidden: false,
            ..Default::default()
        };
        if let Ok(schema_doc) = Document::parse_with_preprocessing_config(html, &schema_preprocess)
            && let Some(extracted) = extract_schema_org_article(&schema_doc)
        {
            let article = Article::from_document(&schema_doc, extracted.content, url.map(|u| u.to_string()));
            if article.word_count >= RETRY_WORD_THRESHOLD {
                return Ok(article);
            }
            update_best(&mut best, article);
        }

        best.ok_or(LectitoError::NotReadable { score: 0.0, threshold: self.config.min_score })
    }

    /// Run a single extraction pass with explicit preprocessing and scoring settings.
    fn try_extract_pass(
        &self, html: &str, url: Option<&str>, preprocess_config: &PreprocessConfig, min_score: f64,
        site_config: Option<&SiteConfig>,
    ) -> Result<Article> {
        let doc = Document::parse_with_preprocessing_config(html, preprocess_config)?;
        self.extract_from_doc(&doc, url, min_score, site_config)
    }

    /// Run content extraction on an already-parsed document with a specific score threshold.
    ///
    /// Shared by passes that differ only in `min_score` and reuse the same document,
    /// avoiding redundant HTML re-parsing.
    fn extract_from_doc(
        &self, doc: &Document, url: Option<&str>, min_score: f64, site_config: Option<&SiteConfig>,
    ) -> Result<Article> {
        let extract_config = ExtractConfig {
            min_score_threshold: min_score,
            max_top_candidates: self.config.nb_top_candidates,
            char_threshold: self.config.char_threshold,
            max_elements: self.config.max_elems_to_parse,
            sibling_threshold: 0.2,
            pre_score_selector_removal: true,
            postprocess: crate::postprocess::PostProcessConfig {
                strip_images: !self.config.preserve_images,
                keep_classes: self.config.keep_classes,
                ..Default::default()
            },
        };

        let extracted = extract_content_with_config(doc, &extract_config, site_config)?;
        if let Some(site_config) = site_config {
            let metadata_patch = build_site_config_metadata_patch(site_config, doc);
            Ok(Article::from_document_with_metadata(
                doc,
                extracted.content,
                url.map(|u| u.to_string()),
                &metadata_patch,
            ))
        } else {
            Ok(Article::from_document(
                doc,
                extracted.content,
                url.map(|u| u.to_string()),
            ))
        }
    }

    /// Checks if content appears readable without full extraction.
    ///
    /// This is a quick heuristic that checks if the page likely contains
    /// readable content based on element scores.
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

    fn resolve_site_config(&self, url: Option<&str>, html: Option<&str>) -> Option<SiteConfig> {
        let mut loader = self.config_loader.clone().or_else(|| url.map(|_| ConfigLoader::default()))?;
        match (url, html) {
            (Some(url), html) => loader.load_merged_for_url(url, html).ok(),
            (None, _) => None,
        }
    }

    fn apply_site_config_headers(&self, fetch_config: &FetchConfig, site_config: Option<&SiteConfig>) -> FetchConfig {
        let mut merged = fetch_config.clone();
        let mut headers = HashMap::new();

        if let Some(site_config) = site_config {
            for (name, value) in &site_config.http_headers {
                if name.eq_ignore_ascii_case("user-agent") {
                    merged.user_agent = value.clone();
                } else {
                    headers.insert(name.clone(), value.clone());
                }
            }
        }

        headers.extend(merged.headers.clone());
        merged.headers = headers;
        merged
    }

    fn try_site_extractor(
        &self, doc: &Document, url: &Url, site_config: Option<&SiteConfig>,
    ) -> Result<Option<Article>> {
        let registry = ExtractorRegistry::new();
        let Some(outcome) = registry.extract(doc, url)? else {
            return Ok(None);
        };

        self.article_from_extractor_outcome(doc, url, outcome, site_config).map(Some)
    }

    async fn try_site_extractor_async(
        &self, doc: &Document, url: &Url, fetch_config: &FetchConfig, site_config: Option<&SiteConfig>,
    ) -> Result<Option<Article>> {
        let registry = ExtractorRegistry::new();
        let Some(outcome) = registry.extract_async(doc, url, fetch_config).await? else {
            return Ok(None);
        };

        self.article_from_extractor_outcome(doc, url, outcome, site_config).map(Some)
    }

    fn article_from_extractor_outcome(
        &self, doc: &Document, url: &Url, outcome: ExtractorOutcome, site_config: Option<&SiteConfig>,
    ) -> Result<Article> {
        let site_config_patch = site_config
            .map(|site_config| build_site_config_metadata_patch(site_config, doc))
            .unwrap_or_default();

        match outcome {
            ExtractorOutcome::Selector { selector } => {
                let selected_html = doc
                    .select(&selector)?
                    .into_iter()
                    .map(|element| element.outer_html())
                    .collect::<Vec<_>>()
                    .join("\n");
                if selected_html.trim().is_empty() {
                    return Err(LectitoError::NoContent);
                }
                Ok(Article::from_document_with_metadata(
                    doc,
                    selected_html,
                    Some(url.to_string()),
                    &site_config_patch,
                ))
            }
            ExtractorOutcome::Html {
                content_html,
                metadata_patch,
            } => {
                let merged_patch = site_config_patch.with_patch(&metadata_patch);
                Ok(Article::from_document_with_metadata(
                    doc,
                    content_html,
                    Some(url.to_string()),
                    &merged_patch,
                ))
            }
        }
    }
}

impl Default for Readability {
    fn default() -> Self {
        Self::new()
    }
}

/// Replace `best` with `article` if `article` has a higher word count.
fn update_best(best: &mut Option<Article>, article: Article) {
    if best.as_ref().map_or(0, |a| a.word_count) < article.word_count {
        *best = Some(article);
    }
}

fn build_site_config_metadata_patch(site_config: &SiteConfig, doc: &Document) -> crate::Metadata {
    let html = doc.as_string();
    crate::Metadata {
        title: site_config.extract_title(&html).ok().flatten(),
        author: site_config.extract_author(&html).ok().flatten(),
        date: site_config.extract_date(&html).ok().flatten(),
        ..Default::default()
    }
}

/// Convenience function for one-liner extraction with defaults.
///
/// This is the simplest way to extract content from HTML.
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
pub async fn fetch_and_parse(url: &str) -> Result<Article> {
    let reader = Readability::new();
    reader.fetch_and_parse(url).await
}

/// Convenience function: Fetch and parse with custom configurations
///
/// This async function fetches HTML from the given URL and extracts
/// readable content using the provided Readability and Fetch configurations.
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

    /// Build a string of `n` distinct words long enough to exceed the threshold.
    fn words(n: usize) -> String {
        (0..n).map(|i| format!("word{}", i)).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn test_retry_pass1_content_behind_unlikely_class() {
        let body = words(250);
        let html = format!(
            r#"<!DOCTYPE html><html><head><title>T</title></head><body>
            <div class="extra-content">{}</div>
            </body></html>"#,
            body
        );

        let reader = Readability::new();
        let result = reader.parse(&html);
        assert!(result.is_ok(), "should succeed via retry");
        let article = result.unwrap();
        assert!(
            article.word_count >= RETRY_WORD_THRESHOLD,
            "word count {} < {}",
            article.word_count,
            RETRY_WORD_THRESHOLD
        );
    }

    #[test]
    fn test_retry_pass3_largest_hidden_subtree() {
        let body = words(250);
        let html = format!(
            r##"<!DOCTYPE html><html><head><title>T</title></head><body>
            <div style="display:none"><article>{}</article></div>
            <nav><a href="nav">Link</a></nav>
            </body></html>"##,
            body
        );

        let reader = Readability::new();
        let result = reader.parse(&html);
        assert!(result.is_ok(), "should succeed via Pass 3");
        let article = result.unwrap();
        assert!(
            article.word_count >= RETRY_WORD_THRESHOLD,
            "word count {} < {}",
            article.word_count,
            RETRY_WORD_THRESHOLD
        );
    }

    #[test]
    fn test_retry_pass5_schema_org_article_body_microdata() {
        let body = words(250);
        let html = format!(
            r##"<!DOCTYPE html><html><head><title>T</title></head><body>
            <nav><a href="nav">Link 1</a></nav>
            <div itemprop="articleBody">{}</div>
            </body></html>"##,
            body
        );

        let reader = Readability::new();
        let result = reader.parse(&html);
        assert!(result.is_ok(), "should succeed via schema.org microdata fallback");
        let article = result.unwrap();
        assert!(
            article.word_count >= RETRY_WORD_THRESHOLD,
            "word count {} < {}",
            article.word_count,
            RETRY_WORD_THRESHOLD
        );
    }

    #[test]
    fn test_retry_pass5_schema_org_json_ld() {
        let body = words(250);
        let html = format!(
            r##"<!DOCTYPE html>
            <html><head><title>T</title>
            <script type="application/ld+json">
            {{"@context":"https://schema.org","@type":"Article","articleBody":"{}"}}
            </script>
            </head><body><nav><a href="nav">x</a></nav></body></html>"##,
            body
        );

        let reader = Readability::new();
        let result = reader.parse(&html);
        assert!(result.is_ok(), "should succeed via schema.org JSON-LD fallback");
        let article = result.unwrap();
        assert!(
            article.word_count >= RETRY_WORD_THRESHOLD,
            "word count {} < {}",
            article.word_count,
            RETRY_WORD_THRESHOLD
        );
    }

    #[test]
    fn test_retry_returns_best_when_below_threshold() {
        let html = r#"<!DOCTYPE html><html><head><title>T</title></head><body>
            <article><p>Short content.</p></article>
            </body></html>"#;

        let reader = Readability::new();
        let result = reader.parse(html);
        assert!(result.is_ok(), "should return best result even when below threshold");
    }
}
