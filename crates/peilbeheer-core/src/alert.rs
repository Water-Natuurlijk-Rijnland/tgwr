//! Alert Rule Engine for configurable threshold-based monitoring.
//!
//! This module provides a flexible alerting system that can evaluate
//! conditions based on water levels, pump status, energy prices, and more.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for an alert rule.
pub type RuleId = String;

/// Unique identifier for an alert instance.
pub type AlertId = String;

/// Alert rule definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Unique rule identifier
    pub id: RuleId,

    /// Human-readable rule name
    pub name: String,

    /// Detailed description
    pub description: Option<String>,

    /// Rule category for grouping
    pub category: AlertCategory,

    /// Severity level when triggered
    pub severity: AlertSeverity,

    /// Conditions that must be met (AND logic)
    pub conditions: Vec<AlertCondition>,

    /// How to combine conditions (AND/OR)
    pub condition_logic: ConditionLogic,

    /// Cooldown period before re-triggering (seconds)
    pub cooldown_seconds: u32,

    /// Whether the rule is currently active
    pub enabled: bool,

    /// Notification channels
    pub notification_channels: Vec<NotificationChannel>,

    /// Template for alert title
    pub title_template: String,

    /// Template for alert message
    pub message_template: String,

    /// Metadata for custom fields
    pub metadata: HashMap<String, serde_json::Value>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,

    /// Creator user ID
    pub created_by: Option<String>,
}

/// Alert category classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertCategory {
    /// Water level monitoring (too high/low)
    WaterLevel,
    /// Pump/gemaal status (offline, error)
    PumpStatus,
    /// Energy price alerts
    EnergyPrice,
    /// Weather-related (rainfall, wind)
    Weather,
    /// System health (API, database)
    SystemHealth,
    /// Simulation/scenario results
    Simulation,
    /// Custom category
    Custom(String),
}

impl AlertCategory {
    pub fn as_str(&self) -> &str {
        match self {
            Self::WaterLevel => "water_level",
            Self::PumpStatus => "pump_status",
            Self::EnergyPrice => "energy_price",
            Self::Weather => "weather",
            Self::SystemHealth => "system_health",
            Self::Simulation => "simulation",
            Self::Custom(s) => s,
        }
    }
}

/// Alert severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info = 1,
    Warning = 2,
    Error = 3,
    Critical = 4,
}

impl AlertSeverity {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Critical => "critical",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "info" => Some(Self::Info),
            "warning" => Some(Self::Warning),
            "error" => Some(Self::Error),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }

    pub fn color_hex(&self) -> &str {
        match self {
            Self::Info => "#3b82f6",    // blue
            Self::Warning => "#f59e0b", // amber
            Self::Error => "#ef4444",   // red
            Self::Critical => "#7f1d1d", // dark red
        }
    }
}

/// Individual condition for rule evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertCondition {
    /// Field/parameter to evaluate (e.g., "water_level", "pump_status")
    pub field: String,

    /// Comparison operator
    pub operator: ComparisonOperator,

    /// Threshold value to compare against
    pub value: AlertValue,

    /// Optional data source filter (e.g., specific gemaal code)
    pub source_filter: Option<String>,

    /// Time window for aggregation (e.g., "5m", "1h", "1d")
    pub time_window: Option<String>,

    /// Aggregation function for time window
    pub aggregation: Option<AggregationFunction>,
}

/// Comparison operators for conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComparisonOperator {
    /// Equals
    Eq,
    /// Not equals
    Ne,
    /// Greater than
    Gt,
    /// Greater than or equal
    Gte,
    /// Less than
    Lt,
    /// Less than or equal
    Lte,
    /// Contains (for strings)
    Contains,
    /// Does not contain
    NotContains,
    /// Is null/empty
    IsNull,
    /// Is not null/empty
    IsNotNull,
}

impl ComparisonOperator {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
            Self::Contains => "contains",
            Self::NotContains => "not_contains",
            Self::IsNull => "is_null",
            Self::IsNotNull => "is_not_null",
        }
    }

    /// Evaluate the operator for numeric values.
    pub fn eval_numeric(&self, left: f64, right: f64) -> bool {
        match self {
            Self::Eq => (left - right).abs() < 0.0001,
            Self::Ne => (left - right).abs() >= 0.0001,
            Self::Gt => left > right,
            Self::Gte => left >= right,
            Self::Lt => left < right,
            Self::Lte => left <= right,
            _ => false,
        }
    }

    /// Evaluate the operator for string values.
    pub fn eval_string(&self, left: &str, right: &str) -> bool {
        match self {
            Self::Eq => left == right,
            Self::Ne => left != right,
            Self::Contains => left.contains(right),
            Self::NotContains => !left.contains(right),
            _ => false,
        }
    }
}

/// Value types for alert conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AlertValue {
    Number(f64),
    String(String),
    Boolean(bool),
    Array(Vec<String>),
}

impl AlertValue {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

/// Aggregation functions for time-windowed conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AggregationFunction {
    /// Average
    Avg,
    /// Minimum
    Min,
    /// Maximum
    Max,
    /// Sum
    Sum,
    /// Count
    Count,
    /// Last (most recent)
    Last,
    /// First (oldest)
    First,
}

/// Logic for combining multiple conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConditionLogic {
    /// All conditions must be true
    And,
    /// At least one condition must be true
    Or,
}

impl ConditionLogic {
    pub fn as_str(&self) -> &str {
        match self {
            Self::And => "and",
            Self::Or => "or",
        }
    }
}

/// Notification channel for alert delivery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannel {
    /// WebSocket broadcast
    WebSocket,
    /// Email notification (not yet implemented)
    Email { recipients: Vec<String> },
    /// Webhook POST
    Webhook { url: String, headers: HashMap<String, String> },
    /// SMS (not yet implemented)
    Sms { recipients: Vec<String> },
}

/// Triggered alert instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Unique alert identifier
    pub id: AlertId,

    /// Reference to the rule that triggered
    pub rule_id: RuleId,

    /// Rule name at time of trigger
    pub rule_name: String,

    /// Alert severity
    pub severity: AlertSeverity,

    /// Alert title
    pub title: String,

    /// Detailed message
    pub message: String,

    /// Category of the alert
    pub category: AlertCategory,

    /// Affected resources (e.g., gemaal codes)
    pub affected_resources: Vec<String>,

    /// Alert state
    pub status: AlertStatus,

    /// When the alert was triggered
    pub triggered_at: DateTime<Utc>,

    /// When the alert was acknowledged (if applicable)
    pub acknowledged_at: Option<DateTime<Utc>>,

    /// Who acknowledged the alert
    pub acknowledged_by: Option<String>,

    /// When the alert was resolved
    pub resolved_at: Option<DateTime<Utc>>,

    /// Additional context data
    pub context: HashMap<String, serde_json::Value>,
}

/// Alert status lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertStatus {
    /// Alert is active
    Active,
    /// Alert has been acknowledged
    Acknowledged,
    /// Alert condition has cleared
    Resolved,
    /// Alert was suppressed (cooldown, duplicate)
    Suppressed,
}

impl AlertStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Active => "active",
            Self::Acknowledged => "acknowledged",
            Self::Resolved => "resolved",
            Self::Suppressed => "suppressed",
        }
    }
}

/// Result of evaluating a single rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEvaluationResult {
    /// Rule that was evaluated
    pub rule_id: RuleId,

    /// Whether the rule conditions were met
    pub triggered: bool,

    /// Alerts generated (may be empty if suppressed)
    pub alerts: Vec<Alert>,

    /// Timestamp of evaluation
    pub evaluated_at: DateTime<Utc>,

    /// Detailed evaluation results per condition
    pub condition_results: Vec<ConditionResult>,

    /// Reason for not triggering (if applicable)
    pub skip_reason: Option<String>,
}

/// Result of evaluating a single condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionResult {
    /// Field that was evaluated
    pub field: String,

    /// Whether the condition was met
    pub passed: bool,

    /// Actual value observed
    pub actual_value: Option<AlertValue>,

    /// Expected threshold value
    pub expected_value: AlertValue,

    /// Operator used
    pub operator: ComparisonOperator,
}

/// Alert statistics for monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertStats {
    /// Total alerts triggered
    pub total_alerts: u64,

    /// Active alerts
    pub active_alerts: u64,

    /// Alerts by severity
    pub by_severity: HashMap<String, u64>,

    /// Alerts by category
    pub by_category: HashMap<String, u64>,

    /// Most common triggers
    pub top_rules: Vec<RuleTriggerCount>,

    /// Average resolution time (seconds)
    pub avg_resolution_seconds: Option<f64>,
}

/// Count of triggers per rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleTriggerCount {
    pub rule_id: RuleId,
    pub rule_name: String,
    pub count: u64,
}

/// Create alert rule request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAlertRuleRequest {
    pub name: String,
    pub description: Option<String>,
    pub category: AlertCategory,
    pub severity: AlertSeverity,
    pub conditions: Vec<AlertCondition>,
    pub condition_logic: ConditionLogic,
    pub cooldown_seconds: u32,
    pub notification_channels: Vec<NotificationChannel>,
    pub title_template: String,
    pub message_template: String,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Update alert rule request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAlertRuleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<AlertCategory>,
    pub severity: Option<AlertSeverity>,
    pub conditions: Option<Vec<AlertCondition>>,
    pub condition_logic: Option<ConditionLogic>,
    pub cooldown_seconds: Option<u32>,
    pub enabled: Option<bool>,
    pub notification_channels: Option<Vec<NotificationChannel>>,
    pub title_template: Option<String>,
    pub message_template: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Query filters for listing alerts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertQuery {
    pub status: Option<AlertStatus>,
    pub severity: Option<AlertSeverity>,
    pub category: Option<AlertCategory>,
    pub rule_id: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

impl Default for AlertQuery {
    fn default() -> Self {
        Self {
            status: None,
            severity: None,
            category: None,
            rule_id: None,
            start_time: None,
            end_time: None,
            acknowledged_by: None,
            limit: Some(100),
            offset: Some(0),
        }
    }
}

/// Data context for rule evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationContext {
    /// Current timestamp
    pub now: DateTime<Utc>,

    /// Field values available for evaluation
    pub values: HashMap<String, AlertValue>,

    /// Time series data for windowed aggregations
    pub time_series: HashMap<String, Vec<TimeSeriesValue>>,

    /// Metadata about the source
    pub source: Option<String>,
}

/// Time series value for aggregation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesValue {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub flag: Option<String>,
}

/// Request to acknowledge an alert.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcknowledgeAlertRequest {
    pub user_id: String,
    pub comment: Option<String>,
}

impl AlertRule {
    /// Create a new alert rule.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        category: AlertCategory,
        severity: AlertSeverity,
        conditions: Vec<AlertCondition>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            category,
            severity,
            conditions,
            condition_logic: ConditionLogic::And,
            cooldown_seconds: 300, // 5 minutes default
            enabled: true,
            notification_channels: vec![NotificationChannel::WebSocket],
            title_template: "{category} Alert: {rule_name}".to_string(),
            message_template: String::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            created_by: None,
        }
    }

    /// Check if the rule is in cooldown period.
    pub fn is_in_cooldown(&self, last_triggered: Option<DateTime<Utc>>) -> bool {
        match last_triggered {
            Some(last) => {
                let elapsed = Utc::now().signed_duration_since(last);
                elapsed.num_seconds() < self.cooldown_seconds as i64
            }
            None => false,
        }
    }

    /// Render the title template with context variables.
    pub fn render_title(&self, context: &HashMap<String, String>) -> String {
        self.render_template(&self.title_template, context)
    }

    /// Render the message template with context variables.
    pub fn render_message(&self, context: &HashMap<String, String>) -> String {
        self.render_template(&self.message_template, context)
    }

    fn render_template(&self, template: &str, context: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in context {
            result = result.replace(&format!("{{{{{}}}}}", key), value);
        }
        result
    }

    /// Validate the rule definition.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.name.is_empty() {
            errors.push("name cannot be empty".to_string());
        }

        if self.conditions.is_empty() {
            errors.push("at least one condition is required".to_string());
        }

        for (i, cond) in self.conditions.iter().enumerate() {
            if cond.field.is_empty() {
                errors.push(format!("condition {}: field cannot be empty", i));
            }
        }

        if self.title_template.is_empty() {
            errors.push("title_template cannot be empty".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Alert {
    /// Create a new alert from a rule.
    pub fn from_rule(rule: &AlertRule, context: &EvaluationContext) -> Self {
        let now = Utc::now();
        let id = format!("ALT_{}_{}", rule.id, now.timestamp());

        // Build context for template rendering
        let mut template_ctx = HashMap::new();
        template_ctx.insert("rule_name".to_string(), rule.name.clone());
        template_ctx.insert("category".to_string(), rule.category.as_str().to_string());
        template_ctx.insert("severity".to_string(), rule.severity.as_str().to_string());

        // Add values from context
        for (key, value) in &context.values {
            match value {
                AlertValue::Number(n) => {
                    template_ctx.insert(key.clone(), n.to_string());
                }
                AlertValue::String(s) => {
                    template_ctx.insert(key.clone(), s.clone());
                }
                AlertValue::Boolean(b) => {
                    template_ctx.insert(key.clone(), b.to_string());
                }
                AlertValue::Array(arr) => {
                    template_ctx.insert(key.clone(), arr.join(", "));
                }
            }
        }

        Self {
            id,
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            severity: rule.severity,
            title: rule.render_title(&template_ctx),
            message: rule.render_message(&template_ctx),
            category: rule.category.clone(),
            affected_resources: context.source.clone().into_iter().collect(),
            status: AlertStatus::Active,
            triggered_at: now,
            acknowledged_at: None,
            acknowledged_by: None,
            resolved_at: None,
            context: context.values.iter().map(|(k, v)| {
                (k.clone(), serde_json::to_value(v).unwrap_or(serde_json::Value::Null))
            }).collect(),
        }
    }

    /// Acknowledge the alert.
    pub fn acknowledge(&mut self, user_id: String) {
        self.status = AlertStatus::Acknowledged;
        self.acknowledged_at = Some(Utc::now());
        self.acknowledged_by = Some(user_id);
    }

    /// Resolve the alert.
    pub fn resolve(&mut self) {
        self.status = AlertStatus::Resolved;
        self.resolved_at = Some(Utc::now());
    }

    /// Check if alert is still active.
    pub fn is_active(&self) -> bool {
        self.status == AlertStatus::Active
    }

    /// Get the duration since triggered.
    pub fn duration(&self) -> chrono::Duration {
        Utc::now().signed_duration_since(self.triggered_at)
    }
}

impl Default for AlertCategory {
    fn default() -> Self {
        Self::Custom("other".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_rule_creation() {
        let rule = AlertRule::new(
            "RULE_001",
            "High Water Level",
            AlertCategory::WaterLevel,
            AlertSeverity::Warning,
            vec![AlertCondition {
                field: "water_level".to_string(),
                operator: ComparisonOperator::Gte,
                value: AlertValue::Number(2.5),
                source_filter: None,
                time_window: None,
                aggregation: None,
            }],
        );

        assert_eq!(rule.name, "High Water Level");
        assert_eq!(rule.conditions.len(), 1);
        assert!(rule.enabled);
    }

    #[test]
    fn test_rule_validation() {
        let mut rule = AlertRule::new(
            "RULE_002",
            "Test Rule",
            AlertCategory::SystemHealth,
            AlertSeverity::Info,
            vec![],
        );

        // Empty conditions should fail
        assert!(rule.validate().is_err());

        // Add a condition
        rule.conditions.push(AlertCondition {
            field: "test_field".to_string(),
            operator: ComparisonOperator::Eq,
            value: AlertValue::Boolean(true),
            source_filter: None,
            time_window: None,
            aggregation: None,
        });

        assert!(rule.validate().is_ok());
    }

    #[test]
    fn test_comparison_operators() {
        assert!(ComparisonOperator::Gt.eval_numeric(5.0, 3.0));
        assert!(!ComparisonOperator::Gt.eval_numeric(3.0, 5.0));
        assert!(ComparisonOperator::Lte.eval_numeric(3.0, 5.0));
        assert!(ComparisonOperator::Lte.eval_numeric(5.0, 5.0));

        assert!(ComparisonOperator::Contains.eval_string("hello world", "world"));
        assert!(!ComparisonOperator::Contains.eval_string("hello world", "goodbye"));
    }

    #[test]
    fn test_alert_from_rule() {
        let rule = AlertRule::new(
            "RULE_003",
            "Pump Offline",
            AlertCategory::PumpStatus,
            AlertSeverity::Error,
            vec![AlertCondition {
                field: "pump_status".to_string(),
                operator: ComparisonOperator::Eq,
                value: AlertValue::String("offline".to_string()),
                source_filter: Some("GEMAAL_001".to_string()),
                time_window: None,
                aggregation: None,
            }],
        );

        let mut context = EvaluationContext {
            now: Utc::now(),
            values: HashMap::new(),
            time_series: HashMap::new(),
            source: Some("GEMAAL_001".to_string()),
        };
        context.values.insert("pump_status".to_string(), AlertValue::String("offline".to_string()));

        let alert = Alert::from_rule(&rule, &context);

        assert_eq!(alert.rule_id, "RULE_003");
        assert_eq!(alert.severity, AlertSeverity::Error);
        assert_eq!(alert.status, AlertStatus::Active);
        assert!(alert.affected_resources.contains(&"GEMAAL_001".to_string()));
    }

    #[test]
    fn test_alert_lifecycle() {
        let rule = AlertRule::new(
            "RULE_004",
            "Test",
            AlertCategory::SystemHealth,
            AlertSeverity::Info,
            vec![],
        );

        let context = EvaluationContext {
            now: Utc::now(),
            values: HashMap::new(),
            time_series: HashMap::new(),
            source: None,
        };

        let mut alert = Alert::from_rule(&rule, &context);

        assert!(alert.is_active());

        alert.acknowledge("user_123".to_string());
        assert_eq!(alert.status, AlertStatus::Acknowledged);
        assert_eq!(alert.acknowledged_by, Some("user_123".to_string()));

        alert.resolve();
        assert_eq!(alert.status, AlertStatus::Resolved);
        assert!(alert.resolved_at.is_some());
    }

    #[test]
    fn test_cooldown_check() {
        let rule = AlertRule::new(
            "RULE_005",
            "Test",
            AlertCategory::SystemHealth,
            AlertSeverity::Info,
            vec![],
        );
        let rule_with_cooldown = AlertRule {
            cooldown_seconds: 60,
            ..rule
        };

        // No previous trigger
        assert!(!rule_with_cooldown.is_in_cooldown(None));

        // Recent trigger within cooldown
        let recent = Utc::now() - chrono::Duration::seconds(30);
        assert!(rule_with_cooldown.is_in_cooldown(Some(recent)));

        // Old trigger outside cooldown
        let old = Utc::now() - chrono::Duration::seconds(120);
        assert!(!rule_with_cooldown.is_in_cooldown(Some(old)));
    }
}
