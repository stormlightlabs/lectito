use std::path::{Component, Path, PathBuf};

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::{Json, Router};
use serde::Serialize;
use tracing::warn;

use crate::AppState;
use crate::db;
use crate::error::AppError;

pub fn build_app(state: AppState) -> Router {
    let api = Router::new().route("/health", get(health));

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

async fn api_not_found() -> AppError {
    AppError::NotFound("API route not found".to_string())
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
}
