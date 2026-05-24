use crate::config::loader::Config;
use axum::Router;
use tokio::net::TcpListener;
use tracing::info;

pub async fn run(
    config: Config,
    app: Router,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let address = format!("{}:{}", config.host, config.port);

    let listener = TcpListener::bind(&address).await?;

    info!(%address, "Starting HTTP server");

    axum::serve(listener, app).await?;

    Ok(())
}