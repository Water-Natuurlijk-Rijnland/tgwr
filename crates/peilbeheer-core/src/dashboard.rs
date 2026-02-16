//! Dashboard API types for KPIs, widgets, and aggregate data.
//!
//! This module provides types for dashboard aggregation:
//! - KPI endpoints for high-level metrics
//! - Widget data for configurable dashboards
//! - Activity feed for recent events
//! - System health monitoring

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Overall dashboard KPIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardKpi {
    /// Current timestamp
    pub timestamp: DateTime<Utc>,

    /// System health KPIs
    pub system: SystemHealthKpi,

    /// Gemaal KPIs
    pub gemalen: GemaalKpi,

    /// Alert KPIs
    pub alerts: AlertKpi,

    /// Scenario KPIs
    pub scenarios: ScenarioKpi,

    /// Data sync KPIs
    pub sync: SyncKpi,

    /// Performance KPIs
    pub performance: PerformanceKpi,
}

/// System health KPIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthKpi {
    /// Overall health status
    pub status: HealthStatus,

    /// API uptime percentage (last 24h)
    pub uptime_percent: f64,

    /// Active services count
    pub services_active: u32,

    /// Total services count
    pub services_total: u32,

    /// Database status
    pub database_status: HealthStatus,

    /// External services status
    pub external_services: HashMap<String, HealthStatus>,
}

/// Health status enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl HealthStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
            Self::Unknown => "unknown",
        }
    }

    pub fn color_hex(&self) -> &str {
        match self {
            Self::Healthy => "#10b981",    // green
            Self::Degraded => "#f59e0b",    // amber
            Self::Unhealthy => "#ef4444",   // red
            Self::Unknown => "#6b7280",     // gray
        }
    }
}

/// Gemaal KPIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GemaalKpi {
    /// Total gemalen count
    pub total: u32,

    /// Currently active
    pub active: u32,

    /// Currently inactive
    pub inactive: u32,

    /// In error state
    pub error: u32,

    /// Unknown status
    pub unknown: u32,

    /// Total current capacity (m³/hour)
    pub total_capacity: f64,

    /// Active capacity (m³/hour)
    pub active_capacity: f64,

    /// Utilization percentage
    pub utilization_percent: f64,

    /// Recently updated (last 5 minutes)
    pub recently_updated: u32,
}

/// Alert KPIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertKpi {
    /// Total active alerts
    pub active_total: u32,

    /// Breakdown by severity
    pub by_severity: HashMap<String, u32>,

    /// Breakdown by category
    pub by_category: HashMap<String, u32>,

    /// Critical alerts count
    pub critical: u32,

    /// Alerts triggered today
    pub triggered_today: u32,

    /// Alerts acknowledged today
    pub acknowledged_today: u32,

    /// Average resolution time (minutes)
    pub avg_resolution_minutes: Option<f64>,
}

/// Scenario KPIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioKpi {
    /// Total scenarios
    pub total: u32,

    /// Active scenarios
    pub active: u32,

    /// Currently running
    pub running: u32,

    /// Completed today
    pub completed_today: u32,

    /// Failed today
    pub failed_today: u32,

    /// Average execution time (seconds)
    pub avg_execution_seconds: Option<f64>,
}

/// Data sync KPIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncKpi {
    /// Last successful sync time for each source
    pub last_sync: HashMap<String, DateTime<Utc>>,

    /// Sync status for each source
    pub sync_status: HashMap<String, HealthStatus>,

    /// Total records cached
    pub total_records: u64,

    /// Records updated today
    pub updated_today: u64,

    /// Data freshness score (0-100)
    pub freshness_score: u32,
}

/// Performance KPIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceKpi {
    /// Average API response time (ms)
    pub avg_response_time_ms: f64,

    /// P95 response time (ms)
    pub p95_response_time_ms: f64,

    /// Requests per second
    pub requests_per_second: f64,

    /// Error rate percentage
    pub error_rate_percent: f64,

    /// Active connections
    pub active_connections: u32,
}

/// Dashboard widget data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardWidget {
    /// Widget identifier
    pub id: String,

    /// Widget type
    pub widget_type: WidgetType,

    /// Widget title
    pub title: String,

    /// Widget data
    pub data: WidgetData,

    /// Widget metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Last refresh time
    pub refreshed_at: DateTime<Utc>,
}

/// Widget type enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetType {
    /// KPI number cards
    KpiCards,
    /// Line chart
    LineChart,
    /// Bar chart
    BarChart,
    /// Pie chart
    PieChart,
    /// Table
    Table,
    /// Map
    Map,
    /// Activity feed
    ActivityFeed,
    /// Status list
    StatusList,
    /// Custom
    Custom(String),
}

/// Widget data variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WidgetData {
    /// KPI cards data
    #[serde(rename = "kpi_cards")]
    KpiCards(KpiCardsData),

    /// Chart data
    #[serde(rename = "chart")]
    Chart(ChartData),

    /// Table data
    #[serde(rename = "table")]
    Table(TableData),

    /// Activity feed data
    #[serde(rename = "activity_feed")]
    ActivityFeed(ActivityFeedData),

    /// Map data
    #[serde(rename = "map")]
    Map(MapData),

    /// Status list data
    #[serde(rename = "status_list")]
    StatusList(StatusListData),

    /// Generic JSON data
    #[serde(rename = "generic")]
    Generic(serde_json::Value),
}

/// KPI cards widget data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiCardsData {
    pub cards: Vec<KpiCard>,
}

/// Single KPI card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiCard {
    pub id: String,
    pub label: String,
    pub value: String,
    pub unit: Option<String>,
    pub trend: Option<TrendDirection>,
    pub trend_percent: Option<f64>,
    pub status: HealthStatus,
    pub link: Option<String>,
}

/// Trend direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    Up,
    Down,
    Stable,
}

/// Chart data for widgets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    pub title: String,
    pub labels: Vec<String>,
    pub datasets: Vec<ChartDataset>,
    pub x_axis_label: Option<String>,
    pub y_axis_label: Option<String>,
    pub y_axis_min: Option<f64>,
    pub y_axis_max: Option<f64>,
}

/// Chart dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartDataset {
    pub label: String,
    pub data: Vec<Option<f64>>,
    pub color: String,
    pub background_color: Option<String>,
    pub border_width: Option<u32>,
    pub fill: Option<bool>,
    pub dataset_type: Option<ChartDatasetType>,
}

/// Chart dataset type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChartDatasetType {
    Line,
    Bar,
    Area,
    Scatter,
}

/// Table widget data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableData {
    pub columns: Vec<TableColumn>,
    pub rows: Vec<TableRow>,
    pub sortable: bool,
    pub pageable: bool,
}

/// Table column definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableColumn {
    pub id: String,
    pub label: String,
    pub data_type: ColumnDataType,
    pub sortable: bool,
    pub width: Option<String>,
}

/// Column data type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColumnDataType {
    Text,
    Number,
    Date,
    Status,
    Link,
}

/// Table row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRow {
    pub id: String,
    pub cells: Vec<TableCell>,
    pub link: Option<String>,
    pub status: Option<HealthStatus>,
}

/// Table cell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCell {
    pub value: serde_json::Value,
    pub display: Option<String>,
    pub class_name: Option<String>,
}

/// Activity feed data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityFeedData {
    pub items: Vec<ActivityFeedItem>,
    pub has_more: bool,
}

/// Activity feed item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityFeedItem {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub type_: ActivityType,
    pub title: String,
    pub description: Option<String>,
    pub actor: Option<String>,
    pub severity: Option<AlertSeverity>,
    pub link: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Activity type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityType {
    Alert,
    Sync,
    Scenario,
    System,
    User,
    Optimization,
}

/// Alert severity for activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Map widget data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapData {
    pub center: MapCenter,
    pub zoom: u8,
    pub markers: Vec<MapMarker>,
    pub layers: Vec<MapLayer>,
    pub show_legend: bool,
}

/// Map center coordinates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapCenter {
    pub lat: f64,
    pub lon: f64,
}

/// Map marker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapMarker {
    pub id: String,
    pub lat: f64,
    pub lon: f64,
    pub marker_type: MarkerType,
    pub status: HealthStatus,
    pub label: String,
    pub value: Option<String>,
    pub popup_content: Option<String>,
    pub link: Option<String>,
}

/// Marker type on map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarkerType {
    Gemaal,
    Peilgebied,
    Meetstation,
    Asset,
    Custom,
}

/// Map layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapLayer {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub layer_type: LayerType,
}

/// Layer type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayerType {
    Markers,
    Polygons,
    Heatmap,
    Tiles,
}

/// Status list data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusListData {
    pub items: Vec<StatusListItem>,
}

/// Status list item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusListItem {
    pub id: String,
    pub name: String,
    pub status: HealthStatus,
    pub secondary_text: Option<String>,
    pub value: Option<String>,
    pub unit: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub link: Option<String>,
}

/// Activity feed query parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityFeedQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub activity_type: Option<ActivityType>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub severity: Option<AlertSeverity>,
}

impl Default for ActivityFeedQuery {
    fn default() -> Self {
        Self {
            limit: Some(50),
            offset: Some(0),
            activity_type: None,
            start_time: None,
            end_time: None,
            severity: None,
        }
    }
}

/// Dashboard configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub layout: DashboardLayout,
    pub widgets: Vec<DashboardWidgetConfig>,
    pub refresh_interval_seconds: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Dashboard layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardLayout {
    pub columns: u32,
    pub rows: Option<u32>,
    pub grid_type: GridType,
}

/// Grid type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GridType {
    Fixed,
    Responsive,
}

/// Widget configuration (positioning).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardWidgetConfig {
    pub widget_id: String,
    pub column: u32,
    pub row: u32,
    pub width: u32,
    pub height: u32,
}

impl DashboardKpi {
    /// Create default/empty KPIs.
    pub fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            system: SystemHealthKpi::default(),
            gemalen: GemaalKpi::default(),
            alerts: AlertKpi::default(),
            scenarios: ScenarioKpi::default(),
            sync: SyncKpi::default(),
            performance: PerformanceKpi::default(),
        }
    }
}

impl Default for DashboardKpi {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SystemHealthKpi {
    fn default() -> Self {
        Self {
            status: HealthStatus::Unknown,
            uptime_percent: 100.0,
            services_active: 0,
            services_total: 0,
            database_status: HealthStatus::Unknown,
            external_services: HashMap::new(),
        }
    }
}

impl Default for GemaalKpi {
    fn default() -> Self {
        Self {
            total: 0,
            active: 0,
            inactive: 0,
            error: 0,
            unknown: 0,
            total_capacity: 0.0,
            active_capacity: 0.0,
            utilization_percent: 0.0,
            recently_updated: 0,
        }
    }
}

impl Default for AlertKpi {
    fn default() -> Self {
        Self {
            active_total: 0,
            by_severity: HashMap::new(),
            by_category: HashMap::new(),
            critical: 0,
            triggered_today: 0,
            acknowledged_today: 0,
            avg_resolution_minutes: None,
        }
    }
}

impl Default for ScenarioKpi {
    fn default() -> Self {
        Self {
            total: 0,
            active: 0,
            running: 0,
            completed_today: 0,
            failed_today: 0,
            avg_execution_seconds: None,
        }
    }
}

impl Default for SyncKpi {
    fn default() -> Self {
        Self {
            last_sync: HashMap::new(),
            sync_status: HashMap::new(),
            total_records: 0,
            updated_today: 0,
            freshness_score: 100,
        }
    }
}

impl Default for PerformanceKpi {
    fn default() -> Self {
        Self {
            avg_response_time_ms: 0.0,
            p95_response_time_ms: 0.0,
            requests_per_second: 0.0,
            error_rate_percent: 0.0,
            active_connections: 0,
        }
    }
}

impl TrendDirection {
    pub fn from_percent_change(percent: f64) -> Self {
        if percent.abs() < 0.01 {
            Self::Stable
        } else if percent > 0.0 {
            Self::Up
        } else {
            Self::Down
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status() {
        assert_eq!(HealthStatus::Healthy.as_str(), "healthy");
        assert_eq!(HealthStatus::Healthy.color_hex(), "#10b981");
    }

    #[test]
    fn test_trend_direction() {
        assert_eq!(TrendDirection::from_percent_change(0.1), TrendDirection::Up);
        assert_eq!(TrendDirection::from_percent_change(-0.1), TrendDirection::Down);
        assert_eq!(TrendDirection::from_percent_change(0.001), TrendDirection::Stable);
    }

    #[test]
    fn test_dashboard_kpi_default() {
        let kpi = DashboardKpi::default();
        assert_eq!(kpi.gemalen.total, 0);
        assert_eq!(kpi.alerts.active_total, 0);
        assert_eq!(kpi.system.status, HealthStatus::Unknown);
    }

    #[test]
    fn test_activity_feed_query_default() {
        let query = ActivityFeedQuery::default();
        assert_eq!(query.limit, Some(50));
        assert_eq!(query.offset, Some(0));
    }

    #[test]
    fn test_widget_type_serialization() {
        let widget_type = WidgetType::LineChart;
        let json = serde_json::to_string(&widget_type).unwrap();
        assert!(json.contains("line_chart"));

        let parsed: WidgetType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, WidgetType::LineChart);
    }
}
