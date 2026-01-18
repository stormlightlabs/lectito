//! Output formatters for converting article content to different formats.
//!
//! This module provides formatters for converting extracted HTML content
//! to various output formats including Markdown, JSON, plain text, and TOML.
//!
//! # Example
//!
//! ```rust
//! use lectito_core::formatters::{convert_to_markdown, convert_to_text};
//! use lectito_core::Metadata;
//!
//! let html = "<h1>Title</h1><p>Content here</p>";
//! let metadata = Metadata::default();
//!
//! // Convert to Markdown
//! let markdown = convert_to_markdown(html, &metadata, &Default::default()).unwrap();
//!
//! // Convert to plain text
//! let text = convert_to_text(html, &metadata, &Default::default()).unwrap();
//! ```

pub mod json;
pub mod markdown;
pub mod text;
pub mod toml;

pub use json::{JsonConfig, JsonFormatter, convert_to_json, metadata_to_json};
pub use markdown::{MarkdownConfig, MarkdownFormatter, convert_to_markdown};
pub use text::{TextConfig, TextFormatter, convert_to_text};
pub use toml::{TomlFormatter, metadata_to_toml};
