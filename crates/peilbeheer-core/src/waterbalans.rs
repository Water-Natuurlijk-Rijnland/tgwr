use serde::{Deserialize, Serialize};

/// Resultaat van een waterbalansberekening voor één tijdstap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterBalance {
    pub water_toevoer: f64,
    pub water_afvoer: f64,
    pub water_verlies: f64,
    pub water_balans: f64,
    pub waterstand_verandering: f64,
    pub nieuwe_waterstand: f64,
}

/// Parameters voor een waterbalans simulatie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatieParams {
    /// Startwaterstand in m NAP
    pub start_waterstand: f64,
    /// Regenintensiteit in mm/uur
    pub regen_intensiteit: f64,
    /// Regenduur in minuten
    pub regen_duur: f64,
    /// Oppervlakte in m²
    pub oppervlakte: f64,
    /// Gemaal debiet in m³/s
    #[serde(default)]
    pub gemaal_debiet: f64,
    /// Verdamping in mm/uur
    #[serde(default)]
    pub verdamping: f64,
    /// Infiltratie in mm/uur
    #[serde(default)]
    pub infiltratie: f64,
    /// Duur na regen in minuten
    #[serde(default = "default_na_regen_duur")]
    pub na_regen_duur: f64,
    /// Tijdstap in minuten
    #[serde(default = "default_tijd_stap")]
    pub tijd_stap: f64,
    /// Smart control (PID) ingeschakeld
    #[serde(default)]
    pub smart_control: bool,
    /// Streefpeil in m NAP
    #[serde(default)]
    pub streefpeil: f64,
    /// Marge in cm
    #[serde(default)]
    pub marge: f64,
    /// Maaiveld niveau in m NAP (voor drooglegging)
    #[serde(default)]
    pub maaiveld_niveau: f64,
}

fn default_na_regen_duur() -> f64 {
    30.0
}

fn default_tijd_stap() -> f64 {
    1.0
}

/// Eén stap in de simulatie tijdreeks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatieStap {
    /// Tijd in minuten
    pub tijd: f64,
    /// Waterstand in m NAP
    pub waterstand: f64,
    /// Water toevoer in m³/s
    pub water_toevoer: f64,
    /// Water afvoer in m³/s
    pub water_afvoer: f64,
    /// Water verlies in m³/s
    pub water_verlies: f64,
    /// Water balans in m³/s
    pub water_balans: f64,
    /// Is het aan het regenen?
    pub is_regen: bool,
    /// Is de pomp aan?
    pub is_pomp_aan: bool,
}

/// Drooglegging resultaat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroogleggingResult {
    pub drooglegging: f64,
    pub streef_drooglegging: f64,
    pub minimale_drooglegging: f64,
    pub overschrijding: f64,
    pub overschrijding_cm: f64,
}

/// Resultaat van find_minimum_debiet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimumDebietResult {
    pub minimaal_debiet: Option<f64>,
    pub max_overschrijding: f64,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub tijdstappen: Vec<SimulatieStap>,
}
