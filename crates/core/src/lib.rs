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
