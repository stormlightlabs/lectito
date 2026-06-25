/// Error returned by Lectito extraction and conversion functions.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// HTML parsing failed before extraction could start.
    #[error("failed to parse HTML")]
    HtmlParse,
    /// The supplied base URL could not be parsed.
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(String),
    /// The input exceeded `ReadabilityOptions::max_elems_to_parse`.
    #[error("document has {actual} elements, exceeding max_elems_to_parse={limit}")]
    MaxElemsExceeded {
        /// Element count found in the document.
        actual: usize,
        /// Configured maximum element count.
        limit: usize,
    },
    /// A site profile could not be parsed or converted into selectors.
    #[error("invalid site profile {name}: {message}")]
    InvalidSiteProfile {
        /// Profile name or source label.
        name: String,
        /// Parse or validation message.
        message: String,
    },
    /// Article HTML serialization failed after extraction.
    #[error("failed to serialize article HTML")]
    Serialization,
}

impl Error {
    pub fn invalid_site_profile(name: impl ToString, message: impl ToString) -> Self {
        Self::InvalidSiteProfile { name: name.to_string(), message: message.to_string() }
    }

    pub fn max_elems_exceeded(actual: usize, limit: usize) -> Self {
        Self::MaxElemsExceeded { actual, limit }
    }
}

/// Result type used by Lectito APIs.
pub type Result<T> = std::result::Result<T, Error>;
