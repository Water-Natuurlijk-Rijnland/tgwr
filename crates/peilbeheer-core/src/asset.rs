use serde::{Deserialize, Serialize};

/// Generiek asset-type voor alle ArcGIS-lagen (gemaal, stuw, sluis, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRegistratie {
    pub layer_type: String,
    pub code: String,
    #[serde(default)]
    pub naam: Option<String>,
    #[serde(default)]
    pub lat: Option<f64>,
    #[serde(default)]
    pub lon: Option<f64>,
    #[serde(default)]
    pub extra_properties: Option<serde_json::Value>,
}
