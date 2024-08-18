use axum::routing::get;
use axum::Router;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new().route("/", get(handler));
    let listener = TcpListener::bind("localhost:8000").await?;

    axum::serve(listener, router.into_make_service()).await?;

    Ok(())
}

async fn handler() -> &'static str {
    "Hello World!"
}
