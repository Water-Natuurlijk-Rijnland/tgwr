//! D-HYDRO API Client
//!
//! This module provides integration with the D-HYDRO hydraulic modeling platform
//! developed by Deltares (https://www.deltares.nl/en/software/d-hydro/).
//!
//! # Important Note
//!
//! D-HYDRO is primarily a desktop modeling suite. This client is designed for
//! integration with a **custom-hosted D-HYDRO instance** or an **API wrapper**
//! that Waterschap Rijnland would deploy. There is no public "dhydro.nl" API.
//!
//! The typical deployment would be:
//! - An internal server running D-HYDRO with REST API wrapper
//! - Or a Deltares-hosted instance with API access
//!
//! # Environment Variables
//!
//! - `DHYDRO_BASE_URL`: URL of the D-HYDRO API instance
//! - `DHYDRO_CLIENT_ID`: OAuth 2.0 client ID
//! - `DHYDRO_CLIENT_SECRET`: OAuth 2.0 client secret
//! - `DHYDRO_TOKEN_URL`: OAuth token endpoint
//! - `DHYDRO_SCOPE`: API scopes to request
//! - `DHYDRO_TIMEOUT`: Request timeout in seconds
//!
//! # Features
//!
//! - OAuth 2.0 authentication
//! - Model management (list, get models)
//! - Time series operations
//! - Scenario management (create, list, execute)
//! - Result retrieval

use chrono::{DateTime, Utc};
use reqwest::{header, Client, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use std::time::Duration;

/// DHYdro API client configuration.
///
/// Note: D-HYDRO is Deltares' hydraulic modeling software suite.
/// The API endpoint would typically be an internally hosted instance
/// or a Deltares-hosted environment, not a public service.
#[derive(Debug, Clone)]
pub struct DhydroConfig {
    /// Base URL of the DHYdro API (e.g., internal server or Deltares hosted instance)
    pub base_url: String,
    /// OAuth 2.0 client ID for authentication
    pub client_id: String,
    /// OAuth 2.0 client secret for authentication
    pub client_secret: String,
    /// OAuth 2.0 token endpoint
    pub token_url: String,
    /// API scope(s) to request
    pub scope: String,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl DhydroConfig {
    /// Create configuration from environment variables.
    ///
    /// Required environment variables:
    /// - DHYDRO_BASE_URL: URL of the DHYdro API instance
    /// - DHYDRO_CLIENT_ID: OAuth client ID
    /// - DHYDRO_CLIENT_SECRET: OAuth client secret
    pub fn from_env() -> anyhow::Result<Self> {
        let base_url = std::env::var("DHYDRO_BASE_URL");

        if base_url.is_err() {
            tracing::warn!("DHYDRO_BASE_URL not set - DHYdro integration will not be functional");
        }

        let client_id = std::env::var("DHYDRO_CLIENT_ID");
        if client_id.is_err() {
            tracing::warn!("DHYDRO_CLIENT_ID not set - DHYdro integration will not be functional");
        }

        Ok(Self {
            base_url: base_url.unwrap_or_else(|_| "https://dhydro.internal.example.com".to_string()),
            client_id: client_id.unwrap_or_else(|_| "".to_string()),
            client_secret: std::env::var("DHYDRO_CLIENT_SECRET")
                .unwrap_or_else(|_| "".to_string()),
            token_url: std::env::var("DHYDRO_TOKEN_URL")
                .unwrap_or_else(|_| "/oauth/token".to_string()),
            scope: std::env::var("DHYDRO_SCOPE")
                .unwrap_or_else(|_| "models timeseries scenarios results".to_string()),
            timeout_secs: std::env::var("DHYDRO_TIMEOUT")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
        })
    }

    /// Check if the configuration has valid credentials.
    pub fn is_configured(&self) -> bool {
        !self.client_id.is_empty()
            && !self.client_secret.is_empty()
            && !self.base_url.is_empty()
            && self.base_url != "https://dhydro.internal.example.com"
    }
}

/// DHYdro API errors.
#[derive(Debug, Error)]
pub enum DhydroError {
    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("API request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("API returned error {0}: {1}")]
    ApiError(StatusCode, String),

    #[error("JSON serialization/deserialization failed: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Configuration(#[from] anyhow::Error),

    #[error("Token expired or invalid")]
    TokenExpired,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

/// OAuth 2.0 token response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub token_type: String,
    #[serde(default)]
    pub expires_in: u64,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub scope: String,
}

impl OAuthToken {
    /// Check if the token is expired based on expiration time.
    pub fn is_expired(&self, expires_at: DateTime<Utc>) -> bool {
        Utc::now() > expires_at
    }
}

/// DHYdro hydraulic model information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DhydroModel {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub model_type: String,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub parameters: serde_json::Value,
}

/// Time series data point.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeSeriesPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    #[serde(default)]
    pub flag: Option<String>,
}

/// Time series metadata and data.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeSeries {
    pub id: String,
    pub name: String,
    pub parameter: String,
    pub unit: String,
    pub location_id: String,
    #[serde(default)]
    pub qualifier: Option<String>,
    pub data: Vec<TimeSeriesPoint>,
}

/// Time series query parameters.
#[derive(Debug, Clone, Serialize)]
pub struct TimeSeriesQuery {
    pub location_id: Option<String>,
    pub parameter: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub aggregation: Option<TimeSeriesAggregation>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeSeriesAggregation {
    Raw,
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

/// Scenario for hydraulic model simulation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Scenario {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub model_id: String,
    pub parameters: ScenarioParameters,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub is_base_scenario: bool,
    #[serde(default)]
    pub base_scenario_id: Option<String>,
}

/// Scenario parameters for model execution.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScenarioParameters {
    /// Simulation start time
    pub start_time: DateTime<Utc>,
    /// Simulation end time
    pub end_time: DateTime<Utc>,
    /// Time step in seconds
    pub time_step: u32,
    /// Boundary conditions
    #[serde(default)]
    pub boundary_conditions: serde_json::Value,
    /// Initial conditions
    #[serde(default)]
    pub initial_conditions: serde_json::Value,
    /// Model-specific parameters
    #[serde(default)]
    pub model_parameters: serde_json::Value,
}

/// Scenario execution status.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ScenarioStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Scenario execution result.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScenarioResult {
    pub id: String,
    pub scenario_id: String,
    pub status: ScenarioStatus,
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub error_message: Option<String>,
    #[serde(default)]
    pub results: Option<ScenarioResults>,
}

/// Output data from scenario execution.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScenarioResults {
    #[serde(default)]
    pub time_series: Vec<TimeSeries>,
    #[serde(default)]
    pub summary: ScenarioSummary,
    #[serde(default)]
    pub output_files: Vec<String>,
}

/// Summary statistics from scenario execution.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ScenarioSummary {
    pub max_water_level: Option<f64>,
    pub min_water_level: Option<f64>,
    pub avg_water_level: Option<f64>,
    pub total_volume: Option<f64>,
    pub peak_discharge: Option<f64>,
}

/// DHYdro API client with OAuth 2.0 authentication.
pub struct DhydroClient {
    config: DhydroConfig,
    http_client: Client,
    access_token: Option<String>,
    token_expires_at: Option<DateTime<Utc>>,
}

impl DhydroClient {
    /// Create a new DHYdro API client.
    pub fn new(config: DhydroConfig) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            http_client,
            access_token: None,
            token_expires_at: None,
        }
    }

    /// Create a new DHYdro API client from environment variables.
    pub fn from_env() -> Result<Self, DhydroError> {
        let config = DhydroConfig::from_env()?;
        Ok(Self::new(config))
    }

    /// Ensure we have a valid access token, refreshing if necessary.
    async fn ensure_token(&mut self) -> Result<(), DhydroError> {
        // Check if current token is still valid
        if let (Some(_token), Some(expires_at)) = (&self.access_token, &self.token_expires_at)
            && expires_at > &Utc::now() {
                return Ok(());
            }

        // Need to fetch a new token
        self.fetch_token().await
    }

    /// Fetch a new OAuth 2.0 access token.
    async fn fetch_token(&mut self) -> Result<(), DhydroError> {
        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
            ("scope", &self.config.scope),
        ];

        let response = self
            .http_client
            .post(&self.config.token_url)
            .form(&params)
            .header(header::ACCEPT, "application/json")
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(DhydroError::Authentication(format!(
                "Token request failed: {} - {}",
                status, body
            )));
        }

        let token: OAuthToken =
            serde_json::from_str(&body).map_err(|e| DhydroError::Authentication(format!(
                "Failed to parse token response: {} - {}",
                e, body
            )))?;

        self.access_token = Some(token.access_token);
        // Set expiration with 5 minute buffer
        self.token_expires_at = Some(Utc::now() + chrono::Duration::seconds(
            token.expires_in as i64 - 300,
        ));

        tracing::info!("DHYdro token refreshed successfully");
        Ok(())
    }

    /// Make an authenticated API request.
    async fn request<T: for<'de> Deserialize<'de>>(
        &mut self,
        method: reqwest::Method,
        path: &str,
        query: Option<&[(&str, String)]>,
        body: Option<serde_json::Value>,
    ) -> Result<T, DhydroError> {
        self.ensure_token().await?;

        let token = self
            .access_token
            .as_ref()
            .ok_or(DhydroError::TokenExpired)?;

        let url = format!("{}{}", self.config.base_url, path);
        let mut request = self.http_client.request(method, &url);

        if let Some(q) = query {
            request = request.query(q);
        }

        request = request.header(
            header::AUTHORIZATION,
            format!("Bearer {}", token),
        );
        request = request.header(header::ACCEPT, "application/json");

        if let Some(b) = body {
            request = request.json(&b);
        }

        let response = request.send().await?;
        let status = response.status();

        // Handle rate limiting
        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(DhydroError::RateLimitExceeded);
        }

        // Handle token expiry
        if status == StatusCode::UNAUTHORIZED {
            self.access_token = None;
            self.token_expires_at = None;
            return Err(DhydroError::TokenExpired);
        }

        let body = response.text().await?;

        if !status.is_success() {
            return Err(DhydroError::ApiError(status, body));
        }

        serde_json::from_str(&body).map_err(Into::into)
    }

    /// List all available hydraulic models.
    pub async fn list_models(&mut self) -> Result<Vec<DhydroModel>, DhydroError> {
        self.request(
            reqwest::Method::GET,
            "/api/v1/models",
            None,
            None,
        )
        .await
    }

    /// Get a specific model by ID.
    pub async fn get_model(&mut self, id: &str) -> Result<DhydroModel, DhydroError> {
        self.request(
            reqwest::Method::GET,
            &format!("/api/v1/models/{}", id),
            None,
            None,
        )
        .await
    }

    /// Fetch time series data.
    pub async fn get_time_series(
        &mut self,
        query: &TimeSeriesQuery,
    ) -> Result<Vec<TimeSeries>, DhydroError> {
        let mut params = Vec::new();

        if let Some(loc) = &query.location_id {
            params.push(("location_id", loc.clone()));
        }
        if let Some(param) = &query.parameter {
            params.push(("parameter", param.clone()));
        }
        if let Some(start) = query.start {
            params.push(("start", start.to_rfc3339()));
        }
        if let Some(end) = query.end {
            params.push(("end", end.to_rfc3339()));
        }

        self.request(
            reqwest::Method::GET,
            "/api/v1/timeseries",
            Some(&params.iter().map(|(k, v)| (*k, v.clone())).collect::<Vec<_>>()),
            None,
        )
        .await
    }

    /// Create a new scenario.
    pub async fn create_scenario(
        &mut self,
        scenario: &Scenario,
    ) -> Result<Scenario, DhydroError> {
        let body = serde_json::to_value(scenario)?;
        self.request(
            reqwest::Method::POST,
            "/api/v1/scenarios",
            None,
            Some(body),
        )
        .await
    }

    /// Get a scenario by ID.
    pub async fn get_scenario(&mut self, id: &str) -> Result<Scenario, DhydroError> {
        self.request(
            reqwest::Method::GET,
            &format!("/api/v1/scenarios/{}", id),
            None,
            None,
        )
        .await
    }

    /// List all scenarios (optionally filtered by model).
    pub async fn list_scenarios(
        &mut self,
        model_id: Option<&str>,
    ) -> Result<Vec<Scenario>, DhydroError> {
        let params = if let Some(mid) = model_id {
            Some(vec![("model_id", mid.to_string())])
        } else {
            None
        };

        self.request(
            reqwest::Method::GET,
            "/api/v1/scenarios",
            params.as_deref(),
            None,
        )
        .await
    }

    /// Execute a scenario.
    pub async fn execute_scenario(
        &mut self,
        scenario_id: &str,
    ) -> Result<ScenarioResult, DhydroError> {
        self.request(
            reqwest::Method::POST,
            &format!("/api/v1/scenarios/{}/execute", scenario_id),
            None,
            None,
        )
        .await
    }

    /// Get scenario execution results.
    pub async fn get_scenario_result(
        &mut self,
        result_id: &str,
    ) -> Result<ScenarioResult, DhydroError> {
        self.request(
            reqwest::Method::GET,
            &format!("/api/v1/results/{}", result_id),
            None,
            None,
        )
        .await
    }

    /// Get results for a scenario (latest execution).
    pub async fn get_scenario_results(
        &mut self,
        scenario_id: &str,
    ) -> Result<ScenarioResult, DhydroError> {
        self.request(
            reqwest::Method::GET,
            &format!("/api/v1/scenarios/{}/results", scenario_id),
            None,
            None,
        )
        .await
    }

    /// Delete a scenario.
    pub async fn delete_scenario(&mut self, scenario_id: &str) -> Result<(), DhydroError> {
        self.request::<serde_json::Value>(
            reqwest::Method::DELETE,
            &format!("/api/v1/scenarios/{}", scenario_id),
            None,
            None,
        )
        .await?;
        Ok(())
    }

    /// Clone a scenario (create a copy with new parameters).
    pub async fn clone_scenario(
        &mut self,
        scenario_id: &str,
        new_name: &str,
    ) -> Result<Scenario, DhydroError> {
        #[derive(Serialize)]
        struct CloneRequest {
            name: String,
        }

        let body = serde_json::to_value(CloneRequest {
            name: new_name.to_string(),
        })?;

        self.request(
            reqwest::Method::POST,
            &format!("/api/v1/scenarios/{}/clone", scenario_id),
            None,
            Some(body),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_series_query_serialization() {
        let query = TimeSeriesQuery {
            location_id: Some("LOC001".to_string()),
            parameter: Some("water_level".to_string()),
            start: Some(DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap().into()),
            end: Some(DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z").unwrap().into()),
            aggregation: Some(TimeSeriesAggregation::Hourly),
        };

        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("LOC001"));
        assert!(json.contains("water_level"));
    }

    #[test]
    fn test_scenario_parameters_serialization() {
        let params = ScenarioParameters {
            start_time: DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap().into(),
            end_time: DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z").unwrap().into(),
            time_step: 300,
            boundary_conditions: serde_json::json!({"inflow": 100.0}),
            initial_conditions: serde_json::json!({"water_level": 1.5}),
            model_parameters: serde_json::json!({"roughness": 0.03}),
        };

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("boundary_conditions"));
        assert!(json.contains("initial_conditions"));
    }

    #[test]
    fn test_scenario_status_deserialization() {
        let json = r#"{"status": "running"}"#;
        let result: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(result["status"], "running");
    }
}
