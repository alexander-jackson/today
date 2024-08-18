use axum::extract::Request;
use axum::routing::get;
use axum::Router;
use color_eyre::eyre::eyre;
use error::ServerResult;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

mod error;

fn build_app() -> Router {
    Router::new()
        .route("/", get(handler))
        .layer(TraceLayer::new_for_http())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    color_eyre::install()?;
    tracing_subscriber::fmt().init();

    let router = build_app();
    let listener = TcpListener::bind("localhost:8000").await?;

    axum::serve(listener, router).await?;

    Ok(())
}

async fn handler(request: Request) -> ServerResult<&'static str> {
    match request
        .headers()
        .get("x-testing")
        .and_then(|h| h.to_str().ok())
    {
        Some("throw-error") => Err(eyre!("Something went wrong!").into()),
        _ => Ok("Hello World!"),
    }
}

#[cfg(test)]
mod tests;
