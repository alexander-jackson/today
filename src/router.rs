use axum::body::Body;
use axum::extract::{Form, Json, Path, State};
use axum::http::header::LOCATION;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, patch, post};
use axum::Router;
use chrono::Utc;
use color_eyre::eyre::Result;
use serde::Deserialize;
use sqlx::PgPool;
use tera::Context;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::error::ServerResult;
use crate::persistence::ItemState;
use crate::templates::{RenderedTemplate, TemplateEngine};

#[derive(Clone)]
struct ApplicationState {
    template_engine: TemplateEngine,
    pool: PgPool,
}

pub fn build(template_engine: TemplateEngine, pool: PgPool) -> Router {
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

async fn templated(
    State(ApplicationState {
        template_engine,
        pool,
        ..
    }): State<ApplicationState>,
) -> ServerResult<RenderedTemplate> {
    let now = Utc::now().date_naive();
    let items = crate::persistence::select_items(&pool, now).await?;

    let mut checked_items = Vec::new();
    let mut unchecked_items = Vec::new();

    for item in items {
        match item.state {
            ItemState::Checked => checked_items.push(item),
            ItemState::Unchecked => unchecked_items.push(item),
            ItemState::Deleted => {
                // intentionally ignored
            }
        };
    }

    let mut context = Context::new();
    context.insert("checked_items", &checked_items);
    context.insert("unchecked_items", &unchecked_items);

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
    let now = Utc::now().naive_local();

    crate::persistence::create_item(&pool, item_uid, &content, now).await?;

    Ok(redirect("/")?)
}

#[derive(Debug, Deserialize)]
struct UpdateItemRequest {
    state: ItemState,
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
