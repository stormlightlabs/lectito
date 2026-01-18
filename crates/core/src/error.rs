//! Error types for Lectito operations.
//!
//! This module defines the main error type [`LectitoError`] which represents
//! all possible errors that can occur during content extraction, fetching,
//! and parsing operations.
//!
//! # Example
//!
//! ```rust
//! use lectito_core::{LectitoError, Result};
//!
//! fn extract_article(html: &str) -> Result<String> {
//!     if html.is_empty() {
//!         return Err(LectitoError::NoContent);
//!     }
//!     // ... extraction logic
//!     # Ok(String::new())
//! }
//! ```

use std::path::PathBuf;
use thiserror::Error;

#[cfg(feature = "siteconfig")]
use sxd_xpath::ExecutionError;

/// Main error type for readability extraction operations.
///
/// This enum represents all possible errors that can occur during
/// content extraction, HTTP fetching, file I/O, and parsing.
///
/// # Example
///
/// ```rust
/// use lectito_core::{LectitoError, parse};
///
/// match parse("<html>...</html>") {
///     Ok(article) => println!("Success: {}", article.metadata.title.unwrap()),
///     Err(LectitoError::NotReadable { score, threshold }) => {
///         println!("Score {} below threshold {}", score, threshold);
///     }
///     Err(e) => println!("Error: {}", e),
/// }
/// ```
#[derive(Error, Debug)]
pub enum LectitoError {
    /// HTTP request errors from reqwest.
    ///
    /// This variant wraps network errors, DNS failures, connection issues,
    /// and other HTTP-related problems.
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    /// Request timeout.
    ///
    /// Returned when an HTTP request exceeds the configured timeout duration.
    #[error("Request timed out after {timeout} seconds")]
    Timeout { timeout: u64 },

    /// Invalid URL provided.
    ///
    /// Returned when a URL cannot be parsed or is malformed.
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// HTML parsing errors.
    ///
    /// Returned when HTML cannot be parsed, often due to malformed markup
    /// or invalid CSS selectors.
    #[error("Failed to parse HTML: {0}")]
    HtmlParseError(String),

    /// Invalid character encoding.
    ///
    /// Returned when the document contains invalid UTF-8 or other encoding issues.
    #[error("Invalid character encoding")]
    InvalidEncoding,

    /// Content is not readable (score below threshold).
    ///
    /// This error occurs when the content scoring algorithm determines
    /// that no element meets the minimum readability score threshold.
    /// This typically happens on navigation pages, search results,
    /// or pages with very little text content.
    #[error("Content is not readable (score {score} below threshold {threshold})")]
    NotReadable { score: f64, threshold: f64 },

    /// No content could be extracted from the document.
    ///
    /// Returned when the document is empty or contains no suitable content candidates.
    #[error("No content could be extracted from the document")]
    NoContent,

    /// File not found.
    ///
    /// Returned when attempting to read a file that doesn't exist.
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    /// File write errors.
    ///
    /// Wraps standard I/O errors for file operations.
    #[error("Failed to write to file: {0}")]
    WriteError(#[from] std::io::Error),

    /// Site configuration errors.
    ///
    /// Returned when site configuration files are missing or invalid.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Site configuration errors (FTR format).
    ///
    /// Returned when parsing FTR-format site configuration files fails.
    #[error("Site configuration error: {0}")]
    SiteConfigError(String),

    /// XPath evaluation errors.
    ///
    /// Returned when XPath expressions in site configurations fail to evaluate.
    /// This variant is only available when the `siteconfig` feature is enabled.
    #[cfg(feature = "siteconfig")]
    #[error("XPath error: {0}")]
    XPathError(String),
}

#[cfg(feature = "siteconfig")]
impl From<ExecutionError> for LectitoError {
    fn from(err: ExecutionError) -> Self {
        LectitoError::XPathError(err.to_string())
    }
}

/// Result type alias for LectitoError.
///
/// This is a convenience alias for `std::result::Result<T, LectitoError>`.
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
    fn test_not_readable_error() {
        let err = LectitoError::NotReadable { score: 15.0, threshold: 20.0 };
        assert!(err.to_string().contains("15"));
        assert!(err.to_string().contains("20"));
    }

    #[test]
    fn test_timeout_error() {
        let err = LectitoError::Timeout { timeout: 30 };
        assert!(err.to_string().contains("30"));
    }
}
