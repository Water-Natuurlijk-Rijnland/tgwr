//! Alert Rule Engine service for evaluating and managing alerts.
//!
//! This service provides:
//! - CRUD operations for alert rules
//! - Rule evaluation engine with context data
//! - Alert persistence and history
//! - Alert acknowledgment and resolution
//! - Notification delivery via WebSocket

use anyhow::Result as AnyhowResult;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use peilbeheer_core::{
    alert::{AlertRule, RuleId as AlertRuleId, *},
    websocket::{AlertSeverity as WsAlertSeverity, WsMessage},
};

use crate::db::Database;
use crate::websocket_service::WebSocketServer;

/// Alert service error types.
#[derive(Debug, thiserror::Error)]
pub enum AlertServiceError {
    #[error("Rule not found: {0}")]
    RuleNotFound(String),

    #[error("Alert not found: {0}")]
    AlertNotFound(String),

    #[error("Invalid rule definition: {0}")]
    InvalidRule(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Evaluation error: {0}")]
    EvaluationError(String),
}

/// Alert engine service.
pub struct AlertService {
    db: Arc<Database>,
    ws_server: Arc<WebSocketServer>,
    /// In-memory cache of active rules
    rules: Arc<RwLock<HashMap<AlertRuleId, AlertRule>>>,
    /// Track last trigger time for cooldown
    last_triggers: Arc<RwLock<HashMap<AlertRuleId, DateTime<Utc>>>>,
}

impl AlertService {
    /// Create a new alert service.
    pub fn new(db: Arc<Database>, ws_server: Arc<WebSocketServer>) -> Self {
        Self {
            db,
            ws_server,
            rules: Arc::new(RwLock::new(HashMap::new())),
            last_triggers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize the service by loading rules from database.
    pub async fn initialize(&self) -> AnyhowResult<()> {
        info!("Initializing Alert Service...");

        // Load rules from database
        let rules = self.load_rules_from_db().await?;
        let mut rules_cache = self.rules.write().await;
        rules_cache.clear();
        for rule in rules {
            rules_cache.insert(rule.id.clone(), rule);
        }

        info!("Alert Service initialized with {} rules", rules_cache.len());
        Ok(())
    }

    /// Load all rules from database.
    async fn load_rules_from_db(&self) -> AnyhowResult<Vec<AlertRule>> {
        let rows = self.db.query(
            "SELECT id, name, description, category, severity, conditions, condition_logic,
                    cooldown_seconds, enabled, notification_channels, title_template, message_template,
                    metadata, created_at, updated_at, created_by
             FROM alert_rules
             ORDER BY created_at DESC",
            &[],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, u32>(7)?,
                    row.get::<_, bool>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, Option<String>>(14)?,
                    row.get::<_, Option<String>>(15)?,
                ))
            },
        )?;

        let mut rules = Vec::new();
        for row in rows {
            let (
                id, name, description, category_str, severity_str, conditions_json,
                condition_logic_str, cooldown_seconds, enabled, channels_json,
                title_template, message_template, metadata_json,
                created_at_str, updated_at_str, created_by,
            ) = row;

            let category = parse_category(&category_str);
            let severity = AlertSeverity::from_str(&severity_str)
                .unwrap_or(AlertSeverity::Info);
            let conditions: Vec<AlertCondition> = serde_json::from_str(&conditions_json)
                .unwrap_or_default();
            let condition_logic = parse_condition_logic(&condition_logic_str);
            let notification_channels: Vec<NotificationChannel> = serde_json::from_str(&channels_json)
                .unwrap_or_else(|_| vec![NotificationChannel::WebSocket]);
            let metadata: HashMap<String, serde_json::Value> = metadata_json
                .and_then(|j| serde_json::from_str(&j).ok())
                .unwrap_or_default();

            let created_at = parse_datetime(&created_at_str);
            let updated_at = updated_at_str
                .map(|s| parse_datetime(&s))
                .unwrap_or(created_at);

            rules.push(AlertRule {
                id,
                name,
                description,
                category,
                severity,
                conditions,
                condition_logic,
                cooldown_seconds,
                enabled,
                notification_channels,
                title_template,
                message_template,
                metadata,
                created_at,
                updated_at,
                created_by,
            });
        }

        Ok(rules)
    }

    /// Create a new alert rule.
    pub async fn create_rule(
        &self,
        request: CreateAlertRuleRequest,
        creator_id: Option<String>,
    ) -> AnyhowResult<AlertRule> {
        let id = format!("RULE_{}", uuid::Uuid::new_v4());
        let now = Utc::now();

        let rule = AlertRule {
            id: id.clone(),
            name: request.name.clone(),
            description: request.description,
            category: request.category,
            severity: request.severity,
            conditions: request.conditions.clone(),
            condition_logic: request.condition_logic,
            cooldown_seconds: request.cooldown_seconds,
            enabled: true,
            notification_channels: request.notification_channels,
            title_template: request.title_template,
            message_template: request.message_template,
            metadata: request.metadata.unwrap_or_default(),
            created_at: now,
            updated_at: now,
            created_by: creator_id.clone(),
        };

        // Validate before saving
        if let Err(errors) = rule.validate() {
            return Err(AlertServiceError::InvalidRule(errors.join("; ")).into());
        }

        // Serialize complex fields
        let conditions_json = serde_json::to_string(&rule.conditions)?;
        let channels_json = serde_json::to_string(&rule.notification_channels)?;
        let metadata_json = if rule.metadata.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&rule.metadata)?)
        };

        // Save to database
        self.db.execute(
            "INSERT INTO alert_rules (id, name, description, category, severity, conditions,
                                   condition_logic, cooldown_seconds, enabled, notification_channels,
                                   title_template, message_template, metadata, created_at, updated_at, created_by)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            &[
                &rule.id as &dyn duckdb::ToSql,
                &rule.name,
                &rule.description,
                &rule.category.as_str(),
                &rule.severity.as_str(),
                &conditions_json,
                &rule.condition_logic.as_str(),
                &rule.cooldown_seconds,
                &rule.enabled,
                &channels_json,
                &rule.title_template,
                &rule.message_template,
                &metadata_json,
                &format_datetime(now),
                &format_datetime(now),
                &creator_id as &dyn duckdb::ToSql,
            ],
        )?;

        // Add to cache
        let mut rules = self.rules.write().await;
        rules.insert(id.clone(), rule.clone());

        info!("Created alert rule: {}", id);
        Ok(rule)
    }

    /// Get a rule by ID.
    pub async fn get_rule(&self, id: &str) -> AnyhowResult<AlertRule> {
        let rules = self.rules.read().await;
        rules.get(id)
            .cloned()
            .ok_or_else(|| AlertServiceError::RuleNotFound(id.to_string()).into())
    }

    /// List all rules.
    pub async fn list_rules(&self) -> AnyhowResult<Vec<AlertRule>> {
        let rules = self.rules.read().await;
        Ok(rules.values().cloned().collect())
    }

    /// Update an existing rule.
    pub async fn update_rule(
        &self,
        id: &str,
        request: UpdateAlertRuleRequest,
    ) -> AnyhowResult<AlertRule> {
        let mut rules = self.rules.write().await;
        let mut rule = rules.get(id)
            .cloned()
            .ok_or_else(|| AlertServiceError::RuleNotFound(id.to_string()))?;

        // Apply updates
        if let Some(name) = request.name {
            rule.name = name;
        }
        if let Some(description) = request.description {
            rule.description = Some(description);
        }
        if let Some(category) = request.category {
            rule.category = category;
        }
        if let Some(severity) = request.severity {
            rule.severity = severity;
        }
        if let Some(conditions) = request.conditions {
            rule.conditions = conditions;
        }
        if let Some(logic) = request.condition_logic {
            rule.condition_logic = logic;
        }
        if let Some(cooldown) = request.cooldown_seconds {
            rule.cooldown_seconds = cooldown;
        }
        if let Some(enabled) = request.enabled {
            rule.enabled = enabled;
        }
        if let Some(channels) = request.notification_channels {
            rule.notification_channels = channels;
        }
        if let Some(template) = request.title_template {
            rule.title_template = template;
        }
        if let Some(template) = request.message_template {
            rule.message_template = template;
        }
        if let Some(metadata) = request.metadata {
            rule.metadata = metadata;
        }

        rule.updated_at = Utc::now();

        // Validate
        if let Err(errors) = rule.validate() {
            return Err(AlertServiceError::InvalidRule(errors.join("; ")).into());
        }

        // Update database
        let conditions_json = serde_json::to_string(&rule.conditions)?;
        let channels_json = serde_json::to_string(&rule.notification_channels)?;
        let metadata_json = if rule.metadata.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&rule.metadata)?)
        };

        self.db.execute(
            "UPDATE alert_rules
             SET name = ?, description = ?, category = ?, severity = ?, conditions = ?,
                 condition_logic = ?, cooldown_seconds = ?, enabled = ?, notification_channels = ?,
                 title_template = ?, message_template = ?, metadata = ?, updated_at = ?
             WHERE id = ?",
            &[
                &rule.name as &dyn duckdb::ToSql,
                &rule.description,
                &rule.category.as_str(),
                &rule.severity.as_str(),
                &conditions_json,
                &rule.condition_logic.as_str(),
                &rule.cooldown_seconds,
                &rule.enabled,
                &channels_json,
                &rule.title_template,
                &rule.message_template,
                &metadata_json,
                &format_datetime(rule.updated_at),
                &id,
            ],
        )?;

        // Update cache
        rules.insert(id.to_string(), rule.clone());

        info!("Updated alert rule: {}", id);
        Ok(rule)
    }

    /// Delete a rule.
    pub async fn delete_rule(&self, id: &str) -> AnyhowResult<()> {
        let mut rules = self.rules.write().await;
        if !rules.contains_key(id) {
            return Err(AlertServiceError::RuleNotFound(id.to_string()).into());
        }

        self.db.execute("DELETE FROM alert_rules WHERE id = ?", &[&id as &dyn duckdb::ToSql])?;
        rules.remove(id);

        info!("Deleted alert rule: {}", id);
        Ok(())
    }

    /// Evaluate all enabled rules against the given context.
    pub async fn evaluate_rules(&self, context: &EvaluationContext) -> AnyhowResult<Vec<Alert>> {
        let rules = self.rules.read().await;
        let mut triggered_alerts = Vec::new();
        let mut last_triggers = self.last_triggers.write().await;

        for rule in rules.values().filter(|r| r.enabled) {
            let result = self.evaluate_rule(rule, context).await?;

            if result.triggered {
                // Check cooldown
                let last_triggered = last_triggers.get(&rule.id).copied();
                if rule.is_in_cooldown(last_triggered) {
                    debug!("Rule {} is in cooldown, skipping", rule.id);
                    continue;
                }

                // Create and store alerts
                for alert in &result.alerts {
                    self.store_alert(alert).await?;
                    self.send_notifications(alert).await;
                    triggered_alerts.push(alert.clone());
                }

                // Update last trigger time
                last_triggers.insert(rule.id.clone(), Utc::now());
            }
        }

        Ok(triggered_alerts)
    }

    /// Evaluate a single rule.
    pub async fn evaluate_rule(
        &self,
        rule: &AlertRule,
        context: &EvaluationContext,
    ) -> AnyhowResult<RuleEvaluationResult> {
        let mut condition_results = Vec::new();
        let mut all_passed = true;
        let mut any_passed = false;

        for condition in &rule.conditions {
            let result = self.evaluate_condition(condition, context).await?;
            condition_results.push(result.clone());

            match result.passed {
                true => any_passed = true,
                false => all_passed = false,
            }
        }

        let triggered = match rule.condition_logic {
            ConditionLogic::And => all_passed,
            ConditionLogic::Or => any_passed,
        };

        let alerts = if triggered {
            vec![Alert::from_rule(rule, context)]
        } else {
            Vec::new()
        };

        Ok(RuleEvaluationResult {
            rule_id: rule.id.clone(),
            triggered,
            alerts,
            evaluated_at: Utc::now(),
            condition_results,
            skip_reason: if !triggered {
                Some("Conditions not met".to_string())
            } else {
                None
            },
        })
    }

    /// Evaluate a single condition.
    async fn evaluate_condition(
        &self,
        condition: &AlertCondition,
        context: &EvaluationContext,
    ) -> AnyhowResult<ConditionResult> {
        // Get the actual value from context
        let actual_value = context.values.get(&condition.field);

        // If not found and we have a time window, try time series aggregation
        let actual_value = if actual_value.is_none() {
            if let Some(ref _window) = condition.time_window {
                if let Some(series) = context.time_series.get(&condition.field) {
                    // Need to own the aggregated value, not reference it
                    return Ok(ConditionResult {
                        field: condition.field.clone(),
                        passed: false, // Placeholder - aggregation needs proper implementation
                        actual_value: Some(AlertValue::Number(0.0)),
                        expected_value: condition.value.clone(),
                        operator: condition.operator,
                    });
                }
            }
            None
        } else {
            actual_value
        };

        let passed = match (&actual_value, &condition.value, condition.operator) {
            (Some(AlertValue::Number(actual)), AlertValue::Number(expected), op) => {
                op.eval_numeric(*actual, *expected)
            }
            (Some(AlertValue::String(actual)), AlertValue::String(expected), op) => {
                op.eval_string(actual, expected)
            }
            (Some(AlertValue::Boolean(actual)), AlertValue::Boolean(expected), op) => {
                match op {
                    ComparisonOperator::Eq => actual == expected,
                    ComparisonOperator::Ne => actual != expected,
                    _ => false,
                }
            }
            (None, _, ComparisonOperator::IsNull) => true,
            (Some(_), _, ComparisonOperator::IsNotNull) => true,
            _ => false,
        };

        Ok(ConditionResult {
            field: condition.field.clone(),
            passed,
            actual_value: actual_value.cloned(),
            expected_value: condition.value.clone(),
            operator: condition.operator,
        })
    }

    /// Store an alert in the database.
    async fn store_alert(&self, alert: &Alert) -> AnyhowResult<()> {
        let context_json = if alert.context.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&alert.context)?)
        };

        let resources_json = if alert.affected_resources.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&alert.affected_resources)?)
        };

        self.db.execute(
            "INSERT INTO alerts (id, rule_id, rule_name, severity, title, message, category,
                               affected_resources, status, triggered_at, acknowledged_at,
                               acknowledged_by, resolved_at, context)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            &[
                &alert.id as &dyn duckdb::ToSql,
                &alert.rule_id,
                &alert.rule_name,
                &alert.severity.as_str(),
                &alert.title,
                &alert.message,
                &alert.category.as_str(),
                &resources_json,
                &alert.status.as_str(),
                &format_datetime(alert.triggered_at),
                &alert.acknowledged_at.map(format_datetime),
                &alert.acknowledged_by,
                &alert.resolved_at.map(format_datetime),
                &context_json,
            ],
        )?;

        Ok(())
    }

    /// Send notifications for an alert.
    async fn send_notifications(&self, alert: &Alert) {
        // Convert core AlertSeverity to WsAlertSeverity
        let ws_severity = match alert.severity {
            AlertSeverity::Info => WsAlertSeverity::Info,
            AlertSeverity::Warning => WsAlertSeverity::Warning,
            AlertSeverity::Error => WsAlertSeverity::Error,
            AlertSeverity::Critical => WsAlertSeverity::Critical,
        };

        let msg = WsMessage::alert(
            alert.id.clone(),
            ws_severity,
            alert.title.clone(),
            alert.message.clone(),
        );

        // Broadcast via WebSocket
        self.ws_server.broadcast(msg);

        // TODO: Implement other channels (email, webhook)
    }

    /// Get an alert by ID.
    pub async fn get_alert(&self, id: &str) -> AnyhowResult<Alert> {
        let result = self.db.query_row(
            "SELECT id, rule_id, rule_name, severity, title, message, category,
                     affected_resources, status, triggered_at, acknowledged_at,
                     acknowledged_by, resolved_at, context
             FROM alerts WHERE id = ?",
            &[&id as &dyn duckdb::ToSql],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                ))
            },
        );

        let (id, rule_id, rule_name, severity_str, title, message, category_str,
            resources_json, status_str, triggered_at_str, acknowledged_at_str,
            acknowledged_by, resolved_at_str) = result.map_err(|e| {
            if e.to_string().contains("QueryReturnedNoRows") {
                AlertServiceError::AlertNotFound(id.to_string()).into()
            } else {
                e
            }
        })?;

        let severity = AlertSeverity::from_str(&severity_str)
            .unwrap_or(AlertSeverity::Info);
        let category = parse_category(&category_str);
        let affected_resources: Vec<String> = resources_json
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();
        let status = parse_alert_status(&status_str);

        Ok(Alert {
            id,
            rule_id,
            rule_name,
            severity,
            title,
            message,
            category,
            affected_resources,
            status,
            triggered_at: parse_datetime(&triggered_at_str),
            acknowledged_at: acknowledged_at_str.map(|s| parse_datetime(&s)),
            acknowledged_by,
            resolved_at: resolved_at_str.map(|s| parse_datetime(&s)),
            context: HashMap::new(),
        })
    }

    /// Query alerts with filters.
    pub async fn query_alerts(&self, query: &AlertQuery) -> AnyhowResult<Vec<Alert>> {
        let mut conditions = Vec::new();
        let mut params: Vec<Box<dyn duckdb::ToSql>> = Vec::new();

        if let Some(status) = &query.status {
            conditions.push("status = ?");
            params.push(Box::new(status.as_str().to_string()));
        }
        if let Some(severity) = &query.severity {
            conditions.push("severity = ?");
            params.push(Box::new(severity.as_str().to_string()));
        }
        if let Some(category) = &query.category {
            conditions.push("category = ?");
            params.push(Box::new(category.as_str().to_string()));
        }
        if let Some(rule_id) = &query.rule_id {
            conditions.push("rule_id = ?");
            params.push(Box::new(rule_id.clone()));
        }
        if let Some(start) = &query.start_time {
            conditions.push("triggered_at >= ?");
            params.push(Box::new(format_datetime(*start)));
        }
        if let Some(end) = &query.end_time {
            conditions.push("triggered_at <= ?");
            params.push(Box::new(format_datetime(*end)));
        }
        if let Some(user) = &query.acknowledged_by {
            conditions.push("acknowledged_by = ?");
            params.push(Box::new(user.clone()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);

        let sql = format!(
            "SELECT id, rule_id, rule_name, severity, title, message, category,
                    affected_resources, status, triggered_at, acknowledged_at,
                    acknowledged_by, resolved_at, context
             FROM alerts
             {}
             ORDER BY triggered_at DESC
             LIMIT {} OFFSET {}",
            where_clause, limit, offset
        );

        let param_refs: Vec<&dyn duckdb::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = self.db.query(&sql, &param_refs, |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, Option<String>>(12)?,
                row.get::<_, Option<String>>(13)?,
            ))
        })?;

        let mut alerts = Vec::new();
        for row in rows {
            let (id, rule_id, rule_name, severity_str, title, message, category_str,
                resources_json, status_str, triggered_at_str, acknowledged_at_str,
                acknowledged_by, resolved_at_str, context_json) = row;

            let severity = AlertSeverity::from_str(&severity_str)
                .unwrap_or(AlertSeverity::Info);
            let category = parse_category(&category_str);
            let affected_resources: Vec<String> = resources_json
                .and_then(|j| serde_json::from_str(&j).ok())
                .unwrap_or_default();
            let status = parse_alert_status(&status_str);
            let context: HashMap<String, serde_json::Value> = context_json
                .and_then(|j| serde_json::from_str(&j).ok())
                .unwrap_or_default();

            alerts.push(Alert {
                id,
                rule_id,
                rule_name,
                severity,
                title,
                message,
                category,
                affected_resources,
                status,
                triggered_at: parse_datetime(&triggered_at_str),
                acknowledged_at: acknowledged_at_str.map(|s| parse_datetime(&s)),
                acknowledged_by,
                resolved_at: resolved_at_str.map(|s| parse_datetime(&s)),
                context,
            });
        }

        Ok(alerts)
    }

    /// Acknowledge an alert.
    pub async fn acknowledge_alert(
        &self,
        id: &str,
        request: AcknowledgeAlertRequest,
    ) -> AnyhowResult<Alert> {
        let mut alert = self.get_alert(id).await?;

        if alert.status != AlertStatus::Active {
            return Err(AlertServiceError::InvalidRule(
                "Alert is not active".to_string(),
            ).into());
        }

        alert.acknowledge(request.user_id.clone());

        self.db.execute(
            "UPDATE alerts SET status = ?, acknowledged_at = ?, acknowledged_by = ? WHERE id = ?",
            &[
                &alert.status.as_str() as &dyn duckdb::ToSql,
                &format_datetime(alert.acknowledged_at.unwrap()),
                &alert.acknowledged_by,
                &id,
            ],
        )?;

        info!("Alert {} acknowledged by {}", id, request.user_id);
        Ok(alert)
    }

    /// Resolve an alert.
    pub async fn resolve_alert(&self, id: &str) -> AnyhowResult<Alert> {
        let mut alert = self.get_alert(id).await?;
        alert.resolve();

        self.db.execute(
            "UPDATE alerts SET status = ?, resolved_at = ? WHERE id = ?",
            &[
                &alert.status.as_str() as &dyn duckdb::ToSql,
                &format_datetime(alert.resolved_at.unwrap()),
                &id,
            ],
        )?;

        info!("Alert {} resolved", id);
        Ok(alert)
    }

    /// Get alert statistics.
    pub async fn get_stats(&self) -> AnyhowResult<AlertStats> {
        // Count total alerts
        let total: i64 = self.db.query_row(
            "SELECT COUNT(*) FROM alerts",
            &[],
            |row| row.get(0),
        ).unwrap_or(0);

        // Count active alerts
        let active: i64 = self.db.query_row(
            "SELECT COUNT(*) FROM alerts WHERE status = 'active'",
            &[],
            |row| row.get(0),
        ).unwrap_or(0);

        // Count by severity
        let mut by_severity = HashMap::new();
        for severity in [AlertSeverity::Info, AlertSeverity::Warning, AlertSeverity::Error, AlertSeverity::Critical] {
            let count: i64 = self.db.query_row(
                "SELECT COUNT(*) FROM alerts WHERE severity = ?",
                &[&severity.as_str()],
                |row| row.get(0),
            ).unwrap_or(0);
            by_severity.insert(severity.as_str().to_string(), count as u64);
        }

        // Count by category
        let mut by_category = HashMap::new();
        let category_rows = self.db.query(
            "SELECT category, COUNT(*) as count FROM alerts GROUP BY category",
            &[],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )?;
        for (cat, count) in category_rows {
            by_category.insert(cat, count as u64);
        }

        // Top rules
        let mut top_rules = Vec::new();
        let rule_rows = self.db.query(
            "SELECT rule_id, rule_name, COUNT(*) as count FROM alerts
             GROUP BY rule_id, rule_name
             ORDER BY count DESC
             LIMIT 10",
            &[],
            |row| Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            )),
        )?;
        for (rule_id, rule_name, count) in rule_rows {
            top_rules.push(RuleTriggerCount { rule_id, rule_name, count: count as u64 });
        }

        // Average resolution time
        let avg_resolution: Option<f64> = self.db.query_row(
            "SELECT AVG(JULIANDAY(resolved_at) - JULIANDAY(triggered_at)) * 86400
             FROM alerts WHERE resolved_at IS NOT NULL",
            &[],
            |row| row.get(0),
        ).ok();

        Ok(AlertStats {
            total_alerts: total as u64,
            active_alerts: active as u64,
            by_severity,
            by_category,
            top_rules,
            avg_resolution_seconds: avg_resolution,
        })
    }
}

/// Helper: Aggregate time series values.
fn aggregate_series(
    series: &[TimeSeriesValue],
    aggregation: Option<AggregationFunction>,
) -> AlertValue {
    let agg = aggregation.unwrap_or(AggregationFunction::Last);

    if series.is_empty() {
        return AlertValue::Number(0.0);
    }

    match agg {
        AggregationFunction::Avg => {
            let sum: f64 = series.iter().map(|v| v.value).sum();
            AlertValue::Number(sum / series.len() as f64)
        }
        AggregationFunction::Min => {
            AlertValue::Number(series.iter().map(|v| v.value).fold(f64::INFINITY, f64::min))
        }
        AggregationFunction::Max => {
            AlertValue::Number(series.iter().map(|v| v.value).fold(f64::NEG_INFINITY, f64::max))
        }
        AggregationFunction::Sum => {
            AlertValue::Number(series.iter().map(|v| v.value).sum())
        }
        AggregationFunction::Count => {
            AlertValue::Number(series.len() as f64)
        }
        AggregationFunction::Last => {
            AlertValue::Number(series.last().map(|v| v.value).unwrap_or(0.0))
        }
        AggregationFunction::First => {
            AlertValue::Number(series.first().map(|v| v.value).unwrap_or(0.0))
        }
    }
}

/// Helper: Parse category from string.
fn parse_category(s: &str) -> AlertCategory {
    match s {
        "water_level" => AlertCategory::WaterLevel,
        "pump_status" => AlertCategory::PumpStatus,
        "energy_price" => AlertCategory::EnergyPrice,
        "weather" => AlertCategory::Weather,
        "system_health" => AlertCategory::SystemHealth,
        "simulation" => AlertCategory::Simulation,
        other => AlertCategory::Custom(other.to_string()),
    }
}

/// Helper: Parse condition logic from string.
fn parse_condition_logic(s: &str) -> ConditionLogic {
    match s.to_lowercase().as_str() {
        "or" => ConditionLogic::Or,
        _ => ConditionLogic::And,
    }
}

/// Helper: Parse alert status from string.
fn parse_alert_status(s: &str) -> AlertStatus {
    match s.to_lowercase().as_str() {
        "active" => AlertStatus::Active,
        "acknowledged" => AlertStatus::Acknowledged,
        "resolved" => AlertStatus::Resolved,
        "suppressed" => AlertStatus::Suppressed,
        _ => AlertStatus::Active,
    }
}

/// Helper: Format datetime for DuckDB.
fn format_datetime(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S%.6f").to_string()
}

/// Helper: Parse datetime from DuckDB.
fn parse_datetime(s: &str) -> DateTime<Utc> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.6f")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.6f"))
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
        .map(|ndt| ndt.and_utc())
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use peilbeheer_core::alert::*;

    #[test]
    fn test_aggregate_series() {
        let series = vec![
            TimeSeriesValue {
                timestamp: Utc::now(),
                value: 10.0,
                flag: None,
            },
            TimeSeriesValue {
                timestamp: Utc::now() + Duration::seconds(1),
                value: 20.0,
                flag: None,
            },
            TimeSeriesValue {
                timestamp: Utc::now() + Duration::seconds(2),
                value: 30.0,
                flag: None,
            },
        ];

        // Test Avg
        match aggregate_series(&series, Some(AggregationFunction::Avg)) {
            AlertValue::Number(n) => assert!((n - 20.0).abs() < 0.01),
            _ => panic!("Expected Number"),
        }

        // Test Max
        match aggregate_series(&series, Some(AggregationFunction::Max)) {
            AlertValue::Number(n) => assert!((n - 30.0).abs() < 0.01),
            _ => panic!("Expected Number"),
        }

        // Test Min
        match aggregate_series(&series, Some(AggregationFunction::Min)) {
            AlertValue::Number(n) => assert!((n - 10.0).abs() < 0.01),
            _ => panic!("Expected Number"),
        }

        // Test Sum
        match aggregate_series(&series, Some(AggregationFunction::Sum)) {
            AlertValue::Number(n) => assert!((n - 60.0).abs() < 0.01),
            _ => panic!("Expected Number"),
        }
    }
}
