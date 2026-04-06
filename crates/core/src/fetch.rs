//! Content fetching from URLs, files, and stdin.
//!
//! This module provides functions for retrieving HTML content from
//! various sources: HTTP/HTTPS URLs, local files, and standard input.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
#[cfg(feature = "fetch")]
use std::time::Duration;

#[cfg(feature = "fetch")]
use reqwest::Client;
#[cfg(feature = "fetch")]
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
    /// Additional request headers to apply.
    pub headers: HashMap<String, String>,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            timeout: 30,
            user_agent: "Mozilla/5.0 (compatible; Lectito/1.0; +https://github.com/stormlightlabs/lectito)".to_string(),
            headers: HashMap::new(),
        }
    }
}

/// Fetches HTML content from a URL.
///
/// This function performs an HTTP GET request and returns the response body as text.
/// It follows redirects, respects the configured timeout, and uses a browser-like
/// User-Agent for better compatibility.
#[cfg(feature = "fetch")]
pub async fn fetch_url(url: &str, config: &FetchConfig) -> Result<String> {
    let parsed_url = Url::parse(url).map_err(|e| LectitoError::InvalidUrl(e.to_string()))?;

    if parsed_url.scheme().is_empty() {
        return Err(LectitoError::InvalidUrl(
            "URL must include a scheme (http:// or https://)".to_string(),
        ));
    }

    let response = match send_request(&parsed_url, config, false).await {
        Ok(response) => response,
        Err(err) if should_retry_with_http1(&err) => send_request(&parsed_url, config, true).await.map_err(|e| {
            if e.is_timeout() {
                LectitoError::Timeout { timeout: config.timeout }
            } else {
                LectitoError::HttpError(e)
            }
        })?,
        Err(err) => {
            return Err(if err.is_timeout() {
                LectitoError::Timeout { timeout: config.timeout }
            } else {
                LectitoError::HttpError(err)
            });
        }
    };

    response.text().await.map_err(LectitoError::HttpError)
}

#[cfg(not(feature = "fetch"))]
pub async fn fetch_url(_url: &str, _config: &FetchConfig) -> Result<String> {
    Err(LectitoError::ConfigError(
        "URL fetching requires the `fetch` feature".to_string(),
    ))
}

#[cfg(feature = "fetch")]
fn build_client(config: &FetchConfig, http1_only: bool) -> std::result::Result<Client, reqwest::Error> {
    let mut builder = Client::builder().timeout(Duration::from_secs(config.timeout));
    if http1_only {
        builder = builder.use_native_tls().http1_only();
    } else {
        builder = builder.use_rustls_tls();
    }
    builder.build()
}

#[cfg(feature = "fetch")]
fn build_request(client: &Client, url: &Url, config: &FetchConfig) -> reqwest::RequestBuilder {
    let mut request = client
        .get(url.clone())
        .header("User-Agent", &config.user_agent)
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .header("Accept-Language", "en-US,en;q=0.9");

    for (name, value) in &config.headers {
        request = request.header(name, value);
    }

    request
}

#[cfg(feature = "fetch")]
async fn send_request(
    parsed_url: &Url, config: &FetchConfig, http1_only: bool,
) -> std::result::Result<reqwest::Response, reqwest::Error> {
    let client = build_client(config, http1_only)?;
    build_request(&client, parsed_url, config).send().await
}

#[cfg(feature = "fetch")]
fn should_retry_with_http1(error: &reqwest::Error) -> bool {
    let message = error.to_string().to_lowercase();
    (error.is_connect() || message.contains("protocol"))
        && (message.contains("bad protocol version")
            || message.contains("alpn")
            || message.contains("http2")
            || message.contains("protocol error"))
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

    #[cfg(feature = "fetch")]
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
        assert!(url::Url::parse("http://example.com").is_ok());
        assert!(url::Url::parse("https://example.com").is_ok());
        assert!(url::Url::parse("example.com").is_err()); // Missing scheme
    }

    #[test]
    fn test_error_timeout_message() {
        let err = LectitoError::Timeout { timeout: 30 };
        assert!(err.to_string().contains("30"));
    }
}
