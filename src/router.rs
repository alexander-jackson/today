use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Form, FromRef, Json, Path, State};
use axum::http::header::LOCATION;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, patch, post, put};
use axum::Router;
use axum_extra::extract::cookie::{Cookie, Key};
use axum_extra::extract::PrivateCookieJar;
use chrono::Utc;
use color_eyre::eyre::Result;
use color_eyre::Report;
use jsonwebtoken::{DecodingKey, EncodingKey, Header};
use moka::future::Cache;
use serde::Deserialize;
use sqlx::PgPool;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::auth::Account;
use crate::error::ServerResult;
use crate::persistence::account::{EmailAddress, HashedPassword};
use crate::persistence::ItemState;
use crate::templates::{IndexContext, RenderedTemplate, TemplateEngine};

pub type IndexCache = Cache<Uuid, Arc<IndexContext>>;

#[derive(Clone)]
struct ApplicationState {
    template_engine: TemplateEngine,
    pool: PgPool,
    index_cache: IndexCache,
    cookie_key: Key,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl FromRef<ApplicationState> for Key {
    fn from_ref(input: &ApplicationState) -> Self {
        input.cookie_key.clone()
    }
}

impl FromRef<ApplicationState> for DecodingKey {
    fn from_ref(input: &ApplicationState) -> Self {
        input.decoding_key.clone()
    }
}

pub fn build(
    template_engine: TemplateEngine,
    pool: PgPool,
    index_cache: IndexCache,
    cookie_key: Key,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
) -> Router {
    let state = ApplicationState {
        template_engine,
        pool,
        index_cache,
        cookie_key,
        encoding_key,
        decoding_key,
    };

    Router::new()
        .route("/", get(templated))
        .route("/register", put(register))
        .route("/login", get(login).post(handle_login))
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
    account: Account,
) -> ServerResult<RenderedTemplate> {
    let now = Utc::now().date_naive();
    let account_uid = account.account_uid;

    let context = match index_cache.get(&account_uid).await {
        Some(ctx) => ctx,
        None => {
            let items = crate::persistence::select_items(&pool, account_uid, now).await?;
            let context = Arc::new(IndexContext::from(items));

            index_cache.insert(account_uid, Arc::clone(&context)).await;

            context
        }
    };

    let rendered = template_engine.render_serialized("index.tera.html", &context)?;

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

async fn login(
    State(ApplicationState {
        template_engine, ..
    }): State<ApplicationState>,
) -> ServerResult<RenderedTemplate> {
    let rendered = template_engine.render_contextless("login.tera.html")?;

    Ok(rendered)
}

async fn handle_login(
    State(ApplicationState {
        pool, encoding_key, ..
    }): State<ApplicationState>,
    cookies: PrivateCookieJar,
    Form(Registration {
        email_address,
        raw_password,
    }): Form<Registration>,
) -> ServerResult<(PrivateCookieJar, Response)> {
    let email_address = EmailAddress::from(email_address);

    let Some(account) =
        crate::persistence::account::fetch_account_by_email(&pool, &email_address).await?
    else {
        return Ok((
            cookies,
            Response::builder().status(404).body(Body::empty()).unwrap(),
        ));
    };

    if !bcrypt::verify(raw_password, &account.password).map_err(Report::from)? {
        return Ok((
            cookies,
            Response::builder().status(403).body(Body::empty()).unwrap(),
        ));
    }

    let claims = Account::new(account.account_uid);

    let header = Header::default();
    let token = jsonwebtoken::encode(&header, &claims, &encoding_key).map_err(Report::from)?;

    Ok((cookies.add(Cookie::new("token", token)), redirect("/")?))
}

#[derive(Debug, Deserialize)]
struct AddItemForm {
    content: String,
}

async fn add_item(
    State(ApplicationState {
        pool, index_cache, ..
    }): State<ApplicationState>,
    account: Account,
    Form(AddItemForm { content }): Form<AddItemForm>,
) -> ServerResult<Response> {
    let account_uid = account.account_uid;

    let item_uid = Uuid::new_v4();
    let now = Utc::now().naive_local();

    crate::persistence::create_item(&pool, account_uid, item_uid, &content, now).await?;
    index_cache.remove(&account_uid).await;

    tracing::info!(%account_uid, %item_uid, "added an item");

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
    account: Account,
    Json(request): Json<UpdateItemRequest>,
) -> ServerResult<Response> {
    let account_uid = account.account_uid;

    crate::persistence::update_item(&pool, account_uid, item_uid, request.state).await?;
    index_cache.remove(&account_uid).await;

    tracing::info!(%account_uid, %item_uid, ?request, "updated an item");

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
