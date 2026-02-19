//! Scenario management types and models.
//!
//! This module provides domain models for hydraulic modeling scenarios,
//! including persistence, execution, and comparison.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Status van een opgeslagen scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StoredScenarioStatus {
    Draft,
    Active,
    Archived,
}

impl StoredScenarioStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "draft" => Some(Self::Draft),
            "active" => Some(Self::Active),
            "archived" => Some(Self::Archived),
            _ => None,
        }
    }
}

impl std::fmt::Display for StoredScenarioStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Uitvoeringsstatus van een scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl ExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

/// Scenario voor opslag in database.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StoredScenario {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_type: Option<String>,

    // Tijdsparameters
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub time_step: u32,

    // Modelparameters (JSON)
    #[serde(default)]
    pub boundary_conditions: serde_json::Value,
    #[serde(default)]
    pub initial_conditions: serde_json::Value,
    #[serde(default)]
    pub model_parameters: serde_json::Value,

    // Metadata
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    pub updated_at: DateTime<Utc>,

    // Scenario relatie
    #[serde(default)]
    pub is_base_scenario: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_scenario_id: Option<String>,

    // Status
    #[serde(default)]
    pub status: String,

    // Tags
    #[serde(default)]
    pub tags: serde_json::Value,
}

/// Request om een nieuw scenario te maken.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateScenarioRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub created_by: Option<String>,
}

/// Request om een scenario te updaten.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateScenarioRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_step: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boundary_conditions: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_conditions: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_parameters: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<StoredScenarioStatus>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

/// Scenario uitvoeringsresultaat.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StoredScenarioResult {
    pub id: String,
    pub scenario_id: String,

    // Uitvoeringsstatus
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<i32>,

    // Resultaat samenvatting (JSON)
    #[serde(default)]
    pub results_summary: serde_json::Value,
    #[serde(default)]
    pub time_series_count: i32,
    #[serde(default)]
    pub output_files: serde_json::Value,

    // Foutinformatie
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,

    // Metadaten
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,

    // DHYdro specifiek
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dhydro_job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dhydro_result_url: Option<String>,
}

/// Tijdreeks resultaat van een scenario uitvoering.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StoredTimeSeriesResult {
    pub id: String,
    pub result_id: String,

    // Metadata
    pub location_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_name: Option<String>,
    pub parameter: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,

    // Data (JSON array)
    pub data: serde_json::Value,

    // Statistiek
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_value: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_data_path: Option<String>,

    pub created_at: DateTime<Utc>,
}

/// Scenario vergelijking.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScenarioComparison {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    pub items: Vec<ScenarioComparisonItem>,
}

/// Item in een scenario vergelijking.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScenarioComparisonItem {
    pub id: String,
    pub comparison_id: String,
    pub scenario_id: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default)]
    pub is_baseline: bool,
}

/// Request om een scenario te klonen.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CloneScenarioRequest {
    pub new_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copy_results: Option<bool>,
}

/// Scenario vergelijking statistieken.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScenarioComparisonStats {
    pub scenario_id: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,

    // Vergelijkingswaarden
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_water_level: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_water_level: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_water_level: Option<f64>,

    // Verschil ten opzichte van baseline (indien van toepassing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_max_level: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_volume: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_status_roundtrip() {
        assert_eq!(StoredScenarioStatus::from_str("draft"), Some(StoredScenarioStatus::Draft));
        assert_eq!(StoredScenarioStatus::from_str("ACTIVE"), Some(StoredScenarioStatus::Active));
        assert_eq!(StoredScenarioStatus::from_str("unknown"), None);
    }

    #[test]
    fn test_execution_status_roundtrip() {
        assert_eq!(ExecutionStatus::from_str("running"), Some(ExecutionStatus::Running));
        assert_eq!(ExecutionStatus::from_str("FAILED"), Some(ExecutionStatus::Failed));
    }

    #[test]
    fn test_create_scenario_serialization() {
        let req = CreateScenarioRequest {
            name: "Test Scenario".to_string(),
            description: Some("Test description".to_string()),
            model_id: "MODEL_001".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now() + chrono::Duration::hours(24),
            time_step: 300,
            boundary_conditions: None,
            initial_conditions: None,
            model_parameters: None,
            base_scenario_id: None,
            tags: vec!["flood".to_string(), "extreme".to_string()],
            created_by: Some("user".to_string()),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("Test Scenario"));
        assert!(json.contains("flood"));
    }
}
