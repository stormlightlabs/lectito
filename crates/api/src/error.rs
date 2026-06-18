use std::fmt::Display;

use super::models::{ErrorBody, ErrorResponse};
use axum::Json;
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
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let code = self.code;
        let mut response = (
            self.status,
            Json(ErrorResponse { error: ErrorBody { code: code.to_string(), message: self.message } }),
        )
            .into_response();
        response
            .headers_mut()
            .insert("x-error-code", HeaderValue::from_static(code.as_str()));
        response
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum ErrorCode {
    InvalidRequest,
    DocumentTooLarge,
    FetchFailed,
    UnsupportedContentType,
    ExtractFailed,
    MarkdownFailed,
    InternalError,
}

impl ErrorCode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::DocumentTooLarge => "document_too_large",
            Self::FetchFailed => "fetch_failed",
            Self::UnsupportedContentType => "unsupported_content_type",
            Self::ExtractFailed => "extract_failed",
            Self::MarkdownFailed => "markdown_failed",
            Self::InternalError => "internal_error",
        }
    }
}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
