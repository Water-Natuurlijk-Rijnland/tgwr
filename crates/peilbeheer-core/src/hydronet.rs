use serde::{Deserialize, Serialize};

/// Een datapunt uit de Hydronet API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: Option<String>,
    pub timestamp_ms: i64,
    pub value: f64,
    #[serde(default)]
    pub status: Option<String>,
}

/// Een tijdreeks uit de Hydronet API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydronetSeries {
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_series_type")]
    pub r#type: String,
    #[serde(default)]
    pub color: String,
    pub data: Vec<DataPoint>,
}

fn default_series_type() -> String {
    "line".to_string()
}

/// Volledige respons van de Hydronet API voor een gemaal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydronetResponse {
    pub feature_identifier: String,
    pub timestamp: String,
    #[serde(default)]
    pub series: Vec<HydronetSeries>,
}

/// Gemaal code met optionele metadata uit GeoJSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoJsonGemaal {
    pub code: String,
    #[serde(default)]
    pub naam: Option<String>,
    #[serde(default)]
    pub lat: Option<f64>,
    #[serde(default)]
    pub lon: Option<f64>,
}
