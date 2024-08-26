use axum::body::Body;
use axum::http::header::{CONTENT_TYPE, COOKIE, LOCATION, SET_COOKIE};
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
    let request = Request::builder()
        .method(Method::PUT)
        .uri("/register")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(
            "email_address=test@test.com&raw_password=password",
        ))?;

    router.call(request).await?;

    // Log in to the account to get a token
    let request = Request::builder()
        .method(Method::POST)
        .uri("/login")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(
            "email_address=test@test.com&raw_password=password",
        ))?;

    let response = router.call(request).await?;
    let cookie = response
        .headers()
        .get(SET_COOKIE)
        .ok_or_else(|| eyre!("Failed to get a cookie from the response"))?
        .to_str()?;

    let request = Request::builder()
        .method(Method::POST)
        .uri("/add")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(COOKIE, cookie)
        .body(Body::from("content=Task"))?;

    let response = router.call(request).await?;
    let status = response.status();
    let location = response
        .headers()
        .get(LOCATION)
        .and_then(|h| h.to_str().ok());

    // Get redirected to the index page
    assert_eq!(status, StatusCode::FOUND);
    assert_eq!(location, Some("/"));

    let request = Request::builder()
        .method(Method::GET)
        .uri("/")
        .header(COOKIE, cookie)
        .body(Body::empty())?;

    let response = router.call(request).await?;

    let status = response.status();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|h| h.to_str().ok());

    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, Some("text/html"));

    let body = read_full_body(response).await?;

    assert!(body.contains("Task"));

    Ok(())
}
