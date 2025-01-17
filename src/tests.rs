use axum::body::Body;
use axum::http::header::{AsHeaderName, CONTENT_TYPE, LOCATION};
use axum::http::{Method, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use http_body_util::BodyExt;
use serde_test::{assert_ser_tokens, Token};
use sqlx::PgPool;
use tower::Service;

use crate::persistence::Content;
use crate::router::IndexCache;
use crate::templates::TemplateEngine;

const FORM_MIME_TYPE: &str = "application/x-www-form-urlencoded";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn build_router(pool: PgPool) -> Result<Router> {
    let template_engine = TemplateEngine::new()?;
    let index_cache = IndexCache::new(1);

    let router = crate::router::build(template_engine, pool, index_cache);

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

    let request = Request::builder()
        .method(Method::POST)
        .uri("/add")
        .header(CONTENT_TYPE, FORM_MIME_TYPE)
        .body(Body::from("content=Task"))?;

    let response = router.call(request).await?;

    // Get redirected to the index page
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(get_response_header(&response, LOCATION), Some("/"));

    let request = Request::builder()
        .method(Method::GET)
        .uri("/")
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

#[test]
fn item_content_without_any_markdown_should_be_transparently_rendered() {
    let content = Content::from("very normal text".to_string());

    assert_ser_tokens(&content, &[Token::Str("very normal text")]);
}

#[test]
fn can_render_code_blocks_in_item_content() {
    let content = Content::from("some `code` block".to_string());

    assert_ser_tokens(&content, &[Token::Str("some <code>code</code> block")]);
}
