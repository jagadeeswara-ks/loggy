//! Loggy - Main entry point

use loggy::init;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "loggy=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    tracing::info!("Starting Loggy - Docker Observability Platform");
    
    // Initialize and run
    let state = init().await?;
    loggy::run(state).await?;
    
    Ok(())
}
