use axum::routing::get;
use axum::Router;
use tokio::net::TcpListener;

fn build_router() -> Router {
    Router::new().route("/", get(handler))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = build_router();
    let listener = TcpListener::bind("localhost:8000").await?;

    axum::serve(listener, router.into_make_service()).await?;

    Ok(())
}

async fn handler() -> &'static str {
    "Hello World!"
}

#[cfg(test)]
mod tests;
