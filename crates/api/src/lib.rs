use std::env;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, Method, Request, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use lectito::MarkdownOptions;
use reqwest::redirect::Policy;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::{Level, Span};
use url::Url;
use utoipa::OpenApi;

mod error;
mod models;

use error::{ApiError, ErrorCode};
use models::{
    ArticleDto, ErrorResponse, ExtractUrlRequest, ExtractUrlResponse, HealthResponse, MarkdownOptionsDto,
    MarkdownRequest, MarkdownResponse, ReadabilityOptionsDto, ReadableOptionsDto, ReadableRequest, ReadableResponse,
};

#[derive(Clone)]
struct AppState {
    client: reqwest::Client,
    config: Config,
}

impl AppState {
    fn new(config: Config) -> Self {
        let client = reqwest::Client::builder()
            .redirect(Policy::none())
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .user_agent("lectito-api/0.1")
            .build()
            .expect("failed to build HTTP client");

        Self { client, config }
    }

    async fn fetch_url(&self, url: &str) -> Result<FetchedDocument, ApiError> {
        let mut url = parse_public_url(url)?;

        for redirect_count in 0..=self.config.redirect_limit {
            if !self.config.allow_private_network {
                reject_private_target(&url).await?;
            }

            let response = self
                .client
                .get(url.clone())
                .send()
                .await
                .map_err(ApiError::fetch_failed)?;

            if response.status().is_redirection() {
                if redirect_count == self.config.redirect_limit {
                    return Err(ApiError::invalid_request("redirect limit exceeded"));
                }
                let location = response
                    .headers()
                    .get(header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or_else(|| ApiError::invalid_request("redirect response missing location"))?;
                url = url
                    .join(location)
                    .map_err(|_| ApiError::invalid_request("invalid redirect location"))?;
                parse_public_url(url.as_str())?;
                continue;
            }

            if !response.status().is_success() {
                return Err(ApiError::fetch_failed(format!(
                    "upstream returned {}",
                    response.status()
                )));
            }

            ensure_html_content_type(response.headers())?;
            let bytes = response.bytes().await.map_err(ApiError::fetch_failed)?;
            if bytes.len() > self.config.max_fetch_bytes {
                return Err(ApiError::new(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    ErrorCode::DocumentTooLarge,
                    "fetched document is too large",
                ));
            }
            let html = String::from_utf8_lossy(&bytes).into_owned();
            return Ok(FetchedDocument { final_url: url, html });
        }

        Err(ApiError::invalid_request("redirect limit exceeded"))
    }
}

#[derive(Clone, Copy)]
enum Limit {
    MaxBodyBytes,
    MaxFetchBytes,
    RedirectLimit,
    RequestTimeoutSecs,
}

impl Limit {
    fn env_var(self) -> &'static str {
        match self {
            Self::MaxBodyBytes => "LECTITO_MAX_BODY_BYTES",
            Self::MaxFetchBytes => "LECTITO_MAX_FETCH_BYTES",
            Self::RedirectLimit => "LECTITO_REDIRECT_LIMIT",
            Self::RequestTimeoutSecs => "LECTITO_REQUEST_TIMEOUT_SECS",
        }
    }
}

impl From<Limit> for usize {
    fn from(limit: Limit) -> Self {
        match limit {
            Limit::MaxBodyBytes => 512 * 1024,
            Limit::MaxFetchBytes => 2 * 1024 * 1024,
            Limit::RedirectLimit => 5,
            Limit::RequestTimeoutSecs => 20,
        }
    }
}

impl From<Limit> for u64 {
    fn from(limit: Limit) -> Self {
        usize::from(limit) as u64
    }
}

struct FetchedDocument {
    final_url: Url,
    html: String,
}

#[derive(Clone)]
pub struct Config {
    pub port: u16,
    max_body_bytes: usize,
    max_fetch_bytes: usize,
    redirect_limit: usize,
    request_timeout_secs: u64,
    allowed_origins: Vec<String>,
    allow_private_network: bool,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: env_u16("PORT", 3000),
            max_body_bytes: env_usize(Limit::MaxBodyBytes),
            max_fetch_bytes: env_usize(Limit::MaxFetchBytes),
            redirect_limit: env_usize(Limit::RedirectLimit),
            request_timeout_secs: env_u64(Limit::RequestTimeoutSecs),
            allowed_origins: env::var("LECTITO_ALLOWED_ORIGINS")
                .ok()
                .map(|value| split_csv(&value))
                .unwrap_or_default(),
            allow_private_network: env_bool("LECTITO_ALLOW_PRIVATE_NETWORK", false),
        }
    }
}

pub fn app(config: Config) -> Router {
    let state = AppState::new(config.clone());
    let timeout = Duration::from_secs(config.request_timeout_secs);

    Router::new()
        .route("/healthz", get(healthz))
        .route("/openapi.json", get(openapi))
        .route("/v1/extract-url", post(extract_url))
        .route("/v1/readable", post(readable))
        .route("/v1/markdown", post(markdown))
        .with_state(state)
        .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, timeout))
        .layer(RequestBodyLimitLayer::new(config.max_body_bytes))
        .layer(cors_layer(&config))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<Body>| {
                    tracing::span!(
                        Level::INFO,
                        "request",
                        method = %request.method(),
                        path = %request.uri().path(),
                        status = tracing::field::Empty,
                        elapsed_ms = tracing::field::Empty,
                        error_code = tracing::field::Empty,
                    )
                })
                .on_request(|_request: &Request<Body>, _span: &Span| {})
                .on_response(|response: &Response, latency: Duration, span: &Span| {
                    span.record("status", response.status().as_u16());
                    span.record("elapsed_ms", latency.as_millis());
                    if let Some(error_code) = response
                        .headers()
                        .get("x-error-code")
                        .and_then(|value| value.to_str().ok())
                    {
                        span.record("error_code", error_code);
                    }
                }),
        )
}

fn cors_layer(config: &Config) -> CorsLayer {
    let mut layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT]);

    if config.allowed_origins.is_empty() {
        layer = layer.allow_origin(tower_http::cors::Any);
    } else {
        let origins = config
            .allowed_origins
            .iter()
            .filter_map(|origin| origin.parse::<HeaderValue>().ok())
            .collect::<Vec<_>>();
        layer = layer.allow_origin(origins);
    }

    layer
}

#[utoipa::path(
    get,
    path = "/healthz",
    responses((status = 200, description = "Health check", body = HealthResponse))
)]
async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true })
}

async fn openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[derive(OpenApi)]
#[openapi(
    paths(healthz, extract_url, readable, markdown),
    components(schemas(
        ArticleDto,
        ErrorResponse,
        ExtractUrlRequest,
        ExtractUrlResponse,
        HealthResponse,
        MarkdownOptionsDto,
        MarkdownRequest,
        MarkdownResponse,
        ReadabilityOptionsDto,
        ReadableOptionsDto,
        ReadableRequest,
        ReadableResponse
    ))
)]
struct ApiDoc;

#[utoipa::path(
    post,
    path = "/v1/extract-url",
    request_body = ExtractUrlRequest,
    responses(
        (status = 200, description = "Extracted article", body = ExtractUrlResponse),
        (status = 400, description = "Structured API error", body = ErrorResponse)
    )
)]
async fn extract_url(
    State(state): State<AppState>, Json(request): Json<ExtractUrlRequest>,
) -> Result<Json<ExtractUrlResponse>, ApiError> {
    let started = Instant::now();
    let options = request.options.unwrap_or_default().into_options();
    let fetched = state.fetch_url(&request.url).await?;
    let report = lectito::extract_with_diagnostics(&fetched.html, Some(fetched.final_url.as_str()), &options)
        .map_err(|err| ApiError::core(ErrorCode::ExtractFailed, err))?;

    let article = report.article.map(ArticleDto::from);
    Ok(Json(ExtractUrlResponse {
        article,
        diagnostics: request
            .diagnostics
            .then(|| serde_json::to_value(report.diagnostics).unwrap_or_default()),
        elapsed_ms: started.elapsed().as_millis(),
    }))
}

#[utoipa::path(
    post,
    path = "/v1/readable",
    request_body = ReadableRequest,
    responses(
        (status = 200, description = "Readability result", body = ReadableResponse),
        (status = 400, description = "Structured API error", body = ErrorResponse)
    )
)]
async fn readable(
    State(state): State<AppState>, Json(request): Json<ReadableRequest>,
) -> Result<Json<ReadableResponse>, ApiError> {
    let options = request.options.unwrap_or_default().into_options();
    let fetched = state.fetch_url(&request.url).await?;
    let readable = lectito::is_probably_readable(&fetched.html, &options)
        .map_err(|err| ApiError::core(ErrorCode::ExtractFailed, err))?;
    Ok(Json(ReadableResponse { readable }))
}

#[utoipa::path(
    post,
    path = "/v1/markdown",
    request_body = MarkdownRequest,
    responses(
        (status = 200, description = "Markdown result", body = MarkdownResponse),
        (status = 400, description = "Structured API error", body = ErrorResponse)
    )
)]
async fn markdown(headers: HeaderMap, Json(request): Json<MarkdownRequest>) -> Result<Response, ApiError> {
    let _options: MarkdownOptions = request.options.unwrap_or_default().into();
    let markdown = lectito::html_to_markdown(&request.html);

    if accepts_markdown(&headers) {
        Ok(([(header::CONTENT_TYPE, "text/markdown; charset=utf-8")], markdown).into_response())
    } else {
        Ok(Json(MarkdownResponse { markdown }).into_response())
    }
}

fn accepts_markdown(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .map(|accept| accept.split(',').any(|part| part.trim().starts_with("text/markdown")))
        .unwrap_or(false)
}

fn parse_public_url(value: &str) -> Result<Url, ApiError> {
    let url = Url::parse(value).map_err(|_| ApiError::invalid_request("url must be absolute"))?;
    match url.scheme() {
        "http" | "https" => Ok(url),
        _ => Err(ApiError::invalid_request("url must use http or https")),
    }
}

async fn reject_private_target(url: &Url) -> Result<(), ApiError> {
    let host = url
        .host_str()
        .ok_or_else(|| ApiError::invalid_request("url is missing a host"))?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| ApiError::invalid_request("url is missing a port"))?;
    let addresses = tokio::net::lookup_host((host, port))
        .await
        .map_err(ApiError::fetch_failed)?
        .collect::<Vec<_>>();

    if addresses.is_empty() {
        return Err(ApiError::fetch_failed("host did not resolve"));
    }

    if addresses.iter().any(|addr| is_private_ip(&addr.ip())) {
        return Err(ApiError::invalid_request("private-network targets are not supported"));
    }

    Ok(())
}

fn is_private_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_documentation()
                || ip.is_unspecified()
        }
        std::net::IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || matches!(ip.segments()[0] & 0xfe00, 0xfc00)
                || matches!(ip.segments()[0] & 0xffc0, 0xfe80)
        }
    }
}

fn ensure_html_content_type(headers: &HeaderMap) -> Result<(), ApiError> {
    let Some(content_type) = headers.get(header::CONTENT_TYPE).and_then(|value| value.to_str().ok()) else {
        return Ok(());
    };
    let media_type = content_type.split(';').next().unwrap_or_default().trim();
    match media_type {
        "text/html" | "application/xhtml+xml" => Ok(()),
        _ => Err(ApiError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ErrorCode::UnsupportedContentType,
            "upstream content type is not supported",
        )),
    }
}

// TODO: static method on Limit
fn env_u16(name: &str, default: u16) -> u16 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

// TODO: instance method on Limit
fn env_usize(limit: Limit) -> usize {
    env::var(limit.env_var())
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| limit.into())
}

// TODO: instance method on Limit
fn env_u64(limit: Limit) -> u64 {
    env::var(limit.env_var())
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| limit.into())
}

// TODO: static method on Limit
fn env_bool(name: &str, default: bool) -> bool {
    env::var(name)
        .ok()
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(default)
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use serde_json::{Value, json};
    use tokio::net::TcpListener;
    use tower::ServiceExt;

    #[tokio::test]
    async fn healthz_smoke() {
        let response = app(test_config())
            .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn markdown_returns_json_by_default() {
        let response = app(test_config())
            .oneshot(json_request(
                "/v1/markdown",
                json!({ "html": "<h1>Hello</h1><p>Body</p>" }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["markdown"], "# Hello\n\nBody");
    }

    #[tokio::test]
    async fn markdown_returns_plain_markdown_when_requested() {
        let response = app(test_config())
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/markdown")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::ACCEPT, "text/markdown")
                    .body(Body::from(r#"{ "html": "<h1>Hello</h1><p>Body</p>" }"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(body.as_ref(), b"# Hello\n\nBody");
    }

    #[tokio::test]
    async fn extract_url_smoke() {
        let source = html_server().await;
        let response = app(test_config())
            .oneshot(json_request(
                "/v1/extract-url",
                json!({ "url": source, "options": { "charThreshold": 20 }, "diagnostics": true }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["article"]["title"], "Smoke Article");
        assert!(body["article"]["markdown"].as_str().unwrap().contains("readability"));
        assert!(body["article"]["content"].as_str().unwrap().contains("<"));
        assert!(body["diagnostics"].is_object());
    }

    #[tokio::test]
    async fn readable_smoke() {
        let source = html_server().await;
        let response = app(test_config())
            .oneshot(json_request(
                "/v1/readable",
                json!({ "url": source, "options": { "minContentLength": 20, "minScore": 0.0 } }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["readable"], true);
    }

    async fn html_server() -> String {
        let app = Router::new().route(
            "/article",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                    r#"<!doctype html>
                    <html>
                      <head><title>Smoke Article</title></head>
                      <body>
                        <main>
                          <h1>Smoke Article</h1>
                          <p>This article has enough real text for the readability
                          smoke test to treat it as article content.</p>
                          <p>Another paragraph keeps the extractor on the article
                          body rather than metadata or page chrome.</p>
                        </main>
                      </body>
                    </html>"#,
                )
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://127.0.0.1:{}/article", addr.port())
    }

    fn test_config() -> Config {
        Config {
            port: 0,
            max_body_bytes: Limit::MaxBodyBytes.into(),
            max_fetch_bytes: Limit::MaxFetchBytes.into(),
            redirect_limit: Limit::RedirectLimit.into(),
            request_timeout_secs: Limit::RequestTimeoutSecs.into(),
            allowed_origins: Vec::new(),
            allow_private_network: true,
        }
    }

    fn json_request(uri: &str, body: Value) -> Request<Body> {
        Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    async fn body_json(response: Response) -> Value {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }
}
