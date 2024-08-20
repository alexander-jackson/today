use std::net::{Ipv4Addr, SocketAddrV4};

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::header::LOCATION;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, patch, post};
use axum::{Form, Json, Router};
use color_eyre::eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx_bootstrap::{ApplicationConfig, BootstrapConfig, ConnectionConfig, RootConfig};
use tera::Context;
use tokio::net::TcpListener;
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
}

fn build_router(template_engine: TemplateEngine, pool: PgPool) -> Router {
    let state = ApplicationState {
        template_engine,
        pool,
    };

    Router::new()
        .route("/", get(templated))
        .route("/add", post(add_item))
        .route("/update/:item_uid", patch(update_item))
        .layer(TraceLayer::new_for_http())
        .nest_service("/assets", ServeDir::new("assets"))
        .with_state(state)
}

fn get_env_var(key: &str) -> Result<String> {
    std::env::var(key).map_err(|_| eyre!("Failed to get environment variable '{key}'"))
}

async fn bootstrap_database() -> Result<PgPool> {
    let root_username = get_env_var("ROOT_USERNAME")?;
    let root_password = get_env_var("ROOT_PASSWORD")?;
    let root_database = get_env_var("ROOT_DATABASE")?;

    let app_username = get_env_var("APP_USERNAME")?;
    let app_password = get_env_var("APP_PASSWORD")?;
    let app_database = get_env_var("APP_DATABASE")?;

    let host = get_env_var("DATABASE_HOST")?;
    let port = get_env_var("DATABASE_PORT")?.parse()?;

    let root_config = RootConfig::new(&root_username, &root_password, &root_database);
    let app_config = ApplicationConfig::new(&app_username, &app_password, &app_database);
    let conn_config = ConnectionConfig::new(&host, port);

    let config = BootstrapConfig::new(root_config, app_config, conn_config);
    let pool = config.bootstrap().await?;

    sqlx::migrate!().run(&pool).await?;

    Ok(pool)
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().ok();

    let pool = bootstrap_database().await?;
    let template_engine = TemplateEngine::new()?;

    let router = build_router(template_engine, pool);

    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8000);
    let listener = TcpListener::bind(addr).await?;

    tracing::info!(?addr, "listening for incoming requests");

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
