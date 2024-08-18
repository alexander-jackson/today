use axum::extract::{Request, State};
use axum::routing::get;
use axum::Router;
use color_eyre::eyre::{eyre, Result};
use error::ServerResult;
use templates::{RenderedTemplate, TemplateEngine};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

mod error;
mod templates;

fn build_router(template_engine: TemplateEngine) -> Router {
    Router::new()
        .route("/", get(handler))
        .route("/index", get(templated))
        .layer(TraceLayer::new_for_http())
        .with_state(template_engine)
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt().init();

    let template_engine = TemplateEngine::new()?;

    let router = build_router(template_engine);
    let listener = TcpListener::bind("localhost:8000").await?;

    axum::serve(listener, router).await?;

    Ok(())
}

async fn handler(request: Request) -> ServerResult<&'static str> {
    match request
        .headers()
        .get("x-testing")
        .and_then(|h| h.to_str().ok())
    {
        Some("throw-error") => Err(eyre!("Something went wrong!").into()),
        _ => Ok("Hello World!"),
    }
}

async fn templated(
    State(template_engine): State<TemplateEngine>,
) -> ServerResult<RenderedTemplate> {
    let rendered = template_engine.render("index.tera.html")?;

    Ok(rendered)
}

#[cfg(test)]
mod tests;
