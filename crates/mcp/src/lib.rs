//! MCP server library for Lectito article-reading tools.

use std::env;

use ddg::{DuckDuckGoSearch, SearchResult};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub mod ddg;

const PROTOCOL_VERSION: &str = "2025-06-18";
const DEFAULT_SEARCH_LIMIT: usize = 5;
const MAX_SEARCH_LIMIT: usize = 10;

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
    Error { error: String },
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
}

impl Config {
    /// Build configuration from environment variables.
    pub fn from_env() -> Self {
        let max_search_limit = env_usize("LECTITO_MCP_MAX_SEARCH_RESULTS", MAX_SEARCH_LIMIT).max(1);
        Self {
            default_search_limit: env_usize("LECTITO_MCP_DEFAULT_SEARCH_RESULTS", DEFAULT_SEARCH_LIMIT)
                .clamp(1, max_search_limit),
            max_search_limit,
        }
    }
}

/// Stdio MCP server for article-reading tools.
pub struct Server {
    search: DuckDuckGoSearch,
    config: Config,
}

impl Server {
    /// Create a server from a search client and runtime configuration.
    pub fn new(search: DuckDuckGoSearch, config: Config) -> Self {
        Self { search, config }
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
                tools: vec![self.search_articles_tool()],
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

    async fn call_tool(&self, params: Value) -> Result<ToolResult, RpcError> {
        let params = serde_json::from_value::<ToolCallParams>(params)
            .map_err(|error| RpcError::invalid_params(format!("invalid tool call params: {error}")))?;

        match params.name.as_str() {
            "search_articles" => self.search_articles(params.arguments).await,
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
}

fn env_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_tool_schema_uses_configured_limit() {
        let search = DuckDuckGoSearch::new().expect("search client initializes");
        let server = Server::new(search, Config { default_search_limit: 3, max_search_limit: 7 });

        let tool = serde_json::to_value(server.search_articles_tool()).expect("tool serializes");

        assert_eq!(tool["name"], "search_articles");
        assert_eq!(tool["inputSchema"]["properties"]["limit"]["maximum"], 7);
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
}
