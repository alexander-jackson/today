use core::fmt;
use std::error::Error;

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

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.inner)
    }
}

impl Error for ServerError {}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}
