use std::path::Path;

use peilbeheer_core::asset::AssetRegistratie;
use peilbeheer_core::hydronet::GeoJsonGemaal;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

const ARCGIS_BASE: &str = "https://rijnland.enl-mcs.nl/arcgis/rest/services";
const PAGE_SIZE: u32 = 1000;

#[derive(Debug, Deserialize)]
struct ArcGisResponse {
    features: Vec<ArcGisFeature>,
    #[serde(default)]
    exceeded_transfer_limit: bool,
}

#[derive(Debug, Deserialize)]
struct ArcGisFeature {
    geometry: Option<ArcGisGeometry>,
    properties: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ArcGisGeometry {
    coordinates: Option<Vec<f64>>,
}

/// Haal alle gemalen op van de ArcGIS MapServer met paginatie.
pub async fn fetch_gemalen_geojson() -> Result<Vec<GeoJsonGemaal>, String> {
    let client = Client::new();
    let url = format!("{ARCGIS_BASE}/Gemaal/MapServer/0/query");
    let mut all_gemalen = Vec::new();
    let mut offset: u32 = 0;

    loop {
        let response = client
            .get(&url)
            .query(&[
                ("where", "1=1"),
                ("outFields", "*"),
                ("f", "geojson"),
                ("resultOffset", &offset.to_string()),
                ("resultRecordCount", &PAGE_SIZE.to_string()),
            ])
            .header(
                "User-Agent",
                "Mozilla/5.0 (compatible; PeilbeheerHHVR/1.0)",
            )
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("ArcGIS request failed: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("ArcGIS HTTP {}", response.status()));
        }

        let body: ArcGisResponse = response
            .json()
            .await
            .map_err(|e| format!("ArcGIS parse failed: {e}"))?;

        let page_count = body.features.len();

        for feature in body.features {
            let props = match feature.properties {
                Some(p) => p,
                None => continue,
            };

            let code = match props.get("CODE").and_then(|v| v.as_str()) {
                Some(c) if !c.is_empty() => c.to_string(),
                _ => continue,
            };

            let lat = props.get("LATITUDE").and_then(|v| v.as_f64());
            let lon = props.get("LONGITUDE").and_then(|v| v.as_f64());

            // Prefer LATITUDE/LONGITUDE properties, fall back to geometry coordinates
            let (lat, lon) = match (lat, lon) {
                (Some(lat), Some(lon)) => (Some(lat), Some(lon)),
                _ => {
                    if let Some(geom) = &feature.geometry {
                        if let Some(coords) = &geom.coordinates {
                            if coords.len() >= 2 {
                                (Some(coords[1]), Some(coords[0]))
                            } else {
                                (None, None)
                            }
                        } else {
                            (None, None)
                        }
                    } else {
                        (None, None)
                    }
                }
            };

            all_gemalen.push(GeoJsonGemaal {
                code,
                naam: props.get("NAAM").and_then(|v| v.as_str()).map(String::from),
                lat,
                lon,
                capaciteit: props.get("MAXIMALECAPACITEIT").and_then(|v| v.as_f64()),
                functie: props.get("FUNCTIEGEMAAL").and_then(|v| v.as_str()).map(String::from),
                soort: props.get("SOORTGEMAAL").and_then(|v| v.as_str()).map(String::from),
                plaats: props.get("PLAATS").and_then(|v| v.as_str()).map(String::from),
                gemeente: props.get("GEMEENTENAAM").and_then(|v| v.as_str()).map(String::from),
            });
        }

        if !body.exceeded_transfer_limit || page_count == 0 {
            break;
        }
        offset += PAGE_SIZE;
    }

    tracing::info!("ArcGIS: {} gemalen opgehaald", all_gemalen.len());
    Ok(all_gemalen)
}

/// Haal assets op van een willekeurige ArcGIS MapServer-laag.
pub async fn fetch_layer_assets(
    service_name: &str,
    layer_id: u32,
    layer_type: &str,
) -> Result<Vec<AssetRegistratie>, String> {
    let client = Client::new();
    let url = format!("{ARCGIS_BASE}/{service_name}/MapServer/{layer_id}/query");
    let mut all_assets = Vec::new();
    let mut offset: u32 = 0;

    loop {
        let response = client
            .get(&url)
            .query(&[
                ("where", "1=1"),
                ("outFields", "*"),
                ("f", "geojson"),
                ("resultOffset", &offset.to_string()),
                ("resultRecordCount", &PAGE_SIZE.to_string()),
            ])
            .header(
                "User-Agent",
                "Mozilla/5.0 (compatible; PeilbeheerHHVR/1.0)",
            )
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("ArcGIS request failed for {service_name}: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("ArcGIS HTTP {} for {service_name}", response.status()));
        }

        let body: ArcGisResponse = response
            .json()
            .await
            .map_err(|e| format!("ArcGIS parse failed for {service_name}: {e}"))?;

        let page_count = body.features.len();

        for feature in body.features {
            let props = match feature.properties {
                Some(p) => p,
                None => continue,
            };

            let code = match props.get("CODE").and_then(|v| v.as_str()) {
                Some(c) if !c.is_empty() => c.to_string(),
                _ => continue,
            };

            let naam = props.get("NAAM").and_then(|v| v.as_str()).map(String::from);

            // Extract coordinates: prefer properties, fall back to geometry
            let prop_lat = props.get("LATITUDE").and_then(|v| v.as_f64());
            let prop_lon = props.get("LONGITUDE").and_then(|v| v.as_f64());

            let (lat, lon) = match (prop_lat, prop_lon) {
                (Some(lat), Some(lon)) => (Some(lat), Some(lon)),
                _ => {
                    if let Some(geom) = &feature.geometry {
                        if let Some(coords) = &geom.coordinates {
                            if coords.len() >= 2 {
                                (Some(coords[1]), Some(coords[0]))
                            } else {
                                (None, None)
                            }
                        } else {
                            (None, None)
                        }
                    } else {
                        (None, None)
                    }
                }
            };

            // Collect extra properties (everything except CODE, NAAM, LATITUDE, LONGITUDE)
            let extra = if let Some(obj) = props.as_object() {
                let filtered: serde_json::Map<String, Value> = obj
                    .iter()
                    .filter(|(k, _)| !matches!(k.as_str(), "CODE" | "NAAM" | "LATITUDE" | "LONGITUDE" | "OBJECTID"))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                if filtered.is_empty() {
                    None
                } else {
                    Some(Value::Object(filtered))
                }
            } else {
                None
            };

            all_assets.push(AssetRegistratie {
                layer_type: layer_type.to_string(),
                code,
                naam,
                lat,
                lon,
                extra_properties: extra,
            });
        }

        if !body.exceeded_transfer_limit || page_count == 0 {
            break;
        }
        offset += PAGE_SIZE;
    }

    tracing::info!("ArcGIS: {} {} assets opgehaald", all_assets.len(), layer_type);
    Ok(all_assets)
}

/// Haal peilgebieden (polygonen) op van ArcGIS en sla op als GeoJSON-bestand.
///
/// Gebruikt paginatie om alle features op te halen. Het resultaat is een
/// GeoJSON FeatureCollection die direct door DuckDB ST_Read geladen kan worden.
pub async fn fetch_peilgebieden_to_file(
    service_name: &str,
    layer_id: u32,
    output_path: &Path,
) -> Result<usize, String> {
    let client = Client::new();
    let url = format!("{ARCGIS_BASE}/{service_name}/MapServer/{layer_id}/query");
    let mut all_features: Vec<Value> = Vec::new();
    let mut offset: u32 = 0;

    loop {
        tracing::info!(
            "ArcGIS peilgebieden: ophalen offset={offset} (tot nu toe {} features)",
            all_features.len()
        );

        let response = client
            .get(&url)
            .query(&[
                ("where", "1=1"),
                ("outFields", "*"),
                ("f", "geojson"),
                ("resultOffset", &offset.to_string()),
                ("resultRecordCount", &PAGE_SIZE.to_string()),
            ])
            .header(
                "User-Agent",
                "Mozilla/5.0 (compatible; PeilbeheerHHVR/1.0)",
            )
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| format!("ArcGIS peilgebieden request failed: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("ArcGIS peilgebieden HTTP {}", response.status()));
        }

        let body: Value = response
            .json()
            .await
            .map_err(|e| format!("ArcGIS peilgebieden parse failed: {e}"))?;

        let features = body
            .get("features")
            .and_then(|f| f.as_array())
            .cloned()
            .unwrap_or_default();

        let page_count = features.len();
        all_features.extend(features);

        let exceeded = body
            .get("exceededTransferLimit")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !exceeded || page_count == 0 {
            break;
        }
        offset += PAGE_SIZE;
    }

    let total = all_features.len();
    tracing::info!("ArcGIS: {total} peilgebieden opgehaald, opslaan naar {}", output_path.display());

    let collection = serde_json::json!({
        "type": "FeatureCollection",
        "features": all_features,
    });

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Kan data-directory niet aanmaken: {e}"))?;
    }

    std::fs::write(output_path, serde_json::to_string(&collection).unwrap())
        .map_err(|e| format!("Kan GeoJSON-bestand niet schrijven: {e}"))?;

    Ok(total)
}
