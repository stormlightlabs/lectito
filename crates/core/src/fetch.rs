//! Content fetching from URLs, files, and stdin.
//!
//! This module provides functions for retrieving HTML content from
//! various sources: HTTP/HTTPS URLs, local files, and standard input.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use reqwest::Client;
use url::Url;

use crate::{LectitoError, Result};

/// HTTP client configuration for fetching web pages.
///
/// This struct controls timeout and user agent settings for HTTP requests.
#[derive(Debug, Clone)]
pub struct FetchConfig {
    /// Request timeout in seconds.
    pub timeout: u64,
    /// Custom User-Agent string.
    pub user_agent: String,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            timeout: 30,
            user_agent: "Mozilla/5.0 (compatible; Lectito/1.0; +https://github.com/stormlightlabs/lectito)".to_string(),
        }
    }
}

/// Fetches HTML content from a URL.
///
/// This function performs an HTTP GET request and returns the response body as text.
/// It follows redirects, respects the configured timeout, and uses a browser-like
/// User-Agent for better compatibility.
pub async fn fetch_url(url: &str, config: &FetchConfig) -> Result<String> {
    let parsed_url = Url::parse(url).map_err(|e| LectitoError::InvalidUrl(e.to_string()))?;

    if parsed_url.scheme().is_empty() {
        return Err(LectitoError::InvalidUrl(
            "URL must include a scheme (http:// or https://)".to_string(),
        ));
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(config.timeout))
        .build()
        .map_err(LectitoError::HttpError)?;

    let response = client
        .get(parsed_url)
        .header("User-Agent", &config.user_agent)
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                LectitoError::Timeout { timeout: config.timeout }
            } else {
                LectitoError::HttpError(e)
            }
        })?;

    let content = response.text().await?;

    Ok(content)
}

/// Reads HTML content from a local file.
///
/// Callers should validate and sanitize the path when accepting user input.
pub fn fetch_file(path: &str) -> Result<String> {
    let path_buf = PathBuf::from(path);

    if !path_buf.exists() {
        Err(LectitoError::FileNotFound(path_buf))
    } else {
        fs::read_to_string(&path_buf).map_err(LectitoError::from)
    }
}

/// Reads HTML content from standard input.
///
/// This function reads all available input from stdin until EOF.
/// Useful for piping content from other commands.
pub fn fetch_stdin() -> Result<String> {
    use std::io::{self, Read};

    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer).map_err(LectitoError::from)?;

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_config_default() {
        let config = FetchConfig::default();
        assert_eq!(config.timeout, 30);
        assert!(config.user_agent.contains("Lectito"));
    }

    #[test]
    fn test_fetch_url_invalid() {
        let config = FetchConfig::default();
        let result = std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(fetch_url("not-a-url", &config))
        })
        .join()
        .unwrap();

        assert!(matches!(result, Err(LectitoError::InvalidUrl(_))));
    }

    #[test]
    fn test_fetch_file_not_found() {
        let result = fetch_file("/nonexistent/path/file.html");
        assert!(matches!(result, Err(LectitoError::FileNotFound(_))));
    }

    #[test]
    fn test_url_validation() {
        assert!(Url::parse("http://example.com").is_ok());
        assert!(Url::parse("https://example.com").is_ok());
        assert!(Url::parse("example.com").is_err()); // Missing scheme
    }

    #[test]
    fn test_error_timeout_message() {
        let err = LectitoError::Timeout { timeout: 30 };
        assert!(err.to_string().contains("30"));
    }
}
