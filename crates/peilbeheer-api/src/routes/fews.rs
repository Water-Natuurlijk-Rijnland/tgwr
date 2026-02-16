//! Delft-FEWS integration routes.
//!
//! These endpoints provide access to Delft-FEWS time series data,
//! location/parameter metadata, and synchronization functionality.

use axum::{
    extract::{Extension, Query},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use peilbeheer_core::{
    FewsConfig, FewsLocation, FewsModuleInstance, FewsParameter, FewsSyncConfig, FewsSyncRequest,
    FewsSyncResult, FewsTimeSeriesQuery, FewsTimeSeriesResponse,
};

use crate::fews_client::{FewsClient, FewsError, FewsSyncService};

/// Query parameters for time series requests.
#[derive(Debug, Deserialize)]
pub struct FewsQueryParams {
    pub location_ids: Option<String>,
    pub parameter_ids: Option<String>,
    pub module_instance_ids: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub qualifier: Option<String>,
    pub hours_back: Option<i64>,
}

/// Response wrapper for Fews errors.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: String,
    detail: Option<String>,
}

/// Fetch time series data from Fews.
pub async fn get_time_series(
    Extension(client): Extension<Arc<FewsClient>>,
    Query(params): Query<FewsQueryParams>,
) -> Result<Json<FewsTimeSeriesResponse>, ErrorResponse> {
    let mut query = FewsTimeSeriesQuery::default();

    if let Some(locs) = &params.location_ids {
        query.location_ids = Some(locs.split(',').map(|s| s.to_string()).collect());
    }

    if let Some(params) = &params.parameter_ids {
        query.parameter_ids = Some(params.split(',').map(|s| s.to_string()).collect());
    }

    if let Some(modules) = &params.module_instance_ids {
        query.module_instance_ids = Some(modules.split(',').map(|s| s.to_string()).collect());
    }

    // Parse time range
    if let Some(hours) = params.hours_back {
        let end_time = Utc::now();
        let start_time = end_time - Duration::hours(hours);
        query.start_time = Some(start_time);
        query.end_time = Some(end_time);
    } else {
        if let Some(start) = &params.start {
            if let Ok(dt) = DateTime::parse_from_rfc3339(start) {
                query.start_time = Some(dt.with_timezone(&Utc));
            }
        }
        if let Some(end) = &params.end {
            if let Ok(dt) = DateTime::parse_from_rfc3339(end) {
                query.end_time = Some(dt.with_timezone(&Utc));
            }
        }
    }

    if let Some(qualifier) = &params.qualifier {
        query.qualifier = Some(qualifier.clone());
    }

    client.get_time_series(&query)
        .await
        .map(Json)
        .map_err(|e| ErrorResponse {
            error: "Failed to fetch time series".to_string(),
            detail: Some(e.to_string()),
        })
}

/// Get available locations from Fews.
pub async fn get_locations(
    Extension(client): Extension<Arc<FewsClient>>,
) -> Result<Json<Vec<FewsLocation>>, ErrorResponse> {
    client.get_locations()
        .await
        .map(Json)
        .map_err(|e| ErrorResponse {
            error: "Failed to fetch locations".to_string(),
            detail: Some(e.to_string()),
        })
}

/// Get available parameters from Fews.
pub async fn get_parameters(
    Extension(client): Extension<Arc<FewsClient>>,
) -> Result<Json<Vec<FewsParameter>>, ErrorResponse> {
    client.get_parameters()
        .await
        .map(Json)
        .map_err(|e| ErrorResponse {
            error: "Failed to fetch parameters".to_string(),
            detail: Some(e.to_string()),
        })
}

/// Get available module instances from Fews.
pub async fn get_module_instances(
    Extension(client): Extension<Arc<FewsClient>>,
) -> Result<Json<Vec<FewsModuleInstance>>, ErrorResponse> {
    client.get_module_instances()
        .await
        .map(Json)
        .map_err(|e| ErrorResponse {
            error: "Failed to fetch module instances".to_string(),
            detail: Some(e.to_string()),
        })
}

/// Sync data from Fews.
pub async fn sync_fews(
    Extension(client): Extension<Arc<FewsClient>>,
    Json(request): Json<FewsSyncRequest>,
) -> Result<Json<FewsSyncResult>, ErrorResponse> {
    client.sync(&request)
        .await
        .map(Json)
        .map_err(|e| ErrorResponse {
            error: "Fews sync failed".to_string(),
            detail: Some(e.to_string()),
        })
}

/// Test Fews connection.
pub async fn ping_fews(
    Extension(client): Extension<Arc<FewsClient>>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let success = client.ping()
        .await
        .map_err(|e| ErrorResponse {
            error: "Fews ping failed".to_string(),
            detail: Some(e.to_string()),
        })?;

    Ok(Json(serde_json::json!({
        "success": success,
        "timestamp": Utc::now().to_rfc3339(),
    })))
}

/// Get Fews sync configurations.
pub async fn get_sync_configs(
    Extension(service): Extension<Arc<FewsSyncService>>,
) -> Json<Vec<FewsSyncConfig>> {
    Json(service.get_configs().to_vec())
}

/// Ping Fews connection status.
pub async fn fews_status(
    Extension(client): Extension<Arc<FewsClient>>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let success = client.ping().await.unwrap_or(false);

    Ok(Json(serde_json::json!({
        "connected": success,
        "timestamp": Utc::now().to_rfc3339(),
        "config": {
            "base_url": client.config.base_url,
            "filter_id": client.config.filter_id,
        }
    })))
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        let status = match self.error.as_str() {
            "Authentication failed" => axum::http::StatusCode::UNAUTHORIZED,
            "Location not found" | "Parameter not found" | "Module instance not found" => axum::http::StatusCode::NOT_FOUND,
            _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_params_parse() {
        let query = "location_ids=LOC1,LOC2&parameter_ids=H&hours_back=24";
        let params: FewsQueryParams = serde_urlencoded::from_str(query).unwrap();
        assert_eq!(params.location_ids, Some("LOC1,LOC2".to_string()));
        assert_eq!(params.parameter_ids, Some("H".to_string()));
        assert_eq!(params.hours_back, Some(24));
    }
}
