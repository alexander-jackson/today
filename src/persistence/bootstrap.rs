use color_eyre::eyre::Result;
use sqlx::PgPool;
use sqlx_bootstrap::{ApplicationConfig, BootstrapConfig, ConnectionConfig, RootConfig};

use crate::utils::get_env_var;

pub async fn run() -> Result<PgPool> {
    let root_username = get_env_var("ROOT_USERNAME")?;
    let root_password = get_env_var("ROOT_PASSWORD")?;
    let root_database = get_env_var("ROOT_DATABASE")?;

    let app_username = get_env_var("APP_USERNAME")?;
    let app_password = get_env_var("APP_PASSWORD")?;
    let app_database = get_env_var("APP_DATABASE")?;

    let host = get_env_var("DATABASE_HOST")?;
    let port = get_env_var("DATABASE_PORT")?.parse()?;

    let root_config = RootConfig::new(&root_username, &root_password, &root_database);
    let app_config = ApplicationConfig::new(&app_username, &app_password, &app_database);
    let conn_config = ConnectionConfig::new(&host, port);

    let config = BootstrapConfig::new(root_config, app_config, conn_config);
    let pool = config.bootstrap().await?;

    sqlx::migrate!().run(&pool).await?;

    Ok(pool)
}
