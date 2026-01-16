pub mod dom_tree;
pub mod error;
pub mod extract;
pub mod fetch;
pub mod formatters;
pub mod metadata;
pub mod parse;
pub mod postprocess;
pub mod preprocess;
pub mod scoring;

pub use dom_tree::{DomNode, DomTree, build_dom_tree};
pub use error::{LectitoError, Result};
pub use extract::{ExtractConfig, ExtractedContent, extract_content};
pub use fetch::{FetchConfig, fetch_file, fetch_stdin, fetch_url};
pub use formatters::{MarkdownConfig, MarkdownFormatter, convert_to_markdown};
pub use metadata::Metadata;
pub use parse::Document;
pub use postprocess::{PostProcessConfig, postprocess_html};
pub use preprocess::{PreprocessConfig, preprocess_html};
pub use scoring::{
    ScoreConfig, ScoreResult, base_tag_score, calculate_score, class_id_weight, content_density_score, link_density,
};
