//! Scenario management routes.
//!
//! RESTful API endpoints for hydraulic modeling scenario CRUD operations,
//! execution management, and result retrieval.

use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use peilbeheer_core::{
    CloneScenarioRequest, CreateScenarioRequest, StoredScenario, StoredScenarioStatus,
    StoredScenarioResult, UpdateScenarioRequest,
};

use crate::scenario_service::ScenarioService;

/// Query parameters for scenario listing.
#[derive(Debug, Deserialize)]
pub struct ScenarioListQuery {
    pub model_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<usize>,
}

/// Response wrapper for API errors.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: String,
    detail: Option<String>,
}

/// List all scenarios.
pub async fn list_scenarios(
    Extension(service): Extension<Arc<ScenarioService>>,
    Query(params): Query<ScenarioListQuery>,
) -> Result<Json<Vec<StoredScenario>>, ErrorResponse> {
    let status = params.status
        .as_ref()
        .and_then(|s| StoredScenarioStatus::from_str(s));

    service
        .list_scenarios(params.model_id.as_deref(), status.as_ref(), params.limit)
        .map(Json)
        .map_err(|e| ErrorResponse {
            error: "Failed to list scenarios".to_string(),
            detail: Some(e.to_string()),
        })
}

/// Get a specific scenario by ID.
pub async fn get_scenario(
    Extension(service): Extension<Arc<ScenarioService>>,
    Path(id): Path<String>,
) -> Result<Json<StoredScenario>, ErrorResponse> {
    match service.get_scenario(&id) {
        Ok(Some(scenario)) => Ok(Json(scenario)),
        Ok(None) => Err(ErrorResponse {
            error: "Scenario not found".to_string(),
            detail: Some(format!("No scenario found with ID: {}", id)),
        }),
        Err(e) => Err(ErrorResponse {
            error: "Failed to get scenario".to_string(),
            detail: Some(e.to_string()),
        }),
    }
}

/// Create a new scenario.
pub async fn create_scenario(
    Extension(service): Extension<Arc<ScenarioService>>,
    Json(req): Json<CreateScenarioRequest>,
) -> Result<Json<StoredScenario>, ErrorResponse> {
    service
        .create_scenario(&req)
        .map(|scenario| {
            tracing::info!("Created scenario: {} ({})", scenario.name, scenario.id);
            Json(scenario)
        })
        .map_err(|e| ErrorResponse {
            error: "Failed to create scenario".to_string(),
            detail: Some(e.to_string()),
        })
}

/// Update an existing scenario.
pub async fn update_scenario(
    Extension(service): Extension<Arc<ScenarioService>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateScenarioRequest>,
) -> Result<Json<StoredScenario>, ErrorResponse> {
    service
        .update_scenario(&id, &req, None)
        .map(|scenario| {
            tracing::info!("Updated scenario: {}", id);
            Json(scenario)
        })
        .map_err(|e| ErrorResponse {
            error: "Failed to update scenario".to_string(),
            detail: Some(e.to_string()),
        })
}

/// Delete a scenario.
pub async fn delete_scenario(
    Extension(service): Extension<Arc<ScenarioService>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ErrorResponse> {
    service
        .delete_scenario(&id)
        .map(|_| {
            tracing::info!("Deleted scenario: {}", id);
            StatusCode::NO_CONTENT
        })
        .map_err(|e| ErrorResponse {
            error: "Failed to delete scenario".to_string(),
            detail: Some(e.to_string()),
        })
}

/// Execute a scenario (create execution record).
pub async fn execute_scenario(
    Extension(service): Extension<Arc<ScenarioService>>,
    Path(id): Path<String>,
) -> Result<Json<StoredScenarioResult>, ErrorResponse> {
    service
        .execute_scenario(&id, None)
        .map(|result_id| {
            tracing::info!("Started execution for scenario: {}", id);
            // Return the result ID as a minimal result object
            Json(StoredScenarioResult {
                id: result_id,
                scenario_id: id.clone(),
                status: "pending".to_string(),
                started_at: None,
                completed_at: None,
                duration_seconds: None,
                results_summary: serde_json::json!({}),
                time_series_count: 0,
                output_files: serde_json::json!([]),
                error_message: None,
                error_code: None,
                created_at: chrono::Utc::now(),
                created_by: None,
                dhydro_job_id: None,
                dhydro_result_url: None,
            })
        })
        .map_err(|e| ErrorResponse {
            error: "Failed to execute scenario".to_string(),
            detail: Some(e.to_string()),
        })
}

/// Get scenario execution results.
pub async fn get_scenario_results(
    Extension(service): Extension<Arc<ScenarioService>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<StoredScenarioResult>>, ErrorResponse> {
    service
        .get_scenario_results(&id)
        .map(Json)
        .map_err(|e| ErrorResponse {
            error: "Failed to get scenario results".to_string(),
            detail: Some(e.to_string()),
        })
}

/// Clone a scenario.
pub async fn clone_scenario(
    Extension(service): Extension<Arc<ScenarioService>>,
    Path(id): Path<String>,
    Json(req): Json<CloneScenarioRequest>,
) -> Result<Json<StoredScenario>, ErrorResponse> {
    service
        .clone_scenario(&id, &req, None)
        .map(|scenario| {
            tracing::info!("Cloned scenario: {} -> {}", id, scenario.id);
            Json(scenario)
        })
        .map_err(|e| ErrorResponse {
            error: "Failed to clone scenario".to_string(),
            detail: Some(e.to_string()),
        })
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        let status = match self.error.as_str() {
            "Scenario not found" => StatusCode::NOT_FOUND,
            "Failed to create scenario" | "Failed to update scenario" => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_list_query_deserialize() {
        let query = "model_id=MODEL_001&status=draft&limit=10";
        let req: ScenarioListQuery = serde_urlencoded::from_str(query).unwrap();
        assert_eq!(req.model_id, Some("MODEL_001".to_string()));
        assert_eq!(req.status, Some("draft".to_string()));
        assert_eq!(req.limit, Some(10));
    }

    #[test]
    fn test_scenario_list_query_empty() {
        let query = "";
        let req: ScenarioListQuery = serde_urlencoded::from_str(query).unwrap();
        assert_eq!(req.model_id, None);
        assert_eq!(req.status, None);
        assert_eq!(req.limit, None);
    }
}
