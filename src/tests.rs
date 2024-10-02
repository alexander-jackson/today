use std::cell::RefCell;

use axum::body::Body;
use axum::http::header::{AsHeaderName, CONTENT_TYPE, COOKIE, LOCATION, SET_COOKIE};
use axum::http::{Method, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use axum_extra::extract::cookie::Key;
use color_eyre::eyre::eyre;
use http_body_util::BodyExt;
use jsonwebtoken::{DecodingKey, EncodingKey};
use sqlx::PgPool;
use tower::Service;

use crate::router::IndexCache;
use crate::templates::TemplateEngine;

const FORM_MIME_TYPE: &str = "application/x-www-form-urlencoded";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct AccountDetails {
    email_address: &'static str,
    password: &'static str,
}

impl AccountDetails {
    fn new(email_address: &'static str, password: &'static str) -> Self {
        Self {
            email_address,
            password,
        }
    }
}

impl Default for AccountDetails {
    fn default() -> Self {
        Self::new("test@test.com", "test")
    }
}

#[derive(Clone)]
struct SharedRouter {
    inner: RefCell<Router>,
}

impl SharedRouter {
    fn new(router: Router) -> Self {
        Self {
            inner: RefCell::new(router),
        }
    }

    async fn call(&self, req: Request<Body>) -> Result<Response> {
        let response = self.inner.borrow_mut().call(req).await?;

        Ok(response)
    }
}

struct AccountClient {
    router: SharedRouter,
    cookie: String,
}

impl AccountClient {
    async fn request_index(&mut self) -> Result<Response> {
        let request = Request::builder()
            .method(Method::GET)
            .uri("/")
            .header(COOKIE, &self.cookie)
            .body(Body::empty())?;

        let response = self.router.call(request).await?;

        Ok(response)
    }

    async fn create_item(&mut self) -> Result<Response> {
        let request = Request::builder()
            .method(Method::POST)
            .uri("/add")
            .header(CONTENT_TYPE, FORM_MIME_TYPE)
            .header(COOKIE, &self.cookie)
            .body(Body::from("content=Task"))?;

        let response = self.router.call(request).await?;

        Ok(response)
    }
}

fn build_router(pool: PgPool) -> Result<SharedRouter> {
    let template_engine = TemplateEngine::new()?;
    let index_cache = IndexCache::new(1);
    let cookie_key = Key::generate();
    let encoding_key = EncodingKey::from_secret(b"");
    let decoding_key = DecodingKey::from_secret(b"");

    let router = crate::router::build(
        template_engine,
        pool,
        index_cache,
        cookie_key,
        encoding_key,
        decoding_key,
    );

    Ok(SharedRouter::new(router))
}

async fn read_full_body(response: Response) -> Result<String> {
    let body = response.into_body().collect().await?.to_bytes();
    let message = String::from_utf8(body.to_vec())?;

    Ok(message)
}

fn get_response_header<'a, K: AsHeaderName>(response: &'a Response, header: K) -> Option<&'a str> {
    response.headers().get(header).and_then(|h| h.to_str().ok())
}

async fn create_account(
    router: &SharedRouter,
    AccountDetails {
        email_address,
        password,
    }: &AccountDetails,
) -> Result<()> {
    let request = Request::builder()
        .method(Method::PUT)
        .uri("/register")
        .header(CONTENT_TYPE, FORM_MIME_TYPE)
        .body(Body::from(format!(
            "email_address={email_address}&raw_password={password}"
        )))?;

    router.call(request).await?;

    Ok(())
}

async fn login_to_account<'a>(
    router: &SharedRouter,
    AccountDetails {
        email_address,
        password,
    }: &AccountDetails,
) -> Result<(Response, AccountClient)> {
    let request = Request::builder()
        .method(Method::POST)
        .uri("/login")
        .header(CONTENT_TYPE, FORM_MIME_TYPE)
        .body(Body::from(format!(
            "email_address={email_address}&raw_password={password}",
        )))?;

    let response = router.call(request).await?;
    let cookie = get_response_header(&response, SET_COOKIE)
        .ok_or_else(|| eyre!("Failed to get a cookie from the response"))?
        .to_owned();

    Ok((
        response,
        AccountClient {
            cookie,
            router: router.clone(),
        },
    ))
}

#[sqlx::test]
async fn invalid_requests_get_404s(pool: PgPool) -> Result<()> {
    let router = build_router(pool)?;
    let request = Request::builder()
        .uri("/unknown-path")
        .body(Body::empty())?;

    let response = router.call(request).await?;
    let status = response.status();

    assert_eq!(status, StatusCode::NOT_FOUND);

    Ok(())
}

#[sqlx::test]
async fn can_add_items(pool: PgPool) -> Result<()> {
    let router = build_router(pool)?;
    let account_details = AccountDetails::default();

    // Create an account
    create_account(&router, &account_details).await?;

    // Log in to the account to get a token
    let (response, mut client) = login_to_account(&router, &account_details).await?;

    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(get_response_header(&response, LOCATION), Some("/"));

    let response = client.create_item().await?;

    // Get redirected to the index page
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(get_response_header(&response, LOCATION), Some("/"));

    let response = client.request_index().await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        get_response_header(&response, CONTENT_TYPE),
        Some("text/html")
    );

    let body = read_full_body(response).await?;

    assert!(body.contains("Task"));

    Ok(())
}

#[sqlx::test]
async fn accounts_cannot_see_items_belonging_to_each_other(pool: PgPool) -> Result<()> {
    let router = build_router(pool)?;
    let account1 = AccountDetails::new("test1@test.com", "password");
    let account2 = AccountDetails::new("test2@test.com", "password");

    // Create accounts and log in to them
    create_account(&router, &account1).await?;
    create_account(&router, &account2).await?;

    let (_, mut client1) = login_to_account(&router, &account1).await?;
    let (_, mut client2) = login_to_account(&router, &account2).await?;

    // Create an item on the first account
    client1.create_item().await?;

    // Check what the other account can see
    let response = client2.request_index().await?;
    let body = read_full_body(response).await?;

    assert!(!body.contains("Task"));

    Ok(())
}
