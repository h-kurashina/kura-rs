use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// API が返すエラー。本文は `{"error": "..."}`。
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("part \"{0}\" not found")]
    PartNotFound(String),
    #[error("not found")]
    RouteNotFound,
    #[error("{0}")]
    BadRequest(String),
    #[error("internal error")]
    Internal(#[from] serde_json::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            ApiError::PartNotFound(_) | ApiError::RouteNotFound => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Internal(e) => {
                tracing::error!(error = %e, "failed to serialize response");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        (
            status,
            Json(ErrorBody {
                error: self.to_string(),
            }),
        )
            .into_response()
    }
}
