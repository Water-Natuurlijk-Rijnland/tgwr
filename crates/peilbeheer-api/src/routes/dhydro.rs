//! DHYdro integration routes.
//!
//! These endpoints provide access to DHYdro platform functionality including
//! model management, time series data, scenario management, and simulation results.

use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use peilbeheer_core::{
    DhydroClient, DhydroModel, Scenario, ScenarioParameters, ScenarioResult,
    TimeSeries, TimeSeriesQuery,
};

/// Query parameters for time series requests.
#[derive(Debug, Deserialize)]
pub struct TimeSeriesRequest {
    pub location_id: Option<String>,
    pub parameter: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub aggregation: Option<String>,
}

/// Query parameters for scenario listing.
#[derive(Debug, Deserialize)]
pub struct ScenarioListRequest {
    pub model_id: Option<String>,
}

/// Request body for creating a new scenario.
#[derive(Debug, Deserialize)]
pub struct CreateScenarioRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub model_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub time_step: u32,
    #[serde(default)]
    pub boundary_conditions: Option<serde_json::Value>,
    #[serde(default)]
    pub initial_conditions: Option<serde_json::Value>,
    #[serde(default)]
    pub model_parameters: Option<serde_json::Value>,
    #[serde(default)]
    pub base_scenario_id: Option<String>,
}

/// Request body for updating a scenario.
#[derive(Debug, Deserialize)]
pub struct UpdateScenarioRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub time_step: Option<u32>,
    #[serde(default)]
    pub boundary_conditions: Option<serde_json::Value>,
    #[serde(default)]
    pub initial_conditions: Option<serde_json::Value>,
    #[serde(default)]
    pub model_parameters: Option<serde_json::Value>,
}

/// Response wrapper for DHYdro API errors.
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    detail: Option<String>,
}

/// List all available DHYdro models.
pub async fn list_models(
    State(client): State<Arc<DhydroClient>>,
) -> Result<Json<Vec<DhydroModel>>, ErrorResponse> {
    // Note: We need interior mutability for the client (token refresh)
    // For now, we'll create a new client per request
    // TODO: Use RwLock or tokio::sync::Mutex for shared mutable client
    Err(ErrorResponse {
        error: "Not implemented".to_string(),
        detail: Some("Use POST /api/dhydro/models/list for now".to_string()),
    })
}

/// List all available DHYdro models (POST endpoint).
pub async fn list_models_post(
    Extension(client): Extension<Arc<DhydroClient>>,
) -> Result<Json<Vec<DhydroModel>>, ErrorResponse> {
    // We need to get a mutable client - for now clone if needed
    // This is a limitation that will be fixed with proper async locking
    Err(ErrorResponse {
        error: "Authentication required".to_string(),
        detail: Some("DHYdro integration requires valid credentials".to_string()),
    })
}

/// Get a specific model by ID.
pub async fn get_model(
    State(client): State<Arc<DhydroClient>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<DhydroModel>, ErrorResponse> {
    Err(ErrorResponse {
        error: "Not implemented".to_string(),
        detail: Some("DHYdro client integration in progress".to_string()),
    })
}

/// Fetch time series data from DHYdro.
pub async fn get_time_series(
    State(client): State<Arc<DhydroClient>>,
    Query(params): Query<TimeSeriesRequest>,
) -> Result<Json<Vec<TimeSeries>>, ErrorResponse> {
    let start = params.start.and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&Utc));
    let end = params.end.and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let aggregation = match params.aggregation.as_deref() {
        Some("raw") => Some(peilbeheer_core::TimeSeriesAggregation::Raw),
        Some("hourly") => Some(peilbeheer_core::TimeSeriesAggregation::Hourly),
        Some("daily") => Some(peilbeheer_core::TimeSeriesAggregation::Daily),
        Some("weekly") => Some(peilbeheer_core::TimeSeriesAggregation::Weekly),
        Some("monthly") => Some(peilbeheer_core::TimeSeriesAggregation::Monthly),
        _ => None,
    };

    let query = TimeSeriesQuery {
        location_id: params.location_id,
        parameter: params.parameter,
        start,
        end,
        aggregation,
    };

    Err(ErrorResponse {
        error: "Not implemented".to_string(),
        detail: Some("DHYdro client integration in progress".to_string()),
    })
}

/// List all scenarios (optionally filtered by model).
pub async fn list_scenarios(
    State(client): State<Arc<DhydroClient>>,
    Query(params): Query<ScenarioListRequest>,
) -> Result<Json<Vec<Scenario>>, ErrorResponse> {
    Err(ErrorResponse {
        error: "Not implemented".to_string(),
        detail: Some("DHYdro client integration in progress".to_string()),
    })
}

/// Get a specific scenario by ID.
pub async fn get_scenario(
    State(client): State<Arc<DhydroClient>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<Scenario>, ErrorResponse> {
    Err(ErrorResponse {
        error: "Not implemented".to_string(),
        detail: Some("DHYdro client integration in progress".to_string()),
    })
}

/// Create a new scenario.
pub async fn create_scenario(
    State(client): State<Arc<DhydroClient>>,
    Json(req): Json<CreateScenarioRequest>,
) -> Result<Json<Scenario>, ErrorResponse> {
    let parameters = ScenarioParameters {
        start_time: req.start_time,
        end_time: req.end_time,
        time_step: req.time_step,
        boundary_conditions: req.boundary_conditions.unwrap_or_default(),
        initial_conditions: req.initial_conditions.unwrap_or_default(),
        model_parameters: req.model_parameters.unwrap_or_default(),
    };

    let scenario = Scenario {
        id: String::new(), // Will be assigned by DHYdro
        name: req.name,
        description: req.description,
        model_id: req.model_id,
        parameters,
        created_at: None,
        created_by: None,
        is_base_scenario: req.base_scenario_id.is_none(),
        base_scenario_id: req.base_scenario_id,
    };

    Err(ErrorResponse {
        error: "Not implemented".to_string(),
        detail: Some("DHYdro client integration in progress".to_string()),
    })
}

/// Execute a scenario.
pub async fn execute_scenario(
    State(client): State<Arc<DhydroClient>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ScenarioResult>, ErrorResponse> {
    Err(ErrorResponse {
        error: "Not implemented".to_string(),
        detail: Some("DHYdro client integration in progress".to_string()),
    })
}

/// Get scenario execution results.
pub async fn get_scenario_results(
    State(client): State<Arc<DhydroClient>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ScenarioResult>, ErrorResponse> {
    Err(ErrorResponse {
        error: "Not implemented".to_string(),
        detail: Some("DHYdro client integration in progress".to_string()),
    })
}

/// Delete a scenario.
pub async fn delete_scenario(
    State(client): State<Arc<DhydroClient>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, ErrorResponse> {
    Err(ErrorResponse {
        error: "Not implemented".to_string(),
        detail: Some("DHYdro client integration in progress".to_string()),
    })
}

/// Clone a scenario.
pub async fn clone_scenario(
    State(client): State<Arc<DhydroClient>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<Scenario>, ErrorResponse> {
    let new_name = req.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Cloned Scenario");

    Err(ErrorResponse {
        error: "Not implemented".to_string(),
        detail: Some("DHYdro client integration in progress".to_string()),
    })
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        let status = match self.error.as_str() {
            "Not implemented" => StatusCode::NOT_IMPLEMENTED,
            "Authentication required" => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_series_request_deserialize() {
        let query = "location_id=LOC001&parameter=water_level&start=2024-01-01T00:00:00Z";
        let req: TimeSeriesRequest = serde_urlencoded::from_str(query).unwrap();
        assert_eq!(req.location_id, Some("LOC001".to_string()));
        assert_eq!(req.parameter, Some("water_level".to_string()));
    }

    #[test]
    fn test_create_scenario_request_deserialize() {
        let json = r#"{
            "name": "Test Scenario",
            "model_id": "MODEL_001",
            "start_time": "2024-01-01T00:00:00Z",
            "end_time": "2024-01-02T00:00:00Z",
            "time_step": 300
        }"#;

        let req: CreateScenarioRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Test Scenario");
        assert_eq!(req.model_id, "MODEL_001");
        assert_eq!(req.time_step, 300);
    }
}
