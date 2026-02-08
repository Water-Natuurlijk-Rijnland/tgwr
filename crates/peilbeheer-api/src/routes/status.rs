use std::sync::Arc;

use axum::{extract::Extension, Json};
use chrono::Utc;
use serde_json::{json, Value};

use crate::config::Config;
use crate::db::Database;
use crate::error::ApiError;

/// GET /api/status - Haal de huidige status samenvatting op.
pub async fn get_status_summary(
    Extension(db): Extension<Arc<Database>>,
) -> Result<Json<Value>, ApiError> {
    let snapshots = db
        .get_all_snapshots()
        .map_err(|e| ApiError::Internal(e))?;

    let total = snapshots.len();
    let active = snapshots
        .iter()
        .filter(|s| matches!(s.status, peilbeheer_core::gemaal::GemaalStatus::Aan))
        .count();
    let total_debiet: f64 = snapshots
        .iter()
        .filter(|s| matches!(s.status, peilbeheer_core::gemaal::GemaalStatus::Aan))
        .map(|s| s.debiet)
        .sum();

    Ok(Json(json!({
        "generated_at": Utc::now().to_rfc3339(),
        "total_stations": total,
        "active_stations": active,
        "total_debiet_m3s": (total_debiet * 1000.0).round() / 1000.0,
        "stations": snapshots,
    })))
}

/// POST /api/status/generate - Genereer nieuwe status door alle gemalen op te halen.
pub async fn generate_status(
    Extension(_db): Extension<Arc<Database>>,
    Extension(_config): Extension<Arc<Config>>,
) -> Result<Json<Value>, ApiError> {
    let generated_at = Utc::now();

    // TODO: In productie gemaal codes laden uit GeoJSON of DB
    // Voor nu returnen we een melding
    Ok(Json(json!({
        "status": "not_implemented",
        "message": "Volledige status generatie vereist gemaal codes uit GeoJSON. Gebruik de CLI tool of configureer de GeoJSON path.",
        "generated_at": generated_at.to_rfc3339(),
    })))
}
