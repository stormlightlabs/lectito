# API Overview

Complete reference for the Lectito Rust library API.

## Core Types

### Article

The main result type containing extracted content and metadata.

```rs
pub struct Article {
    /// Extracted metadata
    pub metadata: Metadata,

    /// Cleaned HTML content
    pub content: String,

    /// Plain text content
    pub text_content: String,

    /// Number of words in content
    pub word_count: usize,

    /// Final readability score
    pub readability_score: f64,
}
```

**Methods:**

- `to_markdown() -> Result<String>` - Convert to Markdown with frontmatter
- `to_json() -> Result<String>` - Convert to JSON
- `to_text() -> String` - Get plain text

### Metadata

Extracted article metadata.

```rs
pub struct Metadata {
    /// Article title
    pub title: Option<String>,

    /// Author name
    pub author: Option<String>,

    /// Publication date
    pub published_date: Option<String>,

    /// Article excerpt/description
    pub excerpt: Option<String>,

    /// Content language
    pub language: Option<String>,
}
```

### LectitoError

Error type for all Lectito operations.

```rs
pub enum LectitoError {
    /// Content not readable: score below threshold
    NotReaderable { score: f64, threshold: f64 },

    /// Invalid URL provided
    InvalidUrl(String),

    /// HTTP request timeout
    Timeout { timeout: u64 },

    /// HTTP error
    HttpError(reqwest::Error),

    /// HTML parsing error
    HtmlParseError(String),

    /// IO error
    IoError(std::io::Error),
}
```

### Result

Type alias for Result with LectitoError.

```rs
pub type Result<T> = std::result::Result<T, LectitoError>;
```

## Configuration Types

### ReadabilityConfig

Main configuration for content extraction.

```rs
pub struct ReadabilityConfig {
    /// Minimum readability score (default: 20.0)
    pub min_score: f64,

    /// Minimum character count (default: 500)
    pub char_threshold: usize,

    /// Preserve images in output (default: true)
    pub preserve_images: bool,

    /// Minimum content length (default: 140)
    pub min_content_length: usize,

    /// Minimum score threshold (default: 20.0)
    pub min_score_threshold: f64,
}
```

**Methods:**

- `builder() -> ReadabilityConfigBuilder` - Create a builder
- `default() -> Self` - Default configuration

### ReadabilityConfigBuilder

Builder for `ReadabilityConfig`.

```rs
pub struct ReadabilityConfigBuilder {
    // ...
}
```

**Methods:**

- `min_score(f64) -> Self` - Set minimum score
- `char_threshold(usize) -> Self` - Set character threshold
- `preserve_images(bool) -> Self` - Set image preservation
- `min_content_length(usize) -> Self` - Set minimum content length
- `min_score_threshold(f64) -> Self` - Set score threshold
- `build() -> ReadabilityConfig` - Build configuration

### FetchConfig

Configuration for HTTP fetching.

```rs
pub struct FetchConfig {
    /// Request timeout in seconds (default: 30)
    pub timeout: u64,

    /// User-Agent header (default: "Lectito/...")
    pub user_agent: String,
}
```

**Trait:**

- `impl Default for FetchConfig`

## Main API Functions

### parse

Parse HTML string and extract article.

```rs
pub fn parse(html: &str) -> Result<Article>
```

**Example:**

```rs
use lectito_core::parse;

let article = parse("<html>...</html>")?;
```

### parse_with_url

Parse HTML with URL context for relative link resolution.

```rs
pub fn parse_with_url(html: &str, url: &str) -> Result<Article>
```

**Example:**

```rs
use lectito_core::parse_with_url;

let article = parse_with_url(html, "https://example.com/article")?;
```

### fetch_and_parse

Fetch URL and extract article.

```rs
pub async fn fetch_and_parse(url: &str) -> Result<Article>
```

**Feature:** `fetch`

**Example:**

```rs
use lectito_core::fetch_and_parse;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let article = fetch_and_parse("https://example.com/article").await?;
    Ok(())
}
```

### fetch_and_parse_with_config

Fetch URL and extract with custom configuration.

```rs
pub async fn fetch_and_parse_with_config(
    url: &str,
    fetch_config: &FetchConfig,
    readability_config: &ReadabilityConfig
) -> Result<Article>
```

**Feature:** `fetch`

**Example:**

```rs
use lectito_core::{fetch_and_parse_with_config, FetchConfig, ReadabilityConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fetch_config = FetchConfig {
        timeout: 60,
        ..Default::default()
    };

    let read_config = ReadabilityConfig::builder()
        .min_score(25.0)
        .build();

    let article = fetch_and_parse_with_config(
        "https://example.com/article",
        &fetch_config,
        &read_config
    ).await?;

    Ok(())
}
```

### is_probably_readable

Check if content likely meets readability thresholds.

```rs
pub fn is_probably_readable(html: &str) -> bool
```

**Example:**

```rs
use lectito_core::is_probably_readable;

if is_probably_readable(html) {
    println!("Content is readable");
}
```

## Readability Type

Main API for configured extraction.

```rs
pub struct Readability {
    config: ReadabilityConfig,
}
```

**Methods:**

- `new() -> Self` - Create with default config
- `with_config(ReadabilityConfig) -> Self` - Create with custom config
- `parse(&str) -> Result<Article>` - Parse HTML

**Example:**

```rs
use lectito_core::{Readability, ReadabilityConfig};

let config = ReadabilityConfig::builder()
    .min_score(25.0)
    .build();

let reader = Readability::with_config(config);
let article = reader.parse(html)?;
```

## Fetch Functions

### fetch_url

Fetch HTML from URL.

```rs
pub async fn fetch_url(url: &str, config: &FetchConfig) -> Result<String>
```

**Feature:** `fetch`

### fetch_file

Read HTML from file.

```rs
pub fn fetch_file(path: &str) -> Result<String>
```

### fetch_stdin

Read HTML from stdin.

```rs
pub fn fetch_stdin() -> Result<String>
```

## DOM Types

### Document

HTML document wrapper for parsing and selection.

```rs
pub struct Document {
    // ...
}
```

**Methods:**

- `parse(&str) -> Result<Self>` - Parse HTML
- `select(&str) -> Result<Vec<Element>>` - CSS selector

### Element

DOM element wrapper.

```rs
pub struct Element<'a> {
    // ...
}
```

**Methods:**

- `text() -> String` - Extract text content
- `html() -> String` - Get inner HTML

## Module Organization

```sh
.
├── article          # Article and Metadata types
├── error            # LectitoError and Result
├── fetch            # HTTP and file fetching
├── formatters       # Output formatters
├── metadata         # Metadata extraction
├── parse            # Document and Element types
├── readability      # Main API (parse, fetch_and_parse)
└── scoring          # Scoring algorithm
```

## Feature Flags

| Feature      | Default | Enables                    |
| ------------ | ------- | -------------------------- |
| `fetch`      | Yes     | URL fetching with reqwest  |
| `markdown`   | Yes     | Markdown output            |
| `siteconfig` | Yes     | Site configuration support |

## Re-exports

The crate re-exports commonly used types at the root:

```rs
// Core types
pub use article::{Article, OutputFormat};
pub use error::{LectitoError, Result};

// Configuration
pub use fetch::FetchConfig;
pub use readability::{
    Readability, ReadabilityConfig, ReadabilityConfigBuilder,
    LectitoConfig, LectitoConfigBuilder
};

// Functions
pub use readability::{
    parse, parse_with_url, fetch_and_parse, fetch_and_parse_with_config,
    is_probably_readable
};

// Fetching
pub use fetch::{fetch_url, fetch_file, fetch_stdin};

// Formatters
pub use formatters::{
    MarkdownFormatter, TextFormatter, JsonFormatter,
    convert_to_markdown, convert_to_text, convert_to_json
};
```

## Complete Example

```rs
use lectito_core::{
    Readability, ReadabilityConfig, FetchConfig,
    fetch_and_parse_with_config
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure
    let fetch_config = FetchConfig {
        timeout: 60,
        user_agent: "MyBot/1.0".to_string(),
    };

    let read_config = ReadabilityConfig::builder()
        .min_score(25.0)
        .char_threshold(1000)
        .build();

    // Fetch and parse
    let article = fetch_and_parse_with_config(
        "https://example.com/article",
        &fetch_config,
        &read_config
    ).await?;

    // Access results
    println!("Title: {:?}", article.metadata.title);
    println!("Word count: {}", article.word_count);

    // Convert to format
    let markdown = article.to_markdown()?;
    println!("{}", markdown);

    Ok(())
}
```

## Further Documentation

- ~~[docs.rs/lectito](https://docs.rs/lectito)~~ - Full API documentation with rustdoc
- [GitHub Repository](https://github.com/stormlightlabs/lectito) - Source code and examples
- [Basic Usage](../library/basic-usage.md) - Usage examples
- [Configuration](../library/configuration.md) - Configuration options

## Next Steps

- [Getting Started](../getting-started/) - Installation and quick start
- [Library Guide](../library/) - In-depth usage documentation
