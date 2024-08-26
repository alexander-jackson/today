use std::net::{Ipv4Addr, SocketAddrV4};

use axum_extra::extract::cookie::Key;
use color_eyre::eyre::Result;
use jsonwebtoken::{DecodingKey, EncodingKey};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

mod auth;
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
    let cookie_key = Key::from(std::env::var("COOKIE_KEY")?.as_bytes());

    let jwt_key = std::env::var("JWT_KEY")?;
    let encoding_key = EncodingKey::from_secret(jwt_key.as_bytes());
    let decoding_key = DecodingKey::from_secret(jwt_key.as_bytes());

    let router = crate::router::build(
        template_engine,
        pool,
        cookie_key,
        encoding_key,
        decoding_key,
    );

    let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8000);
    let listener = TcpListener::bind(addr).await?;

    tracing::info!(?addr, "listening for incoming requests");

    axum::serve(listener, router).await?;

    Ok(())
}

#[cfg(test)]
mod tests;
