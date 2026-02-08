use std::sync::Arc;

use axum::{
    extract::Extension,
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod db;
mod error;
mod hydronet_client;
mod routes;

use db::Database;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting Peilbeheer HHVR API server...");

    // Load configuration
    dotenvy::dotenv().ok();
    let config = config::Config::from_env()?;

    // Initialize DuckDB database
    let db = Database::new(&config.database_path)?;
    db.initialize_schema()?;

    tracing::info!("DuckDB initialized at: {}", config.database_path);

    // Build API router
    let api = Router::new()
        .route("/health", get(routes::health::health_check))
        .route("/gemalen", get(routes::gemalen::list_gemalen))
        .route("/gemalen/{code}", get(routes::gemalen::get_gemaal))
        .route("/status", get(routes::status::get_status_summary))
        .route("/status/generate", post(routes::status::generate_status))
        .route("/simulatie", post(routes::simulatie::run_simulatie));

    // Combine API with static file serving
    let app = Router::new()
        .nest("/api", api)
        .fallback_service(ServeDir::new("static").append_index_html_on_directories(true))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(Extension(Arc::new(db)))
        .layer(Extension(Arc::new(config.clone())));

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
