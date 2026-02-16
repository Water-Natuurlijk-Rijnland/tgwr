use std::sync::Arc;

use axum::{extract::Extension, extract::Path, Json};
use serde_json::{json, Value};

use crate::arcgis_client;
use crate::config::Config;
use crate::db::Database;
use crate::error::ApiError;
use crate::hydronet_client::HydronetClient;

use peilbeheer_core::gemaal::GemaalSnapshot;
use peilbeheer_core::hydronet::GeoJsonGemaal;

/// GET /api/gemalen - Lijst alle gemalen uit de database.
pub async fn list_gemalen(
    Extension(db): Extension<Arc<Database>>,
) -> Result<Json<Vec<GemaalSnapshot>>, ApiError> {
    let snapshots = db
        .get_all_snapshots()
        .map_err(|e| ApiError::Internal(e))?;

    Ok(Json(snapshots))
}

/// GET /api/gemalen/geojson - Serveer cached gemalen als GeoJSON FeatureCollection.
pub async fn get_geojson(
    Extension(db): Extension<Arc<Database>>,
) -> Result<Json<Value>, ApiError> {
    let gemalen = db
        .get_all_registraties()
        .map_err(|e| ApiError::Internal(e))?;

    let features: Vec<Value> = gemalen
        .iter()
        .filter(|g| g.lat.is_some() && g.lon.is_some())
        .map(|g| to_geojson_feature(g))
        .collect();

    Ok(Json(json!({
        "type": "FeatureCollection",
        "features": features,
    })))
}

/// POST /api/gemalen/sync - Haal gemalen op van ArcGIS en sla op in cache.
pub async fn sync_gemalen(
    Extension(db): Extension<Arc<Database>>,
) -> Result<Json<Value>, ApiError> {
    let gemalen = arcgis_client::fetch_gemalen_geojson()
        .await
        .map_err(|e| ApiError::Hydronet(e))?;

    let count = db
        .write_gemaal_registraties(&gemalen)
        .map_err(|e| ApiError::Internal(e))?;

    Ok(Json(json!({
        "status": "ok",
        "synced": count,
    })))
}

/// GET /api/gemalen/:code - Haal live data op voor een specifiek gemaal.
pub async fn get_gemaal(
    Path(code): Path<String>,
    Extension(db): Extension<Arc<Database>>,
    Extension(config): Extension<Arc<Config>>,
) -> Result<Json<Value>, ApiError> {
    // Eerst proberen uit de database
    let snapshot = db
        .get_snapshot(&code)
        .map_err(|e| ApiError::Internal(e))?;

    // Live data ophalen van Hydronet
    let client = HydronetClient::new(config.hydronet_chart_id.clone());
    let live_data = client.fetch_gemaal_data(&code).await;

    let response = json!({
        "code": code,
        "snapshot": snapshot,
        "live_data": live_data.ok(),
    });

    Ok(Json(response))
}

fn to_geojson_feature(g: &GeoJsonGemaal) -> Value {
    json!({
        "type": "Feature",
        "geometry": {
            "type": "Point",
            "coordinates": [g.lon.unwrap_or(0.0), g.lat.unwrap_or(0.0)]
        },
        "properties": {
            "code": g.code,
            "naam": g.naam,
            "capaciteit": g.capaciteit,
            "functie": g.functie,
            "soort": g.soort,
            "plaats": g.plaats,
            "gemeente": g.gemeente,
        }
    })
}
