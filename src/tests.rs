use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use axum::Router;
use http_body_util::BodyExt;
use tower_util::ServiceExt;

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
async fn it_returns_a_static_response() -> Result<()> {
    let router = build_router()?;
    let request = Request::builder().uri("/").body(Body::empty())?;

    let response = router.oneshot(request).await?;
    let status = response.status();
    let body = read_full_body(response).await?;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "Hello World!");

    Ok(())
}

#[tokio::test]
async fn invalid_requests_get_404s() -> Result<()> {
    let router = build_router()?;
    let request = Request::builder()
        .uri("/unknown-path")
        .body(Body::empty())?;

    let response = router.oneshot(request).await?;
    let status = response.status();

    assert_eq!(status, StatusCode::NOT_FOUND);

    Ok(())
}

#[tokio::test]
async fn can_render_templates() -> Result<()> {
    let router = build_router()?;
    let request = Request::builder().uri("/index").body(Body::empty())?;

    let response = router.oneshot(request).await?;
    let status = response.status();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|h| h.to_str().ok());

    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, Some("text/html"));

    let body = read_full_body(response).await?;

    assert!(body.contains("<h1>My First Heading</h1>"));
    assert!(body.contains("<p>My first paragraph.</p>"));

    Ok(())
}
