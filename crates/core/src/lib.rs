//! Extract readable article content from HTML.
//!
//! The crate does not fetch pages. Pass HTML from a crawler, browser, cache, or
//! fixture, plus an optional base URL for resolving relative links.
//!
//! Use [`extract`] for normal application code. Use
//! [`extract_with_diagnostics`] when debugging root selection, cleanup, or site
//! profiles.
//!
//! ```no_run
//! use lectito::{extract, ReadabilityOptions};
//!
//! # fn main() -> Result<(), lectito::Error> {
//! # let html = "<article><p>Article text.</p></article>";
//! let options = ReadabilityOptions::default();
//! let article = extract(html, Some("https://example.com/post"), &options)?;
//! if let Some(article) = article {
//!     println!("{}", article.text_content);
//! }
//! # Ok::<(), lectito::Error>(())
//! # }
//! ```

mod cleanup;
mod config;
mod diagnostics;
mod dom;
mod error;
mod extract;
mod json_schema;
mod markdown;
mod metadata;
mod normalize;
mod patterns;
mod readable;
mod recovery;
mod regexes;
mod rules;
mod scoring;
mod serialize;
mod shared;

pub use config::{Article, MarkdownOptions, MediaRetention, ReadabilityOptions, ReadableOptions};
pub use diagnostics::{
    AttemptDiagnostic, CandidateDiagnostic, CandidateSelection, CleanupDiagnostic, ContentSelectorDiagnostic,
    ExtractionDiagnostics, ExtractionOutcome, ExtractionReport, FlagDiagnostic, NodeDiagnostic, RecoveryDiagnostic,
};
pub use error::{Error, Result};
pub use extract::{clean_article_html, extract, extract_with_diagnostics};
pub use markdown::{html_to_markdown, markdown_to_html, markdown_with_toml_frontmatter};
pub use readable::is_probably_readable;
pub use shared::escape_html;
