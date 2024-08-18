use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use color_eyre::Report;

pub type ServerResult<T> = std::result::Result<T, ServerError>;

#[derive(Debug)]
pub struct ServerError {
    inner: Report,
}

impl From<Report> for ServerError {
    fn from(value: Report) -> Self {
        Self { inner: value }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("{:?}", self.inner),
        )
            .into_response()
    }
}
