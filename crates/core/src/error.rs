use std::path::PathBuf;
use sxd_xpath::ExecutionError;
use thiserror::Error;

/// Main error type for readability extraction operations
#[derive(Error, Debug)]
pub enum LectitoError {
    /// HTTP request errors
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    /// Request timeout
    #[error("Request timed out after {timeout} seconds")]
    Timeout { timeout: u64 },

    /// Invalid URL provided
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// HTML parsing errors
    #[error("Failed to parse HTML: {0}")]
    HtmlParseError(String),

    /// Invalid character encoding
    #[error("Invalid character encoding")]
    InvalidEncoding,

    /// Content is not readable (score below threshold)
    #[error("Content is not readable (score {score} below threshold {threshold})")]
    NotReaderable { score: f64, threshold: f64 },

    /// No content could be extracted
    #[error("No content could be extracted from the document")]
    NoContent,

    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    /// File write errors
    #[error("Failed to write to file: {0}")]
    WriteError(#[from] std::io::Error),

    /// Site configuration errors
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Site configuration errors (FTR format)
    #[error("Site configuration error: {0}")]
    SiteConfigError(String),

    /// XPath evaluation errors
    #[error("XPath error: {0}")]
    XPathError(String),
}

impl From<ExecutionError> for LectitoError {
    fn from(err: ExecutionError) -> Self {
        LectitoError::XPathError(err.to_string())
    }
}

/// Result type alias for LectitoError
pub type Result<T> = std::result::Result<T, LectitoError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = LectitoError::InvalidUrl("not a url".to_string());
        assert!(err.to_string().contains("Invalid URL"));
    }

    #[test]
    fn test_not_readerable_error() {
        let err = LectitoError::NotReaderable { score: 15.0, threshold: 20.0 };
        assert!(err.to_string().contains("15"));
        assert!(err.to_string().contains("20"));
    }

    #[test]
    fn test_timeout_error() {
        let err = LectitoError::Timeout { timeout: 30 };
        assert!(err.to_string().contains("30"));
    }
}
