//! WebSocket message types for real-time updates.
//!
//! This module defines the message types used for WebSocket communication
//! between the API server and connected clients.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// WebSocket message types.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    /// Server welcome message sent after connection
    #[serde(rename = "welcome")]
    Welcome { server_id: String, timestamp: DateTime<Utc> },

    /// Authentication confirmation
    #[serde(rename = "authenticated")]
    Authenticated { user_id: String, username: String },

    /// Heartbeat/ping message
    #[serde(rename = "ping")]
    Ping { timestamp: i64 },

    /// Heartbeat/pong response
    #[serde(rename = "pong")]
    Pong { timestamp: i64 },

    /// Scenario execution status update
    #[serde(rename = "scenario.status")]
    ScenarioStatus { scenario_id: String, status: String },

    /// Scenario execution completed
    #[serde(rename = "scenario.completed")]
    ScenarioCompleted {
        scenario_id: String,
        result_id: String,
        success: bool,
    },

    /// New scenario created
    #[serde(rename = "scenario.created")]
    ScenarioCreated { scenario_id: String, name: String },

    /// Scenario updated
    #[serde(rename = "scenario.updated")]
    ScenarioUpdated { scenario_id: String, name: String },

    /// Scenario deleted
    #[serde(rename = "scenario.deleted")]
    ScenarioDeleted { scenario_id: String },

    /// Gemaal status update
    #[serde(rename = "gemalen.status")]
    GemaalStatus {
        code: String,
        status: String,
        water_level: Option<f64>,
    },

    /// System status update
    #[serde(rename = "system.status")]
    SystemStatus {
        healthy: bool,
        message: Option<String>,
    },

    /// Asset synchronized
    #[serde(rename = "asset.synced")]
    AssetSynced {
        asset_type: String,
        count: usize,
    },

    /// Alert/notification
    #[serde(rename = "alert")]
    Alert {
        id: String,
        severity: AlertSeverity,
        title: String,
        message: String,
        source: Option<String>,
    },

    /// Time series data update
    #[serde(rename = "timeseries.update")]
    TimeSeriesUpdate {
        location_id: String,
        parameter: String,
        value: f64,
        timestamp: DateTime<Utc>,
    },

    /// Bulk time series update
    #[serde(rename = "timeseries.bulk")]
    TimeSeriesBulk { updates: Vec<TimeSeriesPoint> },

    /// Error message
    #[serde(rename = "error")]
    Error { message: String, code: Option<String> },

    /// Generic data message
    #[serde(rename = "data")]
    Data { channel: String, payload: serde_json::Value },
}

/// Alert severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl AlertSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Critical => "critical",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "info" => Some(Self::Info),
            "warning" => Some(Self::Warning),
            "error" => Some(Self::Error),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }
}

/// Time series data point.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeSeriesPoint {
    pub location_id: String,
    pub parameter: String,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    pub flag: Option<String>,
}

/// Client subscription request.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubscribeRequest {
    pub channels: Vec<String>,
}

/// Client unsubscription request.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnsubscribeRequest {
    pub channels: Vec<String>,
}

impl WsMessage {
    /// Convert message to JSON string.
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// Parse JSON string to message.
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Create welcome message.
    pub fn welcome(server_id: String) -> Self {
        Self::Welcome {
            server_id,
            timestamp: Utc::now(),
        }
    }

    /// Create authenticated message.
    pub fn authenticated(user_id: String, username: String) -> Self {
        Self::Authenticated { user_id, username }
    }

    /// Create ping message.
    pub fn ping() -> Self {
        Self::Ping {
            timestamp: Utc::now().timestamp(),
        }
    }

    /// Create pong message.
    pub fn pong() -> Self {
        Self::Pong {
            timestamp: Utc::now().timestamp(),
        }
    }

    /// Create scenario status update.
    pub fn scenario_status(scenario_id: String, status: String) -> Self {
        Self::ScenarioStatus { scenario_id, status }
    }

    /// Create scenario completed message.
    pub fn scenario_completed(scenario_id: String, result_id: String, success: bool) -> Self {
        Self::ScenarioCompleted {
            scenario_id,
            result_id,
            success,
        }
    }

    /// Create alert message.
    pub fn alert(
        id: String,
        severity: AlertSeverity,
        title: String,
        message: String,
    ) -> Self {
        Self::Alert {
            id,
            severity,
            title,
            message,
            source: None,
        }
    }

    /// Create error message.
    pub fn error(message: String) -> Self {
        Self::Error {
            message,
            code: None,
        }
    }
}

/// Channel names for subscription.
pub mod channels {
    pub const ALL: &str = "*";
    pub const SCENARIOS: &str = "scenarios";
    pub const GEMALEN: &str = "gemalen";
    pub const ALERTS: &str = "alerts";
    pub const SYSTEM: &str = "system";
    pub const TIMESERIES: &str = "timeseries";
    pub const ASSETS: &str = "assets";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = WsMessage::Welcome {
            server_id: "test-server".to_string(),
            timestamp: Utc::now(),
        };

        let json = msg.to_json().unwrap();
        assert!(json.contains("welcome"));

        let parsed = WsMessage::from_json(&json).unwrap();
        match parsed {
            WsMessage::Welcome { server_id, .. } => {
                assert_eq!(server_id, "test-server");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_alert_severity() {
        assert_eq!(AlertSeverity::Critical.as_str(), "critical");
        assert_eq!(AlertSeverity::from_str("warning"), Some(AlertSeverity::Warning));
        assert_eq!(AlertSeverity::from_str("invalid"), None);
    }

    #[test]
    fn test_message_constructors() {
        let msg = WsMessage::scenario_status("scen_123".to_string(), "running".to_string());
        let json = msg.to_json().unwrap();
        assert!(json.contains("scenario.status"));
        assert!(json.contains("scen_123"));
        assert!(json.contains("running"));
    }
}
