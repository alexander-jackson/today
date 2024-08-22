use std::net::{Ipv4Addr, SocketAddrV4};

use color_eyre::eyre::Result;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

mod error;
mod persistence;
mod router;
mod templates;

use crate::templates::TemplateEngine;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().ok();

    let pool = crate::persistence::bootstrap::run().await?;
    let template_engine = TemplateEngine::new()?;

    let router = crate::router::build(template_engine, pool);

    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8000);
    let listener = TcpListener::bind(addr).await?;

    tracing::info!(?addr, "listening for incoming requests");

    axum::serve(listener, router).await?;

    Ok(())
}

#[cfg(test)]
mod tests;
