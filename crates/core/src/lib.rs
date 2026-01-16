pub mod error;
pub mod fetch;
pub mod metadata;
pub mod parse;
pub mod preprocess;
pub mod scoring;

pub use error::{LectitoError, Result};
pub use fetch::{FetchConfig, fetch_file, fetch_stdin, fetch_url};
pub use metadata::Metadata;
pub use parse::Document;
pub use preprocess::{PreprocessConfig, preprocess_html};
pub use scoring::{ScoreConfig, ScoreResult, calculate_score, base_tag_score, class_id_weight, content_density_score, link_density};
