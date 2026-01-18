//! # Lectito
//!
//! A library and CLI for extracting readable content from web pages.
//!
//! Lectito implements a content extraction algorithm inspired by Mozilla's
//! [Readability.js], designed to isolate the main article content from
//! navigation, sidebars, advertisements, and other clutter.
//!
//! ## Quick Start
//!
//! ```
//! use lectito_core::{parse, Article};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let html = r#"
//! <!DOCTYPE html>
//! <html>
//!     <head><title>My Article</title></head>
//!     <body>
//!         <article>
//!             <h1>Article Title</h1>
//!             <p>This is article content. Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>
//!         </article>
//!     </body>
//! </html>
//! "#;
//! let config = lectito_core::ReadabilityConfig::builder().min_score(10.0).build();
//! let reader = lectito_core::Readability::with_config(config);
//! let article = reader.parse(html)?;
//! assert_eq!(article.metadata.title, Some("My Article".to_string()));
//! # Ok(())
//! # }
//! ```
//!
//! ### Convert to different output formats
//!
//! ```
//! use lectito_core::{parse, article::OutputFormat};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let html = "<html><head><title>Example</title></head><body><article><h1>Title</h1><p>Content here. Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p></article></body></html>";
//! let config = lectito_core::ReadabilityConfig::builder().min_score(10.0).build();
//! let reader = lectito_core::Readability::with_config(config);
//! let article = reader.parse(html)?;
//!
//! // Get as Markdown with frontmatter
//! let markdown = article.to_markdown().unwrap();
//!
//! // Get as plain text
//! let text = article.to_text();
//!
//! // Get as structured JSON
//! let json = article.to_json()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Configuration
//!
//! Use the builder pattern for advanced configuration:
//!
//! ```
//! use lectito_core::{Readability, ReadabilityConfig, FetchConfig};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ReadabilityConfig::builder()
//!     .min_score(10.0)
//!     .char_threshold(500)
//!     .preserve_images(true)
//!     .build();
//!
//! let reader = Readability::with_config(config);
//! let html = "<html><body><article><h1>Title</h1><p>Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p></article></body></html>";
//! let article = reader.parse(html)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Modules
//!
//! - [`article`] - [`Article`] result type and [`OutputFormat`] options
//! - [`error`] - [`LectitoError`] error type and [`Result`] alias
//! - [`fetch`] - HTTP and file fetching with [`FetchConfig`]
//! - [`formatters`] - Output formatters (Markdown, JSON, text)
//! - [`metadata`] - [`Metadata`] extraction
//! - [`mod@parse`] - [`Document`] and [`parse::Element`] types for DOM manipulation
//! - [`readability`] - Main API: [`Readability`], [`parse()`], [`fetch_and_parse()`]
//!
//! ## Feature Flags
//!
//! - `fetch` (default): Enable URL fetching with reqwest
//! - `markdown` (default): Enable Markdown output
//! - `siteconfig` (default): Enable site configuration support
//! - `json`: Always enabled for Article serialization
//!
//! ## Error Handling
//!
//! Most functions return [`Result<T, LectitoError>`]. Handle errors appropriately:
//!
//! ```rust
//! use lectito_core::{parse, LectitoError};
//!
//! match parse("<html>...</html>") {
//!     Ok(article) => println!("Got article: {}", article.metadata.title.unwrap()),
//!     Err(LectitoError::NotReadable { score, threshold }) => {
//!         eprintln!("Content not readable: score {} < threshold {}", score, threshold);
//!     }
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! ```
//!
//! [Readability.js]: https://github.com/mozilla/readability

pub mod article;
pub mod dom_tree;
pub mod error;
pub mod extract;
pub mod fetch;
pub mod formatters;
pub mod metadata;
pub mod parse;
pub mod postprocess;
pub mod preprocess;
pub mod readability;
pub mod scoring;
#[cfg(feature = "siteconfig")]
pub mod siteconfig;

pub use article::{Article, OutputFormat};
#[doc(hidden)]
pub use dom_tree::{DomNode, DomTree, build_dom_tree};
pub use error::{LectitoError, Result};
#[doc(hidden)]
pub use extract::{ExtractConfig, ExtractedContent};
pub use extract::{extract_content, extract_content_with_config};
pub use fetch::FetchConfig;
pub use fetch::{fetch_file, fetch_stdin, fetch_url};
pub use formatters::{JsonConfig, JsonFormatter, MarkdownConfig, MarkdownFormatter, TextConfig, TextFormatter};
pub use formatters::{convert_to_json, convert_to_markdown, convert_to_text, metadata_to_json};
pub use metadata::Metadata;
pub use parse::Document;
#[doc(hidden)]
pub use postprocess::PostProcessConfig;
pub use postprocess::postprocess_html;
#[doc(hidden)]
pub use preprocess::PreprocessConfig;
pub use preprocess::preprocess_html;
pub use readability::{
    LectitoConfig, LectitoConfigBuilder, Readability, ReadabilityConfig, fetch_and_parse, fetch_and_parse_with_config,
    is_probably_readable, parse, parse_with_url,
};
#[doc(hidden)]
pub use scoring::{
    ScoreConfig, ScoreResult, base_tag_score, calculate_score, class_id_weight, content_density_score, link_density,
};
#[cfg(feature = "siteconfig")]
pub use siteconfig::{ConfigLoader, ConfigLoaderBuilder, ConfigParser, Directive, FingerprintMatcher, SiteConfig};
