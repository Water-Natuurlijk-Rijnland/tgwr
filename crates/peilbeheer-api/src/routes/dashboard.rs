//! Dashboard API routes for KPIs and widget data.
//!
//! Endpoints for dashboard aggregation and widget data.

use axum::{
    extract::{Extension, Query},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

use peilbeheer_core::dashboard::*;

use crate::dashboard_service::DashboardService;

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

/// Query parameters for activity feed.
#[derive(Debug, Deserialize)]
pub struct ActivityQueryParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub activity_type: Option<String>,
}

/// Query parameters for chart data.
#[derive(Debug, Deserialize)]
pub struct ChartQueryParams {
    pub metric: String,
    pub hours_back: Option<u32>,
}

/// Get all dashboard KPIs.
pub async fn get_kpi(
    Extension(service): Extension<Arc<DashboardService>>,
) -> Result<Json<ApiResponse<DashboardKpi>>, Json<ApiResponse<()>>> {
    match service.get_kpi().await {
        Ok(kpi) => Ok(Json(ApiResponse::ok(kpi))),
        Err(e) => {
            tracing::error!("Failed to get dashboard KPIs: {}", e);
            Err(Json(ApiResponse::error(format!("Failed to get KPIs: {}", e))))
        }
    }
}

/// Get system health status.
pub async fn get_health(
    Extension(service): Extension<Arc<DashboardService>>,
) -> Result<Json<ApiResponse<HealthStatus>>, Json<ApiResponse<()>>> {
    match service.get_health_status().await {
        Ok(status) => Ok(Json(ApiResponse::ok(status))),
        Err(e) => {
            tracing::error!("Failed to get health status: {}", e);
            Err(Json(ApiResponse::error(format!("Failed to get status: {}", e))))
        }
    }
}

/// Get activity feed.
pub async fn get_activity_feed(
    Extension(service): Extension<Arc<DashboardService>>,
    Query(params): Query<ActivityQueryParams>,
) -> Result<Json<ApiResponse<ActivityFeedData>>, Json<ApiResponse<()>>> {
    let mut query = ActivityFeedQuery {
        limit: params.limit,
        offset: params.offset,
        activity_type: params.activity_type
            .as_ref()
            .and_then(|t| parse_activity_type(t)),
        ..Default::default()
    };

    match service.get_activity_feed(&query).await {
        Ok(feed) => Ok(Json(ApiResponse::ok(feed))),
        Err(e) => {
            tracing::error!("Failed to get activity feed: {}", e);
            Err(Json(ApiResponse::error(format!("Failed to get feed: {}", e))))
        }
    }
}

/// Get alert summary.
pub async fn get_alert_summary(
    Extension(service): Extension<Arc<DashboardService>>,
) -> Result<Json<ApiResponse<AlertKpi>>, Json<ApiResponse<()>>> {
    match service.get_alert_summary().await {
        Ok(summary) => Ok(Json(ApiResponse::ok(summary))),
        Err(e) => {
            tracing::error!("Failed to get alert summary: {}", e);
            Err(Json(ApiResponse::error(format!("Failed to get summary: {}", e))))
        }
    }
}

/// Get gemaal summary.
pub async fn get_gemaal_summary(
    Extension(service): Extension<Arc<DashboardService>>,
) -> Result<Json<ApiResponse<GemaalKpi>>, Json<ApiResponse<()>>> {
    match service.get_gemaal_summary().await {
        Ok(summary) => Ok(Json(ApiResponse::ok(summary))),
        Err(e) => {
            tracing::error!("Failed to get gemaal summary: {}", e);
            Err(Json(ApiResponse::error(format!("Failed to get summary: {}", e))))
        }
    }
}

/// Get chart data.
pub async fn get_chart(
    Extension(service): Extension<Arc<DashboardService>>,
    Query(params): Query<ChartQueryParams>,
) -> Result<Json<ApiResponse<ChartData>>, Json<ApiResponse<()>>> {
    let hours_back = params.hours_back.unwrap_or(24);

    match service.get_chart_data(&params.metric, hours_back).await {
        Ok(chart) => Ok(Json(ApiResponse::ok(chart))),
        Err(e) => {
            tracing::error!("Failed to get chart data: {}", e);
            Err(Json(ApiResponse::error(format!("Failed to get chart: {}", e))))
        }
    }
}

/// Get system overview widget.
pub async fn get_system_overview_widget(
    Extension(service): Extension<Arc<DashboardService>>,
) -> Result<Json<ApiResponse<DashboardWidget>>, Json<ApiResponse<()>>> {
    match service.get_kpi().await {
        Ok(kpi) => {
            let widget = DashboardWidget {
                id: "system_overview".to_string(),
                widget_type: WidgetType::KpiCards,
                title: "System Overview".to_string(),
                data: WidgetData::KpiCards(KpiCardsData {
                    cards: vec![
                        KpiCard {
                            id: "gemalen_total".to_string(),
                            label: "Gemalen".to_string(),
                            value: kpi.gemalen.total.to_string(),
                            unit: None,
                            trend: None,
                            trend_percent: None,
                            status: if kpi.gemalen.error > 0 {
                                HealthStatus::Degraded
                            } else {
                                HealthStatus::Healthy
                            },
                            link: Some("/api/gemalen".to_string()),
                        },
                        KpiCard {
                            id: "active_alerts".to_string(),
                            label: "Active Alerts".to_string(),
                            value: kpi.alerts.active_total.to_string(),
                            unit: None,
                            trend: None,
                            trend_percent: None,
                            status: match kpi.alerts.critical {
                                0 => HealthStatus::Healthy,
                                _ => HealthStatus::Unhealthy,
                            },
                            link: Some("/api/alerts".to_string()),
                        },
                        KpiCard {
                            id: "system_health".to_string(),
                            label: "System Health".to_string(),
                            value: kpi.system.status.as_str().to_string(),
                            unit: None,
                            trend: None,
                            trend_percent: None,
                            status: kpi.system.status,
                            link: Some("/api/health".to_string()),
                        },
                        KpiCard {
                            id: "data_records".to_string(),
                            label: "Data Records".to_string(),
                            value: kpi.sync.total_records.to_string(),
                            unit: None,
                            trend: None,
                            trend_percent: None,
                            status: HealthStatus::Healthy,
                            link: None,
                        },
                    ],
                }),
                metadata: HashMap::new(),
                refreshed_at: kpi.timestamp,
            };
            Ok(Json(ApiResponse::ok(widget)))
        }
        Err(e) => {
            tracing::error!("Failed to get system overview: {}", e);
            Err(Json(ApiResponse::error(format!("Failed to get widget: {}", e))))
        }
    }
}

/// Get gemaal status widget.
pub async fn get_gemaal_status_widget(
    Extension(service): Extension<Arc<DashboardService>>,
) -> Result<Json<ApiResponse<DashboardWidget>>, Json<ApiResponse<()>>> {
    match service.get_gemaal_summary().await {
        Ok(kpi) => {
            let widget = DashboardWidget {
                id: "gemaal_status".to_string(),
                widget_type: WidgetType::StatusList,
                title: "Gemaal Status".to_string(),
                data: WidgetData::StatusList(StatusListData {
                    items: vec![],
                }),
                metadata: HashMap::new(),
                refreshed_at: Utc::now(),
            };
            Ok(Json(ApiResponse::ok(widget)))
        }
        Err(e) => {
            tracing::error!("Failed to get gemaal widget: {}", e);
            Err(Json(ApiResponse::error(format!("Failed to get widget: {}", e))))
        }
    }
}

/// Helper: Parse activity type from string.
fn parse_activity_type(s: &str) -> Option<ActivityType> {
    match s.to_lowercase().as_str() {
        "alert" => Some(ActivityType::Alert),
        "sync" => Some(ActivityType::Sync),
        "scenario" => Some(ActivityType::Scenario),
        "system" => Some(ActivityType::System),
        "user" => Some(ActivityType::User),
        "optimization" => Some(ActivityType::Optimization),
        _ => None,
    }
}
