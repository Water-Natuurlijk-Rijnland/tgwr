//! Energie-optimalisatie types.
//!
//! This module provides types for pump scheduling optimization
//! based on energy prices and water balance constraints.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

impl Default for OptimalisatieParams {
    fn default() -> Self {
        Self {
            streefpeil: -2.5,
            max_debiet: 50.0,
            oppervlakte: 100000.0,
            verdamping: default_verdamping(),
            infiltratie: default_infiltratie(),
            opvoerhoogte: default_opvoerhoogte(),
            efficiency: default_efficiency(),
            regen_per_uur: vec![0.0; 24],
            prijzen: Vec::new(),
            marge_cm: default_marge_cm(),
            berging_factor: default_berging_factor(),
        }
    }
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

/// Optimization job for background processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationJob {
    /// Unique job identifier
    pub id: String,

    /// Job name/description
    pub name: String,

    /// Peilgebied ID for this optimization
    pub peilgebied_id: String,

    /// Optimization parameters
    pub params: OptimalisatieParams,

    /// Job status
    pub status: JobStatus,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Job start time
    pub started_at: Option<DateTime<Utc>>,

    /// Job completion time
    pub completed_at: Option<DateTime<Utc>>,

    /// Optimization result (when complete)
    pub result: Option<OptimalisatieResultaat>,

    /// Error message (if failed)
    pub error: Option<String>,

    /// Created by user ID
    pub created_by: Option<String>,
}

/// Job status in the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Price forecast data from EnergyZero.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceForecast {
    /// Forecast timestamp
    pub timestamp: DateTime<Utc>,

    /// Hourly prices for next 24-48 hours
    pub hourly_prices: Vec<HourlyPrice>,

    /// Forecast creation time
    pub forecast_created: DateTime<Utc>,

    /// Data source
    pub source: PriceSource,
}

/// Hourly price data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyPrice {
    /// Hour start time (UTC)
    pub hour_start: DateTime<Utc>,

    /// Price in EUR/kWh
    pub price_eur_kwh: f64,

    /// Whether this is actual or forecasted data
    pub is_forecast: bool,
}

/// Price data source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceSource {
    EnergyZero,
    EPEX,
    NordPool,
    Manual,
}

/// Pump schedule from optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpSchedule {
    /// Job ID this schedule belongs to
    pub job_id: String,

    /// Peilgebied ID
    pub peilgebied_id: String,

    /// Schedule start time
    pub start_time: DateTime<Utc>,

    /// Schedule end time
    pub end_time: DateTime<Utc>,

    /// Hourly pump schedule (0-1 fraction of max capacity)
    pub hourly_schedule: Vec<f64>,

    /// Total energy consumption (kWh)
    pub total_energy_kwh: f64,

    /// Total energy cost (EUR)
    pub total_cost_eur: f64,

    /// Water level targets
    pub target_level_m: f64,
    pub min_level_m: f64,
    pub max_level_m: f64,

    /// Schedule metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Queue statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    /// Total jobs in queue
    pub queued: u32,

    /// Currently running jobs
    pub running: u32,

    /// Completed jobs in last 24h
    pub completed_24h: u32,

    /// Failed jobs in last 24h
    pub failed_24h: u32,

    /// Average job duration (seconds)
    pub avg_duration_seconds: Option<f64>,

    /// Workers active
    pub workers_active: u32,
}

impl OptimizationJob {
    /// Create a new optimization job.
    pub fn new(
        name: impl Into<String>,
        peilgebied_id: impl Into<String>,
        params: OptimalisatieParams,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: format!("OPT_{}", uuid::Uuid::new_v4()),
            name: name.into(),
            peilgebied_id: peilgebied_id.into(),
            params,
            status: JobStatus::Queued,
            created_at: now,
            started_at: None,
            completed_at: None,
            result: None,
            error: None,
            created_by: None,
        }
    }

    /// Check if job is terminal (completed/failed/cancelled).
    pub fn is_terminal(&self) -> bool {
        matches!(self.status, JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled)
    }

    /// Get job duration if completed.
    pub fn duration(&self) -> Option<chrono::Duration> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some(end.signed_duration_since(start)),
            _ => None,
        }
    }
}

impl JobStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

impl PriceForecast {
    /// Create from EnergyZero response data.
    pub fn from_energyzero(prices: Vec<f64>) -> Self {
        let now = Utc::now();
        let hourly_prices: Vec<HourlyPrice> = prices
            .into_iter()
            .enumerate()
            .map(|(i, price)| {
                let hour_start = now + chrono::Duration::hours(i as i64);
                HourlyPrice {
                    hour_start,
                    price_eur_kwh: price,
                    is_forecast: true,
                }
            })
            .collect();

        Self {
            timestamp: now,
            hourly_prices,
            forecast_created: now,
            source: PriceSource::EnergyZero,
        }
    }

    /// Get price for a specific hour.
    pub fn get_price_for_hour(&self, hour: u8) -> Option<f64> {
        self.hourly_prices.get(hour as usize)
            .map(|p| p.price_eur_kwh)
    }

    /// Get average price.
    pub fn average_price(&self) -> f64 {
        if self.hourly_prices.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.hourly_prices.iter()
            .map(|p| p.price_eur_kwh)
            .sum();
        sum / self.hourly_prices.len() as f64
    }

    /// Find cheapest hours for a given count.
    pub fn find_cheapest_hours(&self, count: usize) -> Vec<usize> {
        let mut indexed: Vec<_> = self.hourly_prices.iter()
            .enumerate()
            .map(|(i, p)| (i, p.price_eur_kwh))
            .collect();

        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        indexed.iter()
            .take(count)
            .map(|(i, _)| *i)
            .collect()
    }
}

impl PumpSchedule {
    /// Get pump fraction for a specific hour.
    pub fn get_hour_schedule(&self, hour: u8) -> f64 {
        self.hourly_schedule
            .get(hour as usize)
            .copied()
            .unwrap_or(0.0)
    }

    /// Get hours when pump should be active (above threshold).
    pub fn get_active_hours(&self, threshold: f64) -> Vec<u8> {
        self.hourly_schedule
            .iter()
            .enumerate()
            .filter_map(|(i, &frac)| if frac >= threshold { Some(i as u8) } else { None })
            .collect()
    }

    /// Calculate total pump hours.
    pub fn total_pump_hours(&self) -> f64 {
        self.hourly_schedule.iter().sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimization_job_creation() {
        let params = OptimalisatieParams::default();

        let job = OptimizationJob::new("Test Job", "PG_001", params);

        assert!(job.id.starts_with("OPT_"));
        assert_eq!(job.status, JobStatus::Queued);
        assert!(!job.is_terminal());
    }

    #[test]
    fn test_job_status() {
        assert_eq!(JobStatus::Running.as_str(), "running");
        assert_eq!(JobStatus::from_str("completed"), Some(JobStatus::Completed));
    }

    #[test]
    fn test_price_forecast() {
        let prices = vec![0.10, 0.15, 0.25, 0.08, 0.05];
        let forecast = PriceForecast::from_energyzero(prices);

        assert_eq!(forecast.hourly_prices.len(), 5);
        assert_eq!(forecast.average_price(), 0.126);

        let cheapest = forecast.find_cheapest_hours(2);
        assert_eq!(cheapest, vec![4, 3]); // Indices of 0.05 and 0.08
    }

    #[test]
    fn test_pump_schedule() {
        let schedule = PumpSchedule {
            job_id: "test".to_string(),
            peilgebied_id: "PG_001".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now() + chrono::Duration::hours(24),
            hourly_schedule: vec![0.0, 0.5, 1.0, 0.5, 0.0],
            total_energy_kwh: 100.0,
            total_cost_eur: 15.0,
            target_level_m: -2.5,
            min_level_m: -2.7,
            max_level_m: -2.3,
            metadata: HashMap::new(),
        };

        assert_eq!(schedule.get_hour_schedule(2), 1.0);
        assert_eq!(schedule.get_active_hours(0.5), vec![1, 2, 3]);
        assert_eq!(schedule.total_pump_hours(), 2.0);
    }
}
