use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use http_body_util::BodyExt;
use tower_util::ServiceExt;

use crate::build_app;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

async fn get_body(response: Response) -> Result<String> {
    let body = response.into_body().collect().await?.to_bytes();
    let message = String::from_utf8(body.to_vec())?;

    Ok(message)
}

#[tokio::test]
async fn it_returns_a_static_response() -> Result<()> {
    let router = build_app();
    let request = Request::builder().uri("/").body(Body::empty())?;

    let response = router.oneshot(request).await?;
    let status = response.status();
    let body = get_body(response).await?;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "Hello World!");

    Ok(())
}

#[tokio::test]
async fn invalid_requests_get_404s() -> Result<()> {
    let router = build_app();
    let request = Request::builder()
        .uri("/unknown-path")
        .body(Body::empty())?;

    let response = router.oneshot(request).await?;
    let status = response.status();

    assert_eq!(status, StatusCode::NOT_FOUND);

    Ok(())
}
