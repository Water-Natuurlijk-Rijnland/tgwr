use peilbeheer_core::hydronet::{DataPoint, GeoJsonGemaal, HydronetResponse, HydronetSeries};
use reqwest::Client;
use serde_json::Value;

const HYDRONET_BASE_URL: &str =
    "https://watercontrolroom.hydronet.com/service/efsserviceprovider/api";
#[allow(dead_code)]
const API_DELAY_MS: u64 = 150;

/// HTTP client voor de Hydronet Water Control Room API.
pub struct HydronetClient {
    chart_id: String,
    client: Client,
}

impl HydronetClient {
    pub fn new(chart_id: String) -> Self {
        Self {
            chart_id,
            client: Client::new(),
        }
    }

    /// Haal data op voor een specifiek gemaal.
    pub async fn fetch_gemaal_data(
        &self,
        feature_identifier: &str,
    ) -> Result<HydronetResponse, String> {
        let url = format!("{}/chart/{}", HYDRONET_BASE_URL, self.chart_id);

        let response = self
            .client
            .get(&url)
            .query(&[("featureIdentifier", feature_identifier)])
            .header("User-Agent", "Mozilla/5.0 (compatible; PeilbeheerHHVR/1.0)")
            .header("Accept", "application/json, text/plain, */*")
            .header("Referer", "https://rijnland.maps.arcgis.com/")
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }

        let text = response
            .text()
            .await
            .map_err(|e| format!("Read body failed: {}", e))?;

        // Probeer als JSON te parsen
        if let Ok(json) = serde_json::from_str::<Value>(&text) {
            return parse_json_response(feature_identifier, &json);
        }

        // Fallback: parse Highcharts config uit HTML
        parse_highcharts_html(feature_identifier, &text)
    }

    /// Rate-limited delay tussen requests.
    #[allow(dead_code)]
    pub async fn delay() {
        tokio::time::sleep(std::time::Duration::from_millis(API_DELAY_MS)).await;
    }

    /// Laad gemaal codes uit een GeoJSON bestand.
    #[allow(dead_code)]
    pub fn load_gemaal_codes_from_geojson(path: &str) -> Result<Vec<GeoJsonGemaal>, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Read GeoJSON failed: {}", e))?;

        let data: Value =
            serde_json::from_str(&content).map_err(|e| format!("Parse GeoJSON failed: {}", e))?;

        let features = data["features"]
            .as_array()
            .ok_or("No features array in GeoJSON")?;

        let mut gemalen = Vec::new();
        for feature in features {
            let attrs = &feature["attributes"];
            if let Some(code) = attrs["CODE"].as_str() {
                gemalen.push(GeoJsonGemaal {
                    code: code.to_string(),
                    naam: attrs["NAAM"].as_str().map(String::from),
                    lat: attrs["LAT"].as_f64(),
                    lon: attrs["LON"].as_f64(),
                });
            }
        }

        Ok(gemalen)
    }
}

fn parse_json_response(
    feature_identifier: &str,
    json: &Value,
) -> Result<HydronetResponse, String> {
    let series = json["series"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|s| {
            let data = s["data"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|point| {
                    let timestamp_ms = point["x"].as_i64().or(point["timestamp_ms"].as_i64())?;
                    let value = point["y"].as_f64().or(point["value"].as_f64())?;

                    let status = if value > 0.001 {
                        Some("aan".to_string())
                    } else {
                        Some("uit".to_string())
                    };

                    Some(DataPoint {
                        timestamp: chrono::DateTime::from_timestamp_millis(timestamp_ms)
                            .map(|dt| dt.to_rfc3339()),
                        timestamp_ms,
                        value,
                        status,
                    })
                })
                .collect();

            HydronetSeries {
                name: s["name"].as_str().unwrap_or("").to_string(),
                r#type: s["type"].as_str().unwrap_or("line").to_string(),
                color: s["color"].as_str().unwrap_or("").to_string(),
                data,
            }
        })
        .collect();

    Ok(HydronetResponse {
        feature_identifier: feature_identifier.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        series,
    })
}

fn parse_highcharts_html(
    feature_identifier: &str,
    html: &str,
) -> Result<HydronetResponse, String> {
    // Zoek Highcharts.chart('container', { ... });
    let start_marker = "Highcharts.chart('container',";
    let start = html
        .find(start_marker)
        .or_else(|| html.find("Highcharts.chart(\"container\","))
        .ok_or("Highcharts config not found in HTML")?;

    let json_start = start + start_marker.len();

    // Vind het einde van de config (matching brace)
    let bytes = html.as_bytes();
    let mut depth = 0;
    let mut json_end = json_start;

    for (i, &b) in bytes[json_start..].iter().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    json_end = json_start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    if depth != 0 {
        return Err("Could not find matching brace in Highcharts config".to_string());
    }

    let config_str = &html[json_start..json_end];
    let config: Value =
        serde_json::from_str(config_str).map_err(|e| format!("Parse Highcharts JSON: {}", e))?;

    parse_json_response(feature_identifier, &config)
}
