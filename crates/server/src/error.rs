use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use lectito_core::LectitoError;
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Forbidden(String),
    NotFound(String),
    TooManyRequests { retry_after: u32 },
    UnprocessableEntity(String),
    BadGateway(String),
    GatewayTimeout,
    Internal(String),
    ServiceUnavailable(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::TooManyRequests { .. } => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded".to_string()),
            AppError::UnprocessableEntity(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            AppError::BadGateway(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            AppError::GatewayTimeout => (StatusCode::GATEWAY_TIMEOUT, "Upstream fetch timed out".to_string()),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::ServiceUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg.clone()),
        };

        let mut response = (status, Json(json!({ "error": message }))).into_response();

        if let AppError::TooManyRequests { retry_after } = &self
            && let Ok(val) = retry_after.to_string().parse()
        {
            response.headers_mut().insert("Retry-After", val);
        }

        response
    }
}

impl From<deadpool_postgres::PoolError> for AppError {
    fn from(err: deadpool_postgres::PoolError) -> Self {
        AppError::Internal(format!("Database pool error: {err}"))
    }
}

impl From<tokio_postgres::Error> for AppError {
    fn from(err: tokio_postgres::Error) -> Self {
        AppError::Internal(format!("Database error: {err}"))
    }
}

impl From<LectitoError> for AppError {
    fn from(err: LectitoError) -> Self {
        match err {
            LectitoError::InvalidUrl(message)
            | LectitoError::HtmlParseError(message)
            | LectitoError::ConfigError(message)
            | LectitoError::SiteConfigError(message) => Self::BadRequest(message),
            LectitoError::InvalidEncoding => Self::BadRequest("Invalid character encoding".to_string()),
            LectitoError::Timeout { .. } => Self::GatewayTimeout,
            LectitoError::HttpError(source) => {
                if source.is_timeout() {
                    Self::GatewayTimeout
                } else {
                    Self::BadGateway(format!("Upstream fetch failed: {source}"))
                }
            }
            LectitoError::NotReadable { score, threshold } => Self::UnprocessableEntity(format!(
                "Content is not readable (score {score} below threshold {threshold})"
            )),
            LectitoError::NoContent => Self::UnprocessableEntity("No content could be extracted".to_string()),
            LectitoError::FileNotFound(path) => Self::NotFound(format!("File not found: {}", path.display())),
            LectitoError::WriteError(source) => Self::Internal(format!("Failed to write response: {source}")),
            LectitoError::XPathError(message) => Self::BadRequest(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_invalid_url_to_bad_request() {
        let error = AppError::from(LectitoError::InvalidUrl("bad".to_string()));
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn maps_timeout_to_gateway_timeout() {
        let error = AppError::from(LectitoError::Timeout { timeout: 30 });
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
    }
}
