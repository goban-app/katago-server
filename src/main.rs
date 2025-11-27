mod api;
mod config;
mod error;
mod katago_bot;

#[allow(dead_code)] // JSON analysis engine - experimental, not yet working
mod analysis_engine;

use crate::katago_bot::KatagoBot;
use crate::api::create_router;
use crate::config::Config;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "katago_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_file("config.toml")
        .or_else(|_| Config::from_env())
        .unwrap_or_else(|_| {
            info!("Using default configuration");
            Config::default()
        });

    info!("Starting KataGo server with config: {:?}", config);

    // Initialize KataGo bot (GTP mode)
    let bot = Arc::new(KatagoBot::new(config.katago)?);

    // Create router with CORS and tracing
    let app = create_router(bot)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("Listening on http://{}", addr);
    info!("");
    info!("API endpoints:");
    info!("  POST /api/v1/analysis      - Comprehensive position analysis");
    info!("  GET  /api/v1/health        - Health check with details");
    info!("  GET  /api/v1/version       - Server and KataGo version");
    info!("  POST /api/v1/cache/clear   - Clear neural network cache");

    axum::serve(listener, app).await?;

    Ok(())
}
