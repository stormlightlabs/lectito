pub mod error;
pub mod fetch;
pub mod metadata;
pub mod parse;
pub mod preprocess;

pub use error::{LectitoError, Result};
pub use fetch::{FetchConfig, fetch_file, fetch_stdin, fetch_url};
pub use metadata::Metadata;
pub use parse::Document;
pub use preprocess::{PreprocessConfig, preprocess_html};
