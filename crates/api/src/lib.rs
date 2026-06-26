use std::env;
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use lectito::MarkdownOptions;
use reqwest::redirect::Policy;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::{Level, Span};
use url::Url;
use utoipa::OpenApi;

#[cfg(test)]
mod tests;

mod error;
mod models;

use error::{ApiError, ErrorCode, Json};
use models::{
    ArticleDto, ErrorResponse, EvaluateRequest, EvaluateResponse, ExtractRequest, ExtractResponse, HealthResponse,
    MarkdownOptionsDto, ReadabilityOptionsDto, ReadableOptionsDto, TransformRequest, TransformResponse,
};

#[derive(Clone, Copy)]
enum Limit {
    MaxBodyBytes,
    MaxFetchBytes,
    Redirect,
    RequestTimeoutSecs,
}

impl Limit {
    fn env_var(self) -> &'static str {
        match self {
            Self::MaxBodyBytes => "LECTITO_MAX_BODY_BYTES",
            Self::MaxFetchBytes => "LECTITO_MAX_FETCH_BYTES",
            Self::Redirect => "LECTITO_REDIRECT_LIMIT",
            Self::RequestTimeoutSecs => "LECTITO_REQUEST_TIMEOUT_SECS",
        }
    }

    fn from_env_usize(self) -> usize {
        env::var(self.env_var())
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| self.into())
    }

    fn from_env_u64(self) -> u64 {
        env::var(self.env_var())
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| self.into())
    }

    fn env_u16(name: &str, default: u16) -> u16 {
        env::var(name)
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(default)
    }

    fn env_bool(name: &str, default: bool) -> bool {
        env::var(name)
            .ok()
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(default)
    }
}

impl From<Limit> for usize {
    fn from(limit: Limit) -> Self {
        match limit {
            Limit::MaxBodyBytes => 512 * 1024,
            Limit::MaxFetchBytes => 2 * 1024 * 1024,
            Limit::Redirect => 5,
            Limit::RequestTimeoutSecs => 20,
        }
    }
}

impl From<Limit> for u64 {
    fn from(limit: Limit) -> Self {
        usize::from(limit) as u64
    }
}

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
        tracing::info!(url = %url, "fetching document");

        for redirect_count in 0..=self.config.redirect_limit {
            if !self.config.allow_private_network {
                reject_private_target(&url).await?;
            }

            let response = self.client.get(url.clone()).send().await.map_err(|err| {
                tracing::warn!(url = %url, error = %err, "upstream request failed");
                ApiError::fetch_failed(err)
            })?;

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
                tracing::info!(redirected_to = %url, hop = redirect_count + 1, "following redirect");
                continue;
            }

            if !response.status().is_success() {
                tracing::warn!(url = %url, status = %response.status(), "upstream returned non-success");

                return Err(ApiError::fetch_failed(format!(
                    "upstream returned {}",
                    response.status()
                )));
            }

            ensure_html_content_type(response.headers())?;
            let bytes = response.bytes().await.map_err(|err| {
                tracing::warn!(url = %url, error = %err, "failed to read upstream body");
                ApiError::fetch_failed(err)
            })?;

            if bytes.len() > self.config.max_fetch_bytes {
                return Err(ApiError::new(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    ErrorCode::DocumentTooLarge,
                    "fetched document is too large",
                ));
            }

            tracing::info!(url = %url, bytes = bytes.len(), "fetched document");

            let html = String::from_utf8_lossy(&bytes).into_owned();
            return Ok(FetchedDocument { final_url: url, html });
        }

        Err(ApiError::invalid_request("redirect limit exceeded"))
    }
}

struct FetchedDocument {
    final_url: Url,
    html: String,
}

#[derive(OpenApi)]
#[openapi(
    paths(healthz, extract, evaluate, transform),
    components(schemas(
        ArticleDto,
        ErrorResponse,
        EvaluateRequest,
        EvaluateResponse,
        ExtractRequest,
        ExtractResponse,
        HealthResponse,
        MarkdownOptionsDto,
        ReadabilityOptionsDto,
        ReadableOptionsDto,
        TransformRequest,
        TransformResponse
    ))
)]
struct ApiDoc;

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
            port: Limit::env_u16("PORT", 3000),
            max_body_bytes: Limit::MaxBodyBytes.from_env_usize(),
            max_fetch_bytes: Limit::MaxFetchBytes.from_env_usize(),
            redirect_limit: Limit::Redirect.from_env_usize(),
            request_timeout_secs: Limit::RequestTimeoutSecs.from_env_u64(),
            allowed_origins: env::var("LECTITO_ALLOWED_ORIGINS")
                .ok()
                .map(|value| split_csv(&value))
                .unwrap_or_default(),
            allow_private_network: Limit::env_bool("LECTITO_ALLOW_PRIVATE_NETWORK", false),
        }
    }
}

pub fn app(config: Config) -> Router {
    let state = AppState::new(config.clone());
    let timeout = Duration::from_secs(config.request_timeout_secs);

    Router::new()
        .route("/healthz", get(healthz))
        .route("/openapi.json", get(openapi))
        .route("/v1/extract", post(extract))
        .route("/v1/evaluate", post(evaluate))
        .route("/v1/transform", post(transform))
        .with_state(state)
        .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, timeout))
        .layer(RequestBodyLimitLayer::new(config.max_body_bytes))
        .layer(middleware::from_fn(normalize_errors))
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

#[utoipa::path(
    get,
    path = "/healthz",
    responses((status = 200, description = "Health check", body = HealthResponse))
)]
async fn healthz() -> axum::Json<HealthResponse> {
    axum::Json(HealthResponse { ok: true })
}

#[utoipa::path(
    post,
    path = "/v1/extract",
    request_body = ExtractRequest,
    responses(
        (status = 200, description = "Extracted article", body = ExtractResponse),
        (status = 400, description = "Structured API error", body = ErrorResponse),
        (status = 422, description = "Request body failed to deserialize", body = ErrorResponse),
        (status = 413, description = "Request body too large", body = ErrorResponse),
        (status = 408, description = "Request timed out", body = ErrorResponse)
    )
)]
async fn extract(
    State(state): State<AppState>, Json(request): Json<ExtractRequest>,
) -> Result<axum::Json<ExtractResponse>, ApiError> {
    let started = Instant::now();
    let diagnostics = request.diagnostics;
    let options = request.options.unwrap_or_default().into_options();
    let fetched = state.fetch_url(&request.url).await?;
    let report = lectito::extract_with_diagnostics(&fetched.html, Some(fetched.final_url.as_str()), &options).map_err(
        |err| {
            tracing::warn!(url = %fetched.final_url, error = %err, "extraction failed");
            ApiError::core(ErrorCode::ExtractFailed, err)
        },
    )?;

    let content_length = report.article.as_ref().map(|a| a.length).unwrap_or(0);
    let article = report.article.map(ArticleDto::from);

    tracing::info!(
        url = %fetched.final_url,
        found_article = article.is_some(),
        content_length,
        elapsed_ms = started.elapsed().as_millis(),
        "extract complete"
    );

    Ok(axum::Json(ExtractResponse {
        article,
        diagnostics: diagnostics.then(|| serde_json::to_value(report.diagnostics).unwrap_or_default()),
        elapsed_ms: started.elapsed().as_millis(),
    }))
}

#[utoipa::path(
    post,
    path = "/v1/evaluate",
    request_body = EvaluateRequest,
    responses(
        (status = 200, description = "Readability result", body = EvaluateResponse),
        (status = 400, description = "Structured API error", body = ErrorResponse),
        (status = 422, description = "Request body failed to deserialize", body = ErrorResponse),
        (status = 413, description = "Request body too large", body = ErrorResponse),
        (status = 408, description = "Request timed out", body = ErrorResponse)
    )
)]
async fn evaluate(
    State(state): State<AppState>, Json(request): Json<EvaluateRequest>,
) -> Result<axum::Json<EvaluateResponse>, ApiError> {
    let started = Instant::now();
    let options = request.options.unwrap_or_default().into_options();
    let fetched = state.fetch_url(&request.url).await?;
    let readable = lectito::is_probably_readable(&fetched.html, &options).map_err(|err| {
        tracing::warn!(url = %fetched.final_url, error = %err, "readability check failed");
        ApiError::core(ErrorCode::ExtractFailed, err)
    })?;

    tracing::info!(
        url = %fetched.final_url,
        readable,
        elapsed_ms = started.elapsed().as_millis(),
        "evaluate complete"
    );
    Ok(axum::Json(EvaluateResponse { readable }))
}

#[utoipa::path(
    post,
    path = "/v1/transform",
    request_body = TransformRequest,
    responses(
        (status = 200, description = "Markdown result", body = TransformResponse),
        (status = 422, description = "Request body failed to deserialize", body = ErrorResponse),
        (status = 413, description = "Request body too large", body = ErrorResponse),
        (status = 408, description = "Request timed out", body = ErrorResponse)
    )
)]
async fn transform(headers: HeaderMap, Json(request): Json<TransformRequest>) -> Result<Response, ApiError> {
    let started = Instant::now();
    let _options: MarkdownOptions = request.options.unwrap_or_default().into();
    let input_bytes = request.html.len();
    let markdown = lectito::html_to_markdown(&request.html);
    tracing::info!(
        input_bytes,
        output_bytes = markdown.len(),
        elapsed_ms = started.elapsed().as_millis(),
        "transform complete"
    );

    if accepts_markdown(&headers) {
        Ok(([(header::CONTENT_TYPE, "text/markdown; charset=utf-8")], markdown).into_response())
    } else {
        Ok(axum::Json(TransformResponse { markdown }).into_response())
    }
}

/// Response-normalizing middleware: rewrites the plain-text error bodies
/// produced by `tower-http` layers into the structured `ApiError` shape.
/// - `RequestBodyLimitLayer` → 413
/// - `TimeoutLayer` → 408
/// - any other 5xx without `x-error-code` → `internal_error`
///
/// Responses already produced by `ApiError` carry `x-error-code` and pass
/// through untouched.
///
/// Positioned outside the limit/timeout layers but inside `TraceLayer` so
/// tracing records the final `error_code`.
async fn normalize_errors(req: Request, next: Next) -> Response {
    let response = next.run(req).await;

    if response.headers().contains_key("x-error-code") {
        return response;
    }

    match response.status() {
        StatusCode::PAYLOAD_TOO_LARGE => ApiError::body_too_large().into_response(),
        StatusCode::REQUEST_TIMEOUT => ApiError::timeout().into_response(),
        status if status.is_server_error() => {
            tracing::warn!(status = %status, "unhandled server error, normalizing");
            ApiError::internal().into_response()
        }
        _ => response,
    }
}

async fn openapi() -> axum::Json<utoipa::openapi::OpenApi> {
    axum::Json(ApiDoc::openapi())
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

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
