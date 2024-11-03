use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Form, Json, Path, State};
use axum::http::header::LOCATION;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, patch, post};
use axum::Router;
use chrono::Utc;
use color_eyre::eyre::Result;
use moka::future::Cache;
use serde::Deserialize;
use sqlx::PgPool;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::error::ServerResult;
use crate::persistence::ItemState;
use crate::templates::{IndexContext, RenderedTemplate, TemplateEngine};

pub type IndexCache = Cache<(), Arc<IndexContext>>;

#[derive(Clone)]
struct ApplicationState {
    template_engine: TemplateEngine,
    pool: PgPool,
    index_cache: IndexCache,
}

pub fn build(template_engine: TemplateEngine, pool: PgPool, index_cache: IndexCache) -> Router {
    let state = ApplicationState {
        template_engine,
        pool,
        index_cache,
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
        index_cache,
        ..
    }): State<ApplicationState>,
) -> ServerResult<RenderedTemplate> {
    let now = Utc::now().date_naive();

    let context = match index_cache.get(&()).await {
        Some(ctx) => ctx,
        None => {
            let items = crate::persistence::select_items(&pool, now).await?;
            let context = Arc::new(IndexContext::from(items));

            index_cache.insert((), Arc::clone(&context)).await;

            context
        }
    };

    let rendered = template_engine.render_serialized("index.tera.html", &context)?;

    Ok(rendered)
}

#[derive(Debug, Deserialize)]
struct AddItemForm {
    content: String,
}

async fn add_item(
    State(ApplicationState {
        pool, index_cache, ..
    }): State<ApplicationState>,
    Form(AddItemForm { content }): Form<AddItemForm>,
) -> ServerResult<Response> {
    let item_uid = Uuid::new_v4();
    let now = Utc::now().naive_local();

    crate::persistence::create_item(&pool, item_uid, &content, now).await?;
    index_cache.remove(&()).await;

    tracing::info!(%item_uid, "added an item");

    Ok(redirect("/")?)
}

#[derive(Debug, Deserialize)]
struct UpdateItemRequest {
    state: ItemState,
}

async fn update_item(
    State(ApplicationState {
        pool, index_cache, ..
    }): State<ApplicationState>,
    Path(item_uid): Path<Uuid>,
    Json(request): Json<UpdateItemRequest>,
) -> ServerResult<Response> {
    crate::persistence::update_item(&pool, item_uid, request.state).await?;
    index_cache.remove(&()).await;

    tracing::info!(%item_uid, ?request, "updated an item");

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
