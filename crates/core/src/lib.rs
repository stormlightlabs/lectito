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
pub mod siteconfig;

pub use article::Article;
pub use dom_tree::{DomNode, DomTree, build_dom_tree};
pub use error::{LectitoError, Result};
pub use extract::{ExtractConfig, ExtractedContent, extract_content, extract_content_with_config};
pub use fetch::{FetchConfig, fetch_file, fetch_stdin, fetch_url};
pub use formatters::{MarkdownConfig, MarkdownFormatter, convert_to_markdown};
pub use metadata::Metadata;
pub use parse::Document;
pub use postprocess::{PostProcessConfig, postprocess_html};
pub use preprocess::{PreprocessConfig, preprocess_html};
pub use readability::{Readability, ReadabilityConfig, is_probably_readable, parse, parse_with_url};
pub use scoring::{
    ScoreConfig, ScoreResult, base_tag_score, calculate_score, class_id_weight, content_density_score, link_density,
};
pub use siteconfig::{ConfigLoader, ConfigLoaderBuilder, ConfigParser, Directive, FingerprintMatcher, SiteConfig};
