use std::sync::Arc;

use axum::{extract::Extension, extract::Query, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::arcgis_client;
use crate::config::Config;
use crate::db::Database;
use crate::error::ApiError;

#[derive(Debug, Deserialize)]
pub struct LayersQuery {
    pub layers: Option<String>,
}

/// GET /api/assets/layers - Lijst van geconfigureerde lagen met metadata.
pub async fn list_layers(
    Extension(config): Extension<Arc<Config>>,
    Extension(db): Extension<Arc<Database>>,
) -> Result<Json<Value>, ApiError> {
    let mut layers = Vec::new();

    for layer in &config.arcgis_layers {
        let count = db
            .get_assets_by_layer(&layer.layer_type)
            .map(|a| a.len())
            .unwrap_or(0);

        layers.push(json!({
            "layer_type": layer.layer_type,
            "display_label": layer.display_label,
            "color": layer.color,
            "icon_svg": layer.icon_svg,
            "default_visible": layer.default_visible,
            "count": count,
        }));
    }

    Ok(Json(json!(layers)))
}

/// GET /api/assets/geojson?layers=gemaal,stuw - GeoJSON FeatureCollection.
pub async fn get_assets_geojson(
    Query(query): Query<LayersQuery>,
    Extension(config): Extension<Arc<Config>>,
    Extension(db): Extension<Arc<Database>>,
) -> Result<Json<Value>, ApiError> {
    let layer_types: Option<Vec<&str>> = query
        .layers
        .as_ref()
        .map(|l| l.split(',').map(|s| s.trim()).collect());

    let assets = db
        .get_all_assets(layer_types.as_deref())
        .map_err(|e| ApiError::Internal(e))?;

    // Build a lookup of layer configs for color/icon
    let layer_map: std::collections::HashMap<&str, &crate::config::ArcgisLayerConfig> = config
        .arcgis_layers
        .iter()
        .map(|l| (l.layer_type.as_str(), l))
        .collect();

    let features: Vec<Value> = assets
        .iter()
        .filter(|a| a.lat.is_some() && a.lon.is_some())
        .map(|a| {
            let layer_cfg = layer_map.get(a.layer_type.as_str());
            json!({
                "type": "Feature",
                "geometry": {
                    "type": "Point",
                    "coordinates": [a.lon.unwrap_or(0.0), a.lat.unwrap_or(0.0)]
                },
                "properties": {
                    "code": a.code,
                    "naam": a.naam,
                    "layer_type": a.layer_type,
                    "display_label": layer_cfg.map(|c| c.display_label.as_str()).unwrap_or(&a.layer_type),
                    "color": layer_cfg.map(|c| c.color.as_str()).unwrap_or("#666"),
                    "icon_svg": layer_cfg.map(|c| c.icon_svg.as_str()).unwrap_or(""),
                    "extra_properties": a.extra_properties,
                }
            })
        })
        .collect();

    Ok(Json(json!({
        "type": "FeatureCollection",
        "features": features,
    })))
}

/// POST /api/assets/sync - Sync alle lagen van ArcGIS.
pub async fn sync_assets(
    Extension(config): Extension<Arc<Config>>,
    Extension(db): Extension<Arc<Database>>,
) -> Result<Json<Value>, ApiError> {
    let mut results = Vec::new();

    for layer in &config.arcgis_layers {
        match arcgis_client::fetch_layer_assets(
            &layer.service_name,
            layer.layer_id,
            &layer.layer_type,
        )
        .await
        {
            Ok(assets) => {
                let count = db
                    .write_asset_registraties(&assets)
                    .map_err(|e| ApiError::Internal(e))?;
                results.push(json!({
                    "layer_type": layer.layer_type,
                    "synced": count,
                }));
            }
            Err(e) => {
                tracing::warn!("Sync {} mislukt: {e}", layer.layer_type);
                results.push(json!({
                    "layer_type": layer.layer_type,
                    "error": e,
                }));
            }
        }
    }

    Ok(Json(json!({
        "status": "ok",
        "results": results,
    })))
}
