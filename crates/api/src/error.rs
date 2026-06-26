use std::fmt::Display;

use super::models::{ErrorBody, ErrorResponse};
use axum::extract::rejection::JsonRejection;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct ApiError {
    status: StatusCode,
    code: ErrorCode,
    message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: ErrorCode, message: impl Into<String>) -> Self {
        Self { status, code, message: message.into() }
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, ErrorCode::InvalidRequest, message)
    }

    pub fn fetch_failed(error: impl ToString) -> Self {
        Self::new(StatusCode::BAD_GATEWAY, ErrorCode::FetchFailed, error.to_string())
    }

    pub fn core(code: ErrorCode, error: impl ToString) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, code, error.to_string())
    }

    /// Body exceeded the configured `RequestBodyLimitLayer`.
    pub fn body_too_large() -> Self {
        Self::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            ErrorCode::DocumentTooLarge,
            "request body is too large",
        )
    }

    /// Request exceeded the configured `TimeoutLayer`.
    pub fn timeout() -> Self {
        Self::new(StatusCode::REQUEST_TIMEOUT, ErrorCode::Timeout, "request timed out")
    }

    /// Unexpected server error from an unhandled path (5xx catch-all).
    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::InternalError,
            "internal server error",
        )
    }
}

impl From<JsonRejection> for ApiError {
    /// Normalize axum's `JsonRejection` (422/400/415 plain-text bodies) into the
    /// structured `ApiError` shape. `JsonRejection::body_text()` preserves the
    /// underlying serde error message for the response.
    fn from(rejection: JsonRejection) -> Self {
        Self::new(rejection.status(), ErrorCode::InvalidRequest, rejection.body_text())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let code = self.code;
        let mut response = (
            self.status,
            axum::Json(ErrorResponse { error: ErrorBody { code: code.to_string(), message: self.message } }),
        )
            .into_response();
        response
            .headers_mut()
            .insert("x-error-code", HeaderValue::from_static(code.as_str()));
        response
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ErrorCode {
    InvalidRequest,
    DocumentTooLarge,
    Timeout,
    FetchFailed,
    UnsupportedContentType,
    ExtractFailed,
    InternalError,
}

impl ErrorCode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::DocumentTooLarge => "document_too_large",
            Self::Timeout => "timeout",
            Self::FetchFailed => "fetch_failed",
            Self::UnsupportedContentType => "unsupported_content_type",
            Self::ExtractFailed => "extract_failed",
            Self::InternalError => "internal_error",
        }
    }
}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// JSON request extractor that converts axum's `JsonRejection` into the
/// structured `ApiError` shape, so deserialization failures produce the same
/// `{ "error": { "code", "message" } }` body and `x-error-code` header as
/// every other error path — instead of axum's default `text/plain` 422.
///
/// Use this in handlers wherever `axum::Json` would appear as an argument.
/// For building JSON *responses*, keep using `axum::Json` directly.
pub struct Json<T>(pub T);

impl<S, T> axum::extract::FromRequest<S> for Json<T>
where
    axum::Json<T>: axum::extract::FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(value)) => Ok(Self(value)),
            Err(rejection) => Err(ApiError::from(rejection)),
        }
    }
}
