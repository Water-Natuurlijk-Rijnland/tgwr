use serde::{Deserialize, Serialize};

fn api_base() -> String {
    // Use localhost with explicit port to match page origin
    // This avoids mixed origin issues between localhost and 127.0.0.1
    "http://localhost:3000/api".to_string()
}

// ── Domain types (match API JSON responses) ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GemaalStatus {
    Aan,
    Uit,
    Onbekend,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendStrength {
    Strong,
    Moderate,
    Weak,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TrendInfo {
    pub slope_per_hour: f64,
    pub direction: TrendDirection,
    pub r_squared: f64,
    pub strength: TrendStrength,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GemaalTrends {
    #[serde(rename = "30_min")]
    pub min_30: Option<TrendInfo>,
    #[serde(rename = "60_min")]
    pub min_60: Option<TrendInfo>,
    #[serde(rename = "180_min")]
    pub min_180: Option<TrendInfo>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GemaalSnapshot {
    pub gemaal_code: String,
    pub status: GemaalStatus,
    pub debiet: f64,
    pub last_update: Option<String>,
    pub generated_at: Option<String>,
    pub trends: Option<GemaalTrends>,
    pub error: Option<String>,
}

// ── Status response ──

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StatusResponse {
    pub generated_at: String,
    pub total_stations: usize,
    pub active_stations: usize,
    pub total_debiet_m3s: f64,
    #[serde(default)]
    pub registered_gemalen: usize,
    pub stations: Vec<GemaalSnapshot>,
}

// ── Gemaal detail / live data response ──

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GemaalDetailResponse {
    pub code: String,
    pub snapshot: Option<GemaalSnapshot>,
    pub live_data: Option<LiveData>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LiveData {
    pub series: Vec<LiveSeries>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LiveSeries {
    pub name: String,
    pub data: Vec<LiveDataPoint>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LiveDataPoint {
    pub timestamp_ms: i64,
    pub value: f64,
    #[serde(default)]
    pub status: Option<String>,
}

// ── Simulatie types ──

#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct SimulatieParams {
    pub start_waterstand: f64,
    pub regen_intensiteit: f64,
    pub regen_duur: f64,
    pub oppervlakte: f64,
    pub gemaal_debiet: f64,
    pub verdamping: f64,
    pub infiltratie: f64,
    pub na_regen_duur: f64,
    pub tijd_stap: f64,
    pub smart_control: bool,
    pub streefpeil: f64,
    pub marge: f64,
    pub maaiveld_niveau: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SimulatieStap {
    pub tijd: f64,
    pub waterstand: f64,
    #[serde(default)]
    pub water_toevoer: f64,
    #[serde(default)]
    pub water_afvoer: f64,
    #[serde(default)]
    pub water_verlies: f64,
    #[serde(default)]
    pub water_balans: f64,
    pub is_regen: bool,
    pub is_pomp_aan: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SimulatieSamenvatting {
    pub max_waterstand: f64,
    pub min_waterstand: f64,
    #[serde(default)]
    pub aantal_stappen: usize,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DroogleggingResult {
    pub drooglegging: f64,
    pub streef_drooglegging: f64,
    pub minimale_drooglegging: f64,
    pub overschrijding: f64,
    pub overschrijding_cm: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SimulatieResponse {
    pub tijdstappen: Vec<SimulatieStap>,
    pub samenvatting: SimulatieSamenvatting,
    pub drooglegging: Option<DroogleggingResult>,
}

// ── GeoJSON / Asset layer types ──

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GeoJsonGeometry {
    pub coordinates: Vec<f64>,
}


#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LayerConfig {
    pub layer_type: String,
    pub display_label: String,
    pub color: String,
    pub icon_svg: String,
    pub default_visible: bool,
    #[serde(default)]
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AssetFeatureCollection {
    pub features: Vec<AssetFeature>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AssetFeature {
    pub geometry: GeoJsonGeometry,
    pub properties: AssetProperties,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AssetProperties {
    pub code: String,
    pub naam: Option<String>,
    pub layer_type: String,
    pub display_label: String,
    pub color: String,
    pub icon_svg: String,
    #[serde(default)]
    pub extra_properties: Option<serde_json::Value>,
}

// ── API functions ──

pub async fn fetch_status() -> Result<StatusResponse, String> {
    let url = format!("{}/status", api_base());
    reqwest::get(&url)
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .json::<StatusResponse>()
        .await
        .map_err(|e| format!("Parse failed: {e}"))
}

#[allow(dead_code)]
pub async fn fetch_gemalen() -> Result<Vec<GemaalSnapshot>, String> {
    let url = format!("{}/gemalen", api_base());
    reqwest::get(&url)
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .json::<Vec<GemaalSnapshot>>()
        .await
        .map_err(|e| format!("Parse failed: {e}"))
}

pub async fn fetch_gemaal(code: &str) -> Result<GemaalDetailResponse, String> {
    let url = format!("{}/gemalen/{code}", api_base());
    reqwest::get(&url)
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .json::<GemaalDetailResponse>()
        .await
        .map_err(|e| format!("Parse failed: {e}"))
}

#[allow(dead_code)]
pub async fn run_simulatie(params: &SimulatieParams) -> Result<SimulatieResponse, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/simulatie", api_base());
    client
        .post(&url)
        .json(params)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .json::<SimulatieResponse>()
        .await
        .map_err(|e| format!("Parse failed: {e}"))
}

pub async fn fetch_layers() -> Result<Vec<LayerConfig>, String> {
    let url = format!("{}/assets/layers", api_base());
    reqwest::get(&url)
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .json::<Vec<LayerConfig>>()
        .await
        .map_err(|e| format!("Parse failed: {e}"))
}

pub async fn fetch_assets_geojson(layers: Option<&str>) -> Result<AssetFeatureCollection, String> {
    let url = match layers {
        Some(l) => format!("{}/assets/geojson?layers={l}", api_base()),
        None => format!("{}/assets/geojson", api_base()),
    };
    reqwest::get(&url)
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .json::<AssetFeatureCollection>()
        .await
        .map_err(|e| format!("Parse failed: {e}"))
}

pub async fn fetch_assets_geojson_raw(layers: Option<&str>) -> Result<String, String> {
    let url = match layers {
        Some(l) => format!("{}/assets/geojson?layers={l}", api_base()),
        None => format!("{}/assets/geojson", api_base()),
    };
    reqwest::get(&url)
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Read failed: {e}"))
}

pub async fn fetch_peilgebieden_geojson() -> Result<String, String> {
    // Cache-bust to avoid stale browser cache (endpoint sets max-age=86400)
    let ts = js_sys::Date::now() as u64;
    let url = format!("{}/peilgebieden/geojson?_t={ts}", api_base());
    reqwest::get(&url)
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Read failed: {e}"))
}

#[allow(dead_code)]
pub async fn fetch_gemaal_peilgebied_mapping() -> Result<std::collections::HashMap<String, String>, String> {
    let url = format!("{}/peilgebieden/mapping", api_base());
    reqwest::get(&url)
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .json::<std::collections::HashMap<String, String>>()
        .await
        .map_err(|e| format!("Parse failed: {e}"))
}

// ── Energieoptimalisatie types ──

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UurPrijs {
    pub uur: u8,
    pub prijs_eur_kwh: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OptimalisatieParams {
    pub streefpeil: f64,
    pub max_debiet: f64,
    pub oppervlakte: f64,
    pub verdamping: f64,
    pub infiltratie: f64,
    pub opvoerhoogte: f64,
    pub efficiency: f64,
    pub regen_per_uur: Vec<f64>,
    pub prijzen: Vec<UurPrijs>,
    pub marge_cm: f64,
    pub berging_factor: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OptimalisatieUurResultaat {
    pub uur: u8,
    pub prijs_eur_kwh: f64,
    pub regen_mm_uur: f64,
    pub pomp_fractie_optimaal: f64,
    pub pomp_fractie_naief: f64,
    pub waterstand_eind_optimaal: f64,
    pub waterstand_eind_naief: f64,
    pub kosten_optimaal: f64,
    pub kosten_naief: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SimulatieStapUitgebreid {
    pub tijd_minuten: f64,
    pub uur: u8,
    pub waterstand: f64,
    pub water_afvoer: f64,
    pub water_toevoer: f64,
    pub is_regen: bool,
    pub is_pomp_aan: bool,
    pub cumulatieve_kosten: f64,
    pub prijs_eur_kwh: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OptimalisatieResultaat {
    pub uren: Vec<OptimalisatieUurResultaat>,
    pub totale_kosten_optimaal: f64,
    pub totale_kosten_naief: f64,
    pub besparing_eur: f64,
    pub besparing_pct: f64,
    pub max_afwijking_optimaal_cm: f64,
    pub max_afwijking_naief_cm: f64,
    pub tijdstappen_optimaal: Vec<SimulatieStapUitgebreid>,
    pub tijdstappen_naief: Vec<SimulatieStapUitgebreid>,
    pub prijzen: Vec<UurPrijs>,
}

// ── Energieoptimalisatie API functions ──

pub async fn fetch_energieprijzen() -> Result<Vec<UurPrijs>, String> {
    let url = format!("{}/energieprijzen", api_base());
    reqwest::get(&url)
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .json::<Vec<UurPrijs>>()
        .await
        .map_err(|e| format!("Parse failed: {e}"))
}

pub async fn run_optimalisatie(params: &OptimalisatieParams) -> Result<OptimalisatieResultaat, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/optimalisatie", api_base());
    client
        .post(&url)
        .json(params)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .json::<OptimalisatieResultaat>()
        .await
        .map_err(|e| format!("Parse failed: {e}"))
}
