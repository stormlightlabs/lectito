//! MCP server library for Lectito article-reading tools.

use std::env;
use std::net::IpAddr;
use std::time::Duration;

use ddg::{DuckDuckGoSearch, SearchResult};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use url::Url;

pub mod ddg;

const PROTOCOL_VERSION: &str = "2025-06-18";
const DEFAULT_SEARCH_LIMIT: usize = 5;
const MAX_SEARCH_LIMIT: usize = 10;
const DEFAULT_MAX_FETCH_BYTES: usize = 2 * 1024 * 1024;
const DEFAULT_REDIRECT_LIMIT: usize = 5;
const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 20;
const DEFAULT_MAX_ARTICLE_CHARS: usize = 12_000;
const MAX_ARTICLE_CHARS: usize = 30_000;

/// Errors returned by the stdio MCP server loop.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// Reading from stdin or writing to stdout failed.
    #[error("I/O failed: {0}")]
    Io(#[from] std::io::Error),
    /// Serializing a JSON-RPC response failed.
    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ArticleFormat {
    #[default]
    Markdown,
    Text,
    Html,
    Json,
}

#[derive(Debug, Deserialize)]
struct RpcRequest {
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct RpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(flatten)]
    payload: RpcPayload,
}

impl RpcResponse {
    fn result(id: Value, result: McpResult) -> Self {
        Self { jsonrpc: "2.0", id, payload: RpcPayload::Result { result } }
    }

    fn error(id: Value, error: RpcError) -> Self {
        Self { jsonrpc: "2.0", id, payload: RpcPayload::Error { error: error.into_body() } }
    }
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum RpcPayload {
    Result { result: McpResult },
    Error { error: RpcErrorBody },
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum McpResult {
    Initialize(InitializeResult),
    Empty(EmptyObject),
    ToolsList(ToolsListResult),
    Tool(ToolResult),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InitializeResult {
    protocol_version: &'static str,
    capabilities: Capabilities,
    server_info: ServerInfo,
}

#[derive(Debug, Serialize)]
struct Capabilities {
    tools: EmptyObject,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerInfo {
    name: &'static str,
    version: &'static str,
}

#[derive(Debug, Serialize)]
struct ToolsListResult {
    tools: Vec<ToolDefinition>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolDefinition {
    name: &'static str,
    description: &'static str,
    input_schema: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolResult {
    content: Vec<ToolContent>,
    structured_content: ToolStructuredContent,
    is_error: bool,
}

impl ToolResult {
    fn new(text: impl Into<String>, contents: ToolStructuredContent, is_error: bool) -> Self {
        Self {
            content: vec![ToolContent { kind: "text", text: text.into() }],
            structured_content: contents,
            is_error,
        }
    }
}

#[derive(Debug, Serialize)]
struct ToolContent {
    #[serde(rename = "type")]
    kind: &'static str,
    text: String,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ToolStructuredContent {
    SearchResults { results: Vec<SearchResult> },
    Article { article: ReadArticleOutput },
    Error { error: String },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadArticleOutput {
    url: String,
    final_url: String,
    title: Option<String>,
    byline: Option<String>,
    site_name: Option<String>,
    published_time: Option<String>,
    excerpt: Option<String>,
    content_length: usize,
    format: ArticleFormat,
    offset: usize,
    max_chars: usize,
    content_char_length: usize,
    next_offset: Option<usize>,
    truncated: bool,
    content: String,
}

#[derive(Debug, Default, Serialize)]
struct EmptyObject {}

#[derive(Debug, Serialize)]
struct RpcErrorBody {
    code: i64,
    message: String,
}

#[derive(Debug, Deserialize)]
struct ToolCallParams {
    name: String,
    #[serde(default)]
    arguments: Value,
}

#[derive(Debug, Deserialize)]
struct SearchArticlesArgs {
    query: String,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadArticleArgs {
    url: String,
    #[serde(default)]
    format: ArticleFormat,
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_max_article_chars")]
    max_chars: usize,
}

struct FetchedDocument {
    final_url: Url,
    html: String,
}

#[derive(Debug)]
struct RpcError {
    code: i64,
    message: String,
}

impl RpcError {
    fn method_not_found(method: String) -> Self {
        Self { code: -32601, message: format!("method not found: {method}") }
    }

    fn invalid_params(message: impl Into<String>) -> Self {
        Self { code: -32602, message: message.into() }
    }

    fn into_body(self) -> RpcErrorBody {
        RpcErrorBody { code: self.code, message: self.message }
    }
}

/// Runtime configuration for the MCP server.
pub struct Config {
    default_search_limit: usize,
    max_search_limit: usize,
    max_fetch_bytes: usize,
    redirect_limit: usize,
    request_timeout_secs: u64,
    max_article_chars: usize,
    allow_private_network: bool,
}

impl Config {
    /// Build configuration from environment variables.
    pub fn from_env() -> Self {
        let max_search_limit = env_usize("LECTITO_MCP_MAX_SEARCH_RESULTS", MAX_SEARCH_LIMIT).max(1);
        let max_article_chars = env_usize("LECTITO_MCP_MAX_ARTICLE_CHARS", MAX_ARTICLE_CHARS).max(1);
        Self {
            default_search_limit: env_usize("LECTITO_MCP_DEFAULT_SEARCH_RESULTS", DEFAULT_SEARCH_LIMIT)
                .clamp(1, max_search_limit),
            max_search_limit,
            max_fetch_bytes: env_usize("LECTITO_MCP_MAX_FETCH_BYTES", DEFAULT_MAX_FETCH_BYTES),
            redirect_limit: env_usize("LECTITO_MCP_REDIRECT_LIMIT", DEFAULT_REDIRECT_LIMIT),
            request_timeout_secs: env_u64("LECTITO_MCP_REQUEST_TIMEOUT_SECS", DEFAULT_REQUEST_TIMEOUT_SECS),
            max_article_chars,
            allow_private_network: env_bool("LECTITO_MCP_ALLOW_PRIVATE_NETWORK", false),
        }
    }
}

/// Stdio MCP server for article-reading tools.
pub struct Server {
    search: DuckDuckGoSearch,
    http: reqwest::Client,
    config: Config,
}

impl Server {
    /// Create a server from a search client and runtime configuration.
    pub fn new(search: DuckDuckGoSearch, config: Config) -> Self {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .user_agent("lectito-mcp/0.1")
            .build()
            .expect("failed to initialize HTTP client");
        Self { search, http, config }
    }

    /// Run the server over newline-delimited JSON-RPC on stdin/stdout.
    pub async fn run_stdio(&self) -> Result<(), ServerError> {
        let stdin = tokio::io::stdin();
        let mut lines = BufReader::new(stdin).lines();
        let mut stdout = tokio::io::stdout();

        while let Some(line) = lines.next_line().await? {
            let Some(response) = self.handle_line(&line).await else {
                continue;
            };
            stdout.write_all(serde_json::to_string(&response)?.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }

        Ok(())
    }

    async fn handle_line(&self, line: &str) -> Option<RpcResponse> {
        let request = match serde_json::from_str::<RpcRequest>(line) {
            Ok(request) => request,
            Err(error) => {
                return Some(RpcResponse::error(
                    Value::Null,
                    RpcError { code: -32700, message: format!("parse error: {error}") },
                ));
            }
        };

        let Some(id) = request.id.clone() else {
            return None;
        };

        match self.handle_request(request).await {
            Ok(result) => Some(RpcResponse::result(id, result)),
            Err(error) => Some(RpcResponse::error(id, error)),
        }
    }

    async fn handle_request(&self, request: RpcRequest) -> Result<McpResult, RpcError> {
        match request.method.as_str() {
            "initialize" => Ok(McpResult::Initialize(InitializeResult {
                protocol_version: PROTOCOL_VERSION,
                capabilities: Capabilities { tools: EmptyObject::default() },
                server_info: ServerInfo { name: "lectito-mcp", version: env!("CARGO_PKG_VERSION") },
            })),
            "ping" => Ok(McpResult::Empty(EmptyObject::default())),
            "tools/list" => Ok(McpResult::ToolsList(ToolsListResult {
                tools: vec![self.search_articles_tool(), self.read_article_tool()],
            })),
            "tools/call" => self.call_tool(request.params).await.map(McpResult::Tool),
            _ => Err(RpcError::method_not_found(request.method)),
        }
    }

    fn search_articles_tool(&self) -> ToolDefinition {
        ToolDefinition {
            name: "search_articles",
            description: "Search DuckDuckGo HTML results for article-like pages.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The web search query."
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": self.config.max_search_limit,
                        "description": "Maximum number of results to return."
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        }
    }

    fn read_article_tool(&self) -> ToolDefinition {
        ToolDefinition {
            name: "read_article",
            description: "Fetch a public URL and extract readable article content.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "Absolute http or https URL to read."
                    },
                    "format": {
                        "type": "string",
                        "enum": ["markdown", "text", "html", "json"],
                        "default": "markdown",
                        "description": "Output format for the returned content chunk."
                    },
                    "offset": {
                        "type": "integer",
                        "minimum": 0,
                        "default": 0,
                        "description": "Character offset into the selected content format."
                    },
                    "maxChars": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": self.config.max_article_chars,
                        "default": DEFAULT_MAX_ARTICLE_CHARS,
                        "description": "Maximum characters to return."
                    }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
        }
    }

    async fn call_tool(&self, params: Value) -> Result<ToolResult, RpcError> {
        let params = serde_json::from_value::<ToolCallParams>(params)
            .map_err(|error| RpcError::invalid_params(format!("invalid tool call params: {error}")))?;

        match params.name.as_str() {
            "search_articles" => self.search_articles(params.arguments).await,
            "read_article" => self.read_article(params.arguments).await,
            name => Err(RpcError::invalid_params(format!("unknown tool: {name}"))),
        }
    }

    async fn search_articles(&self, arguments: Value) -> Result<ToolResult, RpcError> {
        let arguments = serde_json::from_value::<SearchArticlesArgs>(arguments)
            .map_err(|error| RpcError::invalid_params(format!("invalid search_articles arguments: {error}")))?;
        let query = arguments.query.trim();
        if query.is_empty() {
            return Err(RpcError::invalid_params("query must not be empty"));
        }

        let limit = arguments
            .limit
            .unwrap_or(self.config.default_search_limit)
            .clamp(1, self.config.max_search_limit);

        match self.search.search(query, limit).await {
            Ok(results) => Ok(ToolResult::new(
                search_summary(&results),
                ToolStructuredContent::SearchResults { results },
                false,
            )),
            Err(error) => Ok(ToolResult::new(
                format!("search_articles failed: {error}"),
                ToolStructuredContent::Error { error: error.to_string() },
                true,
            )),
        }
    }

    async fn read_article(&self, arguments: Value) -> Result<ToolResult, RpcError> {
        let arguments = serde_json::from_value::<ReadArticleArgs>(arguments)
            .map_err(|error| RpcError::invalid_params(format!("invalid read_article arguments: {error}")))?;
        let max_chars = arguments.max_chars.clamp(1, self.config.max_article_chars);

        match self.read_article_output(&arguments, max_chars).await {
            Ok(article) => Ok(ToolResult::new(
                article.content.clone(),
                ToolStructuredContent::Article { article },
                false,
            )),
            Err(error) => Ok(ToolResult::new(
                format!("read_article failed: {error}"),
                ToolStructuredContent::Error { error: error.to_string() },
                true,
            )),
        }
    }

    async fn read_article_output(
        &self, arguments: &ReadArticleArgs, max_chars: usize,
    ) -> Result<ReadArticleOutput, ReadArticleError> {
        let fetched = self.fetch_url(&arguments.url).await?;
        let report = lectito::extract_with_diagnostics(
            &fetched.html,
            Some(fetched.final_url.as_str()),
            &lectito::ReadabilityOptions::default(),
        )?;
        let article = report.article.ok_or(ReadArticleError::NoArticle)?;
        let content = article_content(&article, arguments.format)?;
        let content_char_length = content.chars().count();
        let (content, next_offset, truncated) = chunk_chars(&content, arguments.offset, max_chars);

        Ok(ReadArticleOutput {
            url: arguments.url.clone(),
            final_url: fetched.final_url.to_string(),
            title: article.title,
            byline: article.byline,
            site_name: article.site_name,
            published_time: article.published_time,
            excerpt: article.excerpt,
            content_length: article.length,
            format: arguments.format,
            offset: arguments.offset,
            max_chars,
            content_char_length,
            next_offset,
            truncated,
            content,
        })
    }

    async fn fetch_url(&self, url: &str) -> Result<FetchedDocument, ReadArticleError> {
        let mut url = parse_public_url(url)?;

        for redirect_count in 0..=self.config.redirect_limit {
            if !self.config.allow_private_network {
                reject_private_target(&url).await?;
            }

            let response = self.http.get(url.clone()).send().await?;

            if response.status().is_redirection() {
                if redirect_count == self.config.redirect_limit {
                    return Err(ReadArticleError::InvalidRequest("redirect limit exceeded".to_string()));
                }
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or_else(|| {
                        ReadArticleError::InvalidRequest("redirect response missing location".to_string())
                    })?;
                url = url
                    .join(location)
                    .map_err(|_| ReadArticleError::InvalidRequest("invalid redirect location".to_string()))?;
                parse_public_url(url.as_str())?;
                continue;
            }

            if !response.status().is_success() {
                return Err(ReadArticleError::Fetch(format!(
                    "upstream returned {}",
                    response.status()
                )));
            }

            ensure_html_content_type(response.headers())?;
            let bytes = response.bytes().await?;
            if bytes.len() > self.config.max_fetch_bytes {
                return Err(ReadArticleError::DocumentTooLarge);
            }

            return Ok(FetchedDocument { final_url: url, html: String::from_utf8_lossy(&bytes).into_owned() });
        }

        Err(ReadArticleError::InvalidRequest("redirect limit exceeded".to_string()))
    }
}

fn env_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_bool(name: &str, default: bool) -> bool {
    env::var(name)
        .ok()
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(default)
}

fn search_summary(results: &[SearchResult]) -> String {
    if results.is_empty() {
        return "No search results found.".to_string();
    }

    results
        .iter()
        .enumerate()
        .map(|(index, result)| {
            let snippet = result
                .snippet
                .as_ref()
                .map(|snippet| format!("\n   {snippet}"))
                .unwrap_or_default();
            format!("{}. {}\n   {}{}", index + 1, result.title, result.url, snippet)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_public_url(value: &str) -> Result<Url, ReadArticleError> {
    let url = Url::parse(value).map_err(|_| ReadArticleError::InvalidRequest("url must be absolute".to_string()))?;
    match url.scheme() {
        "http" | "https" => Ok(url),
        _ => Err(ReadArticleError::InvalidRequest(
            "url must use http or https".to_string(),
        )),
    }
}

fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_documentation()
                || ip.is_unspecified()
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || matches!(ip.segments()[0] & 0xfe00, 0xfc00)
                || matches!(ip.segments()[0] & 0xffc0, 0xfe80)
        }
    }
}

fn ensure_html_content_type(headers: &reqwest::header::HeaderMap) -> Result<(), ReadArticleError> {
    let Some(content_type) = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
    else {
        return Ok(());
    };
    match content_type.split(';').next().unwrap_or_default().trim() {
        "text/html" | "application/xhtml+xml" => Ok(()),
        _ => Err(ReadArticleError::UnsupportedContentType),
    }
}

fn article_content(article: &lectito::Article, format: ArticleFormat) -> Result<String, ReadArticleError> {
    match format {
        ArticleFormat::Markdown => Ok(article.markdown.clone()),
        ArticleFormat::Text => Ok(article.text_content.clone()),
        ArticleFormat::Html => Ok(article.content.clone()),
        ArticleFormat::Json => serde_json::to_string(article).map_err(ReadArticleError::Serialize),
    }
}

fn chunk_chars(text: &str, offset: usize, max_chars: usize) -> (String, Option<usize>, bool) {
    let total_chars = text.chars().count();
    if offset >= total_chars {
        return (String::new(), None, false);
    }

    let content = text.chars().skip(offset).take(max_chars).collect::<String>();
    let next_offset = offset + content.chars().count();
    let truncated = next_offset < total_chars;

    (content, truncated.then_some(next_offset), truncated)
}

fn default_max_article_chars() -> usize {
    DEFAULT_MAX_ARTICLE_CHARS
}

async fn reject_private_target(url: &Url) -> Result<(), ReadArticleError> {
    let host = url
        .host_str()
        .ok_or_else(|| ReadArticleError::InvalidRequest("url is missing a host".to_string()))?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| ReadArticleError::InvalidRequest("url is missing a port".to_string()))?;
    let addresses = tokio::net::lookup_host((host, port))
        .await?
        .map(|addr| addr.ip())
        .collect::<Vec<_>>();

    if addresses.is_empty() {
        return Err(ReadArticleError::Fetch("host did not resolve".to_string()));
    }

    if addresses.iter().any(is_private_ip) {
        return Err(ReadArticleError::InvalidRequest(
            "private-network targets are not supported".to_string(),
        ));
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum ReadArticleError {
    #[error("{0}")]
    InvalidRequest(String),
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("fetch failed: {0}")]
    Fetch(String),
    #[error("DNS lookup failed: {0}")]
    Dns(#[from] std::io::Error),
    #[error("upstream content type is not supported")]
    UnsupportedContentType,
    #[error("fetched document is too large")]
    DocumentTooLarge,
    #[error("extraction failed: {0}")]
    Extract(#[from] lectito::Error),
    #[error("no readable article found")]
    NoArticle,
    #[error("failed to serialize article JSON: {0}")]
    Serialize(serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            default_search_limit: 3,
            max_search_limit: 7,
            max_fetch_bytes: DEFAULT_MAX_FETCH_BYTES,
            redirect_limit: DEFAULT_REDIRECT_LIMIT,
            request_timeout_secs: DEFAULT_REQUEST_TIMEOUT_SECS,
            max_article_chars: 9_000,
            allow_private_network: false,
        }
    }

    #[test]
    fn search_tool_schema_uses_configured_limit() {
        let search = DuckDuckGoSearch::new().expect("search client initializes");
        let server = Server::new(search, test_config());

        let tool = serde_json::to_value(server.search_articles_tool()).expect("tool serializes");

        assert_eq!(tool["name"], "search_articles");
        assert_eq!(tool["inputSchema"]["properties"]["limit"]["maximum"], 7);
    }

    #[test]
    fn read_tool_schema_uses_configured_content_limit() {
        let search = DuckDuckGoSearch::new().expect("search client initializes");
        let server = Server::new(search, test_config());

        let tool = serde_json::to_value(server.read_article_tool()).expect("tool serializes");

        assert_eq!(tool["name"], "read_article");
        assert_eq!(tool["inputSchema"]["properties"]["maxChars"]["maximum"], 9_000);
    }

    #[test]
    fn formats_search_summary() {
        let summary = search_summary(&[SearchResult {
            title: "Example".to_string(),
            url: "https://example.com/post".to_string(),
            snippet: Some("A short result.".to_string()),
        }]);

        assert!(summary.contains("1. Example"));
        assert!(summary.contains("https://example.com/post"));
        assert!(summary.contains("A short result."));
    }

    #[test]
    fn chunks_text_by_character_offset() {
        let (content, next_offset, truncated) = chunk_chars("ab😀de", 2, 2);

        assert_eq!(content, "😀d");
        assert_eq!(next_offset, Some(4));
        assert!(truncated);
    }

    #[test]
    fn chunk_past_end_returns_empty_result() {
        let (content, next_offset, truncated) = chunk_chars("abc", 3, 2);

        assert_eq!(content, "");
        assert_eq!(next_offset, None);
        assert!(!truncated);
    }
}
