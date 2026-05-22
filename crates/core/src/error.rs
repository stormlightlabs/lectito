#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to parse HTML")]
    HtmlParse,
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(String),
    #[error("document has {actual} elements, exceeding max_elems_to_parse={limit}")]
    MaxElemsExceeded { actual: usize, limit: usize },
    #[error("invalid site profile {name}: {message}")]
    InvalidSiteProfile { name: String, message: String },
    #[error("failed to serialize article HTML")]
    Serialization,
}

pub type Result<T> = std::result::Result<T, Error>;
