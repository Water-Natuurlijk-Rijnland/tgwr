//! Scenario management service with database persistence.
//!
//! This module provides CRUD operations for hydraulic modeling scenarios,
//! including execution, result storage, and comparison functionality.

use chrono::{DateTime, Utc};
use serde_json::json;
use std::sync::Arc;

use peilbeheer_core::{
    CloneScenarioRequest, CreateScenarioRequest, ExecutionStatus, ScenarioComparison,
    ScenarioComparisonItem, StoredScenario, StoredScenarioStatus, StoredScenarioResult,
    UpdateScenarioRequest,
};

use crate::db::Database;

/// Scenario management service.
pub struct ScenarioService {
    db: Arc<Database>,
}

impl ScenarioService {
    /// Create a new scenario service.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Create a new scenario.
    pub fn create_scenario(
        &self,
        req: &CreateScenarioRequest,
    ) -> anyhow::Result<StoredScenario> {
        let id = Self::generate_id();

        let tags_json = serde_json::to_value(&req.tags).unwrap_or(json!([]));
        let boundary_json = req.boundary_conditions.clone().unwrap_or_default();
        let initial_json = req.initial_conditions.clone().unwrap_or_default();
        let model_json = req.model_parameters.clone().unwrap_or_default();

        let now = Utc::now();
        let now_str = now.format("%Y-%m-%d %H:%M:%S%.6f").to_string();
        let start_str = req.start_time.format("%Y-%m-%d %H:%M:%S%.6f").to_string();
        let end_str = req.end_time.format("%Y-%m-%d %H:%M:%S%.6f").to_string();

        self.db.execute(
            r#"
            INSERT INTO scenarios (
                id, name, description, model_id, model_type,
                start_time, end_time, time_step,
                boundary_conditions, initial_conditions, model_parameters,
                created_at, created_by, updated_at,
                is_base_scenario, base_scenario_id, status, tags
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            &[
                &id.as_bytes(),
                &req.name.as_bytes(),
                &req.description.as_ref().map(|s| s.as_bytes()),
                &req.model_id.as_bytes(),
                &None::<&[u8]>, // model_type
                &start_str.as_bytes(),
                &end_str.as_bytes(),
                &(req.time_step.to_string()).as_bytes(),
                &serde_json::to_string(&boundary_json).unwrap().as_bytes(),
                &serde_json::to_string(&initial_json).unwrap().as_bytes(),
                &serde_json::to_string(&model_json).unwrap().as_bytes(),
                &now_str.as_bytes(),
                &req.created_by.as_ref().map(|s| s.as_bytes()),
                &now_str.as_bytes(),
                &(if req.base_scenario_id.is_none() { "1" } else { "0" }).as_bytes(),
                &req.base_scenario_id.as_ref().map(|s| s.as_bytes()),
                &StoredScenarioStatus::Draft.as_str().as_bytes(),
                &serde_json::to_string(&tags_json).unwrap().as_bytes(),
            ],
        )?;

        // Log naar history
        self.log_scenario_change(&id, "created", None, Some(&json!(req)))?;

        Ok(StoredScenario {
            id,
            name: req.name.clone(),
            description: req.description.clone(),
            model_id: req.model_id.clone(),
            model_type: None,
            start_time: req.start_time,
            end_time: req.end_time,
            time_step: req.time_step,
            boundary_conditions: boundary_json,
            initial_conditions: initial_json,
            model_parameters: model_json,
            created_at: now,
            created_by: req.created_by.clone(),
            updated_at: now,
            is_base_scenario: req.base_scenario_id.is_none(),
            base_scenario_id: req.base_scenario_id.clone(),
            status: StoredScenarioStatus::Draft.as_str().to_string(),
            tags: tags_json,
        })
    }

    /// Get a scenario by ID.
    pub fn get_scenario(&self, id: &str) -> anyhow::Result<Option<StoredScenario>> {
        let result = self.db.query_row(
            r#"
            SELECT id, name, description, model_id, model_type,
                   start_time, end_time, time_step,
                   boundary_conditions, initial_conditions, model_parameters,
                   created_at, created_by, updated_at,
                   is_base_scenario, base_scenario_id, status, tags
            FROM scenarios WHERE id = ?
            "#,
            &[&id.as_bytes()],
            |row| {
                Ok(StoredScenario {
                    id: row.get::<_, String>(0)?,
                    name: row.get::<_, String>(1)?,
                    description: row.get::<_, Option<String>>(2)?,
                    model_id: row.get::<_, String>(3)?,
                    model_type: row.get::<_, Option<String>>(4)?,
                    start_time: parse_timestamp(row.get::<_, String>(5)?.as_str()),
                    end_time: parse_timestamp(row.get::<_, String>(6)?.as_str()),
                    time_step: row.get::<_, i32>(7)? as u32,
                    boundary_conditions: parse_json_value(row.get::<_, Option<String>>(8)?),
                    initial_conditions: parse_json_value(row.get::<_, Option<String>>(9)?),
                    model_parameters: parse_json_value(row.get::<_, Option<String>>(10)?),
                    created_at: parse_timestamp(row.get::<_, String>(11)?.as_str()),
                    created_by: row.get::<_, Option<String>>(12)?,
                    updated_at: parse_timestamp(row.get::<_, String>(13)?.as_str()),
                    is_base_scenario: row.get::<_, i32>(14)? == 1,
                    base_scenario_id: row.get::<_, Option<String>>(15)?,
                    status: row.get::<_, String>(16)?,
                    tags: parse_json_value(row.get::<_, Option<String>>(17)?),
                })
            },
        );

        match result {
            Ok(scenario) => Ok(Some(scenario)),
            Err(e) if e.to_string().contains("QueryReturnedNoRows") => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all scenarios, optionally filtered.
    pub fn list_scenarios(
        &self,
        model_id: Option<&str>,
        status: Option<&StoredScenarioStatus>,
        limit: Option<usize>,
    ) -> anyhow::Result<Vec<StoredScenario>> {
        let mut query = String::from(
            r#"
            SELECT id, name, description, model_id, model_type,
                   start_time, end_time, time_step,
                   boundary_conditions, initial_conditions, model_parameters,
                   created_at, created_by, updated_at,
                   is_base_scenario, base_scenario_id, status, tags
            FROM scenarios WHERE 1=1
            "#,
        );

        let mut params: Vec<String> = Vec::new();

        if let Some(mid) = model_id {
            query.push_str(&format!(" AND model_id = '{}'", mid));
        }

        if let Some(s) = status {
            query.push_str(&format!(" AND status = '{}'", s.as_str()));
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(n) = limit {
            query.push_str(&format!(" LIMIT {}", n));
        }

        self.db.query(&query, &[], |row| {
            Ok(StoredScenario {
                id: row.get::<_, String>(0)?,
                name: row.get::<_, String>(1)?,
                description: row.get::<_, Option<String>>(2)?,
                model_id: row.get::<_, String>(3)?,
                model_type: row.get::<_, Option<String>>(4)?,
                start_time: parse_timestamp(row.get::<_, String>(5)?.as_str()),
                end_time: parse_timestamp(row.get::<_, String>(6)?.as_str()),
                time_step: row.get::<_, i32>(7)? as u32,
                boundary_conditions: parse_json_value(row.get::<_, Option<String>>(8)?),
                initial_conditions: parse_json_value(row.get::<_, Option<String>>(9)?),
                model_parameters: parse_json_value(row.get::<_, Option<String>>(10)?),
                created_at: parse_timestamp(row.get::<_, String>(11)?.as_str()),
                created_by: row.get::<_, Option<String>>(12)?,
                updated_at: parse_timestamp(row.get::<_, String>(13)?.as_str()),
                is_base_scenario: row.get::<_, i32>(14)? == 1,
                base_scenario_id: row.get::<_, Option<String>>(15)?,
                status: row.get::<_, String>(16)?,
                tags: parse_json_value(row.get::<_, Option<String>>(17)?),
            })
        })
    }

    /// Update a scenario.
    pub fn update_scenario(
        &self,
        id: &str,
        req: &UpdateScenarioRequest,
        _user: Option<&str>,
    ) -> anyhow::Result<StoredScenario> {
        // Haal huidige scenario op voor logging
        let old = self.get_scenario(id)?.ok_or_else(|| {
            anyhow::anyhow!("Scenario not found: {}", id)
        })?;

        let mut updates = Vec::new();

        if let Some(name) = &req.name {
            updates.push(format!("name = '{}'", name));
        }
        if let Some(desc) = &req.description {
            updates.push(format!("description = '{}'", desc));
        }
        if let Some(status) = &req.status {
            updates.push(format!("status = '{}'", status.as_str()));
        }
        if let Some(ref tags) = req.tags {
            updates.push(format!("tags = '{}'", serde_json::to_string(tags).unwrap()));
        }

        if updates.is_empty() {
            return Ok(old);
        }

        let now = Utc::now().format("%Y-%m-%d %H:%M:%S%.6f").to_string();
        updates.push(format!("updated_at = '{}'", now));

        let query = format!(
            "UPDATE scenarios SET {} WHERE id = '{}'",
            updates.join(", "),
            id
        );

        self.db.execute(&query, &[])?;

        // Log wijziging
        self.log_scenario_change(id, "updated", Some(&json!(old)), None)?;

        // Haal geüpdatete scenario op
        self.get_scenario(id)?.map(Ok).unwrap()
    }

    /// Delete a scenario.
    pub fn delete_scenario(&self, id: &str) -> anyhow::Result<()> {
        // Log voor verwijdering
        if let Ok(scenario) = self.get_scenario(id) {
            self.log_scenario_change(id, "deleted", Some(&json!(scenario)), None)?;
        }

        self.db.execute(
            &format!("DELETE FROM scenarios WHERE id = '{}'", id),
            &[],
        )?;
        Ok(())
    }

    /// Clone a scenario.
    pub fn clone_scenario(
        &self,
        id: &str,
        req: &CloneScenarioRequest,
        user: Option<&str>,
    ) -> anyhow::Result<StoredScenario> {
        let source = self.get_scenario(id)?.ok_or_else(|| {
            anyhow::anyhow!("Source scenario not found: {}", id)
        })?;

        // Create a JSON copy for logging before any moves
        let source_json = json!(source);

        let new_id = Self::generate_id();
        let now = Utc::now();
        let now_str = now.format("%Y-%m-%d %H:%M:%S%.6f").to_string();
        let start_str = source.start_time.format("%Y-%m-%d %H:%M:%S%.6f").to_string();
        let end_str = source.end_time.format("%Y-%m-%d %H:%M:%S%.6f").to_string();

        self.db.execute(
            r#"
            INSERT INTO scenarios (
                id, name, description, model_id, model_type,
                start_time, end_time, time_step,
                boundary_conditions, initial_conditions, model_parameters,
                created_at, created_by, updated_at,
                is_base_scenario, base_scenario_id, status, tags
            ) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', {}, '{}', '{}', '{}', '{}', '{}', '{}', {}, {}, '{}', '{}')
            "#,
            &[
                &new_id,
                &req.new_name,
                &req.new_description.as_ref().unwrap_or(&source.description.clone().unwrap_or_default()),
                &source.model_id,
                &source.model_type.unwrap_or_default(),
                &start_str,
                &end_str,
                &source.time_step,
                &serde_json::to_string(&source.boundary_conditions).unwrap(),
                &serde_json::to_string(&source.initial_conditions).unwrap(),
                &serde_json::to_string(&source.model_parameters).unwrap(),
                &now_str,
                &user.unwrap_or(&String::new()),
                &now_str,
                &"0",
                &id,
                &StoredScenarioStatus::Draft.as_str(),
                &serde_json::to_string(&source.tags).unwrap(),
            ],
        )?;

        self.log_scenario_change(&new_id, "cloned", Some(&source_json), None)?;

        self.get_scenario(&new_id)?.map(Ok).unwrap()
    }

    /// Execute a scenario (create result record).
    pub fn execute_scenario(
        &self,
        scenario_id: &str,
        user: Option<&str>,
    ) -> anyhow::Result<String> {
        let result_id = Self::generate_id();
        let now = Utc::now();
        let now_str = now.format("%Y-%m-%d %H:%M:%S%.6f").to_string();

        self.db.execute(
            r#"
            INSERT INTO scenario_results (
                id, scenario_id, status, started_at, created_at, created_by
            ) VALUES ('{}', '{}', '{}', '{}', '{}', '{}')
            "#,
            &[
                &result_id.as_bytes(),
                &scenario_id.as_bytes(),
                &ExecutionStatus::Pending.as_str().as_bytes(),
                &now_str.as_bytes(),
                &now_str.as_bytes(),
                &user.unwrap_or("").as_bytes(),
            ],
        )?;

        // Update scenario status
        self.db.execute(
            &format!("UPDATE scenarios SET status = '{}' WHERE id = '{}'", StoredScenarioStatus::Active.as_str(), scenario_id),
            &[],
        )?;

        Ok(result_id)
    }

    /// Update scenario execution result.
    pub fn update_scenario_result(
        &self,
        result_id: &str,
        status: ExecutionStatus,
        results_summary: Option<&serde_json::Value>,
        error_message: Option<&str>,
        dhydro_job_id: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut updates = vec![format!("status = '{}'", status.as_str())];

        if status == ExecutionStatus::Completed || status == ExecutionStatus::Failed {
            let now = Utc::now().format("%Y-%m-%d %H:%M:%S%.6f").to_string();
            updates.push(format!("completed_at = '{}'", now));
        }

        if let Some(summary) = results_summary {
            updates.push(format!("results_summary = '{}'", serde_json::to_string(summary).unwrap()));
        }

        if let Some(msg) = error_message {
            updates.push(format!("error_message = '{}'", msg));
        }

        if let Some(job_id) = dhydro_job_id {
            updates.push(format!("dhydro_job_id = '{}'", job_id));
        }

        let query = format!(
            "UPDATE scenario_results SET {} WHERE id = '{}'",
            updates.join(", "),
            result_id
        );

        self.db.execute(&query, &[])?;
        Ok(())
    }

    /// Get scenario results.
    pub fn get_scenario_results(
        &self,
        scenario_id: &str,
    ) -> anyhow::Result<Vec<StoredScenarioResult>> {
        self.db.query(
            &format!(
                r#"
                SELECT id, scenario_id, status, started_at, completed_at, duration_seconds,
                       results_summary, time_series_count, output_files,
                       error_message, error_code, created_at, created_by,
                       dhydro_job_id, dhydro_result_url
                FROM scenario_results
                WHERE scenario_id = '{}'
                ORDER BY created_at DESC
                "#,
                scenario_id
            ),
            &[],
            |row| {
                Ok(StoredScenarioResult {
                    id: row.get::<_, String>(0)?,
                    scenario_id: row.get::<_, String>(1)?,
                    status: row.get::<_, String>(2)?,
                    started_at: row.get::<_, Option<String>>(3)?.map(|s| parse_timestamp(&s)),
                    completed_at: row.get::<_, Option<String>>(4)?.map(|s| parse_timestamp(&s)),
                    duration_seconds: row.get::<_, Option<i32>>(5)?,
                    results_summary: parse_json_value(row.get::<_, Option<String>>(6)?),
                    time_series_count: row.get::<_, i32>(7)?,
                    output_files: parse_json_value(row.get::<_, Option<String>>(8)?),
                    error_message: row.get::<_, Option<String>>(9)?,
                    error_code: row.get::<_, Option<String>>(10)?,
                    created_at: parse_timestamp(row.get::<_, String>(11)?.as_str()),
                    created_by: row.get::<_, Option<String>>(12)?,
                    dhydro_job_id: row.get::<_, Option<String>>(13)?,
                    dhydro_result_url: row.get::<_, Option<String>>(14)?,
                })
            },
        )
    }

    /// Create a scenario comparison.
    pub fn create_comparison(
        &self,
        name: &str,
        description: Option<&str>,
        scenarios: &[(String, String, Option<String>)],
        user: Option<&str>,
    ) -> anyhow::Result<String> {
        let comparison_id = Self::generate_id();
        let now = Utc::now();
        let now_str = now.format("%Y-%m-%d %H:%M:%S%.6f").to_string();

        self.db.execute(
            r#"
            INSERT INTO scenario_comparisons (id, name, description, created_at, created_by)
            VALUES ('{}', '{}', '{}', '{}', '{}')
            "#,
            &[
                &comparison_id.as_bytes(),
                &name.as_bytes(),
                &description.unwrap_or("").as_bytes(),
                &now_str.as_bytes(),
                &user.unwrap_or("").as_bytes(),
            ],
        )?;

        for (i, (scenario_id, display_name, color)) in scenarios.iter().enumerate() {
            let item_id = Self::generate_id();
            let is_baseline = i == 0;

            self.db.execute(
                &format!(
                    r#"
                INSERT INTO scenario_comparison_items (
                    id, comparison_id, scenario_id, display_name, color, is_baseline
                ) VALUES ('{}', '{}', '{}', '{}', '{}', {})
                "#,
                    item_id,
                    comparison_id,
                    scenario_id,
                    display_name,
                    color.as_deref().unwrap_or(""),
                    is_baseline as i32
                ),
                &[],
            )?;
        }

        Ok(comparison_id)
    }

    /// Get scenario comparison with statistics.
    pub fn get_comparison(
        &self,
        comparison_id: &str,
    ) -> anyhow::Result<ScenarioComparison> {
        // Haal comparison header op
        let header = self.db.query_row(
            &format!(
                r#"
                SELECT id, name, description, created_at, created_by
                FROM scenario_comparisons WHERE id = '{}'
                "#,
                comparison_id
            ),
            &[],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )?;

        // Haal items op
        let items = self.db.query(
            &format!(
                r#"
                SELECT id, comparison_id, scenario_id, display_name, color, is_baseline
                FROM scenario_comparison_items
                WHERE comparison_id = '{}'
                ORDER BY is_baseline DESC, id
                "#,
                comparison_id
            ),
            &[],
            |row| {
                Ok(ScenarioComparisonItem {
                    id: row.get::<_, String>(0)?,
                    comparison_id: row.get::<_, String>(1)?,
                    scenario_id: row.get::<_, String>(2)?,
                    display_name: row.get::<_, String>(3)?,
                    color: row.get::<_, Option<String>>(4)?,
                    is_baseline: row.get::<_, i32>(5)? == 1,
                })
            },
        )?;

        Ok(ScenarioComparison {
            id: header.0,
            name: header.1,
            description: header.2,
            created_at: parse_timestamp(&header.3),
            created_by: header.4,
            items,
        })
    }

    /// Generate a unique ID.
    fn generate_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("{:016x}", nanos)
    }

    /// Log een scenario wijziging naar de history tabel.
    fn log_scenario_change(
        &self,
        scenario_id: &str,
        change_type: &str,
        old_values: Option<&serde_json::Value>,
        new_values: Option<&serde_json::Value>,
    ) -> anyhow::Result<()> {
        let history_id = Self::generate_id();
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S%.6f").to_string();
        let old_json = old_values.and_then(|v| serde_json::to_string(v).ok());
        let new_json = new_values.and_then(|v| serde_json::to_string(v).ok());

        self.db.execute(
            r#"
            INSERT INTO scenarios_history (
                id, scenario_id, changed_at, change_type, old_values, new_values
            ) VALUES ('{}', '{}', '{}', '{}', '{}', '{}')
            "#,
            &[
                &history_id.as_bytes(),
                &scenario_id.as_bytes(),
                &now.as_bytes(),
                &change_type.as_bytes(),
                &old_json.unwrap_or(String::new()).as_bytes(),
                &new_json.unwrap_or(String::new()).as_bytes(),
            ],
        )?;

        Ok(())
    }
}

/// Helper function to parse timestamp strings.
fn parse_timestamp(s: &str) -> DateTime<Utc> {
    use chrono::NaiveDateTime;

    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.6f")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.6f"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
        .map(|ndt| ndt.and_utc())
        .unwrap_or_else(|_| Utc::now())
}

/// Helper function to parse JSON values from DuckDB strings.
fn parse_json_value(s: Option<String>) -> serde_json::Value {
    s.and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_generate_id() {
        let id1 = ScenarioService::generate_id();
        // Add a small delay to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = ScenarioService::generate_id();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 16);
        assert_eq!(id2.len(), 16);
    }

    #[test]
    fn test_parse_timestamp() {
        let ts = "2024-01-01 12:00:00.000000";
        let dt = parse_timestamp(ts);
        assert_eq!(dt.year(), 2024);
    }
}
