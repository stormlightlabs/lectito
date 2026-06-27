//! DuckDuckGo HTML search support for the MCP article search tool.
//!
//! This module uses DuckDuckGo's HTML endpoint because it returns server-rendered
//! result markup that can be parsed without browser automation.
//!
//! The parser is extracts the public result title, target URL, and
//! snippet, then leaves article extraction to the `read_article` tool.

use reqwest::header;
use scraper::{Html, Selector};
use serde::Serialize;
use url::Url;

/// DuckDuckGo's form-backed HTML search endpoint.
pub const DUCKDUCKGO_HTML_URL: &str = "https://html.duckduckgo.com/html/";

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36";

/// Errors returned while searching or parsing DuckDuckGo HTML results.
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    /// The HTTP client failed before a response body was available.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    /// DuckDuckGo returned a non-success HTTP status.
    #[error("HTTP request returned {status}: {body}")]
    HttpStatus { status: reqwest::StatusCode, body: String },
    /// DuckDuckGo returned an anti-bot page instead of search results.
    #[error("{0}")]
    Blocked(String),
    /// The configured DuckDuckGo endpoint was not a valid URL.
    #[error("invalid URL: {0}")]
    Url(#[from] url::ParseError),
    /// A hard-coded selector failed to parse.
    #[error("invalid CSS selector: {0}")]
    InvalidSelector(&'static str),
}

/// One result from DuckDuckGo's HTML search page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    /// The result title as displayed by DuckDuckGo.
    pub title: String,
    /// The normalized target URL.
    pub url: String,
    /// DuckDuckGo's result snippet, when present.
    pub snippet: Option<String>,
}

/// Minimal client for DuckDuckGo HTML search.
#[derive(Debug, Clone)]
pub struct DuckDuckGoSearch {
    http: reqwest::Client,
    endpoint: Url,
}

impl DuckDuckGoSearch {
    /// Build a search client using the default DuckDuckGo HTML endpoint.
    pub fn new() -> Result<Self, SearchError> {
        Self::with_endpoint(DUCKDUCKGO_HTML_URL)
    }

    fn with_endpoint(endpoint: impl AsRef<str>) -> Result<Self, SearchError> {
        let endpoint = Url::parse(endpoint.as_ref())?;
        let http = reqwest::Client::builder().user_agent(USER_AGENT).build()?;
        Ok(Self { http, endpoint })
    }

    /// Search DuckDuckGo and return up to `limit` parsed results.
    ///
    /// Empty queries and zero limits return an empty result set without making a
    /// network request.
    ///
    /// The caller is responsible for applying any product-level cap before passing `limit`.
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let query = query.trim();
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let form = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("q", query)
            .append_pair("b", "")
            .append_pair("l", "us-en")
            .finish();
        let response = self
            .http
            .post(self.endpoint.clone())
            .header(header::ACCEPT, "text/html,application/xhtml+xml")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded; charset=UTF-8")
            .body(form)
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(SearchError::HttpStatus { status, body: body.chars().take(500).collect() });
        }

        parse_duckduckgo_html(&body, limit)
    }
}

/// Parse DuckDuckGo HTML search results.
///
/// The parser accepts the same result markup currently used by
/// `html.duckduckgo.com`, detects the common bot-challenge page, and normalizes
/// DuckDuckGo redirect links into their `uddg` target URLs.
pub fn parse_duckduckgo_html(html: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    if html.contains("anomaly-modal")
        || html.contains("Unfortunately, bots use DuckDuckGo too")
        || html.contains("/anomaly.js")
    {
        return Err(SearchError::Blocked(
            "DuckDuckGo returned a bot challenge instead of search results".to_string(),
        ));
    }

    let document = Html::parse_document(html);
    let result_selector = selector(".result")?;
    let title_selector = selector(".result__title a, a.result__a")?;
    let snippet_selector = selector(".result__snippet")?;
    let url_selector = selector(".result__url")?;
    let mut results = Vec::new();

    for result in document.select(&result_selector) {
        let Some(link) = result.select(&title_selector).next() else {
            continue;
        };
        let Some(href) = link.value().attr("href") else {
            continue;
        };

        let title = clean_text(&link.text().collect::<Vec<_>>().join(" "));
        if title.is_empty() {
            continue;
        }

        let snippet = result
            .select(&snippet_selector)
            .next()
            .map(|node| clean_text(&node.text().collect::<Vec<_>>().join(" ")))
            .filter(|text| !text.is_empty());
        let fallback_url = result
            .select(&url_selector)
            .next()
            .map(|node| clean_text(&node.text().collect::<Vec<_>>().join(" ")))
            .filter(|text| !text.is_empty());

        let Some(url) = normalize_duckduckgo_url(href).or(fallback_url) else {
            continue;
        };

        results.push(SearchResult { title, url, snippet });
        if results.len() >= limit {
            break;
        }
    }

    Ok(results)
}

fn selector(css: &'static str) -> Result<Selector, SearchError> {
    Selector::parse(css).map_err(|_| SearchError::InvalidSelector(css))
}

fn clean_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_duckduckgo_url(href: &str) -> Option<String> {
    let decoded = html_unescape(href);
    if decoded.starts_with("http://") || decoded.starts_with("https://") {
        return Some(decoded);
    }

    let base = Url::parse(DUCKDUCKGO_HTML_URL).ok()?;
    let url = base.join(&decoded).ok()?;
    if url.path() == "/l/" {
        return url
            .query_pairs()
            .find_map(|(key, value)| (key == "uddg").then(|| value.into_owned()));
    }

    Some(url.to_string())
}

fn html_unescape(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_duckduckgo_html_results() {
        let html = r#"
            <div class="result">
              <h2 class="result__title">
                <a class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fpost%3Fx%3D1&amp;rut=abc">
                  Example Result
                </a>
              </h2>
              <a class="result__url">example.com/post</a>
              <a class="result__snippet">A compact result snippet.</a>
            </div>
        "#;

        let results = parse_duckduckgo_html(html, 10).expect("html parses");

        assert_eq!(
            results,
            vec![SearchResult {
                title: "Example Result".to_string(),
                url: "https://example.com/post?x=1".to_string(),
                snippet: Some("A compact result snippet.".to_string()),
            }]
        );
    }

    #[test]
    fn respects_result_limit() {
        let html = r#"
            <div class="result"><a class="result__a" href="https://a.test">A</a></div>
            <div class="result"><a class="result__a" href="https://b.test">B</a></div>
        "#;

        let results = parse_duckduckgo_html(html, 1).expect("html parses");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "A");
    }

    #[test]
    fn reports_duckduckgo_bot_challenge() {
        let html = r#"
            <form id="challenge-form" action="//duckduckgo.com/anomaly.js">
              <div class="anomaly-modal__title">
                Unfortunately, bots use DuckDuckGo too.
              </div>
            </form>
        "#;

        let error = parse_duckduckgo_html(html, 10).expect_err("challenge is an error");
        assert!(matches!(error, SearchError::Blocked(_)));
    }
}
