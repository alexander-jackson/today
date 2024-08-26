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

use crate::templates::TemplateEngine;

const FORM_MIME_TYPE: &str = "application/x-www-form-urlencoded";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn build_router(pool: PgPool) -> Result<Router> {
    let template_engine = TemplateEngine::new()?;
    let cookie_key = Key::generate();
    let encoding_key = EncodingKey::from_secret(b"");
    let decoding_key = DecodingKey::from_secret(b"");

    let router = crate::router::build(
        template_engine,
        pool,
        cookie_key,
        encoding_key,
        decoding_key,
    );

    Ok(router)
}

async fn read_full_body(response: Response) -> Result<String> {
    let body = response.into_body().collect().await?.to_bytes();
    let message = String::from_utf8(body.to_vec())?;

    Ok(message)
}

fn get_response_header<'a, K: AsHeaderName>(response: &'a Response, header: K) -> Option<&'a str> {
    response.headers().get(header).and_then(|h| h.to_str().ok())
}

async fn create_account(router: &mut Router) -> Result<()> {
    let request = Request::builder()
        .method(Method::PUT)
        .uri("/register")
        .header(CONTENT_TYPE, FORM_MIME_TYPE)
        .body(Body::from(
            "email_address=test@test.com&raw_password=password",
        ))?;

    router.call(request).await?;

    Ok(())
}

async fn login_to_account(router: &mut Router) -> Result<(Response, String)> {
    let request = Request::builder()
        .method(Method::POST)
        .uri("/login")
        .header(CONTENT_TYPE, FORM_MIME_TYPE)
        .body(Body::from(
            "email_address=test@test.com&raw_password=password",
        ))?;

    let response = router.call(request).await?;
    let cookie = response
        .headers()
        .get(SET_COOKIE)
        .ok_or_else(|| eyre!("Failed to get a cookie from the response"))?
        .to_str()?
        .to_owned();

    Ok((response, cookie))
}

#[sqlx::test]
async fn invalid_requests_get_404s(pool: PgPool) -> Result<()> {
    let mut router = build_router(pool)?;
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
    let mut router = build_router(pool)?;

    // Create an account
    create_account(&mut router).await?;

    // Log in to the account to get a token
    let (response, cookie) = login_to_account(&mut router).await?;

    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(get_response_header(&response, LOCATION), Some("/"));

    let request = Request::builder()
        .method(Method::POST)
        .uri("/add")
        .header(CONTENT_TYPE, FORM_MIME_TYPE)
        .header(COOKIE, &cookie)
        .body(Body::from("content=Task"))?;

    let response = router.call(request).await?;

    // Get redirected to the index page
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(get_response_header(&response, LOCATION), Some("/"));

    let request = Request::builder()
        .method(Method::GET)
        .uri("/")
        .header(COOKIE, &cookie)
        .body(Body::empty())?;

    let response = router.call(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        get_response_header(&response, CONTENT_TYPE),
        Some("text/html")
    );

    let body = read_full_body(response).await?;

    assert!(body.contains("Task"));

    Ok(())
}
