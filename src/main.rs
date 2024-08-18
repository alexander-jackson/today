use std::ops::Deref;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::header::LOCATION;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Form, Router};
use color_eyre::eyre::Result;
use error::ServerResult;
use serde::Deserialize;
use templates::{RenderedTemplate, TemplateEngine};
use tera::Context;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

mod error;
mod templates;

#[derive(Clone)]
struct ApplicationState {
    template_engine: TemplateEngine,
    items: Arc<Mutex<Vec<String>>>,
}

fn build_router(template_engine: TemplateEngine) -> Router {
    let state = ApplicationState {
        template_engine,
        items: Arc::default(),
    };

    Router::new()
        .route("/", get(templated))
        .route("/add", post(add_item))
        .layer(TraceLayer::new_for_http())
        .nest_service("/assets", ServeDir::new("assets"))
        .with_state(state)
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let template_engine = TemplateEngine::new()?;

    let router = build_router(template_engine);
    let listener = TcpListener::bind("localhost:8000").await?;

    axum::serve(listener, router).await?;

    Ok(())
}

async fn templated(
    State(ApplicationState {
        template_engine,
        items,
    }): State<ApplicationState>,
) -> ServerResult<RenderedTemplate> {
    let mut context = Context::new();
    context.insert("items", items.lock().await.deref());

    let rendered = template_engine.render("index.tera.html", &context)?;

    Ok(rendered)
}

#[derive(Debug, Deserialize)]
struct AddItemForm {
    item: String,
}

async fn add_item(
    State(ApplicationState { items, .. }): State<ApplicationState>,
    Form(AddItemForm { item }): Form<AddItemForm>,
) -> ServerResult<Response> {
    tracing::info!(?item, "Got something from the client");

    items.lock().await.push(item);

    Ok(redirect("/")?)
}

fn redirect(path: &'static str) -> Result<Response> {
    let res = Response::builder()
        .status(StatusCode::FOUND)
        .header(LOCATION, path)
        .body(Body::empty())?;

    Ok(res)
}

#[cfg(test)]
mod tests;
