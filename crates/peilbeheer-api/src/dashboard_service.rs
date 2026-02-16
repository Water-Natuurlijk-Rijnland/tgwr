//! Dashboard service for aggregating KPIs and widget data.
//!
//! This service provides:
//! - System-wide KPIs for dashboard
//! - Widget data generation
//! - Activity feed aggregation
//! - System health monitoring

use anyhow::Result as AnyhowResult;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;

use peilbeheer_core::dashboard::*;

use crate::db::Database;

/// Dashboard service.
pub struct DashboardService {
    db: Arc<Database>,
}

impl DashboardService {
    /// Create a new dashboard service.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Get all dashboard KPIs.
    pub async fn get_kpi(&self) -> AnyhowResult<DashboardKpi> {
        let now = Utc::now();

        let system = self.get_system_health_kpi().await?;
        let gemalen = self.get_gemaal_kpi().await?;
        let alerts = self.get_alert_kpi().await?;
        let scenarios = self.get_scenario_kpi().await?;
        let sync = self.get_sync_kpi().await?;
        let performance = self.get_performance_kpi().await?;

        Ok(DashboardKpi {
            timestamp: now,
            system,
            gemalen,
            alerts,
            scenarios,
            sync,
            performance,
        })
    }

    /// Get system health KPIs.
    async fn get_system_health_kpi(&self) -> AnyhowResult<SystemHealthKpi> {
        // Check database connectivity
        let db_status = self.check_database_health().await;

        // Count services (for now, static)
        let services_total = 6; // api, db, ws, alert, fews, scenario
        let services_active = services_total; // All assumed active if db is healthy

        let mut external_services = HashMap::new();

        // Check Fews connectivity
        external_services.insert(
            "fews".to_string(),
            self.check_fews_health().await,
        );

        // Check Hydronet connectivity
        external_services.insert(
            "hydronet".to_string(),
            self.check_hydronet_health().await,
        );

        // Overall status based on components
        let status = match (db_status, services_active) {
            (HealthStatus::Healthy, _) if services_active == services_total => HealthStatus::Healthy,
            (HealthStatus::Degraded, _) | (_, _) if services_active < services_total => HealthStatus::Degraded,
            (HealthStatus::Unhealthy, _) => HealthStatus::Unhealthy,
            _ => HealthStatus::Healthy,
        };

        Ok(SystemHealthKpi {
            status,
            uptime_percent: 100.0, // TODO: calculate from logs
            services_active,
            services_total,
            database_status: db_status,
            external_services,
        })
    }

    /// Get gemaal KPIs.
    async fn get_gemaal_kpi(&self) -> AnyhowResult<GemaalKpi> {
        // Get gemaal counts from database
        let snapshots = self.db.get_all_snapshots()?;

        let mut total = snapshots.len() as u32;
        let mut active = 0;
        let mut inactive = 0;
        let mut error = 0;
        let mut unknown = 0;
        let mut total_capacity = 0.0;
        let mut active_capacity = 0.0;
        let mut recently_updated = 0;

        let five_minutes_ago = Utc::now() - Duration::minutes(5);

        for snapshot in &snapshots {
            match &snapshot.status {
                peilbeheer_core::gemaal::GemaalStatus::Aan => active += 1,
                peilbeheer_core::gemaal::GemaalStatus::Uit => inactive += 1,
                peilbeheer_core::gemaal::GemaalStatus::Error => error += 1,
                peilbeheer_core::gemaal::GemaalStatus::Onbekend => unknown += 1,
            }

            if snapshot.debiet > 0.0 {
                total_capacity += snapshot.debiet;
                if matches!(snapshot.status, peilbeheer_core::gemaal::GemaalStatus::Aan) {
                    active_capacity += snapshot.debiet;
                }
            }

            if let Some(updated) = snapshot.generated_at {
                if updated > five_minutes_ago {
                    recently_updated += 1;
                }
            }
        }

        let utilization_percent = if total_capacity > 0.0 {
            (active_capacity / total_capacity) * 100.0
        } else {
            0.0
        };

        Ok(GemaalKpi {
            total,
            active,
            inactive,
            error,
            unknown,
            total_capacity,
            active_capacity,
            utilization_percent,
            recently_updated,
        })
    }

    /// Get alert KPIs.
    async fn get_alert_kpi(&self) -> AnyhowResult<AlertKpi> {
        // For now, return default values
        // TODO: Query from alerts table when alert_service is integrated

        let mut by_severity = HashMap::new();
        by_severity.insert("critical".to_string(), 0);
        by_severity.insert("error".to_string(), 0);
        by_severity.insert("warning".to_string(), 0);
        by_severity.insert("info".to_string(), 0);

        let mut by_category = HashMap::new();
        by_category.insert("water_level".to_string(), 0);
        by_category.insert("pump_status".to_string(), 0);
        by_category.insert("energy_price".to_string(), 0);

        Ok(AlertKpi {
            active_total: 0,
            by_severity,
            by_category,
            critical: 0,
            triggered_today: 0,
            acknowledged_today: 0,
            avg_resolution_minutes: None,
        })
    }

    /// Get scenario KPIs.
    async fn get_scenario_kpi(&self) -> AnyhowResult<ScenarioKpi> {
        // TODO: Query from scenarios table
        Ok(ScenarioKpi {
            total: 0,
            active: 0,
            running: 0,
            completed_today: 0,
            failed_today: 0,
            avg_execution_seconds: None,
        })
    }

    /// Get sync KPIs.
    async fn get_sync_kpi(&self) -> AnyhowResult<SyncKpi> {
        let mut last_sync = HashMap::new();
        let mut sync_status = HashMap::new();

        // Get asset sync times
        let asset_count = self.db.get_total_asset_count().unwrap_or(0);
        let peilgebied_count = self.db.get_peilgebied_count().unwrap_or(0);
        let gemaal_count = self.db.get_registratie_count().unwrap_or(0);

        let total_records = (asset_count + peilgebied_count + gemaal_count) as u64;

        // Assume recent sync if we have data
        if asset_count > 0 {
            last_sync.insert("assets".to_string(), Utc::now());
            sync_status.insert("assets".to_string(), HealthStatus::Healthy);
        }
        if peilgebied_count > 0 {
            last_sync.insert("peilgebieden".to_string(), Utc::now());
            sync_status.insert("peilgebieden".to_string(), HealthStatus::Healthy);
        }
        if gemaal_count > 0 {
            last_sync.insert("gemalen".to_string(), Utc::now());
            sync_status.insert("gemalen".to_string(), HealthStatus::Healthy);
        }

        // Calculate freshness score
        let freshness_score = if total_records > 0 {
            let now = Utc::now();
            let hours_since_sync = last_sync.values()
                .map(|ts| (now - *ts).num_hours().max(0))
                .sum::<i64>() / last_sync.len().max(1) as i64;

            (100 - (hours_since_sync * 2).min(100)).max(0) as u32
        } else {
            0
        };

        Ok(SyncKpi {
            last_sync,
            sync_status,
            total_records,
            updated_today: 0, // TODO: track today's updates
            freshness_score,
        })
    }

    /// Get performance KPIs.
    async fn get_performance_kpi(&self) -> AnyhowResult<PerformanceKpi> {
        // TODO: Track actual performance metrics
        Ok(PerformanceKpi {
            avg_response_time_ms: 50.0,
            p95_response_time_ms: 150.0,
            requests_per_second: 0.0,
            error_rate_percent: 0.0,
            active_connections: 0,
        })
    }

    /// Get activity feed.
    pub async fn get_activity_feed(
        &self,
        query: &ActivityFeedQuery,
    ) -> AnyhowResult<ActivityFeedData> {
        let limit = query.limit.unwrap_or(50) as usize;
        let mut items = Vec::new();

        // Get recent gemaal sync activity
        let asset_count = self.db.get_total_asset_count().unwrap_or(0);
        if asset_count > 0 {
            items.push(ActivityFeedItem {
                id: format!("act_{}", uuid::Uuid::new_v4()),
                timestamp: Utc::now() - Duration::minutes(5),
                type_: ActivityType::Sync,
                title: format!("{} assets synchronized", asset_count),
                description: Some("Data refreshed from ArcGIS".to_string()),
                actor: None,
                severity: None,
                link: Some("/api/assets".to_string()),
                metadata: HashMap::new(),
            });
        }

        // Get peilgebied activity
        let peilgebied_count = self.db.get_peilgebied_count().unwrap_or(0);
        if peilgebied_count > 0 {
            items.push(ActivityFeedItem {
                id: format!("act_{}", uuid::Uuid::new_v4()),
                timestamp: Utc::now() - Duration::minutes(10),
                type_: ActivityType::Sync,
                title: format!("{} peilgebieden loaded", peilgebied_count),
                description: Some("Geo polygons loaded for map display".to_string()),
                actor: None,
                severity: None,
                link: Some("/api/peilgebieden/geojson".to_string()),
                metadata: HashMap::new(),
            });
        }

        // Get gemaal status
        let snapshots = self.db.get_all_snapshots().unwrap_or_default();
        let active_count = snapshots.iter()
            .filter(|s| matches!(s.status, peilbeheer_core::gemaal::GemaalStatus::Aan))
            .count();

        items.push(ActivityFeedItem {
            id: format!("act_{}", uuid::Uuid::new_v4()),
            timestamp: Utc::now() - Duration::minutes(1),
            type_: ActivityType::System,
            title: format!("{} gemalen monitored", snapshots.len()),
            description: Some(format!("{} currently active", active_count)),
            actor: None,
            severity: None,
            link: Some("/api/gemalen".to_string()),
            metadata: HashMap::new(),
        });

        // Sort by timestamp and limit
        items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        items.truncate(limit);

        let has_more = items.len() >= limit;

        Ok(ActivityFeedData { items, has_more })
    }

    /// Get system health status.
    pub async fn get_health_status(&self) -> AnyhowResult<HealthStatus> {
        let kpi = self.get_system_health_kpi().await?;
        Ok(kpi.status)
    }

    /// Get alert summary.
    pub async fn get_alert_summary(&self) -> AnyhowResult<AlertKpi> {
        self.get_alert_kpi().await
    }

    /// Get gemaal summary.
    pub async fn get_gemaal_summary(&self) -> AnyhowResult<GemaalKpi> {
        self.get_gemaal_kpi().await
    }

    /// Get chart data for a specific metric.
    pub async fn get_chart_data(
        &self,
        metric: &str,
        hours_back: u32,
    ) -> AnyhowResult<ChartData> {
        let end = Utc::now();
        let start = end - Duration::hours(hours_back as i64);

        match metric {
            "gemalen_status" => self.get_gemalen_status_chart(start, end).await,
            "water_levels" => self.get_water_levels_chart(start, end).await,
            "energy_prices" => self.get_energy_prices_chart(start, end).await,
            _ => Ok(ChartData::default()),
        }
    }

    async fn get_gemalen_status_chart(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> AnyhowResult<ChartData> {
        let snapshots = self.db.get_all_snapshots()?;

        let active = snapshots.iter()
            .filter(|s| matches!(s.status, peilbeheer_core::gemaal::GemaalStatus::Aan))
            .count() as f64;

        Ok(ChartData {
            title: "Gemaal Status".to_string(),
            labels: vec!["Actief".to_string(), "Stilstand".to_string(), "Storing".to_string()],
            datasets: vec![ChartDataset {
                label: "Gemalen".to_string(),
                data: vec![
                    Some(active),
                    Some(snapshots.iter().filter(|s| matches!(s.status, peilbeheer_core::gemaal::GemaalStatus::Uit)).count() as f64),
                    Some(snapshots.iter().filter(|s| matches!(s.status, peilbeheer_core::gemaal::GemaalStatus::Error)).count() as f64),
                ],
                color: "#3b82f6".to_string(),
                background_color: Some("#3b82f620".to_string()),
                border_width: Some(2),
                fill: Some(false),
                dataset_type: Some(ChartDatasetType::Bar),
            }],
            x_axis_label: None,
            y_axis_label: Some("Aantal".to_string()),
            y_axis_min: Some(0.0),
            y_axis_max: None,
        })
    }

    async fn get_water_levels_chart(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> AnyhowResult<ChartData> {
        // TODO: Implement with actual water level data
        Ok(ChartData {
            title: "Water Levels".to_string(),
            labels: vec!["Now".to_string()],
            datasets: vec![ChartDataset {
                label: "NAP".to_string(),
                data: vec![None],
                color: "#0ea5e9".to_string(),
                background_color: Some("#0ea5e920".to_string()),
                border_width: Some(2),
                fill: Some(true),
                dataset_type: Some(ChartDatasetType::Line),
            }],
            x_axis_label: Some("Tijd".to_string()),
            y_axis_label: Some("Niveau (m NAP)".to_string()),
            y_axis_min: None,
            y_axis_max: None,
        })
    }

    async fn get_energy_prices_chart(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> AnyhowResult<ChartData> {
        // TODO: Implement with EnergyZero data
        let hours = ((end - start).num_hours().max(1)) as usize;
        let labels: Vec<String> = (0..hours)
            .map(|i| (start + Duration::hours(i as i64)).format("%H:%M").to_string())
            .collect();

        Ok(ChartData {
            title: "Energy Prices".to_string(),
            labels,
            datasets: vec![ChartDataset {
                label: "EUR/MWh".to_string(),
                data: vec![None; hours],
                color: "#f59e0b".to_string(),
                background_color: Some("#f59e0b20".to_string()),
                border_width: Some(2),
                fill: Some(true),
                dataset_type: Some(ChartDatasetType::Area),
            }],
            x_axis_label: Some("Tijd".to_string()),
            y_axis_label: Some("Prijs (€/MWh)".to_string()),
            y_axis_min: Some(0.0),
            y_axis_max: None,
        })
    }

    /// Check database health.
    async fn check_database_health(&self) -> HealthStatus {
        match self.db.query_row("SELECT 1", [], |_| Ok(())) {
            Ok(_) => HealthStatus::Healthy,
            Err(_) => HealthStatus::Unhealthy,
        }
    }

    /// Check Fews health.
    async fn check_fews_health(&self) -> HealthStatus {
        // TODO: Actually ping Fews service
        HealthStatus::Unknown
    }

    /// Check Hydronet health.
    async fn check_hydronet_health(&self) -> HealthStatus {
        // TODO: Actually ping Hydronet service
        HealthStatus::Unknown
    }
}

// Default implementation moved to peilbeheer-core/src/dashboard.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status() {
        assert_eq!(HealthStatus::Healthy.as_str(), "healthy");
        assert_eq!(HealthStatus::Degraded.as_str(), "degraded");
    }

    #[test]
    fn test_trend_direction() {
        assert_eq!(TrendDirection::from_percent_change(0.05), TrendDirection::Up);
        assert_eq!(TrendDirection::from_percent_change(-0.05), TrendDirection::Down);
        assert_eq!(TrendDirection::from_percent_change(0.0), TrendDirection::Stable);
    }
}
