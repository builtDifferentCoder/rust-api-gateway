use rust_api_gateway::server;
use tracing::error;

#[tokio::main]
async fn main() {
    if let Err(error) = server::run_server().await {
        error!(%error, "Server error");
        std::process::exit(1);
    }
}
