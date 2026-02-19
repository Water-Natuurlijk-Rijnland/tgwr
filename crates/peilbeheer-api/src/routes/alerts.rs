//! Alert Rule Engine API routes.
//!
//! Endpoints for managing alert rules and viewing triggered alerts.

use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};

use peilbeheer_core::alert::*;

use crate::alert_service::AlertService;
use crate::auth_service::AuthService;

/// Response wrapper for API responses.
#[derive(Debug, Serialize)]
struct ApiResponse<T> {
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

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        let status = if self.success {
            StatusCode::OK
        } else {
            StatusCode::BAD_REQUEST
        };
        (status, Json(self)).into_response()
    }
}

/// Query parameters for listing rules.
#[derive(Debug, Deserialize)]
pub struct ListRulesQuery {
    pub category: Option<String>,
    pub severity: Option<String>,
    pub enabled: Option<bool>,
}

/// Query parameters for listing alerts.
#[derive(Debug, Deserialize)]
pub struct ListAlertsQuery {
    pub status: Option<String>,
    pub severity: Option<String>,
    pub category: Option<String>,
    pub rule_id: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub acknowledged_by: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

/// Manual evaluation request.
#[derive(Debug, Deserialize)]
pub struct EvaluateRulesRequest {
    pub context: EvaluationContextBody,
}

/// Evaluation context for manual evaluation.
#[derive(Debug, Deserialize, Serialize)]
pub struct EvaluationContextBody {
    pub values: HashMap<String, serde_json::Value>,
    pub source: Option<String>,
}

/// List all alert rules.
pub async fn list_rules(
    Extension(service): Extension<Arc<AlertService>>,
    Query(params): Query<ListRulesQuery>,
) -> impl IntoResponse {
    match service.list_rules().await {
        Ok(rules) => {
            let filtered: Vec<_> = rules
                .into_iter()
                .filter(|r| {
                    if let Some(cat) = &params.category
                        && r.category.as_str() != cat {
                            return false;
                        }
                    if let Some(sev) = &params.severity
                        && r.severity.as_str() != sev {
                            return false;
                        }
                    if let Some(enabled) = params.enabled
                        && r.enabled != enabled {
                            return false;
                        }
                    true
                })
                .collect();

            Json(ApiResponse::ok(filtered))
        }
        Err(e) => {
            error!("Failed to list rules: {}", e);
            Json(ApiResponse::<Vec<AlertRule>>::error(e.to_string()))
        }
    }
}

/// Get a specific rule by ID.
pub async fn get_rule(
    Extension(service): Extension<Arc<AlertService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match service.get_rule(&id).await {
        Ok(rule) => Json(ApiResponse::ok(rule)),
        Err(e) => {
            error!("Failed to get rule {}: {}", id, e);
            Json(ApiResponse::<AlertRule>::error(e.to_string()))
        }
    }
}

/// Create a new alert rule.
pub async fn create_rule(
    Extension(service): Extension<Arc<AlertService>>,
    Extension(_auth): Extension<Arc<AuthService>>,
    Json(request): Json<CreateAlertRuleRequest>,
) -> impl IntoResponse {
    match service.create_rule(request, None).await {
        Ok(rule) => {
            info!("Created alert rule: {}", rule.id);
            Json(ApiResponse::ok(rule))
        }
        Err(e) => {
            error!("Failed to create rule: {}", e);
            Json(ApiResponse::<AlertRule>::error(e.to_string()))
        }
    }
}

/// Update an existing rule.
pub async fn update_rule(
    Extension(service): Extension<Arc<AlertService>>,
    Path(id): Path<String>,
    Json(request): Json<UpdateAlertRuleRequest>,
) -> impl IntoResponse {
    match service.update_rule(&id, request).await {
        Ok(rule) => {
            info!("Updated alert rule: {}", id);
            Json(ApiResponse::ok(rule))
        }
        Err(e) => {
            error!("Failed to update rule {}: {}", id, e);
            Json(ApiResponse::<AlertRule>::error(e.to_string()))
        }
    }
}

/// Delete a rule.
pub async fn delete_rule(
    Extension(service): Extension<Arc<AlertService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match service.delete_rule(&id).await {
        Ok(()) => {
            info!("Deleted alert rule: {}", id);
            Json(ApiResponse::ok(serde_json::json!({"deleted": true})))
        }
        Err(e) => {
            error!("Failed to delete rule {}: {}", id, e);
            Json(ApiResponse::<serde_json::Value>::error(e.to_string()))
        }
    }
}

/// List triggered alerts.
pub async fn list_alerts(
    Extension(service): Extension<Arc<AlertService>>,
    Query(params): Query<ListAlertsQuery>,
) -> impl IntoResponse {
    let query = build_alert_query(params);
    match service.query_alerts(&query).await {
        Ok(alerts) => Json(ApiResponse::ok(alerts)),
        Err(e) => {
            error!("Failed to list alerts: {}", e);
            Json(ApiResponse::<Vec<Alert>>::error(e.to_string()))
        }
    }
}

/// Get a specific alert by ID.
pub async fn get_alert(
    Extension(service): Extension<Arc<AlertService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match service.get_alert(&id).await {
        Ok(alert) => Json(ApiResponse::ok(alert)),
        Err(e) => {
            error!("Failed to get alert {}: {}", id, e);
            Json(ApiResponse::<Alert>::error(e.to_string()))
        }
    }
}

/// Acknowledge an alert.
pub async fn acknowledge_alert(
    Extension(service): Extension<Arc<AlertService>>,
    Path(id): Path<String>,
    Json(request): Json<AcknowledgeAlertRequest>,
) -> impl IntoResponse {
    match service.acknowledge_alert(&id, request).await {
        Ok(alert) => {
            info!("Alert {} acknowledged", id);
            Json(ApiResponse::ok(alert))
        }
        Err(e) => {
            error!("Failed to acknowledge alert {}: {}", id, e);
            Json(ApiResponse::<Alert>::error(e.to_string()))
        }
    }
}

/// Resolve an alert.
pub async fn resolve_alert(
    Extension(service): Extension<Arc<AlertService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match service.resolve_alert(&id).await {
        Ok(alert) => {
            info!("Alert {} resolved", id);
            Json(ApiResponse::ok(alert))
        }
        Err(e) => {
            error!("Failed to resolve alert {}: {}", id, e);
            Json(ApiResponse::<Alert>::error(e.to_string()))
        }
    }
}

/// Get alert statistics.
pub async fn get_alert_stats(
    Extension(service): Extension<Arc<AlertService>>,
) -> impl IntoResponse {
    match service.get_stats().await {
        Ok(stats) => Json(ApiResponse::ok(stats)),
        Err(e) => {
            error!("Failed to get alert stats: {}", e);
            Json(ApiResponse::<AlertStats>::error(e.to_string()))
        }
    }
}

/// Manually evaluate rules with given context.
pub async fn evaluate_rules(
    Extension(service): Extension<Arc<AlertService>>,
    Json(request): Json<EvaluateRulesRequest>,
) -> impl IntoResponse {
    // Convert JSON values to AlertValues
    let mut values = HashMap::new();
    for (key, json_val) in &request.context.values {
        let alert_value = match json_val {
            serde_json::Value::Number(n) => {
                AlertValue::Number(n.as_f64().unwrap_or(0.0))
            }
            serde_json::Value::String(s) => {
                AlertValue::String(s.clone())
            }
            serde_json::Value::Bool(b) => {
                AlertValue::Boolean(*b)
            }
            serde_json::Value::Array(arr) => {
                let strings: Vec<String> = arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                AlertValue::Array(strings)
            }
            _ => continue,
        };
        values.insert(key.clone(), alert_value);
    }

    let context = EvaluationContext {
        now: chrono::Utc::now(),
        values,
        time_series: HashMap::new(),
        source: request.context.source,
    };

    match service.evaluate_rules(&context).await {
        Ok(alerts) => {
            info!("Manual rule evaluation triggered {} alerts", alerts.len());
            Json(ApiResponse::ok(serde_json::json!({
                "triggered_count": alerts.len(),
                "alerts": alerts,
            })))
        }
        Err(e) => {
            error!("Failed to evaluate rules: {}", e);
            Json(ApiResponse::<serde_json::Value>::error(e.to_string()))
        }
    }
}

/// Get alert categories.
pub async fn get_categories() -> impl IntoResponse {
    let categories = vec![
        serde_json::json!({"value": "water_level", "label": "Water Level", "description": "Water level thresholds"}),
        serde_json::json!({"value": "pump_status", "label": "Pump Status", "description": "Pump/gemaal operational status"}),
        serde_json::json!({"value": "energy_price", "label": "Energy Price", "description": "Electricity price alerts"}),
        serde_json::json!({"value": "weather", "label": "Weather", "description": "Weather-related alerts"}),
        serde_json::json!({"value": "system_health", "label": "System Health", "description": "System and infrastructure health"}),
        serde_json::json!({"value": "simulation", "label": "Simulation", "description": "Simulation/scenario results"}),
    ];
    Json(ApiResponse::ok(categories))
}

/// Get comparison operators.
pub async fn get_operators() -> impl IntoResponse {
    let operators = vec![
        serde_json::json!({"value": "eq", "symbol": "==", "label": "Equals", "types": ["number", "string", "boolean"]}),
        serde_json::json!({"value": "ne", "symbol": "!=", "label": "Not Equals", "types": ["number", "string", "boolean"]}),
        serde_json::json!({"value": "gt", "symbol": ">", "label": "Greater Than", "types": ["number"]}),
        serde_json::json!({"value": "gte", "symbol": ">=", "label": "Greater Than or Equal", "types": ["number"]}),
        serde_json::json!({"value": "lt", "symbol": "<", "label": "Less Than", "types": ["number"]}),
        serde_json::json!({"value": "lte", "symbol": "<=", "label": "Less Than or Equal", "types": ["number"]}),
        serde_json::json!({"value": "contains", "symbol": "contains", "label": "Contains", "types": ["string"]}),
        serde_json::json!({"value": "not_contains", "symbol": "not_contains", "label": "Does Not Contain", "types": ["string"]}),
        serde_json::json!({"value": "is_null", "symbol": "is_null", "label": "Is Null/Empty", "types": ["any"]}),
        serde_json::json!({"value": "is_not_null", "symbol": "is_not_null", "label": "Is Not Null", "types": ["any"]}),
    ];
    Json(ApiResponse::ok(operators))
}

/// Helper: Build AlertQuery from request parameters.
fn build_alert_query(params: ListAlertsQuery) -> AlertQuery {
    AlertQuery {
        status: params.status.and_then(|s| match s.as_str() {
            "active" => Some(AlertStatus::Active),
            "acknowledged" => Some(AlertStatus::Acknowledged),
            "resolved" => Some(AlertStatus::Resolved),
            "suppressed" => Some(AlertStatus::Suppressed),
            _ => None,
        }),
        severity: params.severity.and_then(|s| AlertSeverity::from_str(&s)),
        category: params.category.map(|s| match s.as_str() {
            "water_level" => AlertCategory::WaterLevel,
            "pump_status" => AlertCategory::PumpStatus,
            "energy_price" => AlertCategory::EnergyPrice,
            "weather" => AlertCategory::Weather,
            "system_health" => AlertCategory::SystemHealth,
            "simulation" => AlertCategory::Simulation,
            other => AlertCategory::Custom(other.to_string()),
        }),
        rule_id: params.rule_id,
        start_time: params.start_time.and_then(|s| parse_datetime_iso(&s)),
        end_time: params.end_time.and_then(|s| parse_datetime_iso(&s)),
        acknowledged_by: params.acknowledged_by,
        limit: params.limit,
        offset: params.offset,
    }
}

/// Helper: Parse ISO datetime string.
fn parse_datetime_iso(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}
