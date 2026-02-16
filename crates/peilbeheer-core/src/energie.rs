use serde::{Deserialize, Serialize};

/// Stroomprijs voor één uur.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UurPrijs {
    pub uur: u8,
    pub prijs_eur_kwh: f64,
}

/// Parameters voor de energieoptimalisatie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimalisatieParams {
    /// Streefpeil in m NAP
    pub streefpeil: f64,
    /// Maximaal gemaal debiet in m³/s
    pub max_debiet: f64,
    /// Oppervlakte peilgebied in m²
    pub oppervlakte: f64,
    /// Verdamping in mm/uur
    #[serde(default = "default_verdamping")]
    pub verdamping: f64,
    /// Infiltratie in mm/uur
    #[serde(default = "default_infiltratie")]
    pub infiltratie: f64,
    /// Pompopvoerhoogte in m
    #[serde(default = "default_opvoerhoogte")]
    pub opvoerhoogte: f64,
    /// Pompefficiëntie (0-1)
    #[serde(default = "default_efficiency")]
    pub efficiency: f64,
    /// Regenintensiteit per uur, 24 waarden in mm/uur
    pub regen_per_uur: Vec<f64>,
    /// Stroomprijzen per uur, 24 entries (leeg = API fetcht ze)
    #[serde(default)]
    pub prijzen: Vec<UurPrijs>,
    /// Toegestane marge rond streefpeil in cm
    #[serde(default = "default_marge_cm")]
    pub marge_cm: f64,
    /// Fractie open water (bergingsoppervlak / totaal oppervlak).
    /// Regen valt op het hele peilgebied maar de waterstand stijgt alleen
    /// in het open water. Typisch 0.05–0.15 voor agrarische polders.
    #[serde(default = "default_berging_factor")]
    pub berging_factor: f64,
}

fn default_verdamping() -> f64 { 0.5 }
fn default_infiltratie() -> f64 { 0.2 }
fn default_opvoerhoogte() -> f64 { 2.0 }
fn default_efficiency() -> f64 { 0.70 }
fn default_marge_cm() -> f64 { 20.0 }
fn default_berging_factor() -> f64 { 0.10 }

/// Resultaat per uur van de optimalisatie.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// Totaalresultaat van de optimalisatie.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// Uitgebreide simulatiestap (per minuut) met kostinformatie.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
