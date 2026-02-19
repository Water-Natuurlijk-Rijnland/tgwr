//! Delft-FEWS PI-REST client service.
//!
//! This module provides HTTP client functionality for interacting with
//! Delft-FEWS (Flood Early Warning System) through its PI-REST API.

use anyhow::Result as AnyhowResult;
use chrono::{Duration, Utc};
use reqwest::Client;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tracing::{debug, info, warn};

use peilbeheer_core::{
    FewsConfig, FewsLocation, FewsModuleInstance, FewsParameter, FewsSyncConfig, FewsSyncRequest,
    FewsSyncResult, FewsTimeSeriesQuery, FewsTimeSeriesResponse,
};

/// Fews client error types.
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum FewsError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Authentication failed")]
    AuthenticationFailed,
}

/// Fews PI-REST API client.
pub struct FewsClient {
    pub config: FewsConfig,
    http_client: Client,
}

impl FewsClient {
    /// Create a new Fews client.
    pub fn new(config: FewsConfig) -> Self {
        let timeout = StdDuration::from_secs(config.timeout_secs);
        let http_client = Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_default();

        Self { config, http_client }
    }

    /// Build the base API URL.
    fn build_url(&self, path: &str) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        format!("{}/{}", base, path.trim_start_matches('/'))
    }

    /// Add authentication headers to the request.
    fn add_auth_headers(&self, mut builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(api_key) = &self.config.api_key {
            builder = builder.header("Authorization", format!("Bearer {}", api_key));
        }
        builder
    }

    /// Fetch time series data from Fews.
    pub async fn get_time_series(
        &self,
        query: &FewsTimeSeriesQuery,
    ) -> AnyhowResult<FewsTimeSeriesResponse> {
        let mut url = self.build_url("timeSeries");

        // Add query parameters
        let mut params = Vec::new();
        if let Some(locs) = &query.location_ids {
            for loc in locs {
                params.push(format!("locationIds={}", loc));
            }
        }
        if let Some(p_ids) = &query.parameter_ids {
            for p in p_ids {
                params.push(format!("parameterIds={}", p));
            }
        }
        if let Some(modules) = &query.module_instance_ids {
            for m in modules {
                params.push(format!("moduleInstanceIds={}", m));
            }
        }
        if let Some(start) = &query.start_time {
            params.push(format!("startTime={}", start.format("%Y-%m-%dT%H:%M:%SZ")));
        }
        if let Some(end) = &query.end_time {
            params.push(format!("endTime={}", end.format("%Y-%m-%dT%H:%M:%SZ")));
        }
        if let Some(qualifier) = &query.qualifier {
            params.push(format!("qualifier={}", qualifier));
        }
        if let Some(show) = query.show_enumeration {
            params.push(format!("showEnumeration={}", show));
        }
        if let Some(version) = &query.version {
            params.push(format!("version={}", version));
        }
        if let Some(headers_only) = query.only_headers {
            params.push(format!("onlyHeaders={}", headers_only));
        }

        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        debug!("Fetching Fews time series: {}", url);

        let req = self.add_auth_headers(self.http_client.get(&url));
        let resp = req.send().await?;

        if !resp.status().is_success() {
            return Err(FewsError::InvalidResponse(format!(
                "HTTP {}: {}",
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default()
            ))
            .into());
        }

        let json = resp.text().await?;
        let response: FewsTimeSeriesResponse = serde_json::from_str(&json)
            .map_err(|e| FewsError::InvalidResponse(format!("Parse error: {}", e)))?;

        info!("Retrieved {} time series from Fews", response.time_series.len());

        Ok(response)
    }

    /// Fetch available locations.
    pub async fn get_locations(&self) -> AnyhowResult<Vec<FewsLocation>> {
        let url = self.build_url(&format!(
            "filters/{}",
            urlencoding::encode(&self.config.filter_id)
        ));

        debug!("Fetching Fews locations: {}", url);

        let req = self.add_auth_headers(self.http_client.get(&url));
        let resp = req.send().await?;

        if !resp.status().is_success() {
            return Err(FewsError::InvalidResponse(format!(
                "HTTP {}: {}",
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default()
            ))
            .into());
        }

        let json = resp.text().await?;

        // Parse based on Fews PI-REST response structure
        let value: serde_json::Value = serde_json::from_str(&json)
            .map_err(|e| FewsError::InvalidResponse(format!("Parse error: {}", e)))?;

        // Try to extract locations from different possible response structures
        let locations = if let Some(locs) = value.get("locations").and_then(|v| v.as_array()) {
            serde_json::from_value::<Vec<FewsLocation>>(serde_json::json!(locs))
                .unwrap_or_default()
        } else if let Some(locs) = value.get("location").and_then(|v| v.as_array()) {
            serde_json::from_value::<Vec<FewsLocation>>(serde_json::json!(locs))
                .unwrap_or_default()
        } else if value.is_array() {
            serde_json::from_value::<Vec<FewsLocation>>(value)
                .unwrap_or_default()
        } else {
            warn!("Unexpected Fews locations response structure");
            Vec::new()
        };

        info!("Retrieved {} locations from Fews", locations.len());

        Ok(locations)
    }

    /// Fetch available parameters.
    pub async fn get_parameters(&self) -> AnyhowResult<Vec<FewsParameter>> {
        let url = self.build_url(&format!(
            "filters/{}",
            urlencoding::encode(&self.config.filter_id)
        ));

        debug!("Fetching Fews parameters: {}", url);

        let req = self.add_auth_headers(self.http_client.get(&url));
        let resp = req.send().await?;

        if !resp.status().is_success() {
            return Err(FewsError::InvalidResponse(format!(
                "HTTP {}: {}",
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default()
            ))
            .into());
        }

        let json = resp.text().await?;
        let value: serde_json::Value = serde_json::from_str(&json)
            .map_err(|e| FewsError::InvalidResponse(format!("Parse error: {}", e)))?;

        // Try to extract parameters from different possible response structures
        let parameters = if let Some(params) = value.get("parameters").and_then(|v| v.as_array()) {
            serde_json::from_value::<Vec<FewsParameter>>(serde_json::json!(params))
                .unwrap_or_default()
        } else if let Some(params) = value.get("parameter").and_then(|v| v.as_array()) {
            serde_json::from_value::<Vec<FewsParameter>>(serde_json::json!(params))
                .unwrap_or_default()
        } else if value.is_array() {
            serde_json::from_value::<Vec<FewsParameter>>(value)
                .unwrap_or_default()
        } else {
            warn!("Unexpected Fews parameters response structure");
            Vec::new()
        };

        info!("Retrieved {} parameters from Fews", parameters.len());

        Ok(parameters)
    }

    /// Fetch available module instances.
    pub async fn get_module_instances(&self) -> AnyhowResult<Vec<FewsModuleInstance>> {
        let url = self.build_url("modules");

        debug!("Fetching Fews module instances: {}", url);

        let req = self.add_auth_headers(self.http_client.get(&url));
        let resp = req.send().await?;

        if !resp.status().is_success() {
            return Err(FewsError::InvalidResponse(format!(
                "HTTP {}: {}",
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default()
            ))
            .into());
        }

        let json = resp.text().await?;
        let value: serde_json::Value = serde_json::from_str(&json)
            .map_err(|e| FewsError::InvalidResponse(format!("Parse error: {}", e)))?;

        // Try to extract modules from different possible response structures
        let modules = if let Some(mods) = value.get("modules").and_then(|v| v.as_array()) {
            serde_json::from_value::<Vec<FewsModuleInstance>>(serde_json::json!(mods))
                .unwrap_or_default()
        } else if let Some(mods) = value.get("module").and_then(|v| v.as_array()) {
            serde_json::from_value::<Vec<FewsModuleInstance>>(serde_json::json!(mods))
                .unwrap_or_default()
        } else if value.is_array() {
            serde_json::from_value::<Vec<FewsModuleInstance>>(value)
                .unwrap_or_default()
        } else {
            warn!("Unexpected Fews modules response structure");
            Vec::new()
        };

        info!("Retrieved {} module instances from Fews", modules.len());

        Ok(modules)
    }

    /// Perform a sync operation to fetch time series data.
    pub async fn sync(&self, request: &FewsSyncRequest) -> AnyhowResult<FewsSyncResult> {
        info!("Starting Fews sync from {:?} to {:?}", request.start_time, request.end_time);

        let mut query = FewsTimeSeriesQuery::default();

        if let Some(locs) = &request.location_ids {
            query.location_ids = Some(locs.clone());
        }
        if let Some(params) = &request.parameter_ids {
            query.parameter_ids = Some(params.clone());
        }

        query.start_time = Some(request.start_time);
        query.end_time = Some(request.end_time);

        let response = self.get_time_series(&query).await?;

        let mut data_points_count = 0;
        let mut locations = HashSet::new();
        let mut parameters = HashSet::new();

        for ts in &response.time_series {
            locations.insert(ts.header.location_id.clone());
            parameters.insert(ts.header.parameter_id.clone());
            data_points_count += ts.data.len();
        }

        let result = FewsSyncResult {
            time_series_count: response.time_series.len(),
            data_points_count,
            locations: locations.into_iter().collect(),
            parameters: parameters.into_iter().collect(),
            start_time: request.start_time,
            end_time: request.end_time,
            synced_at: Utc::now(),
        };

        info!(
            "Fews sync completed: {} time series, {} data points",
            result.time_series_count,
            result.data_points_count
        );

        Ok(result)
    }

    /// Test the connection to Fews.
    pub async fn ping(&self) -> AnyhowResult<bool> {
        let url = self.build_url("version");

        debug!("Pinging Fews API: {}", url);

        let req = self.add_auth_headers(self.http_client.get(&url));
        let resp = req.send().await?;

        let success = resp.status().is_success();

        if success {
            info!("Fews API ping successful");
        } else {
            warn!("Fews API ping failed: HTTP {}", resp.status().as_u16());
        }

        Ok(success)
    }
}

/// Fews sync service for managing periodic data synchronization.
pub struct FewsSyncService {
    #[allow(dead_code)]
    client: Arc<FewsClient>,
    config: Vec<FewsSyncConfig>,
}

#[allow(dead_code)]
impl FewsSyncService {
    /// Create a new Fews sync service.
    pub fn new(#[allow(dead_code)]
    client: Arc<FewsClient>, config: Vec<FewsSyncConfig>) -> Self {
        Self { client, config }
    }

    /// Run sync for a specific peilgebied.
    pub async fn sync_peilgebied(
        &self,
        peilgebied_id: &str,
        hours_back: i64,
    ) -> AnyhowResult<Option<FewsSyncResult>> {
        let config = self.config.iter()
            .find(|c| c.peilgebied_id == peilgebied_id);

        let config = match config {
            Some(c) => c,
            None => {
                debug!("No Fews sync config for peilgebied: {}", peilgebied_id);
                return Ok(None);
            }
        };

        let end_time = Utc::now();
        let start_time = end_time - Duration::hours(hours_back);

        let request = FewsSyncRequest {
            start_time,
            end_time,
            location_ids: Some(config.location_mapping.values().cloned().collect()),
            parameter_ids: Some(config.parameter_mapping.values().cloned().collect()),
            sync_results: Some(true),
        };

        // Apply filter ID
        let mut client_config = self.client.config.clone();
        client_config.filter_id = config.fews_filter_id.clone();
        let client = FewsClient::new(client_config);

        let result = client.sync(&request).await?;

        Ok(Some(result))
    }

    /// Get all sync configurations.
    pub fn get_configs(&self) -> &[FewsSyncConfig] {
        &self.config
    }

    /// Add or update a sync configuration.
    pub fn upsert_config(&mut self, config: FewsSyncConfig) {
        let pos = self.config.iter()
            .position(|c| c.peilgebied_id == config.peilgebied_id);

        if let Some(idx) = pos {
            self.config[idx] = config;
        } else {
            self.config.push(config);
        }
    }

    /// Remove a sync configuration.
    pub fn remove_config(&mut self, peilgebied_id: &str) -> Option<FewsSyncConfig> {
        self.config.iter()
            .position(|c| c.peilgebied_id == peilgebied_id)
            .map(|idx| self.config.remove(idx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fews_config_default() {
        let config = FewsConfig::default();
        assert_eq!(config.filter_id, "WatershedFilter");
        assert_eq!(config.timeout_secs, 30);
    }
}
