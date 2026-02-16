//! Delft-FEWS PI-REST client for time series data exchange.
//!
//! This module provides types and client functionality for interacting with
//! Delft-FEWS (Flood Early Warning System) through its PI-REST API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Fews client configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewsConfig {
    /// Base URL of the Fews PI-REST API
    pub base_url: String,
    /// Filter ID for the specific watershed/region
    pub filter_id: String,
    /// Optional: API key for authentication
    pub api_key: Option<String>,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for FewsConfig {
    fn default() -> Self {
        Self {
            base_url: "https://fews.example.com/PI-rest".to_string(),
            filter_id: "WatershedFilter".to_string(),
            api_key: None,
            timeout_secs: 30,
        }
    }
}

/// Time series identifier in Fews.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FewsTimeSeriesId {
    pub location_id: String,
    pub parameter_id: String,
    pub module_instance_id: String,
    pub time_step: Option<FewsTimeStep>,
    pub qualifier: Option<String>,
}

/// Time step types in Fews.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FewsTimeStep {
    Second,
    Minute,
    Hour,
    Day,
    Month,
    Year,
    Decade,
}

impl FewsTimeStep {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Second => "second",
            Self::Minute => "minute",
            Self::Hour => "hour",
            Self::Day => "day",
            Self::Month => "month",
            Self::Year => "year",
            Self::Decade => "decade",
        }
    }
}

/// Time series query parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewsTimeSeriesQuery {
    pub location_ids: Option<Vec<String>>,
    pub parameter_ids: Option<Vec<String>>,
    pub module_instance_ids: Option<Vec<String>>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub qualifier: Option<String>,
    pub show_enumeration: Option<bool>,
    pub version: Option<String>,
    pub only_headers: Option<bool>,
}

impl Default for FewsTimeSeriesQuery {
    fn default() -> Self {
        Self {
            location_ids: None,
            parameter_ids: None,
            module_instance_ids: None,
            start_time: None,
            end_time: None,
            qualifier: None,
            show_enumeration: None,
            version: None,
            only_headers: None,
        }
    }
}

/// Fews time series data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewsTimeSeries {
    pub header: FewsTimeSeriesHeader,
    pub data: Vec<FewsTimeSeriesPoint>,
    #[serde(default)]
    pub misses: Vec<i64>,
}

/// Time series header/metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewsTimeSeriesHeader {
    pub location_id: String,
    pub parameter_id: String,
    pub module_instance_id: String,
    pub time_step: FewsTimeStep,
    pub start_date: String,
    pub end_date: String,
    pub units: String,
    pub type_description: String,
    pub value_type: FewsValueType,
    pub station_name: String,
    pub parameter_description: String,
    pub module_description: String,
    pub geo_delta: Option<f64>,
    pub geo_datum: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub qualifier: Option<String>,
    pub miss_val: Option<f64>,
}

/// Value type in Fews.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FewsValueType {
    Instantaneous,
    Accumulative,
}

/// Individual time series data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewsTimeSeriesPoint {
    pub date: String,
    pub value: f64,
    pub flag: Option<i64>,
}

/// Response from Fews time series query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewsTimeSeriesResponse {
    pub version: String,
    pub time_series: Vec<FewsTimeSeries>,
    pub only_headers: Option<bool>,
}

/// Filter locations query response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewsLocation {
    pub id: String,
    pub name: String,
    pub short_name: Option<String>,
    pub description: Option<String>,
    pub region_id: Option<String>,
    pub region_name: Option<String>,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub geo_datum: Option<String>,
    pub geo_delta: Option<f64>,
    pub properties: Option<HashMap<String, serde_json::Value>>,
}

/// Parameters query response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewsParameter {
    pub id: String,
    pub name: String,
    pub short_name: Option<String>,
    pub description: Option<String>,
    pub unit: String,
    pub parameter_type: Option<String>,
    pub shows_branching: Option<bool>,
}

/// Module instances query response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewsModuleInstance {
    pub id: String,
    pub name: String,
    pub short_name: Option<String>,
    pub description: Option<String>,
    pub module_id: Option<String>,
    pub filter_id: String,
}

/// Sync request for importing Fews data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewsSyncRequest {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub location_ids: Option<Vec<String>>,
    pub parameter_ids: Option<Vec<String>>,
    pub sync_results: Option<bool>,
}

/// Sync result summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewsSyncResult {
    pub time_series_count: usize,
    pub data_points_count: usize,
    pub locations: Vec<String>,
    pub parameters: Vec<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub synced_at: DateTime<Utc>,
}

/// Fews sync configuration for a peilgebied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewsSyncConfig {
    pub peilgebied_id: String,
    pub fews_filter_id: String,
    pub location_mapping: HashMap<String, String>, // local_id -> fews_id
    pub parameter_mapping: HashMap<String, String>, // local_id -> fews_id
    pub sync_interval_hours: Option<u32>,
    pub auto_sync: bool,
}

impl FewsTimeSeriesQuery {
    /// Create a new query with location and parameter.
    pub fn new(location_id: impl Into<String>, parameter_id: impl Into<String>) -> Self {
        Self {
            location_ids: Some(vec![location_id.into()]),
            parameter_ids: Some(vec![parameter_id.into()]),
            ..Default::default()
        }
    }

    /// Set time range for the query.
    pub fn with_time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    /// Set filter ID.
    pub fn with_filter(mut self, filter_id: impl Into<String>) -> Self {
        self.module_instance_ids = Some(vec![filter_id.into()]);
        self
    }
}

impl FewsTimeSeries {
    /// Convert to generic Hydronet format for compatibility.
    pub fn to_hydronet_series(&self) -> crate::hydronet::HydronetSeries {
        crate::hydronet::HydronetSeries {
            name: self.header.location_id.clone(),
            r#type: "line".to_string(),
            color: "#1a5276".to_string(),
            data: self.data.iter().map(|p| crate::hydronet::DataPoint {
                timestamp: None,
                timestamp_ms: chrono::DateTime::parse_from_rfc3339(&p.date)
                    .ok()
                    .map(|dt| dt.timestamp_millis())
                    .unwrap_or(0),
                value: p.value,
                status: None,
            }).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_series_query_builder() {
        let query = FewsTimeSeriesQuery::new("LOC_001", "waterlevel")
            .with_time_range(
                Utc::now() - chrono::Duration::hours(24),
                Utc::now(),
            );

        assert_eq!(query.location_ids, Some(vec!["LOC_001".to_string()]));
        assert_eq!(query.parameter_ids, Some(vec!["waterlevel".to_string()]));
        assert!(query.start_time.is_some());
        assert!(query.end_time.is_some());
    }

    #[test]
    fn test_time_step_serialization() {
        let step = FewsTimeStep::Hour;
        assert_eq!(step.as_str(), "hour");
    }

    #[test]
    fn test_fews_config_default() {
        let config = FewsConfig::default();
        assert_eq!(config.filter_id, "WatershedFilter");
        assert_eq!(config.timeout_secs, 30);
    }
}
