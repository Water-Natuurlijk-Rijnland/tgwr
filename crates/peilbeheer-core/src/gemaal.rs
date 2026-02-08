use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Status van een gemaal (pompstation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GemaalStatus {
    Aan,
    Uit,
    Onbekend,
    Error,
}

impl GemaalStatus {
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "aan" => Self::Aan,
            "uit" => Self::Uit,
            "error" => Self::Error,
            _ => Self::Onbekend,
        }
    }
}

impl std::fmt::Display for GemaalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aan => write!(f, "aan"),
            Self::Uit => write!(f, "uit"),
            Self::Onbekend => write!(f, "unknown"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// Richting van een trend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

impl std::fmt::Display for TrendDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Increasing => write!(f, "increasing"),
            Self::Decreasing => write!(f, "decreasing"),
            Self::Stable => write!(f, "stable"),
        }
    }
}

/// Sterkte van een trend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendStrength {
    Strong,
    Moderate,
    Weak,
}

/// Trend informatie voor een sliding window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendInfo {
    /// Verandering per seconde
    pub slope: f64,
    /// Verandering per uur
    pub slope_per_hour: f64,
    /// Richting van de trend
    pub direction: TrendDirection,
    /// Betrouwbaarheid (0-1)
    pub r_squared: f64,
    /// Sterkte van de trend
    pub strength: TrendStrength,
}

/// Trends over meerdere tijdvensters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GemaalTrends {
    #[serde(rename = "30_min")]
    pub min_30: Option<TrendInfo>,
    #[serde(rename = "60_min")]
    pub min_60: Option<TrendInfo>,
    #[serde(rename = "180_min")]
    pub min_180: Option<TrendInfo>,
}

/// Gemaal definitie (uit GeoJSON / database).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gemaal {
    pub code: String,
    pub naam: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
}

/// Momentopname van een gemaal status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GemaalSnapshot {
    pub gemaal_code: String,
    pub status: GemaalStatus,
    pub debiet: f64,
    pub last_update: Option<DateTime<Utc>>,
    pub generated_at: Option<DateTime<Utc>>,
    pub trends: Option<GemaalTrends>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Station data in de summary (per gemaal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationData {
    pub status: GemaalStatus,
    #[serde(default)]
    pub debiet: f64,
    pub timestamp: Option<String>,
    pub last_update: Option<String>,
    pub trends: Option<GemaalTrends>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Samenvatting van alle stations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationSummary {
    pub generated_at: DateTime<Utc>,
    pub total_stations: usize,
    pub active_stations: usize,
    pub total_debiet_m3s: f64,
    pub stations: HashMap<String, StationData>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemaal_status_from_str() {
        assert_eq!(GemaalStatus::from_str_loose("aan"), GemaalStatus::Aan);
        assert_eq!(GemaalStatus::from_str_loose("uit"), GemaalStatus::Uit);
        assert_eq!(GemaalStatus::from_str_loose("error"), GemaalStatus::Error);
        assert_eq!(GemaalStatus::from_str_loose("xyz"), GemaalStatus::Onbekend);
    }

    #[test]
    fn test_station_summary_serialize() {
        let summary = StationSummary {
            generated_at: Utc::now(),
            total_stations: 378,
            active_stations: 45,
            total_debiet_m3s: 17.69,
            stations: HashMap::new(),
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("378"));
    }
}
