//! Time Series Storage for efficient historical data management.
//!
//! This module provides types and functionality for:
//! - Multi-resolution time series storage (raw, 1min, 5min, 15min, 1hour, 1day)
//! - Automatic downsampling on write
//! - Fast range queries with aggregation
//! - Gap detection and interpolation

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a time series.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimeSeriesId {
    /// Location identifier (e.g., gemaal code, peilgebied ID)
    pub location_id: String,
    /// Parameter name (e.g., "water_level", "debiet", "energy_price")
    pub parameter: String,
    /// Optional qualifier for distinguishing multiple series
    pub qualifier: Option<String>,
}

impl TimeSeriesId {
    /// Create a new time series ID.
    pub fn new(location_id: impl Into<String>, parameter: impl Into<String>) -> Self {
        Self {
            location_id: location_id.into(),
            parameter: parameter.into(),
            qualifier: None,
        }
    }

    /// Create a new time series ID with qualifier.
    pub fn with_qualifier(
        location_id: impl Into<String>,
        parameter: impl Into<String>,
        qualifier: impl Into<String>,
    ) -> Self {
        Self {
            location_id: location_id.into(),
            parameter: parameter.into(),
            qualifier: Some(qualifier.into()),
        }
    }

    /// Get the composite key for this ID.
    pub fn key(&self) -> String {
        match &self.qualifier {
            Some(q) => format!("{}|{}|{}", self.location_id, self.parameter, q),
            None => format!("{}|{}", self.location_id, self.parameter),
        }
    }

    /// Parse a key back into TimeSeriesId.
    pub fn from_key(key: &str) -> Option<Self> {
        let parts: Vec<&str> = key.split('|').collect();
        match parts.len() {
            2 => Some(Self {
                location_id: parts[0].to_string(),
                parameter: parts[1].to_string(),
                qualifier: None,
            }),
            3 => Some(Self {
                location_id: parts[0].to_string(),
                parameter: parts[1].to_string(),
                qualifier: Some(parts[2].to_string()),
            }),
            _ => None,
        }
    }
}

/// Aggregation granularity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AggregationLevel {
    /// Raw data (no aggregation)
    Raw,
    /// 1 minute
    Min1,
    /// 5 minutes
    Min5,
    /// 15 minutes
    Min15,
    /// 1 hour
    Hour1,
    /// 6 hours
    Hour6,
    /// 1 day
    Day1,
    /// 1 week
    Week1,
    /// 1 month
    Month1,
}

impl AggregationLevel {
    /// Get the duration for this aggregation level.
    pub fn duration(&self) -> Duration {
        match self {
            Self::Raw => Duration::seconds(0),
            Self::Min1 => Duration::minutes(1),
            Self::Min5 => Duration::minutes(5),
            Self::Min15 => Duration::minutes(15),
            Self::Hour1 => Duration::hours(1),
            Self::Hour6 => Duration::hours(6),
            Self::Day1 => Duration::days(1),
            Self::Week1 => Duration::weeks(1),
            Self::Month1 => Duration::days(30), // Approximate
        }
    }

    /// Get the interval in seconds for this level.
    pub fn interval_seconds(&self) -> i64 {
        self.duration().num_seconds()
    }

    /// Parse from string.
    #[allow(clippy::should_implement_trait)]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "raw" => Some(Self::Raw),
            "1m" | "min1" => Some(Self::Min1),
            "5m" | "min5" => Some(Self::Min5),
            "15m" | "min15" => Some(Self::Min15),
            "1h" | "hour1" => Some(Self::Hour1),
            "6h" | "hour6" => Some(Self::Hour6),
            "1d" | "day1" => Some(Self::Day1),
            "1w" | "week1" => Some(Self::Week1),
            "1mo" | "month1" => Some(Self::Month1),
            _ => None,
        }
    }

    /// Get the next coarser aggregation level.
    pub fn coarser(&self) -> Option<Self> {
        match self {
            Self::Raw => Some(Self::Min1),
            Self::Min1 => Some(Self::Min5),
            Self::Min5 => Some(Self::Min15),
            Self::Min15 => Some(Self::Hour1),
            Self::Hour1 => Some(Self::Hour6),
            Self::Hour6 => Some(Self::Day1),
            Self::Day1 => Some(Self::Week1),
            Self::Week1 => Some(Self::Month1),
            Self::Month1 => None,
        }
    }
}

/// Data quality flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QualityFlag {
    /// Good quality data
    Good,
    /// Questionable data
    Questionable,
    /// Bad/missing data
    Bad,
    /// Missing value
    Missing,
    /// Interpolated value
    Interpolated,
}

impl QualityFlag {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Good => "good",
            Self::Questionable => "questionable",
            Self::Bad => "bad",
            Self::Missing => "missing",
            Self::Interpolated => "interpolated",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "good" => Some(Self::Good),
            "questionable" => Some(Self::Questionable),
            "bad" => Some(Self::Bad),
            "missing" => Some(Self::Missing),
            "interpolated" => Some(Self::Interpolated),
            _ => None,
        }
    }

    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Good | Self::Questionable | Self::Interpolated)
    }
}

/// Single time series data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesDataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub flag: QualityFlag,
}

impl TimeSeriesDataPoint {
    /// Create a new data point.
    pub fn new(timestamp: DateTime<Utc>, value: f64) -> Self {
        Self {
            timestamp,
            value,
            flag: QualityFlag::Good,
        }
    }

    /// Create a data point with quality flag.
    pub fn with_flag(timestamp: DateTime<Utc>, value: f64, flag: QualityFlag) -> Self {
        Self {
            timestamp,
            value,
            flag,
        }
    }

    /// Create a missing data point.
    pub fn missing(timestamp: DateTime<Utc>) -> Self {
        Self {
            timestamp,
            value: f64::NAN,
            flag: QualityFlag::Missing,
        }
    }

    /// Check if the value is valid (not NaN/Inf and good quality).
    pub fn is_valid(&self) -> bool {
        self.value.is_finite() && self.flag.is_valid()
    }
}

/// Time series metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesMetadata {
    pub id: TimeSeriesId,
    pub display_name: String,
    pub description: Option<String>,
    pub units: Option<String>,
    pub data_type: TimeSeriesDataType,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub source: String,
    pub source_type: TimeSeriesSourceType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub retention_days: Option<u32>,
    pub attributes: HashMap<String, serde_json::Value>,
}

/// Data type of the time series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeSeriesDataType {
    /// Instantaneous measurement
    Instantaneous,
    /// Accumulated value (counter)
    Accumulated,
    /// Average over interval
    Average,
    /// Total over interval
    Total,
    /// Boolean state
    Boolean,
    /// String enum
    Enum,
}

/// Source type of the time series.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeSeriesSourceType {
    /// Delft-FEWS
    Fews,
    /// Hydronet
    Hydronet,
    /// D-HYDRO simulation
    DHydro,
    /// EnergyZero API
    EnergyZero,
    /// Manual entry
    Manual,
    /// Calculated/derived
    Calculated,
    /// Custom source
    Custom(String),
}

/// Query for time series data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesQuery {
    pub series_id: TimeSeriesId,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub aggregation: Option<AggregationLevel>,
    pub function: Option<AggregationFunction>,
    pub fill_gaps: Option<FillMethod>,
    pub max_gap_seconds: Option<i64>,
}

/// How to fill gaps in time series data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FillMethod {
    /// No filling, return nulls for gaps
    None,
    /// Linear interpolation between points
    Linear,
    /// Forward fill (use previous value)
    Forward,
    /// Backward fill (use next value)
    Backward,
    /// Fill with a specific value
    Constant,
}

/// Aggregation function for downsampled data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AggregationFunction {
    Average,
    Min,
    Max,
    Sum,
    Count,
    First,
    Last,
    Median,
    StdDev,
}

/// Result of a time series query with aggregation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedSeries {
    pub series_id: TimeSeriesId,
    pub aggregation: AggregationLevel,
    pub function: AggregationFunction,
    pub data: Vec<TimeSeriesDataPoint>,
    pub metadata: AggregationMetadata,
}

/// Metadata about aggregated data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationMetadata {
    pub data_points: usize,
    pub gaps_filled: usize,
    pub quality_flags: HashMap<String, usize>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Gap detected in time series.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesGap {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub duration_seconds: i64,
    pub expected_points: usize,
    pub actual_points: usize,
}

/// Result of gap analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapAnalysisResult {
    pub series_id: TimeSeriesId,
    pub gaps: Vec<TimeSeriesGap>,
    pub total_gap_seconds: i64,
    pub coverage_percent: f64,
}

/// Batch write request for time series data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesWriteBatch {
    pub series_id: TimeSeriesId,
    pub data: Vec<TimeSeriesDataPoint>,
    pub attributes: Option<HashMap<String, serde_json::Value>>,
}

/// Result of a write operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesWriteResult {
    pub series_id: TimeSeriesId,
    pub points_written: usize,
    pub points_updated: usize,
    pub points_rejected: usize,
    pub first_timestamp: Option<DateTime<Utc>>,
    pub last_timestamp: Option<DateTime<Utc>>,
}

/// Configuration for automatic downsampling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownsampleConfig {
    pub enabled: bool,
    pub levels: Vec<AggregationLevel>,
    pub functions: Vec<AggregationFunction>,
    pub real_time: bool,
    pub batch_size_mb: Option<u32>,
}

impl Default for DownsampleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            levels: vec![
                AggregationLevel::Min1,
                AggregationLevel::Min5,
                AggregationLevel::Min15,
                AggregationLevel::Hour1,
                AggregationLevel::Day1,
            ],
            functions: vec![
                AggregationFunction::Average,
                AggregationFunction::Min,
                AggregationFunction::Max,
            ],
            real_time: true,
            batch_size_mb: Some(100),
        }
    }
}

/// Time series catalog entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesCatalogEntry {
    pub id: TimeSeriesId,
    pub display_name: String,
    pub units: Option<String>,
    pub source: String,
    pub has_raw_data: bool,
    pub first_timestamp: Option<DateTime<Utc>>,
    pub last_timestamp: Option<DateTime<Utc>>,
    pub point_count: u64,
}

impl AggregatedSeries {
    /// Get the minimum value in the series.
    pub fn min_value(&self) -> Option<f64> {
        self.data
            .iter()
            .filter(|p| p.is_valid())
            .map(|p| p.value)
            .fold(f64::INFINITY, f64::min)
            .finite()
    }

    /// Get the maximum value in the series.
    pub fn max_value(&self) -> Option<f64> {
        self.data
            .iter()
            .filter(|p| p.is_valid())
            .map(|p| p.value)
            .fold(f64::NEG_INFINITY, f64::max)
            .finite()
    }

    /// Get the average value.
    pub fn average(&self) -> Option<f64> {
        let valid: Vec<_> = self.data.iter().filter(|p| p.is_valid()).collect();
        if valid.is_empty() {
            None
        } else {
            let sum: f64 = valid.iter().map(|p| p.value).sum();
            Some(sum / valid.len() as f64)
        }
    }
}

/// Helper to check if a float is finite.
trait FiniteExt {
    fn finite(self) -> Option<f64>;
}

impl FiniteExt for f64 {
    fn finite(self) -> Option<f64> {
        if self.is_finite() {
            Some(self)
        } else {
            None
        }
    }
}

impl TimeSeriesQuery {
    /// Create a new query for a series.
    pub fn new(series_id: TimeSeriesId, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self {
            series_id,
            start,
            end,
            aggregation: None,
            function: None,
            fill_gaps: None,
            max_gap_seconds: None,
        }
    }

    /// Set the aggregation level.
    pub fn with_aggregation(mut self, level: AggregationLevel) -> Self {
        self.aggregation = Some(level);
        self
    }

    /// Set the aggregation function.
    pub fn with_function(mut self, func: AggregationFunction) -> Self {
        self.function = Some(func);
        self
    }

    /// Set gap filling method.
    pub fn with_fill_method(mut self, method: FillMethod) -> Self {
        self.fill_gaps = Some(method);
        self
    }

    /// Validate the query.
    pub fn validate(&self) -> Result<(), String> {
        if self.start >= self.end {
            return Err("Start time must be before end time".to_string());
        }

        // Check if aggregation and function are compatible
        if self.aggregation.is_some() && self.function.is_none() {
            return Err("Aggregation function required when aggregation level is set".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeseries_id_key() {
        let id = TimeSeriesId::new("GEMAAL_001", "water_level");
        assert_eq!(id.key(), "GEMAAL_001|water_level");

        let id_with_qual = TimeSeriesId::with_qualifier("GEMAAL_001", "water_level", "inlet");
        assert_eq!(id_with_qual.key(), "GEMAAL_001|water_level|inlet");

        // Round-trip
        assert_eq!(TimeSeriesId::from_key(&id.key()), Some(id));
        assert_eq!(TimeSeriesId::from_key(&id_with_qual.key()), Some(id_with_qual));
    }

    #[test]
    fn test_aggregation_level_duration() {
        assert_eq!(AggregationLevel::Min1.duration(), Duration::minutes(1));
        assert_eq!(AggregationLevel::Hour1.duration(), Duration::hours(1));
        assert_eq!(AggregationLevel::Day1.duration(), Duration::days(1));
    }

    #[test]
    fn test_quality_flag() {
        assert!(QualityFlag::Good.is_valid());
        assert!(QualityFlag::Interpolated.is_valid());
        assert!(!QualityFlag::Bad.is_valid());
        assert!(!QualityFlag::Missing.is_valid());
    }

    #[test]
    fn test_data_point_validation() {
        let good = TimeSeriesDataPoint::new(Utc::now(), 10.0);
        assert!(good.is_valid());

        let bad = TimeSeriesDataPoint::with_flag(Utc::now(), 10.0, QualityFlag::Bad);
        assert!(!bad.is_valid());

        let nan_val = TimeSeriesDataPoint::new(Utc::now(), f64::NAN);
        assert!(!nan_val.is_valid());
    }

    #[test]
    fn test_aggregated_series_stats() {
        let series = AggregatedSeries {
            series_id: TimeSeriesId::new("test", "value"),
            aggregation: AggregationLevel::Hour1,
            function: AggregationFunction::Average,
            data: vec![
                TimeSeriesDataPoint::new(Utc::now(), 10.0),
                TimeSeriesDataPoint::new(Utc::now() + Duration::hours(1), 20.0),
                TimeSeriesDataPoint::new(Utc::now() + Duration::hours(2), 30.0),
            ],
            metadata: AggregationMetadata {
                data_points: 3,
                gaps_filled: 0,
                quality_flags: HashMap::new(),
                start: Utc::now(),
                end: Utc::now() + Duration::hours(2),
            },
        };

        assert_eq!(series.min_value(), Some(10.0));
        assert_eq!(series.max_value(), Some(30.0));
        assert_eq!(series.average(), Some(20.0));
    }

    #[test]
    fn test_query_validation() {
        let id = TimeSeriesId::new("test", "value");
        let start = Utc::now();
        let end = start + Duration::hours(1);

        let valid_query = TimeSeriesQuery::new(id.clone(), start, end);
        assert!(valid_query.validate().is_ok());

        // Invalid: start >= end
        let invalid_query = TimeSeriesQuery::new(id.clone(), end, start);
        assert!(invalid_query.validate().is_err());

        // Invalid: aggregation without function
        let query = TimeSeriesQuery::new(id, start, end)
            .with_aggregation(AggregationLevel::Hour1);
        assert!(query.validate().is_err());
    }
}
