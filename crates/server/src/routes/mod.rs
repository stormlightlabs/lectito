use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::{Extension, Json, Path as AxumPath, Query, Request, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{any, delete, get, post};
use lectito_core::article::Article;
use lectito_core::formatters::{
    JsonConfig, MarkdownConfig, TextConfig, convert_to_json, convert_to_markdown, convert_to_text,
};
use lectito_core::parse::Document;
use lectito_core::{
    ExtractionDiagnostics, FetchConfig, PostProcessConfig, Readability, ReadabilityConfig, fetch_url, postprocess_html,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::format_description::{self, well_known::Rfc3339};
use time::{Date, OffsetDateTime};
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

const OPENAPI_TEMPLATE: &str = include_str!("openapi.json");
const SWAGGER_HTML: &str = include_str!("swagger.html");
const CACHE_UPSERT_SQL: &str = "INSERT INTO extracted_articles
    (id, url, url_hash, format, content, metadata, fetched_at, expires_at, hit_count)
 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0)
 ON CONFLICT (url_hash, format)
 DO UPDATE SET
    url = EXCLUDED.url,
    content = EXCLUDED.content,
    metadata = EXCLUDED.metadata,
    fetched_at = EXCLUDED.fetched_at,
    expires_at = EXCLUDED.expires_at
 RETURNING id";

pub fn build_app(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .route("/extract", get(extract_get).post(extract_post))
        .route("/library", get(library_list))
        .route("/library/{id}", get(library_detail))
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
        .route("/api-docs", get(swagger_ui))
        .route("/api-docs/openapi.json", get(openapi_json))
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
    id: Option<Uuid>,
    url: String,
    format: CachedFormat,
    content: String,
    metadata: CachedMetadata,
    confidence: f64,
    diagnostics: Option<ExtractionDiagnostics>,
    cached: bool,
    extracted_at: String,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum LibrarySort {
    #[default]
    Recent,
    Popular,
    Alpha,
}

impl LibrarySort {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Recent => "recent",
            Self::Popular => "popular",
            Self::Alpha => "alpha",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct LibraryQuery {
    #[serde(default)]
    page: Option<u32>,
    #[serde(default)]
    per_page: Option<u32>,
    #[serde(default)]
    sort: Option<LibrarySort>,
    #[serde(default)]
    q: Option<String>,
    #[serde(default)]
    domain: Option<String>,
    #[serde(default)]
    date_from: Option<String>,
    #[serde(default)]
    date_to: Option<String>,
}

#[derive(Debug, Clone)]
struct NormalizedLibraryQuery {
    page: u32,
    per_page: u32,
    sort: LibrarySort,
    q: Option<String>,
    domain: Option<String>,
    date_from: Option<Date>,
    date_to: Option<Date>,
}

#[derive(Debug, Clone, Serialize)]
struct LibraryItemResponse {
    id: Uuid,
    url: String,
    domain: String,
    format: CachedFormat,
    title: Option<String>,
    author: Option<String>,
    site_name: Option<String>,
    favicon: Option<String>,
    excerpt: Option<String>,
    date: Option<String>,
    word_count: Option<usize>,
    reading_time_minutes: Option<f64>,
    hit_count: u64,
    fetched_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct TopDomainResponse {
    domain: String,
    count: u64,
}

#[derive(Debug, Clone, Serialize)]
struct LibraryStatsResponse {
    total_articles: u64,
    total_reads: u64,
    unique_domains: u64,
    total_reading_time_minutes: f64,
    top_domains: Vec<TopDomainResponse>,
}

#[derive(Debug, Clone, Serialize)]
struct LibraryResponse {
    items: Vec<LibraryItemResponse>,
    total: u64,
    page: u32,
    per_page: u32,
    stats: LibraryStatsResponse,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CachedArticleRecord {
    #[serde(flatten)]
    metadata: CachedMetadata,
    #[serde(default)]
    confidence: Option<f64>,
    #[serde(default)]
    diagnostics: Option<ExtractionDiagnostics>,
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

async fn swagger_ui() -> Html<String> {
    Html(swagger_ui_html())
}

async fn openapi_json(State(state): State<AppState>) -> Json<Value> {
    Json(openapi_spec(state.version))
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

async fn library_list(
    State(state): State<AppState>, Query(query): Query<LibraryQuery>,
) -> Result<Json<LibraryResponse>, AppError> {
    let query = normalize_library_query(query)?;
    let client = state.pool.get().await?;
    let params: [&(dyn tokio_postgres::types::ToSql + Sync); 7] = [
        &query.q,
        &query.domain,
        &query.date_from,
        &query.date_to,
        &query.sort.as_str(),
        &(query.per_page as i64),
        &(((query.page - 1) * query.per_page) as i64),
    ];

    let rows = client
        .query(
            "WITH latest_entries AS (
                SELECT DISTINCT ON (url)
                    id, url, format, metadata, fetched_at, hit_count
                FROM extracted_articles
                WHERE expires_at > now()
                ORDER BY url, fetched_at DESC
             ),
             filtered AS (
                SELECT *
                FROM latest_entries
                WHERE ($1::text IS NULL
                       OR COALESCE(metadata->>'title', '') ILIKE '%' || $1 || '%'
                       OR lower(split_part(regexp_replace(url, '^https?://', ''), '/', 1)) ILIKE '%' || $1 || '%'
                       OR url ILIKE '%' || $1 || '%')
                  AND ($2::text IS NULL
                       OR lower(split_part(regexp_replace(url, '^https?://', ''), '/', 1)) = lower($2)
                       OR url ILIKE '%' || $2 || '%')
                  AND ($3::date IS NULL OR fetched_at::date >= $3)
                  AND ($4::date IS NULL OR fetched_at::date <= $4)
             )
             SELECT id, url, format, metadata, fetched_at, hit_count
             FROM filtered
             ORDER BY
                CASE WHEN $5 = 'popular' THEN hit_count END DESC NULLS LAST,
                CASE WHEN $5 = 'alpha' THEN lower(COALESCE(metadata->>'title', url)) END ASC NULLS LAST,
                CASE WHEN $5 = 'recent' THEN fetched_at END DESC NULLS LAST,
                fetched_at DESC,
                lower(url) ASC
             LIMIT $6
             OFFSET $7",
            &params,
        )
        .await?;

    let total = client
        .query_one(
            "WITH latest_entries AS (
                SELECT DISTINCT ON (url)
                    id, url, format, metadata, fetched_at, hit_count
                FROM extracted_articles
                WHERE expires_at > now()
                ORDER BY url, fetched_at DESC
             ),
             filtered AS (
                SELECT *
                FROM latest_entries
                WHERE ($1::text IS NULL
                       OR COALESCE(metadata->>'title', '') ILIKE '%' || $1 || '%'
                       OR lower(split_part(regexp_replace(url, '^https?://', ''), '/', 1)) ILIKE '%' || $1 || '%'
                       OR url ILIKE '%' || $1 || '%')
                  AND ($2::text IS NULL
                       OR lower(split_part(regexp_replace(url, '^https?://', ''), '/', 1)) = lower($2)
                       OR url ILIKE '%' || $2 || '%')
                  AND ($3::date IS NULL OR fetched_at::date >= $3)
                  AND ($4::date IS NULL OR fetched_at::date <= $4)
             )
             SELECT COUNT(*) AS total
             FROM filtered",
            &[&query.q, &query.domain, &query.date_from, &query.date_to],
        )
        .await?
        .get::<_, i64>("total") as u64;

    let stats_row = client
        .query_one(
            "WITH latest_entries AS (
                SELECT DISTINCT ON (url)
                    url, metadata, hit_count
                FROM extracted_articles
                WHERE expires_at > now()
                ORDER BY url, fetched_at DESC
             )
             SELECT
                COUNT(*) AS total_articles,
                COALESCE(SUM(hit_count), 0) AS total_reads,
                COUNT(DISTINCT lower(split_part(regexp_replace(url, '^https?://', ''), '/', 1))) AS unique_domains,
                COALESCE(SUM(COALESCE(NULLIF(metadata->>'reading_time_minutes', ''), '0')::double precision), 0) AS total_reading_time_minutes
             FROM latest_entries",
            &[],
        )
        .await?;

    let top_domains = client
        .query(
            "WITH latest_entries AS (
                SELECT DISTINCT ON (url)
                    url
                FROM extracted_articles
                WHERE expires_at > now()
                ORDER BY url, fetched_at DESC
             )
             SELECT
                lower(split_part(regexp_replace(url, '^https?://', ''), '/', 1)) AS domain,
                COUNT(*) AS count
             FROM latest_entries
             GROUP BY domain
             ORDER BY count DESC, domain ASC
             LIMIT 5",
            &[],
        )
        .await?
        .into_iter()
        .map(|row| TopDomainResponse {
            domain: row.get::<_, String>("domain"),
            count: row.get::<_, i64>("count") as u64,
        })
        .collect();

    Ok(Json(LibraryResponse {
        items: rows.into_iter().map(|row| library_item_from_row(&row)).collect(),
        total,
        page: query.page,
        per_page: query.per_page,
        stats: LibraryStatsResponse {
            total_articles: stats_row.get::<_, i64>("total_articles") as u64,
            total_reads: stats_row.get::<_, i64>("total_reads") as u64,
            unique_domains: stats_row.get::<_, i64>("unique_domains") as u64,
            total_reading_time_minutes: stats_row.get::<_, f64>("total_reading_time_minutes"),
            top_domains,
        },
    }))
}

async fn library_detail(
    State(state): State<AppState>, AxumPath(id): AxumPath<Uuid>,
) -> Result<Json<ExtractResponse>, AppError> {
    let client = state.pool.get().await?;
    let row = client
        .query_opt(
            "SELECT id, url, format, content, metadata, fetched_at
             FROM extracted_articles
             WHERE id = $1 AND expires_at > now()",
            &[&id],
        )
        .await?
        .ok_or_else(|| AppError::NotFound("Cached article not found".to_string()))?;

    let format = parse_cached_format(row.get::<_, String>("format").as_str())
        .ok_or_else(|| AppError::Internal("Stored article format is invalid".to_string()))?;
    let fetched_at = row.get::<_, OffsetDateTime>("fetched_at");
    let record = parse_cached_article_record(row.get("metadata"));

    Ok(Json(ExtractResponse {
        id: Some(row.get("id")),
        url: row.get("url"),
        format,
        content: row.get("content"),
        metadata: record.metadata,
        confidence: record.confidence.unwrap_or(0.0),
        diagnostics: record.diagnostics,
        cached: true,
        extracted_at: fetched_at.format(&Rfc3339).unwrap_or_else(|_| fetched_at.to_string()),
    }))
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

    let web_request = rate_limit::is_web_app_request(&headers);
    let parsed_url = state.spam_filter.validate_extract_url(&state.pool, &input.url).await?;
    let format = input.format.unwrap_or(CachedFormat::Markdown);
    let normalized_url = cache::normalize_url(parsed_url.as_str())?;
    let cache_key = cache::build_cache_key(&normalized_url, format);
    let bypass_cache = !is_cacheable_request(&input) || cache::should_bypass_cache(&headers);

    if !bypass_cache && let Some(cached) = read_cached_article(&state, &cache_key, format).await {
        return Ok(ExtractResponse {
            id: Some(cached.id),
            url: cached.url,
            format: cached.format,
            content: cached.content,
            metadata: cached.metadata,
            confidence: cached.confidence,
            diagnostics: cached.diagnostics,
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
    let mut cached_id = if is_cacheable_request(&input) {
        write_cached_article(
            &state,
            CachedExtractedArticle {
                id: Uuid::new_v4(),
                url: normalized_url.clone(),
                format,
                content: content.clone(),
                metadata: extracted.metadata.clone(),
                confidence: extracted.article.confidence,
                diagnostics: extracted.article.diagnostics.clone(),
                fetched_at,
            },
            cache_key,
        )
        .await
    } else {
        None
    };

    if cached_id.is_none() && web_request {
        cached_id = ensure_reader_cache_id(&state, &normalized_url, &extracted, fetched_at).await;
    }

    let response = ExtractResponse {
        id: cached_id,
        url: normalized_url,
        format,
        content: content.clone(),
        metadata: extracted.metadata.clone(),
        confidence: extracted.article.confidence,
        diagnostics: extracted.article.diagnostics.clone(),
        cached: false,
        extracted_at: fetched_at.format(&Rfc3339).unwrap_or_else(|_| fetched_at.to_string()),
    };

    Ok(response)
}

async fn ensure_reader_cache_id(
    state: &AppState, normalized_url: &str, extracted: &ExtractedArticle, fetched_at: OffsetDateTime,
) -> Option<Uuid> {
    write_cached_article(
        state,
        CachedExtractedArticle {
            id: Uuid::new_v4(),
            url: normalized_url.to_string(),
            format: CachedFormat::Html,
            content: extracted.article.content.clone(),
            metadata: extracted.metadata.clone(),
            confidence: extracted.article.confidence,
            diagnostics: extracted.article.diagnostics.clone(),
            fetched_at,
        },
        cache::build_cache_key(normalized_url, CachedFormat::Html),
    )
    .await
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

    let metadata = CachedMetadata::from_article(&article);

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
        Err(err) => match err.kind() {
            std::io::ErrorKind::NotFound => match asset_like {
                true => Err(AppError::NotFound("Static asset not found".to_string())),
                false => Ok(web_dir.join("index.html")),
            },
            _ => Err(AppError::Internal(format!(
                "Failed to read static file metadata: {err}"
            ))),
        },
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

fn swagger_ui_html() -> String {
    SWAGGER_HTML.to_string()
}

fn openapi_spec(version: &str) -> Value {
    let json = OPENAPI_TEMPLATE.replace("__LECTITO_VERSION__", version);
    serde_json::from_str(&json).expect("embedded OpenAPI spec must be valid JSON")
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
        Ok(Some(row)) => {
            let record = parse_cached_article_record(row.get("metadata"));
            Some(CachedExtractedArticle {
                id: row.get("id"),
                url: row.get("url"),
                format: parse_cached_format(row.get::<_, String>("format").as_str()).unwrap_or(format),
                content: row.get("content"),
                metadata: record.metadata,
                confidence: record.confidence.unwrap_or(0.0),
                diagnostics: record.diagnostics,
                fetched_at: row.get("fetched_at"),
            })
        }
        Ok(None) => None,
        Err(err) => {
            warn!("cache read failed: {err}");
            None
        }
    }
}

async fn write_cached_article(state: &AppState, article: CachedExtractedArticle, cache_key: Vec<u8>) -> Option<Uuid> {
    let client = (acquire_cache_client(state).await)?;

    log_cache_write_result(
        execute_cache_upsert(&client, state, &article, &cache_key).await,
        &article.url,
    )
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
) -> Result<Uuid, tokio_postgres::Error> {
    let expires_at = article.fetched_at + time::Duration::seconds(state.config.cache_ttl_secs as i64);
    let metadata_json = serde_json::to_value(CachedArticleRecord {
        metadata: article.metadata.clone(),
        confidence: Some(article.confidence),
        diagnostics: article.diagnostics.clone(),
    })
    .unwrap_or_default();
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

    client
        .query_one(CACHE_UPSERT_SQL, &params)
        .await
        .map(|row| row.get("id"))
}

fn log_cache_write_result(result: Result<Uuid, tokio_postgres::Error>, url: &str) -> Option<Uuid> {
    match result {
        Ok(id) => {
            info!("cached extracted article for {url}");
            Some(id)
        }
        Err(err) => {
            warn!("cache write failed: {err}");
            None
        }
    }
}

fn normalize_library_query(query: LibraryQuery) -> Result<NormalizedLibraryQuery, AppError> {
    let page = query.page.unwrap_or(1);
    if page == 0 {
        return Err(AppError::BadRequest("page must be greater than zero".to_string()));
    }

    let per_page = query.per_page.unwrap_or(20);
    if per_page == 0 || per_page > 100 {
        return Err(AppError::BadRequest("per_page must be between 1 and 100".to_string()));
    }

    let date_from = query
        .date_from
        .as_deref()
        .map(|value| parse_query_date(value, "date_from"))
        .transpose()?;
    let date_to = query
        .date_to
        .as_deref()
        .map(|value| parse_query_date(value, "date_to"))
        .transpose()?;

    if let (Some(from), Some(to)) = (date_from, date_to)
        && from > to
    {
        return Err(AppError::BadRequest(
            "date_from must be earlier than or equal to date_to".to_string(),
        ));
    }

    Ok(NormalizedLibraryQuery {
        page,
        per_page,
        sort: query.sort.unwrap_or_default(),
        q: trim_optional(query.q),
        domain: trim_optional(query.domain),
        date_from,
        date_to,
    })
}

fn parse_query_date(value: &str, key: &str) -> Result<Date, AppError> {
    let format = format_description::parse("[year]-[month]-[day]")
        .map_err(|err| AppError::Internal(format!("failed to parse date format description: {err}")))?;

    Date::parse(value, &format).map_err(|_| AppError::BadRequest(format!("{key} must be in YYYY-MM-DD format")))
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn library_item_from_row(row: &tokio_postgres::Row) -> LibraryItemResponse {
    let url = row.get::<_, String>("url");
    let format = parse_cached_format(row.get::<_, String>("format").as_str()).unwrap_or(CachedFormat::Markdown);
    let metadata = parse_cached_article_record(row.get("metadata")).metadata;
    let fetched_at = row.get::<_, OffsetDateTime>("fetched_at");

    LibraryItemResponse {
        id: row.get("id"),
        domain: domain_from_url(&url).unwrap_or_else(|| url.clone()),
        url,
        format,
        title: metadata.title,
        author: metadata.author,
        site_name: metadata.site_name,
        favicon: metadata.favicon,
        excerpt: metadata.excerpt,
        date: metadata.date,
        word_count: metadata.word_count,
        reading_time_minutes: metadata.reading_time_minutes,
        hit_count: row.get::<_, i32>("hit_count") as u64,
        fetched_at: fetched_at.format(&Rfc3339).unwrap_or_else(|_| fetched_at.to_string()),
    }
}

fn parse_cached_article_record(value: Value) -> CachedArticleRecord {
    serde_json::from_value::<CachedArticleRecord>(value.clone()).unwrap_or_else(|_| CachedArticleRecord {
        metadata: serde_json::from_value(value).unwrap_or_default(),
        confidence: None,
        diagnostics: None,
    })
}

fn domain_from_url(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()?
        .host_str()
        .map(|host| host.trim_start_matches("www.").to_string())
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
    fn from_article(article: &Article) -> Self {
        Self {
            title: article.metadata.title.clone(),
            author: article.metadata.author.clone(),
            date: article.metadata.date.clone(),
            excerpt: article.metadata.excerpt.clone(),
            site_name: article.metadata.site_name.clone(),
            language: article.metadata.language.clone(),
            word_count: article.metadata.word_count.or(Some(article.word_count)),
            reading_time_minutes: article.metadata.reading_time_minutes.or(Some(article.reading_time)),
            image: article.metadata.image.clone(),
            favicon: article.metadata.favicon.clone(),
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

    #[test]
    fn swagger_ui_points_at_openapi_document() {
        let html = swagger_ui_html();

        assert!(html.contains("/api-docs/openapi.json"));
        assert!(html.contains("SwaggerUIBundle"));
    }

    #[test]
    fn openapi_spec_includes_library_detail_path() {
        let spec = openapi_spec("0.1.0");
        let paths = spec.get("paths").and_then(Value::as_object).unwrap();
        let extract_response = spec
            .pointer("/components/schemas/ExtractResponse/properties")
            .and_then(Value::as_object)
            .unwrap();

        assert!(paths.contains_key("/api/v1/library"));
        assert!(paths.contains_key("/api/v1/library/{id}"));
        assert!(paths.contains_key("/api/v1/extract"));
        assert!(extract_response.contains_key("confidence"));
        assert!(extract_response.contains_key("diagnostics"));
    }
}
