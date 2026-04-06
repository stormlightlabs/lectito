# API Overview

Reference for the Lectito Rust library API.

## Core Types

### Article

The main result type containing extracted content, metadata, and derived metrics.

```rs
pub struct Article {
    pub content: String,
    pub text_content: String,
    pub metadata: Metadata,
    pub length: usize,
    pub word_count: usize,
    pub reading_time: f64,
    pub source_url: Option<String>,
    pub confidence: f64,
    pub diagnostics: Option<ExtractionDiagnostics>,
}
```

Common methods:

- `to_markdown() -> Result<String>`
- `to_markdown_with_config(&MarkdownConfig) -> Result<String>`
- `to_json() -> Result<serde_json::Value>`
- `to_text() -> String`
- `to_format(OutputFormat) -> Result<String>`

### Metadata

Extracted article metadata.

```rs
pub struct Metadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub excerpt: Option<String>,
    pub site_name: Option<String>,
    pub image: Option<String>,
    pub favicon: Option<String>,
    pub word_count: Option<usize>,
    pub reading_time_minutes: Option<f64>,
    pub language: Option<String>,
}
```

### LectitoError

Main error type for extraction, parsing, and fetch failures.

Notable variants:

- `NotReadable { score, threshold }`
- `InvalidUrl(String)`
- `Timeout { timeout }`
- `HtmlParseError(String)`
- `NoContent`
- `FileNotFound(PathBuf)`
- `ConfigError(String)`
- `SiteConfigError(String)`

`HttpError(reqwest::Error)` is available when the `fetch` feature is enabled.

## Configuration Types

### ReadabilityConfig

Main configuration for content extraction.

```rs
pub struct ReadabilityConfig {
    pub min_score: f64,
    pub char_threshold: usize,
    pub nb_top_candidates: usize,
    pub max_elems_to_parse: usize,
    pub remove_unlikely: bool,
    pub keep_classes: bool,
    pub preserve_images: bool,
    pub preserve_video_embeds: bool,
}
```

Build with `ReadabilityConfig::builder()`.

### FetchConfig

Configuration for HTTP fetching.

```rs
pub struct FetchConfig {
    pub timeout: u64,
    pub user_agent: String,
    pub headers: HashMap<String, String>,
}
```

## Main API Functions

### `parse`

Parse an HTML string and extract an `Article`.

```rs
pub fn parse(html: &str) -> Result<Article>
```

### `parse_with_url`

Parse HTML with URL context for relative link resolution.

```rs
pub fn parse_with_url(html: &str, url: &str) -> Result<Article>
```

### `is_probably_readable`

Cheap pre-check for likely article pages.

```rs
pub fn is_probably_readable(html: &str) -> bool
```

### `fetch_url`

Fetch raw HTML from a URL.

```rs
pub async fn fetch_url(url: &str, config: &FetchConfig) -> Result<String>
```

Requires the `fetch` feature.

### `fetch_and_parse`

Fetch a URL and extract an article with default configuration.

```rs
pub async fn fetch_and_parse(url: &str) -> Result<Article>
```

Requires the `fetch` feature.

### `fetch_and_parse_with_config`

Fetch a URL and extract an article with custom readability and fetch settings.

```rs
pub async fn fetch_and_parse_with_config(
    url: &str,
    readability_config: &ReadabilityConfig,
    fetch_config: &FetchConfig,
) -> Result<Article>
```

Requires the `fetch` feature.

## Readability Type

`Readability` is the main stateful API:

```rs
pub struct Readability { /* ... */ }
```

Common constructors and methods:

- `Readability::new()`
- `Readability::with_config(ReadabilityConfig)`
- `Readability::with_config_and_loader(ReadabilityConfig, ConfigLoader)`
- `parse(&self, html: &str) -> Result<Article>`
- `parse_with_url(&self, html: &str, url: &str) -> Result<Article>`
- `is_probably_readable(&self, html: &str) -> bool`
- `fetch_and_parse(&self, url: &str) -> Result<Article>`
- `fetch_and_parse_with_config(&self, url: &str, fetch_config: &FetchConfig) -> Result<Article>`

## Lower-Level Types

For callers that need more control, Lectito also exposes:

- `Document` and `Element` for DOM access
- `ConfigLoader` and `ConfigLoaderBuilder` for site configuration loading
- `MarkdownConfig`, `JsonConfig`, and formatter types for output control

## Feature Flags

| Feature      | Default | Purpose                           |
| ------------ | ------- | --------------------------------- |
| `fetch`      | Yes     | Async URL fetching with `reqwest` |
| `markdown`   | Yes     | Markdown conversion support       |
| `siteconfig` | Yes     | Site configuration support        |
