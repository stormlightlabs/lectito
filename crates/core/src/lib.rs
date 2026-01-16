pub mod error;
pub mod fetch;
pub mod parse;

pub use error::{LectitoError, Result};
pub use fetch::{FetchConfig, fetch_file, fetch_stdin, fetch_url};
pub use parse::Document;
