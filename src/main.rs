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
mod utils;

use crate::templates::TemplateEngine;
use crate::utils::get_env_var;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().ok();

    let pool = crate::persistence::bootstrap::run().await?;
    let template_engine = TemplateEngine::new()?;
    let cookie_key = Key::from(get_env_var("COOKIE_KEY")?.as_bytes());

    let jwt_key = get_env_var("JWT_KEY")?;
    let encoding_key = EncodingKey::from_secret(jwt_key.as_bytes());
    let decoding_key = DecodingKey::from_secret(jwt_key.as_bytes());

    let router = crate::router::build(
        template_engine,
        pool,
        cookie_key,
        encoding_key,
        decoding_key,
    );

    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8000);
    let listener = TcpListener::bind(addr).await?;

    tracing::info!(?addr, "listening for incoming requests");

    axum::serve(listener, router).await?;

    Ok(())
}

#[cfg(test)]
mod tests;
