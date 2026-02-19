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
        .map_err(ApiError::Internal)?;

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

    let registratie_count = db.get_registratie_count().unwrap_or(0);

    Ok(Json(json!({
        "generated_at": Utc::now().to_rfc3339(),
        "total_stations": total,
        "active_stations": active,
        "total_debiet_m3s": (total_debiet * 1000.0).round() / 1000.0,
        "registered_gemalen": registratie_count,
        "stations": snapshots,
    })))
}

/// POST /api/status/generate - Genereer nieuwe status door alle gemalen op te halen.
pub async fn generate_status(
    Extension(db): Extension<Arc<Database>>,
    Extension(_config): Extension<Arc<Config>>,
) -> Result<Json<Value>, ApiError> {
    let generated_at = Utc::now();

    let codes: Vec<String> = db
        .get_all_registraties()
        .map_err(ApiError::Internal)?
        .into_iter()
        .map(|g| g.code)
        .collect();

    if codes.is_empty() {
        return Ok(Json(json!({
            "status": "no_data",
            "message": "Geen gemalen in cache. Gebruik POST /api/gemalen/sync om de cache te vullen.",
            "generated_at": generated_at.to_rfc3339(),
        })));
    }

    Ok(Json(json!({
        "status": "ok",
        "message": format!("{} gemaal codes beschikbaar uit cache", codes.len()),
        "gemaal_count": codes.len(),
        "generated_at": generated_at.to_rfc3339(),
    })))
}
