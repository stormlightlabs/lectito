use std::path::{Component, Path, PathBuf};

use axum::body::Body;
use axum::extract::{Extension, Query, Request, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::{Json, Router};
use lectito_core::{FetchConfig, Readability};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tracing::{info, warn};
use uuid::Uuid;

use crate::AppState;
use crate::cache::{self, CachedExtractedArticle, CachedFormat, CachedMetadata};
use crate::db;
use crate::error::AppError;
use crate::rate_limit::{self, ClientRateLimitContext, LimitsResponse};

pub fn build_app(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .route("/extract", get(extract_get).post(extract_post))
        .route("/limits", get(limits))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit::middleware));

    Router::new()
        .nest("/api/v1", api)
        .route("/api/{*path}", any(api_not_found))
        .fallback(get(serve_spa))
        .layer(middleware::from_fn(set_static_cache_headers))
        .with_state(state)
}

#[derive(Serialize)]
struct HealthResponse<'a> {
    status: &'a str,
    version: &'a str,
    database: &'a str,
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
    reject_unsupported_extract_options(&input)?;

    let format = input.format.unwrap_or(CachedFormat::Markdown);
    let normalized_url = cache::normalize_url(&input.url)?;
    let cache_key = cache::build_cache_key(&normalized_url, format);
    let bypass_cache = cache::should_bypass_cache(&headers);

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

    let reader = Readability::new();
    let fetch_config = FetchConfig { timeout: state.config.fetch_timeout_secs, ..Default::default() };
    let article = reader.fetch_and_parse_with_config(&input.url, &fetch_config).await?;
    let metadata = CachedMetadata::from_article(&article);
    let fetched_at = OffsetDateTime::now_utc();
    let content = render_article(&article, format)?;

    let response = ExtractResponse {
        content: content.clone(),
        metadata: metadata.clone(),
        cached: false,
        extracted_at: fetched_at.format(&Rfc3339).unwrap_or_else(|_| fetched_at.to_string()),
    };

    write_cached_article(
        &state,
        CachedExtractedArticle { id: Uuid::new_v4(), url: normalized_url, format, content, metadata, fetched_at },
        cache_key,
    )
    .await;

    Ok(response)
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
    let client = match state.pool.get().await {
        Ok(client) => client,
        Err(err) => {
            warn!("cache write skipped because DB connection failed: {err}");
            return;
        }
    };

    let expires_at = article.fetched_at + time::Duration::seconds(state.config.cache_ttl_secs as i64);
    if let Err(err) = client
        .execute(
            "INSERT INTO extracted_articles
                (id, url, url_hash, format, content, metadata, fetched_at, expires_at, hit_count)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0)
             ON CONFLICT (url_hash, format)
             DO UPDATE SET
                url = EXCLUDED.url,
                content = EXCLUDED.content,
                metadata = EXCLUDED.metadata,
                fetched_at = EXCLUDED.fetched_at,
                expires_at = EXCLUDED.expires_at",
            &[
                &article.id,
                &article.url,
                &cache_key,
                &article.format.as_str(),
                &article.content,
                &serde_json::to_value(&article.metadata).unwrap_or_default(),
                &article.fetched_at,
                &expires_at,
            ],
        )
        .await
    {
        warn!("cache write failed: {err}");
    } else {
        info!("cached extracted article for {}", article.url);
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

fn reject_unsupported_extract_options(input: &ExtractRequest) -> Result<(), AppError> {
    if input.include_frontmatter.unwrap_or(false)
        || input.include_references.unwrap_or(false)
        || input.strip_images.unwrap_or(false)
        || input
            .content_selector
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
    {
        return Err(AppError::BadRequest(
            "Advanced extract options will be added with Milestone G; Milestones D and E only support url and format"
                .to_string(),
        ));
    }

    if input.url.trim().is_empty() {
        return Err(AppError::BadRequest("url is required".to_string()));
    }

    Ok(())
}

fn render_article(article: &lectito_core::Article, format: CachedFormat) -> Result<String, AppError> {
    match format {
        CachedFormat::Html => Ok(article.content.clone()),
        CachedFormat::Markdown => article.to_markdown().map_err(AppError::from),
        CachedFormat::Text => Ok(article.to_text()),
        CachedFormat::Json => article.to_json().map(|json| json.to_string()).map_err(AppError::from),
    }
}

impl CachedMetadata {
    fn from_article(article: &lectito_core::Article) -> Self {
        Self {
            title: article.metadata.title.clone(),
            author: article.metadata.author.clone(),
            date: article.metadata.date.clone(),
            excerpt: article.metadata.excerpt.clone(),
            site_name: article.metadata.site_name.clone(),
            language: article.metadata.language.clone(),
            word_count: article.metadata.word_count.or(Some(article.word_count)),
            reading_time_minutes: article.metadata.reading_time_minutes.or(Some(article.reading_time)),
            image: None,
            favicon: None,
        }
    }
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
    fn rejects_advanced_extract_options_before_milestone_g() {
        let error = reject_unsupported_extract_options(&ExtractRequest {
            url: "https://example.com".to_string(),
            format: Some(CachedFormat::Markdown),
            include_frontmatter: Some(true),
            include_references: None,
            strip_images: None,
            content_selector: None,
        })
        .unwrap_err();

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
