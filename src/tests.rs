use axum::body::Body;
use axum::http::header::{CONTENT_TYPE, LOCATION};
use axum::http::{Method, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use http_body_util::BodyExt;
use tower::Service;

use crate::templates::TemplateEngine;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn build_router() -> Result<Router> {
    let template_engine = TemplateEngine::new()?;
    let router = crate::build_router(template_engine);

    Ok(router)
}

async fn read_full_body(response: Response) -> Result<String> {
    let body = response.into_body().collect().await?.to_bytes();
    let message = String::from_utf8(body.to_vec())?;

    Ok(message)
}

#[tokio::test]
async fn can_view_the_index_page() -> Result<()> {
    let mut router = build_router()?;
    let request = Request::builder().uri("/").body(Body::empty())?;

    let response = router.call(request).await?;
    let status = response.status();

    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|h| h.to_str().ok());

    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, Some("text/html"));

    let body = read_full_body(response).await?;

    assert!(body.contains("<h1>Today</h1>"));

    Ok(())
}

#[tokio::test]
async fn invalid_requests_get_404s() -> Result<()> {
    let mut router = build_router()?;
    let request = Request::builder()
        .uri("/unknown-path")
        .body(Body::empty())?;

    let response = router.call(request).await?;
    let status = response.status();

    assert_eq!(status, StatusCode::NOT_FOUND);

    Ok(())
}

#[tokio::test]
async fn can_add_items() -> Result<()> {
    let mut router = build_router()?;
    let request = Request::builder()
        .method(Method::POST)
        .uri("/add")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
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
