use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use url::{Url, form_urlencoded};
use uuid::Uuid;

use crate::error::AppError;

const TRACKING_QUERY_PARAMS: &[&str] = &["fbclid", "gclid", "mc_cid", "mc_eid", "ref", "ref_src"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CachedFormat {
    Html,
    Markdown,
    Text,
    Json,
}

impl CachedFormat {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Markdown => "markdown",
            Self::Text => "text",
            Self::Json => "json",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedExtractedArticle {
    pub id: Uuid,
    pub url: String,
    pub format: CachedFormat,
    pub content: String,
    pub metadata: CachedMetadata,
    pub fetched_at: OffsetDateTime,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CachedMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub excerpt: Option<String>,
    pub site_name: Option<String>,
    pub language: Option<String>,
    pub word_count: Option<usize>,
    pub reading_time_minutes: Option<f64>,
    pub image: Option<String>,
    pub favicon: Option<String>,
}

pub fn normalize_url(url: &str) -> Result<String, AppError> {
    let mut parsed = Url::parse(url).map_err(|err| AppError::BadRequest(format!("Invalid URL: {err}")))?;

    let scheme = parsed.scheme().to_ascii_lowercase();
    parsed
        .set_scheme(&scheme)
        .map_err(|_| AppError::BadRequest("Invalid URL scheme".to_string()))?;

    if let Some(host) = parsed.host_str().map(str::to_ascii_lowercase) {
        parsed
            .set_host(Some(&host))
            .map_err(|_| AppError::BadRequest("Invalid URL host".to_string()))?;
    }

    parsed.set_fragment(None);

    if parsed.query().is_some() {
        let mut pairs = parsed
            .query_pairs()
            .filter(|(key, _)| !is_tracking_param(key))
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect::<Vec<_>>();
        pairs.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

        if pairs.is_empty() {
            parsed.set_query(None);
        } else {
            let mut serializer = form_urlencoded::Serializer::new(String::new());
            for (key, value) in pairs {
                serializer.append_pair(&key, &value);
            }
            parsed.set_query(Some(&serializer.finish()));
        }
    }

    Ok(parsed.to_string())
}

#[must_use]
pub fn build_cache_key(normalized_url: &str, format: CachedFormat) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(normalized_url.as_bytes());
    hasher.update(b"\n");
    hasher.update(format.as_str().as_bytes());
    hasher.finalize().to_vec()
}

#[must_use]
pub fn should_bypass_cache(headers: &HeaderMap) -> bool {
    headers
        .get(axum::http::header::CACHE_CONTROL)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .map(str::trim)
                .any(|token| token.eq_ignore_ascii_case("no-cache"))
        })
}

fn is_tracking_param(param: &str) -> bool {
    let lowercase = param.to_ascii_lowercase();
    lowercase.starts_with("utm_") || TRACKING_QUERY_PARAMS.contains(&lowercase.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn normalizes_scheme_host_query_and_fragment() {
        let normalized =
            normalize_url("HTTPS://Example.COM/path?utm_source=newsletter&b=2&a=1&fbclid=abc#section").unwrap();

        assert_eq!(normalized, "https://example.com/path?a=1&b=2");
    }

    #[test]
    fn cache_key_changes_with_format() {
        let url = "https://example.com/article";
        let markdown = build_cache_key(url, CachedFormat::Markdown);
        let html = build_cache_key(url, CachedFormat::Html);

        assert_ne!(markdown, html);
    }

    #[test]
    fn no_cache_header_is_detected() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::CACHE_CONTROL,
            HeaderValue::from_static("max-age=0, no-cache"),
        );

        assert!(should_bypass_cache(&headers));
    }
}
