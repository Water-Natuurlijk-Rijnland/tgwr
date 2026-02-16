use std::sync::Arc;

use axum::{
    extract::Extension,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::arcgis_client;
use crate::config::Config;
use crate::db::Database;

/// GET /api/peilgebieden/geojson — retourneert de volledige FeatureCollection (cached).
pub async fn get_peilgebieden_geojson(Extension(db): Extension<Arc<Database>>) -> Response {
    match db.get_all_peilgebieden_geojson() {
        Ok(geojson) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/geo+json"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            geojson,
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Peilgebieden GeoJSON ophalen mislukt: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Peilgebieden ophalen mislukt",
            )
                .into_response()
        }
    }
}

/// GET /api/peilgebieden/mapping — retourneert {gemaal_code: peilgebied_code} mapping.
pub async fn get_peilgebied_mapping(Extension(db): Extension<Arc<Database>>) -> Response {
    match db.get_gemaal_peilgebied_mapping() {
        Ok(mapping) => Json(mapping).into_response(),
        Err(e) => {
            tracing::error!("Peilgebied mapping ophalen mislukt: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Mapping ophalen mislukt",
            )
                .into_response()
        }
    }
}

/// POST /api/peilgebieden/sync — ophalen van ArcGIS, opslaan als bestand, laden in DuckDB.
pub async fn sync_peilgebieden(
    Extension(config): Extension<Arc<Config>>,
    Extension(db): Extension<Arc<Database>>,
) -> Response {
    let geojson_path = std::path::Path::new(&config.peilgebieden_geojson_path);

    // Stap 1: Ophalen van ArcGIS en opslaan als bestand
    let fetch_count = match arcgis_client::fetch_peilgebieden_to_file(
        &config.peilgebieden_arcgis_service,
        config.peilgebieden_arcgis_layer_id,
        geojson_path,
    )
    .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("Peilgebieden sync ArcGIS mislukt: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response();
        }
    };

    // Stap 2: Laden in DuckDB (vervangt bestaande data)
    match db.reload_peilgebieden_from_geojson(&config.peilgebieden_geojson_path) {
        Ok(n) => Json(json!({
            "status": "ok",
            "fetched_from_arcgis": fetch_count,
            "loaded_in_db": n,
        }))
        .into_response(),
        Err(e) => {
            tracing::error!("Peilgebieden laden in DuckDB mislukt: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("DuckDB laden mislukt: {e}"),
            )
                .into_response()
        }
    }
}
