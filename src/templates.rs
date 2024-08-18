use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use color_eyre::eyre::Result;
use tera::{Context, Tera};

#[derive(Clone)]
pub struct TemplateEngine {
    inner: Tera,
}

impl TemplateEngine {
    pub fn new() -> Result<Self> {
        let inner = Tera::new("templates/**.tera.html")?;

        Ok(Self { inner })
    }

    pub fn render(&self, template: &str) -> Result<RenderedTemplate> {
        let rendered = self.inner.render(template, &Context::default())?;

        Ok(RenderedTemplate { inner: rendered })
    }
}

pub struct RenderedTemplate {
    inner: String,
}

impl IntoResponse for RenderedTemplate {
    fn into_response(self) -> Response {
        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "text/html")
            .body(Body::from(self.inner))
            .unwrap()
    }
}