use std::sync::Arc;

use axum::{
    extract::Extension,
    routing::{delete, get, post, put},
    Router,
};
use peilbeheer_core::FewsConfig;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod arcgis_client;
mod auth_service;
mod config;
mod db;
mod energyzero_client;
mod error;
mod fews_client;
mod hydronet_client;
mod routes;
mod scenario_service;
mod websocket_service;

use auth_service::AuthService;
use db::Database;
use fews_client::{FewsClient, FewsSyncService};
use scenario_service::ScenarioService;
use websocket_service::WebSocketServer;

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

    // Auto-sync gemalen from ArcGIS if cache is empty
    let registratie_count = db.get_registratie_count().unwrap_or(0);
    if registratie_count == 0 {
        tracing::info!("Gemaal cache leeg, ophalen van ArcGIS...");
        match arcgis_client::fetch_gemalen_geojson().await {
            Ok(gemalen) => {
                match db.write_gemaal_registraties(&gemalen) {
                    Ok(n) => tracing::info!("Auto-sync: {n} gemalen gecached"),
                    Err(e) => tracing::warn!("Auto-sync schrijven mislukt: {e}"),
                }
            }
            Err(e) => tracing::warn!("Auto-sync ArcGIS ophalen mislukt: {e}"),
        }
    } else {
        tracing::info!("Gemaal cache bevat {registratie_count} registraties");
    }

    // Auto-sync alle ArcGIS-lagen als asset_registratie leeg is
    let asset_count = db.get_total_asset_count().unwrap_or(0);
    if asset_count == 0 {
        tracing::info!("Asset cache leeg, ophalen van alle ArcGIS-lagen...");
        for layer in &config.arcgis_layers {
            match arcgis_client::fetch_layer_assets(
                &layer.service_name,
                layer.layer_id,
                &layer.layer_type,
            )
            .await
            {
                Ok(assets) => match db.write_asset_registraties(&assets) {
                    Ok(n) => tracing::info!("Auto-sync {}: {n} assets gecached", layer.layer_type),
                    Err(e) => tracing::warn!("Auto-sync {} schrijven mislukt: {e}", layer.layer_type),
                },
                Err(e) => tracing::warn!("Auto-sync {} ophalen mislukt: {e}", layer.layer_type),
            }
        }
    } else {
        tracing::info!("Asset cache bevat {asset_count} registraties");
    }

    // Auto-sync peilgebieden: ophalen van ArcGIS → opslaan als GeoJSON → laden in DuckDB
    let peilgebied_count = db.get_peilgebied_count().unwrap_or(0);
    if peilgebied_count == 0 {
        let geojson_path = std::path::Path::new(&config.peilgebieden_geojson_path);

        // Stap 1: Als het bestand nog niet bestaat, ophalen van ArcGIS
        if !geojson_path.exists() {
            tracing::info!(
                "Peilgebieden GeoJSON niet gevonden, ophalen van ArcGIS ({})...",
                config.peilgebieden_arcgis_service
            );
            match arcgis_client::fetch_peilgebieden_to_file(
                &config.peilgebieden_arcgis_service,
                config.peilgebieden_arcgis_layer_id,
                geojson_path,
            )
            .await
            {
                Ok(n) => tracing::info!("ArcGIS: {n} peilgebieden opgeslagen naar {}", geojson_path.display()),
                Err(e) => tracing::warn!("ArcGIS peilgebieden ophalen mislukt: {e}"),
            }
        }

        // Stap 2: Laden vanuit GeoJSON-bestand in DuckDB
        if geojson_path.exists() {
            tracing::info!(
                "Peilgebieden laden vanuit {} naar DuckDB...",
                config.peilgebieden_geojson_path
            );
            match db.load_peilgebieden_from_geojson(&config.peilgebieden_geojson_path) {
                Ok(n) => tracing::info!("{n} peilgebieden geladen in DuckDB"),
                Err(e) => tracing::warn!("Peilgebieden laden in DuckDB mislukt: {e}"),
            }
        } else {
            tracing::warn!("Geen peilgebieden GeoJSON beschikbaar — kaart zal geen polygonen tonen");
        }
    } else {
        tracing::info!("Peilgebied tabel bevat {peilgebied_count} records");
    }

    // Initialize services
    let db_arc = Arc::new(db);
    let scenario_service = Arc::new(ScenarioService::new(db_arc.clone()));
    let auth_service = Arc::new(AuthService::with_default_config(db_arc.clone())?);
    let ws_server = Arc::new(WebSocketServer::new());

    // Initialize Fews client (if configured)
    let fews_config = FewsConfig {
        base_url: std::env::var("FEWS_BASE_URL").unwrap_or_else(|_| "https://fews.example.com/PI-rest".to_string()),
        filter_id: std::env::var("FEWS_FILTER_ID").unwrap_or_else(|_| "WatershedFilter".to_string()),
        api_key: std::env::var("FEWS_API_KEY").ok(),
        timeout_secs: std::env::var("FEWS_TIMEOUT")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .unwrap_or(30),
    };
    let fews_client = Arc::new(FewsClient::new(fews_config.clone()));
    let fews_sync_service = Arc::new(FewsSyncService::new(fews_client.clone(), vec![]));

    // Ensure default admin user exists
    if auth_service.ensure_default_admin()? {
        tracing::warn!("Default admin user created - username: admin, password: admin123");
    }

    tracing::info!("Scenario service initialized");
    tracing::info!("Authentication service initialized");
    tracing::info!("WebSocket server initialized (ID: {})", ws_server.server_id());
    tracing::info!("Fews client initialized (filter: {})", fews_config.filter_id);

    // Build API router
    let api = Router::new()
        .route("/health", get(routes::health::health_check))
        .route("/gemalen", get(routes::gemalen::list_gemalen))
        .route("/gemalen/geojson", get(routes::gemalen::get_geojson))
        .route("/gemalen/sync", post(routes::gemalen::sync_gemalen))
        .route("/gemalen/{code}", get(routes::gemalen::get_gemaal))
        .route("/status", get(routes::status::get_status_summary))
        .route("/status/generate", post(routes::status::generate_status))
        .route("/simulatie", post(routes::simulatie::run_simulatie))
        .route("/assets/layers", get(routes::assets::list_layers))
        .route("/assets/geojson", get(routes::assets::get_assets_geojson))
        .route("/assets/sync", post(routes::assets::sync_assets))
        .route("/peilgebieden/geojson", get(routes::peilgebieden::get_peilgebieden_geojson))
        .route("/peilgebieden/mapping", get(routes::peilgebieden::get_peilgebied_mapping))
        .route("/peilgebieden/sync", post(routes::peilgebieden::sync_peilgebieden))
        .route("/energieprijzen", get(routes::optimalisatie::get_energieprijzen))
        .route("/optimalisatie", post(routes::optimalisatie::run_optimalisatie))
        // Authentication routes
        .route("/auth/login", post(routes::auth::login))
        .route("/auth/logout", post(routes::auth::logout))
        .route("/auth/me", get(routes::auth::get_current_user))
        .route("/auth/users", get(routes::auth::list_users))
        .route("/auth/users", post(routes::auth::create_user))
        .route("/auth/users/:id", get(routes::auth::get_user))
        .route("/auth/users/:id", post(routes::auth::update_user))
        .route("/auth/users/:id/delete", post(routes::auth::delete_user))
        .route("/auth/users/:id/password", post(routes::auth::change_password))
        .route("/auth/users/:id/permissions", get(routes::auth::get_user_permissions))
        // Scenario management routes
        .route("/scenarios", get(routes::scenarios::list_scenarios))
        .route("/scenarios", post(routes::scenarios::create_scenario))
        .route("/scenarios/{id}", get(routes::scenarios::get_scenario))
        .route("/scenarios/{id}", put(routes::scenarios::update_scenario))
        .route("/scenarios/{id}", delete(routes::scenarios::delete_scenario))
        .route("/scenarios/{id}/execute", post(routes::scenarios::execute_scenario))
        .route("/scenarios/{id}/results", get(routes::scenarios::get_scenario_results))
        .route("/scenarios/{id}/clone", post(routes::scenarios::clone_scenario))
        // WebSocket routes
        .route("/ws", get(routes::websocket::websocket_handler))
        .route("/ws/status", get(routes::websocket::ws_status))
        // Fews integration routes
        .route("/fews/timeseries", get(routes::fews::get_time_series))
        .route("/fews/locations", get(routes::fews::get_locations))
        .route("/fews/parameters", get(routes::fews::get_parameters))
        .route("/fews/modules", get(routes::fews::get_module_instances))
        .route("/fews/sync", post(routes::fews::sync_fews))
        .route("/fews/ping", get(routes::fews::ping_fews))
        .route("/fews/status", get(routes::fews::fews_status))
        .route("/fews/config", get(routes::fews::get_sync_configs));

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
        .layer(Extension(db_arc))
        .layer(Extension(Arc::new(config.clone())))
        .layer(Extension(scenario_service))
        .layer(Extension(auth_service))
        .layer(Extension(ws_server))
        .layer(Extension(fews_client))
        .layer(Extension(fews_sync_service));

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
