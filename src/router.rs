use axum::body::Body;
use axum::extract::{Form, Json, Path, State};
use axum::http::header::LOCATION;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, patch, post, put};
use axum::Router;
use chrono::Utc;
use color_eyre::eyre::{eyre, Result};
use serde::Deserialize;
use sqlx::PgPool;
use tera::Context;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::error::ServerResult;
use crate::persistence::account::{EmailAddress, HashedPassword};
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
        .route("/register", put(register))
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
struct Registration {
    email_address: String,
    raw_password: String,
}

async fn register(
    State(ApplicationState { pool, .. }): State<ApplicationState>,
    Form(Registration {
        email_address,
        raw_password,
    }): Form<Registration>,
) -> ServerResult<Response> {
    let account_uid = Uuid::new_v4();
    let email_address = EmailAddress::from(email_address);
    let hashed_password = HashedPassword::from_raw(&raw_password)?;

    crate::persistence::account::create_account(
        &pool,
        account_uid,
        &email_address,
        &hashed_password,
    )
    .await?;

    Ok(success()?)
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

    // Select the oldest account, since that's the only one that is presumed to exist
    let account_uid = crate::persistence::account::select_oldest(&pool)
        .await?
        .ok_or_else(|| eyre!("No accounts currently exist to create items for"))?;

    crate::persistence::create_item(&pool, account_uid, item_uid, &content, now).await?;

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
