use std::net::{Ipv4Addr, SocketAddrV4};

use color_eyre::eyre::Result;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use crate::router::IndexCache;

mod error;
mod persistence;
mod router;
mod templates;
mod utils;

use crate::templates::TemplateEngine;
use crate::utils::get_env_var;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    color_eyre::install()?;
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let template_engine = TemplateEngine::new()?;
    let pool = crate::persistence::bootstrap::run().await?;
    let index_cache = IndexCache::new(32);

    let addr = get_env_var("SERVER_ADDR")
        .and_then(|v| Ok(v.parse()?))
        .unwrap_or_else(|_| SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8000));

    let router = crate::router::build(template_engine, pool, index_cache);
    let listener = TcpListener::bind(addr).await?;

    tracing::info!(?addr, "listening for incoming requests");

    axum::serve(listener, router).await?;

    Ok(())
}

#[cfg(test)]
mod tests;
