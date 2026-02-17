//! Time Series Storage API routes.
//!
//! Endpoints for managing and querying time series data.

use axum::{
    extract::{Extension, Path, Query},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

use peilbeheer_core::timeseries::*;

use crate::timeseries_service::TimeSeriesService;

/// Response wrapper for API responses.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// Query parameters for time series query.
#[derive(Debug, Deserialize)]
pub struct TimeSeriesQueryParams {
    pub location_id: String,
    pub parameter: String,
    pub qualifier: Option<String>,
    pub start: String,
    pub end: String,
    pub aggregation: Option<String>,
    pub function: Option<String>,
    pub fill_gaps: Option<String>,
}

/// Request to write time series data.
#[derive(Debug, Deserialize)]
pub struct WriteTimeSeriesRequest {
    pub location_id: String,
    pub parameter: String,
    pub qualifier: Option<String>,
    pub data: Vec<DataPointBody>,
    pub metadata: Option<SeriesMetadataBody>,
}

/// Data point in write request.
#[derive(Debug, Deserialize, Serialize)]
pub struct DataPointBody {
    pub timestamp: String,
    pub value: f64,
    pub flag: Option<String>,
}

/// Metadata for series registration.
#[derive(Debug, Deserialize)]
pub struct SeriesMetadataBody {
    pub display_name: String,
    pub description: Option<String>,
    pub units: Option<String>,
    pub data_type: Option<String>,
    pub source: Option<String>,
}

/// Request to register a time series.
#[derive(Debug, Deserialize)]
pub struct RegisterSeriesRequest {
    pub location_id: String,
    pub parameter: String,
    pub qualifier: Option<String>,
    pub display_name: String,
    pub description: Option<String>,
    pub units: Option<String>,
    pub data_type: Option<String>,
    pub source: Option<String>,
    pub source_type: Option<String>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub retention_days: Option<u32>,
}

/// Query time series data.
pub async fn query_timeseries(
    Extension(service): Extension<Arc<TimeSeriesService>>,
    Query(params): Query<TimeSeriesQueryParams>,
) -> Result<Json<ApiResponse<AggregatedSeries>>, Json<ApiResponse<()>>> {
    let series_id = if let Some(q) = &params.qualifier {
        TimeSeriesId::with_qualifier(&params.location_id, &params.parameter, q)
    } else {
        TimeSeriesId::new(&params.location_id, &params.parameter)
    };

    let start = match parse_timestamp_iso(&params.start) {
        Some(dt) => dt,
        None => return Err(Json(ApiResponse::error("Invalid start timestamp"))),
    };

    let end = match parse_timestamp_iso(&params.end) {
        Some(dt) => dt,
        None => return Err(Json(ApiResponse::error("Invalid end timestamp"))),
    };

    let mut query = TimeSeriesQuery::new(series_id, start, end);

    if let Some(agg) = &params.aggregation {
        if let Some(level) = AggregationLevel::from_str(agg) {
            query.aggregation = Some(level);
        }
    }

    if let Some(func) = &params.function {
        if let Some(f) = parse_aggregation_function(func) {
            query.function = Some(f);
        }
    }

    if let Some(fill) = &params.fill_gaps {
        if let Some(method) = parse_fill_method(fill) {
            query.fill_gaps = Some(method);
        }
    }

    match service.query(&query).await {
        Ok(series) => Ok(Json(ApiResponse::ok(series))),
        Err(e) => {
            warn!("Time series query error: {}", e);
            Err(Json(ApiResponse::error(format!("Query failed: {}", e))))
        }
    }
}

/// Write time series data.
pub async fn write_timeseries(
    Extension(service): Extension<Arc<TimeSeriesService>>,
    Json(req): Json<WriteTimeSeriesRequest>,
) -> Result<Json<ApiResponse<TimeSeriesWriteResult>>, Json<ApiResponse<()>>> {
    let series_id = if let Some(q) = &req.qualifier {
        TimeSeriesId::with_qualifier(&req.location_id, &req.parameter, q)
    } else {
        TimeSeriesId::new(&req.location_id, &req.parameter)
    };

    let mut data = Vec::new();
    for point in &req.data {
        let timestamp = match parse_timestamp_iso(&point.timestamp) {
            Some(dt) => dt,
            None => {
                return Err(Json(ApiResponse::error(format!(
                    "Invalid timestamp: {}",
                    point.timestamp
                ))))
            }
        };

        let flag = point.flag
            .as_ref()
            .and_then(|f| QualityFlag::from_str(f))
            .unwrap_or(QualityFlag::Good);

        data.push(TimeSeriesDataPoint::with_flag(timestamp, point.value, flag));
    }

    let batch = TimeSeriesWriteBatch {
        series_id,
        data,
        attributes: None,
    };

    match service.write_batch(batch).await {
        Ok(result) => {
            info!("Wrote {} time series points", result.points_written);
            Ok(Json(ApiResponse::ok(result)))
        }
        Err(e) => {
            warn!("Time series write error: {}", e);
            Err(Json(ApiResponse::error(format!("Write failed: {}", e))))
        }
    }
}

/// Register a new time series.
pub async fn register_series(
    Extension(service): Extension<Arc<TimeSeriesService>>,
    Json(req): Json<RegisterSeriesRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, Json<ApiResponse<()>>> {
    let series_id = if let Some(q) = &req.qualifier {
        TimeSeriesId::with_qualifier(&req.location_id, &req.parameter, q)
    } else {
        TimeSeriesId::new(&req.location_id, &req.parameter)
    };

    let metadata = TimeSeriesMetadata {
        id: series_id,
        display_name: req.display_name,
        description: req.description,
        units: req.units,
        data_type: req.data_type
            .and_then(|s| parse_data_type(&s))
            .unwrap_or(TimeSeriesDataType::Instantaneous),
        min_value: req.min_value,
        max_value: req.max_value,
        source: req.source.unwrap_or_else(|| "manual".to_string()),
        source_type: req.source_type
            .and_then(|s| parse_source_type(&s))
            .unwrap_or(TimeSeriesSourceType::Manual),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        retention_days: req.retention_days,
        attributes: HashMap::new(),
    };

    match service.register_series(metadata).await {
        Ok(()) => Ok(Json(ApiResponse::ok(serde_json::json!({"registered": true})))),
        Err(e) => {
            warn!("Series registration error: {}", e);
            Err(Json(ApiResponse::error(format!("Registration failed: {}", e))))
        }
    }
}

/// Get series metadata.
pub async fn get_series_metadata(
    Extension(service): Extension<Arc<TimeSeriesService>>,
    Path((location_id, parameter)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<TimeSeriesMetadata>>, Json<ApiResponse<()>>> {
    let qualifier = params.get("qualifier").map(|s| s.as_str());
    let series_id = if let Some(q) = qualifier {
        TimeSeriesId::with_qualifier(&location_id, &parameter, q)
    } else {
        TimeSeriesId::new(&location_id, &parameter)
    };

    match service.get_metadata(&series_id).await {
        Ok(Some(metadata)) => Ok(Json(ApiResponse::ok(metadata))),
        Ok(None) => Err(Json(ApiResponse::error("Series not found"))),
        Err(e) => {
            warn!("Get metadata error: {}", e);
            Err(Json(ApiResponse::error(format!("Query failed: {}", e))))
        }
    }
}

/// List all time series.
pub async fn list_series(
    Extension(service): Extension<Arc<TimeSeriesService>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<TimeSeriesCatalogEntry>>>, Json<ApiResponse<()>>> {
    let source_type = params.get("source_type").map(|s| s.as_str());

    match service.list_series(source_type, None).await {
        Ok(series) => Ok(Json(ApiResponse::ok(series))),
        Err(e) => {
            warn!("List series error: {}", e);
            Err(Json(ApiResponse::error(format!("List failed: {}", e))))
        }
    }
}

/// Get available aggregation levels.
pub async fn get_aggregation_levels() -> Json<ApiResponse<Vec<LevelInfo>>> {
    let levels = vec![
        LevelInfo {
            value: "raw".to_string(),
            label: "Raw Data".to_string(),
            duration_seconds: 0,
            description: "No aggregation, raw data points".to_string(),
        },
        LevelInfo {
            value: "1m".to_string(),
            label: "1 Minute".to_string(),
            duration_seconds: 60,
            description: "1 minute aggregation".to_string(),
        },
        LevelInfo {
            value: "5m".to_string(),
            label: "5 Minutes".to_string(),
            duration_seconds: 300,
            description: "5 minute aggregation".to_string(),
        },
        LevelInfo {
            value: "15m".to_string(),
            label: "15 Minutes".to_string(),
            duration_seconds: 900,
            description: "15 minute aggregation".to_string(),
        },
        LevelInfo {
            value: "1h".to_string(),
            label: "1 Hour".to_string(),
            duration_seconds: 3600,
            description: "1 hour aggregation".to_string(),
        },
        LevelInfo {
            value: "1d".to_string(),
            label: "1 Day".to_string(),
            duration_seconds: 86400,
            description: "1 day aggregation".to_string(),
        },
    ];

    Json(ApiResponse::ok(levels))
}

/// Get available aggregation functions.
pub async fn get_aggregation_functions() -> Json<ApiResponse<Vec<FunctionInfo>>> {
    let functions = vec![
        FunctionInfo {
            value: "average".to_string(),
            label: "Average".to_string(),
            description: "Mean value over the period".to_string(),
        },
        FunctionInfo {
            value: "min".to_string(),
            label: "Minimum".to_string(),
            description: "Minimum value over the period".to_string(),
        },
        FunctionInfo {
            value: "max".to_string(),
            label: "Maximum".to_string(),
            description: "Maximum value over the period".to_string(),
        },
        FunctionInfo {
            value: "sum".to_string(),
            label: "Sum".to_string(),
            description: "Sum of all values over the period".to_string(),
        },
        FunctionInfo {
            value: "count".to_string(),
            label: "Count".to_string(),
            description: "Number of data points".to_string(),
        },
        FunctionInfo {
            value: "first".to_string(),
            label: "First".to_string(),
            description: "First value in the period".to_string(),
        },
        FunctionInfo {
            value: "last".to_string(),
            label: "Last".to_string(),
            description: "Last value in the period".to_string(),
        },
    ];

    Json(ApiResponse::ok(functions))
}

#[derive(Debug, Serialize)]
pub struct LevelInfo {
    value: String,
    label: String,
    duration_seconds: i64,
    description: String,
}

#[derive(Debug, Serialize)]
pub struct FunctionInfo {
    value: String,
    label: String,
    description: String,
}

/// Helper: Parse ISO timestamp.
fn parse_timestamp_iso(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

/// Helper: Parse aggregation function.
fn parse_aggregation_function(s: &str) -> Option<AggregationFunction> {
    match s.to_lowercase().as_str() {
        "average" | "avg" | "mean" => Some(AggregationFunction::Average),
        "min" | "minimum" => Some(AggregationFunction::Min),
        "max" | "maximum" => Some(AggregationFunction::Max),
        "sum" | "total" => Some(AggregationFunction::Sum),
        "count" => Some(AggregationFunction::Count),
        "first" => Some(AggregationFunction::First),
        "last" => Some(AggregationFunction::Last),
        "median" => Some(AggregationFunction::Median),
        "stddev" => Some(AggregationFunction::StdDev),
        _ => None,
    }
}

/// Helper: Parse fill method.
fn parse_fill_method(s: &str) -> Option<FillMethod> {
    match s.to_lowercase().as_str() {
        "none" | "null" => Some(FillMethod::None),
        "linear" | "interpolate" => Some(FillMethod::Linear),
        "forward" | "ffill" => Some(FillMethod::Forward),
        "backward" | "bfill" => Some(FillMethod::Backward),
        "constant" => Some(FillMethod::Constant),
        _ => None,
    }
}

/// Helper: Parse data type.
fn parse_data_type(s: &str) -> Option<TimeSeriesDataType> {
    match s.to_lowercase().as_str() {
        "instantaneous" => Some(TimeSeriesDataType::Instantaneous),
        "accumulated" => Some(TimeSeriesDataType::Accumulated),
        "average" => Some(TimeSeriesDataType::Average),
        "total" => Some(TimeSeriesDataType::Total),
        "boolean" => Some(TimeSeriesDataType::Boolean),
        "enum" => Some(TimeSeriesDataType::Enum),
        _ => None,
    }
}

/// Helper: Parse source type.
fn parse_source_type(s: &str) -> Option<TimeSeriesSourceType> {
    match s.to_lowercase().as_str() {
        "fews" => Some(TimeSeriesSourceType::Fews),
        "hydronet" => Some(TimeSeriesSourceType::Hydronet),
        "dhydro" => Some(TimeSeriesSourceType::DHydro),
        "energyzero" => Some(TimeSeriesSourceType::EnergyZero),
        "manual" => Some(TimeSeriesSourceType::Manual),
        "calculated" => Some(TimeSeriesSourceType::Calculated),
        _ => Some(TimeSeriesSourceType::Custom(s.to_string())),
    }
}
