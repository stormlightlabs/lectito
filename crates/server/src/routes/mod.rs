use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::{Extension, Json, Path as AxumPath, Query, Request, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, delete, get, post};
use lectito_core::article::Article;
use lectito_core::formatters::{
    JsonConfig, MarkdownConfig, TextConfig, convert_to_json, convert_to_markdown, convert_to_text,
};
use lectito_core::parse::Document;
use lectito_core::{FetchConfig, PostProcessConfig, Readability, ReadabilityConfig, fetch_url, postprocess_html};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tracing::{info, warn};
use uuid::Uuid;

use crate::AppState;
use crate::cache::{self, CachedExtractedArticle, CachedFormat, CachedMetadata};
use crate::db;
use crate::error::AppError;
use crate::rate_limit::{self, ClientRateLimitContext, LimitsResponse};

const CACHE_UPSERT_SQL: &str = "INSERT INTO extracted_articles
    (id, url, url_hash, format, content, metadata, fetched_at, expires_at, hit_count)
 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0)
 ON CONFLICT (url_hash, format)
 DO UPDATE SET
    url = EXCLUDED.url,
    content = EXCLUDED.content,
    metadata = EXCLUDED.metadata,
    fetched_at = EXCLUDED.fetched_at,
    expires_at = EXCLUDED.expires_at";

pub fn build_app(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .route("/extract", get(extract_get).post(extract_post))
        .route("/limits", get(limits))
        .route("/admin/block-domain", post(admin_block_domain))
        .route("/admin/block-domain/{domain}", delete(admin_delete_block_domain))
        .route("/admin/ban-ip", post(admin_ban_ip))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit::middleware));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers(Any)
        .expose_headers([
            HeaderName::from_static("x-ratelimit-limit"),
            HeaderName::from_static("x-ratelimit-remaining"),
            HeaderName::from_static("x-ratelimit-reset"),
            HeaderName::from_static("retry-after"),
        ]);

    Router::new()
        .nest("/api/v1", api)
        .route("/api/{*path}", any(api_not_found))
        .fallback(get(serve_spa))
        .layer(middleware::from_fn(set_static_cache_headers))
        .layer(
            ServiceBuilder::new()
                .layer(CompressionLayer::new())
                .layer(cors)
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::GATEWAY_TIMEOUT,
                    Duration::from_secs(state.config.request_timeout_secs),
                )),
        )
        .with_state(state)
}

#[derive(Serialize)]
struct HealthResponse<'a> {
    status: &'a str,
    version: &'a str,
    database: &'a str,
}

#[derive(Debug, Clone, Deserialize)]
struct ExtractRequest {
    url: String,
    #[serde(default)]
    format: Option<CachedFormat>,
    #[serde(default)]
    include_frontmatter: Option<bool>,
    #[serde(default)]
    include_references: Option<bool>,
    #[serde(default)]
    strip_images: Option<bool>,
    #[serde(default)]
    content_selector: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ExtractResponse {
    content: String,
    metadata: CachedMetadata,
    cached: bool,
    extracted_at: String,
}

#[derive(Debug, Deserialize)]
struct BlockDomainRequest {
    domain: String,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BanIpRequest {
    ip: std::net::IpAddr,
    reason: String,
    duration_hours: u64,
}

#[derive(Debug, Serialize)]
struct AdminStatusResponse<'a> {
    status: &'a str,
}

struct ExtractedArticle {
    article: Article,
    metadata: CachedMetadata,
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    match db::ping(&state.pool).await {
        Ok(()) => (
            StatusCode::OK,
            Json(HealthResponse { status: "ok", version: state.version, database: "ok" }),
        ),
        Err(message) => {
            warn!("health check failed: {message}");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(HealthResponse { status: "degraded", version: state.version, database: "unreachable" }),
            )
        }
    }
}

async fn api_not_found() -> AppError {
    AppError::NotFound("API route not found".to_string())
}

async fn extract_get(
    State(state): State<AppState>, headers: HeaderMap, Query(input): Query<ExtractRequest>,
) -> Result<Json<ExtractResponse>, AppError> {
    handle_extract(state, headers, input).await.map(Json)
}

async fn extract_post(
    State(state): State<AppState>, headers: HeaderMap, Json(input): Json<ExtractRequest>,
) -> Result<Json<ExtractResponse>, AppError> {
    handle_extract(state, headers, input).await.map(Json)
}

async fn limits(Extension(context): Extension<ClientRateLimitContext>) -> Json<LimitsResponse> {
    Json(context.snapshot.as_limits_response())
}

async fn admin_block_domain(
    State(state): State<AppState>, headers: HeaderMap, Json(input): Json<BlockDomainRequest>,
) -> Result<Json<AdminStatusResponse<'static>>, AppError> {
    state.spam_filter.require_admin(&headers)?;
    state
        .spam_filter
        .insert_blocked_domain(&state.pool, &input.domain, input.reason.as_deref())
        .await?;
    Ok(Json(AdminStatusResponse { status: "ok" }))
}

async fn admin_delete_block_domain(
    State(state): State<AppState>, headers: HeaderMap, AxumPath(domain): AxumPath<String>,
) -> Result<Json<AdminStatusResponse<'static>>, AppError> {
    state.spam_filter.require_admin(&headers)?;
    state.spam_filter.delete_blocked_domain(&state.pool, &domain).await?;
    Ok(Json(AdminStatusResponse { status: "ok" }))
}

async fn admin_ban_ip(
    State(state): State<AppState>, headers: HeaderMap, Json(input): Json<BanIpRequest>,
) -> Result<Json<AdminStatusResponse<'static>>, AppError> {
    state.spam_filter.require_admin(&headers)?;
    state
        .spam_filter
        .insert_ip_ban(&state.pool, input.ip, &input.reason, input.duration_hours)
        .await?;
    Ok(Json(AdminStatusResponse { status: "ok" }))
}

async fn serve_spa(State(state): State<AppState>, uri: Uri) -> Result<Response<Body>, AppError> {
    let requested = sanitize_path(uri.path())?;
    let asset_like = has_file_extension(&requested);
    let target = resolve_static_target(&state.config.web_dir, &requested, asset_like).await?;
    let bytes = tokio::fs::read(&target)
        .await
        .map_err(|err| map_static_read_error(&target, &err))?;

    let mime = content_type_for_path(&target);
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(mime));
    Ok(response)
}

async fn set_static_cache_headers(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_string();
    let mut response = next.run(request).await;

    if path.starts_with("/api/") {
        return response;
    }

    let cache_control = cache_control_for_path(&path);
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static(cache_control));
    response
}

async fn handle_extract(
    state: AppState, headers: HeaderMap, input: ExtractRequest,
) -> Result<ExtractResponse, AppError> {
    validate_extract_options(&input)?;

    let parsed_url = state.spam_filter.validate_extract_url(&state.pool, &input.url).await?;
    let format = input.format.unwrap_or(CachedFormat::Markdown);
    let normalized_url = cache::normalize_url(parsed_url.as_str())?;
    let cache_key = cache::build_cache_key(&normalized_url, format);
    let bypass_cache = !is_cacheable_request(&input) || cache::should_bypass_cache(&headers);

    if !bypass_cache && let Some(cached) = read_cached_article(&state, &cache_key, format).await {
        return Ok(ExtractResponse {
            content: cached.content,
            metadata: cached.metadata,
            cached: true,
            extracted_at: cached
                .fetched_at
                .format(&Rfc3339)
                .unwrap_or_else(|_| cached.fetched_at.to_string()),
        });
    }

    let extracted = fetch_and_extract_article(&parsed_url, &input, state.config.fetch_timeout_secs).await?;
    let fetched_at = OffsetDateTime::now_utc();
    let content = render_article(&extracted.article, format, &input)?;

    let response = ExtractResponse {
        content: content.clone(),
        metadata: extracted.metadata.clone(),
        cached: false,
        extracted_at: fetched_at.format(&Rfc3339).unwrap_or_else(|_| fetched_at.to_string()),
    };

    if is_cacheable_request(&input) {
        write_cached_article(
            &state,
            CachedExtractedArticle {
                id: Uuid::new_v4(),
                url: normalized_url,
                format,
                content,
                metadata: extracted.metadata,
                fetched_at,
            },
            cache_key,
        )
        .await;
    }

    Ok(response)
}

async fn fetch_and_extract_article(
    parsed_url: &url::Url, input: &ExtractRequest, fetch_timeout_secs: u64,
) -> Result<ExtractedArticle, AppError> {
    let fetch_config = FetchConfig { timeout: fetch_timeout_secs, ..Default::default() };
    let html = fetch_url(parsed_url.as_str(), &fetch_config).await?;

    let article = if let Some(selector) = input.content_selector.as_deref() {
        extract_with_selector(&html, parsed_url, selector)?
    } else {
        let readability_config = ReadabilityConfig::builder()
            .preserve_images(!input.strip_images.unwrap_or(false))
            .build();
        let reader = Readability::with_config(readability_config);
        reader.parse_with_url(&html, parsed_url.as_str())?
    };

    let metadata_doc = Document::parse_with_base_url(&html, Some(parsed_url.clone())).map_err(AppError::from)?;
    let metadata = CachedMetadata::from_article(
        &article,
        extract_image(&metadata_doc, parsed_url),
        extract_favicon(&metadata_doc, parsed_url),
    );

    Ok(ExtractedArticle { article, metadata })
}

fn extract_with_selector(html: &str, parsed_url: &url::Url, selector: &str) -> Result<Article, AppError> {
    let document = Document::parse_with_base_url(html, Some(parsed_url.clone())).map_err(AppError::from)?;
    let selected = document
        .select(selector)
        .map_err(AppError::from)?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::BadRequest("content_selector did not match any element".to_string()))?;

    Ok(Article::new(
        selected.outer_html(),
        document.extract_metadata(),
        Some(parsed_url.to_string()),
    ))
}

async fn resolve_static_target(web_dir: &Path, requested: &Path, asset_like: bool) -> Result<PathBuf, AppError> {
    if requested.as_os_str().is_empty() {
        return Ok(web_dir.join("index.html"));
    }

    let candidate = web_dir.join(requested);
    match tokio::fs::metadata(&candidate).await {
        Ok(metadata) if metadata.is_file() => Ok(candidate),
        Ok(_) if !asset_like => Ok(web_dir.join("index.html")),
        Ok(_) => Err(AppError::NotFound("Static asset not found".to_string())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound && !asset_like => Ok(web_dir.join("index.html")),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Err(AppError::NotFound("Static asset not found".to_string()))
        }
        Err(err) => Err(AppError::Internal(format!(
            "Failed to read static file metadata: {err}"
        ))),
    }
}

fn sanitize_path(raw_path: &str) -> Result<PathBuf, AppError> {
    let trimmed = raw_path.trim_start_matches('/');
    let mut clean = PathBuf::new();

    for component in Path::new(trimmed).components() {
        match component {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(AppError::NotFound("Static asset not found".to_string()));
            }
        }
    }

    Ok(clean)
}

fn has_file_extension(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.contains('.'))
}

fn map_static_read_error(path: &Path, err: &std::io::Error) -> AppError {
    match err.kind() {
        std::io::ErrorKind::NotFound => AppError::NotFound(format!("Static asset not found: {}", path.display())),
        _ => AppError::Internal(format!("Failed to serve static file {}: {err}", path.display())),
    }
}

fn cache_control_for_path(path: &str) -> &'static str {
    if path == "/" || !path.rsplit('/').next().is_some_and(|segment| segment.contains('.')) {
        "no-cache"
    } else {
        "public, max-age=31536000, immutable"
    }
}

fn content_type_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()).unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "txt" => "text/plain; charset=utf-8",
        "map" => "application/json",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

async fn read_cached_article(
    state: &AppState, cache_key: &[u8], format: CachedFormat,
) -> Option<CachedExtractedArticle> {
    let client = match state.pool.get().await {
        Ok(client) => client,
        Err(err) => {
            warn!("cache read skipped because DB connection failed: {err}");
            return None;
        }
    };

    match client
        .query_opt(
            "UPDATE extracted_articles
             SET hit_count = hit_count + 1
             WHERE url_hash = $1 AND format = $2 AND expires_at > now()
             RETURNING id, url, format, content, metadata, fetched_at",
            &[&cache_key, &format.as_str()],
        )
        .await
    {
        Ok(Some(row)) => Some(CachedExtractedArticle {
            id: row.get("id"),
            url: row.get("url"),
            format: parse_cached_format(row.get::<_, String>("format").as_str()).unwrap_or(format),
            content: row.get("content"),
            metadata: serde_json::from_value(row.get("metadata")).unwrap_or_default(),
            fetched_at: row.get("fetched_at"),
        }),
        Ok(None) => None,
        Err(err) => {
            warn!("cache read failed: {err}");
            None
        }
    }
}

async fn write_cached_article(state: &AppState, article: CachedExtractedArticle, cache_key: Vec<u8>) {
    let Some(client) = acquire_cache_client(state).await else {
        return;
    };

    log_cache_write_result(
        execute_cache_upsert(&client, state, &article, &cache_key).await,
        &article.url,
    );
}

async fn acquire_cache_client(state: &AppState) -> Option<deadpool_postgres::Client> {
    match state.pool.get().await {
        Ok(client) => Some(client),
        Err(err) => {
            warn!("cache write skipped because DB connection failed: {err}");
            None
        }
    }
}

async fn execute_cache_upsert(
    client: &deadpool_postgres::Client, state: &AppState, article: &CachedExtractedArticle, cache_key: &[u8],
) -> Result<u64, tokio_postgres::Error> {
    let expires_at = article.fetched_at + time::Duration::seconds(state.config.cache_ttl_secs as i64);
    let metadata_json = serde_json::to_value(&article.metadata).unwrap_or_default();
    let params: [&(dyn tokio_postgres::types::ToSql + Sync); 8] = [
        &article.id,
        &article.url,
        &cache_key,
        &article.format.as_str(),
        &article.content,
        &metadata_json,
        &article.fetched_at,
        &expires_at,
    ];

    client.execute(CACHE_UPSERT_SQL, &params).await
}

fn log_cache_write_result(result: Result<u64, tokio_postgres::Error>, url: &str) {
    match result {
        Ok(_) => info!("cached extracted article for {url}"),
        Err(err) => warn!("cache write failed: {err}"),
    }
}

fn parse_cached_format(value: &str) -> Option<CachedFormat> {
    match value {
        "html" => Some(CachedFormat::Html),
        "markdown" => Some(CachedFormat::Markdown),
        "text" => Some(CachedFormat::Text),
        "json" => Some(CachedFormat::Json),
        _ => None,
    }
}

fn validate_extract_options(input: &ExtractRequest) -> Result<(), AppError> {
    if input.url.trim().is_empty() {
        return Err(AppError::BadRequest("url is required".to_string()));
    }

    if let Some(selector) = &input.content_selector
        && selector.trim().is_empty()
    {
        return Err(AppError::BadRequest("content_selector must not be empty".to_string()));
    }

    Ok(())
}

fn is_cacheable_request(input: &ExtractRequest) -> bool {
    !input.include_frontmatter.unwrap_or(false)
        && !input.include_references.unwrap_or(false)
        && !input.strip_images.unwrap_or(false)
        && input.content_selector.is_none()
}

fn render_article(article: &Article, format: CachedFormat, input: &ExtractRequest) -> Result<String, AppError> {
    let html_content = output_html(article, input);

    match format {
        CachedFormat::Html => Ok(html_content),
        CachedFormat::Markdown => convert_to_markdown(
            &html_content,
            &article.metadata,
            &MarkdownConfig {
                include_frontmatter: input.include_frontmatter.unwrap_or(false),
                include_references: input.include_references.unwrap_or(false),
                strip_images: false,
                include_title_heading: false,
            },
        )
        .map_err(AppError::from),
        CachedFormat::Text => {
            convert_to_text(&html_content, &article.metadata, &TextConfig::default()).map_err(AppError::from)
        }
        CachedFormat::Json => {
            let markdown = convert_to_markdown(
                &html_content,
                &article.metadata,
                &MarkdownConfig {
                    include_frontmatter: input.include_frontmatter.unwrap_or(false),
                    include_references: input.include_references.unwrap_or(false),
                    strip_images: false,
                    include_title_heading: false,
                },
            )
            .ok();

            convert_to_json(
                &html_content,
                &article.metadata,
                &JsonConfig {
                    include_markdown: markdown.is_some(),
                    include_text: true,
                    include_html: true,
                    include_references: input.include_references.unwrap_or(false),
                    pretty: false,
                },
                markdown.as_deref(),
            )
            .map_err(AppError::from)
        }
    }
}

fn output_html(article: &Article, input: &ExtractRequest) -> String {
    if input.strip_images.unwrap_or(false) {
        postprocess_html(
            &article.content,
            &PostProcessConfig { strip_images: true, ..Default::default() },
        )
    } else {
        article.content.clone()
    }
}

impl CachedMetadata {
    fn from_article(article: &Article, image: Option<String>, favicon: Option<String>) -> Self {
        Self {
            title: article.metadata.title.clone(),
            author: article.metadata.author.clone(),
            date: article.metadata.date.clone(),
            excerpt: article.metadata.excerpt.clone(),
            site_name: article.metadata.site_name.clone(),
            language: article.metadata.language.clone(),
            word_count: article.metadata.word_count.or(Some(article.word_count)),
            reading_time_minutes: article.metadata.reading_time_minutes.or(Some(article.reading_time)),
            image,
            favicon,
        }
    }
}

fn extract_image(document: &Document, parsed_url: &url::Url) -> Option<String> {
    meta_content(document, "property", "og:image")
        .or_else(|| meta_content(document, "name", "twitter:image"))
        .or_else(|| extract_json_ld_image(document))
        .and_then(|value| absolutize_url(parsed_url, &value))
}

fn extract_favicon(document: &Document, parsed_url: &url::Url) -> Option<String> {
    if let Ok(elements) = document.select("link[rel~=\"icon\" i], link[rel=\"shortcut icon\" i]") {
        for element in elements {
            if let Some(href) = element.attr("href")
                && let Some(url) = absolutize_url(parsed_url, href)
            {
                return Some(url);
            }
        }
    }

    parsed_url.join("/favicon.ico").ok().map(|url| url.to_string())
}

fn meta_content(document: &Document, attr_name: &str, attr_value: &str) -> Option<String> {
    let selector = format!("meta[{attr_name}=\"{attr_value}\"]");
    document
        .select(&selector)
        .ok()?
        .into_iter()
        .find_map(|element| element.attr("content").map(ToOwned::to_owned))
}

fn extract_json_ld_image(document: &Document) -> Option<String> {
    let elements = document.select("script[type=\"application/ld+json\"]").ok()?;
    for element in elements {
        let text = element.text();
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text.trim())
            && let Some(url) = image_value_to_string(&json)
        {
            return Some(url);
        }
    }
    None
}

fn image_value_to_string(value: &serde_json::Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }

    if let Some(obj) = value.as_object()
        && let Some(url) = obj.get("url").and_then(serde_json::Value::as_str)
    {
        return Some(url.to_string());
    }

    if let Some(items) = value.as_array() {
        for item in items {
            if let Some(url) = image_value_to_string(item) {
                return Some(url);
            }
        }
    }

    None
}

fn absolutize_url(base_url: &url::Url, value: &str) -> Option<String> {
    url::Url::parse(value)
        .ok()
        .or_else(|| base_url.join(value).ok())
        .map(|url| url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_directory_escape() {
        let error = sanitize_path("/../../secret").unwrap_err();
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn cache_control_for_routes_is_no_cache() {
        assert_eq!(cache_control_for_path("/"), "no-cache");
        assert_eq!(cache_control_for_path("/reader/example"), "no-cache");
    }

    #[test]
    fn cache_control_for_assets_is_immutable() {
        assert_eq!(
            cache_control_for_path("/assets/app.12345.js"),
            "public, max-age=31536000, immutable"
        );
    }

    #[test]
    fn infers_common_content_types() {
        assert_eq!(
            content_type_for_path(Path::new("index.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(content_type_for_path(Path::new("app.css")), "text/css; charset=utf-8");
        assert_eq!(content_type_for_path(Path::new("logo.svg")), "image/svg+xml");
    }

    #[test]
    fn rejects_empty_content_selector() {
        let error = validate_extract_options(&ExtractRequest {
            url: "https://example.com".to_string(),
            format: Some(CachedFormat::Markdown),
            include_frontmatter: None,
            include_references: None,
            strip_images: None,
            content_selector: Some("   ".to_string()),
        })
        .unwrap_err();

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn advanced_extract_requests_skip_cache() {
        let request = ExtractRequest {
            url: "https://example.com".to_string(),
            format: Some(CachedFormat::Markdown),
            include_frontmatter: Some(true),
            include_references: None,
            strip_images: None,
            content_selector: None,
        };

        assert!(!is_cacheable_request(&request));
    }
}
