use std::ops::Deref;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::header::LOCATION;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, patch, post};
use axum::{Form, Json, Router};
use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tera::Context;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

mod error;
mod persistence;
mod templates;

use crate::error::ServerResult;
use crate::templates::{RenderedTemplate, TemplateEngine};

#[derive(Serialize)]
struct Item {
    item_uid: Uuid,
    content: String,
    state: bool,
}

#[derive(Clone)]
struct ApplicationState {
    template_engine: TemplateEngine,
    pool: PgPool,
    items: Arc<Mutex<Vec<Item>>>,
}

fn build_router(template_engine: TemplateEngine, pool: PgPool) -> Router {
    let state = ApplicationState {
        template_engine,
        pool,
        items: Arc::default(),
    };

    Router::new()
        .route("/", get(templated))
        .route("/add", post(add_item))
        .route("/update/:item_uid", patch(update_item))
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

    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    sqlx::migrate!().run(&pool).await?;

    let template_engine = TemplateEngine::new()?;

    let router = build_router(template_engine, pool);
    let listener = TcpListener::bind("localhost:8000").await?;

    axum::serve(listener, router).await?;

    Ok(())
}

async fn templated(
    State(ApplicationState {
        template_engine,
        pool,
        ..
    }): State<ApplicationState>,
) -> ServerResult<RenderedTemplate> {
    let items = crate::persistence::select_items(&pool).await?;

    let mut context = Context::new();
    context.insert("items", &items);

    let rendered = template_engine.render("index.tera.html", &context)?;

    Ok(rendered)
}

#[derive(Debug, Deserialize)]
struct AddItemForm {
    content: String,
}

async fn add_item(
    State(ApplicationState { pool, .. }): State<ApplicationState>,
    Form(AddItemForm { content }): Form<AddItemForm>,
) -> ServerResult<Response> {
    tracing::info!(?content, "Got something from the client");

    let item_uid = Uuid::new_v4();

    crate::persistence::create_item(&pool, item_uid, &content).await?;

    Ok(redirect("/")?)
}

#[derive(Debug, Deserialize)]
struct UpdateItemRequest {
    state: bool,
}

async fn update_item(
    State(ApplicationState { pool, .. }): State<ApplicationState>,
    Path(item_uid): Path<Uuid>,
    Json(request): Json<UpdateItemRequest>,
) -> ServerResult<Response> {
    crate::persistence::update_item(&pool, item_uid, request.state).await?;

    Ok(success()?)
}

fn success() -> Result<Response> {
    let res = Response::builder()
        .status(StatusCode::OK)
        .body(Body::empty())?;

    Ok(res)
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
