use std::sync::Arc;

use axum::{extract::Extension, extract::Path, Json};

use crate::config::Config;
use crate::db::Database;
use crate::error::ApiError;
use crate::hydronet_client::HydronetClient;

use peilbeheer_core::gemaal::GemaalSnapshot;

/// GET /api/gemalen - Lijst alle gemalen uit de database.
pub async fn list_gemalen(
    Extension(db): Extension<Arc<Database>>,
) -> Result<Json<Vec<GemaalSnapshot>>, ApiError> {
    let snapshots = db
        .get_all_snapshots()
        .map_err(|e| ApiError::Internal(e))?;

    Ok(Json(snapshots))
}

/// GET /api/gemalen/:code - Haal live data op voor een specifiek gemaal.
pub async fn get_gemaal(
    Path(code): Path<String>,
    Extension(db): Extension<Arc<Database>>,
    Extension(config): Extension<Arc<Config>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Eerst proberen uit de database
    let snapshot = db
        .get_snapshot(&code)
        .map_err(|e| ApiError::Internal(e))?;

    // Live data ophalen van Hydronet
    let client = HydronetClient::new(config.hydronet_chart_id.clone());
    let live_data = client.fetch_gemaal_data(&code).await;

    let response = serde_json::json!({
        "code": code,
        "snapshot": snapshot,
        "live_data": live_data.ok(),
    });

    Ok(Json(response))
}
