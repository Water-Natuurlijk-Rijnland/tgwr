//! Time Series Storage service for efficient historical data management.
//!
//! This service provides:
//! - Multi-resolution time series storage
//! - Automatic downsampling on write
//! - Fast range queries with aggregation
//! - Gap detection and analysis

use anyhow::Result as AnyhowResult;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

use peilbeheer_core::timeseries::*;
use peilbeheer_core::fews::FewsTimeSeries as FewsSeries;

use crate::db::Database;

/// Time series storage service.
pub struct TimeSeriesService {
    db: Arc<Database>,
    downsample_config: DownsampleConfig,
}

impl TimeSeriesService {
    /// Create a new time series service.
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            downsample_config: DownsampleConfig::default(),
        }
    }

    /// Set the downsample configuration.
    #[allow(dead_code)]
    pub fn with_downsample_config(mut self, config: DownsampleConfig) -> Self {
        self.downsample_config = config;
        self
    }

    /// Register a new time series in the catalog.
    pub async fn register_series(
        &self,
        metadata: TimeSeriesMetadata,
    ) -> AnyhowResult<()> {
        info!("Registering time series: {}", metadata.id.key());

        let _key = metadata.id.key();
        let now = Utc::now();

        self.db.execute(
            "INSERT INTO timeseries_catalog
                (location_id, parameter, qualifier, display_name, description, units,
                 data_type, source, source_type, min_value, max_value, retention_days,
                 created_at, updated_at, attributes)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (location_id, parameter, qualifier)
             DO UPDATE SET
                 display_name = excluded.display_name,
                 description = excluded.description,
                 units = excluded.units,
                 data_type = excluded.data_type,
                 source = excluded.source,
                 source_type = excluded.source_type,
                 min_value = excluded.min_value,
                 max_value = excluded.max_value,
                 retention_days = excluded.retention_days,
                 updated_at = excluded.updated_at,
                 attributes = excluded.attributes",
            &[
                &metadata.id.location_id as &dyn duckdb::ToSql,
                &metadata.id.parameter,
                &metadata.id.qualifier,
                &metadata.display_name,
                &metadata.description,
                &metadata.units,
                &serde_str(&metadata.data_type),
                &metadata.source,
                &serde_str(&metadata.source_type),
                &metadata.min_value,
                &metadata.max_value,
                &metadata.retention_days,
                &format_datetime(now),
                &format_datetime(metadata.updated_at),
                &json_str(&metadata.attributes),
            ],
        )?;

        Ok(())
    }

    /// Write a batch of data points for a time series.
    pub async fn write_batch(
        &self,
        batch: TimeSeriesWriteBatch,
    ) -> AnyhowResult<TimeSeriesWriteResult> {
        let series_key = batch.series_id.key();
        debug!("Writing {} points for series {}", batch.data.len(), series_key);

        // Ensure series exists in catalog
        self.ensure_catalog_entry(&batch.series_id).await?;

        let mut points_written = 0;
        let points_updated = 0;
        let mut points_rejected = 0;
        let mut first_ts: Option<DateTime<Utc>> = None;
        let mut last_ts: Option<DateTime<Utc>> = None;

        // Write to raw table
        for point in &batch.data {
            if !point.is_valid() && point.flag != QualityFlag::Missing {
                points_rejected += 1;
                continue;
            }

            let ts_str = format_datetime(point.timestamp);

            // Try insert, update if exists
            match self.db.execute(
                "INSERT INTO timeseries_data_raw (series_id, timestamp, value, quality)
                 VALUES (?, ?, ?, ?)
                 ON CONFLICT (series_id, timestamp)
                 DO UPDATE SET value = excluded.value, quality = excluded.quality",
                &[
                    &series_key as &dyn duckdb::ToSql,
                    &ts_str,
                    &point.value,
                    &point.flag.as_str(),
                ],
            ) {
                Ok(_) => {
                    // Check if it was an insert or update
                    // DuckDB doesn't return affected rows easily, so we count both as written
                    points_written += 1;
                }
                Err(e) => {
                    warn!("Failed to write point for {}: {}", series_key, e);
                    points_rejected += 1;
                }
            }

            if first_ts.is_none() || point.timestamp < first_ts.unwrap() {
                first_ts = Some(point.timestamp);
            }
            if last_ts.is_none() || point.timestamp > last_ts.unwrap() {
                last_ts = Some(point.timestamp);
            }
        }

        // Update catalog statistics
        self.update_catalog_stats(&series_key, first_ts, last_ts, points_written).await?;

        // Queue downsampling if enabled
        if self.downsample_config.enabled && points_written > 0 {
            self.queue_downsampling(&series_key, first_ts, last_ts).await?;
        }

        info!(
            "Wrote {} points for series {} (written: {}, rejected: {})",
            batch.data.len(),
            series_key,
            points_written + points_updated,
            points_rejected
        );

        Ok(TimeSeriesWriteResult {
            series_id: batch.series_id,
            points_written: points_written + points_updated,
            points_updated,
            points_rejected,
            first_timestamp: first_ts,
            last_timestamp: last_ts,
        })
    }

    /// Query time series data.
    pub async fn query(
        &self,
        query: &TimeSeriesQuery,
    ) -> AnyhowResult<AggregatedSeries> {
        query.validate().map_err(|e| anyhow::anyhow!("Invalid query: {}", e))?;

        let series_key = query.series_id.key();

        // Determine which table to query based on aggregation
        let (table_name, interval_sec) = match query.aggregation {
            Some(level) => {
                let table = match level {
                    AggregationLevel::Raw => "timeseries_data_raw",
                    AggregationLevel::Min1 => "timeseries_data_1m",
                    AggregationLevel::Min5 => "timeseries_data_5m",
                    AggregationLevel::Min15 => "timeseries_data_15m",
                    AggregationLevel::Hour1 => "timeseries_data_1h",
                    AggregationLevel::Hour6 => "timeseries_data_1h",
                    AggregationLevel::Day1 => "timeseries_data_1d",
                    AggregationLevel::Week1 => "timeseries_data_1d",
                    AggregationLevel::Month1 => "timeseries_data_1d",
                };
                (table, level.interval_seconds())
            }
            None => ("timeseries_data_raw", 0),
        };

        // Build query based on aggregation function
        let (value_col, _quality_handling) = match query.function {
            Some(func) => match func {
                AggregationFunction::Average => ("avg_value", "AVG"),
                AggregationFunction::Min => ("min_value", "MIN"),
                AggregationFunction::Max => ("max_value", "MAX"),
                AggregationFunction::Sum => ("sum_value", "SUM"),
                AggregationFunction::Count => ("count", "COUNT"),
                AggregationFunction::First => ("first_value", "FIRST"),
                AggregationFunction::Last => ("last_value", "LAST"),
                _ => ("avg_value", "AVG"),
            },
            None => ("value", ""),
        };

        let start_str = format_datetime(query.start);
        let end_str = format_datetime(query.end);

        let sql = if query.aggregation.is_some() || query.function.is_some() {
            // Aggregated query
            format!(
                "SELECT timestamp, {} as value
                 FROM {}
                 WHERE series_id = ? AND timestamp >= ? AND timestamp < ?
                 ORDER BY timestamp",
                value_col, table_name
            )
        } else {
            // Raw query
            format!(
                "SELECT timestamp, value, quality as flag
                 FROM {}
                 WHERE series_id = ? AND timestamp >= ? AND timestamp < ?
                 ORDER BY timestamp",
                table_name
            )
        };

        let rows = self.db.query(
            &sql,
            &[
                &series_key as &dyn duckdb::ToSql,
                &start_str,
                &end_str,
            ],
            |row| {
                if query.aggregation.is_some() || query.function.is_some() {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<f64>>(1)?,
                        Some(QualityFlag::Good),
                    ))
                } else {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<f64>>(1)?,
                        row.get::<_, Option<String>>(2)?.and_then(|s| QualityFlag::from_str(&s)),
                    ))
                }
            },
        )?;

        let mut data = Vec::new();
        for (ts_str, value, flag) in rows {
            let timestamp = parse_datetime(&ts_str);
            let flag = flag.unwrap_or(QualityFlag::Good);

            if let Some(v) = value {
                data.push(TimeSeriesDataPoint::with_flag(timestamp, v, flag));
            } else {
                data.push(TimeSeriesDataPoint::with_flag(timestamp, f64::NAN, QualityFlag::Missing));
            }
        }

        // Apply gap filling if requested
        if let Some(fill_method) = query.fill_gaps
            && !data.is_empty() && interval_sec > 0 {
                data = self.fill_gaps(data, query.start, query.end, interval_sec, fill_method)?;
            }

        let function = query.function.unwrap_or(AggregationFunction::Average);
        let aggregation = query.aggregation.unwrap_or(AggregationLevel::Raw);

        Ok(AggregatedSeries {
            series_id: query.series_id.clone(),
            aggregation,
            function,
            data,
            metadata: AggregationMetadata {
                data_points: 0,
                gaps_filled: 0,
                quality_flags: HashMap::new(),
                start: query.start,
                end: query.end,
            },
        })
    }

    /// Get series metadata from catalog.
    pub async fn get_metadata(&self, id: &TimeSeriesId) -> AnyhowResult<Option<TimeSeriesMetadata>> {
        let _key = id.key();

        let result = self.db.query_row(
            "SELECT location_id, parameter, qualifier, display_name, description, units,
                     data_type, source, source_type, min_value, max_value, retention_days,
                     created_at, updated_at, first_timestamp, last_timestamp, point_count, attributes
             FROM timeseries_catalog
             WHERE location_id = ? AND parameter = ? AND COALESCE(qualifier, '') = COALESCE(?, '')",
            &[
                &id.location_id as &dyn duckdb::ToSql,
                &id.parameter,
                &id.qualifier,
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, Option<f64>>(9)?,
                    row.get::<_, Option<f64>>(10)?,
                    row.get::<_, Option<u32>>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, Option<String>>(14)?,
                    row.get::<_, Option<String>>(15)?,
                    row.get::<_, Option<i64>>(16)?,
                    row.get::<_, Option<String>>(17)?,
                ))
            },
        );

        match result {
            Ok((_loc, _param, _qual, name, desc, units, data_type, source, source_type,
                min_val, max_val, retention, created_str, updated_str,
                _first_str, _last_str, _point_count, attr_str)) =>
            {
                let attributes: HashMap<String, serde_json::Value> = attr_str
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                Ok(Some(TimeSeriesMetadata {
                    id: id.clone(),
                    display_name: name,
                    description: desc,
                    units,
                    data_type: parse_data_type(&data_type),
                    min_value: min_val,
                    max_value: max_val,
                    source,
                    source_type: parse_source_type(&source_type),
                    created_at: parse_datetime(&created_str),
                    updated_at: parse_datetime(&updated_str),
                    retention_days: retention,
                    attributes,
                }))
            }
            Err(e) if e.to_string().contains("QueryReturnedNoRows") => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// List all time series in the catalog.
    pub async fn list_series(
        &self,
        source_type: Option<&str>,
        limit: Option<usize>,
    ) -> AnyhowResult<Vec<TimeSeriesCatalogEntry>> {
        let sql = if let Some(_st) = source_type {
            format!(
                "SELECT location_id, parameter, qualifier, display_name, units, source,
                         first_timestamp, last_timestamp, point_count
                 FROM timeseries_catalog
                 WHERE source_type = ?
                 ORDER BY location_id, parameter
                 LIMIT {}",
                limit.unwrap_or(1000)
            )
        } else {
            format!(
                "SELECT location_id, parameter, qualifier, display_name, units, source,
                         first_timestamp, last_timestamp, point_count
                 FROM timeseries_catalog
                 ORDER BY location_id, parameter
                 LIMIT {}",
                limit.unwrap_or(1000)
            )
        };

        let rows = if let Some(st) = source_type {
            self.db.query(
                &sql,
                &[&st as &dyn duckdb::ToSql],
                parse_catalog_row,
            )?
        } else {
            self.db.query(&sql, &[], parse_catalog_row)?
        };

        Ok(rows)
    }

    /// Import Fews time series data.
    #[allow(dead_code)]
    pub async fn import_from_fews(
        &self,
        series: &FewsSeries,
        location_id: &str,
    ) -> AnyhowResult<TimeSeriesWriteResult> {
        let ts_id = TimeSeriesId::new(
            location_id,
            series.header.parameter_id.clone(),
        );

        let data: Vec<TimeSeriesDataPoint> = series.data.iter()
            .filter_map(|p| {
                match chrono::DateTime::parse_from_rfc3339(&p.date) {
                    Ok(dt) => Some(TimeSeriesDataPoint::with_flag(
                        dt.with_timezone(&Utc),
                        p.value,
                        QualityFlag::Good,
                    )),
                    Err(_) => None,
                }
            })
            .collect();

        let batch = TimeSeriesWriteBatch {
            series_id: ts_id,
            data,
            attributes: None,
        };

        self.write_batch(batch).await
    }

    /// Ensure catalog entry exists for a series.
    async fn ensure_catalog_entry(&self, id: &TimeSeriesId) -> AnyhowResult<()> {
        // Check if exists
        let exists = self.db.query_row(
            "SELECT COUNT(*) FROM timeseries_catalog
             WHERE location_id = ? AND parameter = ? AND COALESCE(qualifier, '') = COALESCE(?, '')",
            &[
                &id.location_id as &dyn duckdb::ToSql,
                &id.parameter,
                &id.qualifier,
            ],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) > 0;

        if !exists {
            // Create default entry
            let metadata = TimeSeriesMetadata {
                id: id.clone(),
                display_name: format!("{} - {}", id.location_id, id.parameter),
                description: None,
                units: None,
                data_type: TimeSeriesDataType::Instantaneous,
                min_value: None,
                max_value: None,
                source: "system".to_string(),
                source_type: TimeSeriesSourceType::Custom("system".to_string()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                retention_days: None,
                attributes: HashMap::new(),
            };
            self.register_series(metadata).await?;
        }

        Ok(())
    }

    /// Update catalog statistics after write.
    async fn update_catalog_stats(
        &self,
        series_key: &str,
        first_ts: Option<DateTime<Utc>>,
        last_ts: Option<DateTime<Utc>>,
        points_written: usize,
    ) -> AnyhowResult<()> {
        if points_written == 0 {
            return Ok(());
        }

        self.db.execute(
            "UPDATE timeseries_catalog
             SET point_count = point_count + ?,
                 first_timestamp = COALESCE(MIN(first_timestamp), ?),
                 last_timestamp = COALESCE(MAX(last_timestamp), ?),
                 updated_at = ?
             WHERE (location_id || '|' || parameter || COALESCE('|' || qualifier, '')) = ?",
            &[
                &points_written as &dyn duckdb::ToSql,
                &first_ts.map(format_datetime),
                &last_ts.map(format_datetime),
                &format_datetime(Utc::now()),
                &series_key,
            ],
        )?;

        Ok(())
    }

    /// Queue downsampling task for new data.
    async fn queue_downsampling(
        &self,
        series_key: &str,
        first_ts: Option<DateTime<Utc>>,
        last_ts: Option<DateTime<Utc>>,
    ) -> AnyhowResult<()> {
        let (first, last) = match (first_ts, last_ts) {
            (Some(f), Some(l)) => (f, l),
            _ => return Ok(()),
        };

        for level in &self.downsample_config.levels {
            let id = format!("DS_{}_{}", series_key, uuid::Uuid::new_v4());

            self.db.execute(
                "INSERT INTO timeseries_downsample_queue
                    (id, series_id, level, start_timestamp, end_timestamp, status, priority)
                 VALUES (?, ?, ?, ?, ?, 'pending', 0)",
                &[
                    &id as &dyn duckdb::ToSql,
                    &series_key,
                    &level_to_str(*level),
                    &format_datetime(first),
                    &format_datetime(last),
                ],
            )?;
        }

        Ok(())
    }

    /// Fill gaps in time series data.
    fn fill_gaps(
        &self,
        data: Vec<TimeSeriesDataPoint>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        interval_sec: i64,
        method: FillMethod,
    ) -> AnyhowResult<Vec<TimeSeriesDataPoint>> {
        if data.is_empty() {
            return Ok(data);
        }

        match method {
            FillMethod::None => Ok(data),
            FillMethod::Forward => self.fill_forward(data, start, end, interval_sec),
            FillMethod::Linear => self.fill_linear(data, start, end, interval_sec),
            _ => Ok(data), // TODO: implement other methods
        }
    }

    /// Forward fill gaps.
    fn fill_forward(
        &self,
        data: Vec<TimeSeriesDataPoint>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        interval_sec: i64,
    ) -> AnyhowResult<Vec<TimeSeriesDataPoint>> {
        let mut result = Vec::new();
        let mut data_iter = data.into_iter().peekable();
        let mut current_ts = start;
        let mut last_valid_value: Option<f64> = None;

        while current_ts < end {
            // Find if we have data for this timestamp
            while data_iter.peek().is_some()
                && data_iter.peek().unwrap().timestamp < current_ts
            {
                let point = data_iter.next().unwrap();
                if point.is_valid() {
                    last_valid_value = Some(point.value);
                }
            }

            let point = if data_iter.peek().is_some()
                && data_iter.peek().unwrap().timestamp == current_ts
            {
                let p = data_iter.next().unwrap();
                if p.is_valid() {
                    last_valid_value = Some(p.value);
                }
                p
            } else {
                // Fill with last valid value
                TimeSeriesDataPoint::with_flag(
                    current_ts,
                    last_valid_value.unwrap_or(f64::NAN),
                    QualityFlag::Interpolated,
                )
            };

            result.push(point);
            current_ts += Duration::seconds(interval_sec);
        }

        Ok(result)
    }

    /// Linear interpolation for gaps.
    fn fill_linear(
        &self,
        data: Vec<TimeSeriesDataPoint>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        interval_sec: i64,
    ) -> AnyhowResult<Vec<TimeSeriesDataPoint>> {
        let mut result = Vec::new();
        let valid_points: Vec<_> = data.into_iter()
            .filter(|p| p.is_valid())
            .collect();

        if valid_points.is_empty() {
            return Ok(result);
        }

        let mut current_ts = start;
        let mut valid_idx = 0;

        while current_ts < end {
            // Find surrounding valid points
            while valid_idx + 1 < valid_points.len()
                && valid_points[valid_idx + 1].timestamp < current_ts
            {
                valid_idx += 1;
            }

            let value = if valid_points[valid_idx].timestamp == current_ts {
                valid_points[valid_idx].value
            } else if valid_idx + 1 < valid_points.len() {
                // Linear interpolate
                let p1 = &valid_points[valid_idx];
                let p2 = &valid_points[valid_idx + 1];

                if p1.timestamp >= p2.timestamp {
                    p1.value
                } else {
                    let t1 = p1.timestamp.timestamp();
                    let t2 = p2.timestamp.timestamp();
                    let t = current_ts.timestamp();

                    let ratio = (t - t1) as f64 / (t2 - t1) as f64;
                    p1.value + ratio * (p2.value - p1.value)
                }
            } else {
                // No next point, use last valid
                valid_points[valid_idx].value
            };

            result.push(TimeSeriesDataPoint::with_flag(
                current_ts,
                value,
                QualityFlag::Interpolated,
            ));

            current_ts += Duration::seconds(interval_sec);
        }

        Ok(result)
    }
}

/// Helper: Serialize enum as string.
fn serde_str<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .and_then(|s| serde_json::from_str::<String>(&s))
        .unwrap_or_default()
}

/// Helper: Serialize value as JSON string.
fn json_str(value: &HashMap<String, serde_json::Value>) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        serde_json::to_string(value).ok()
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
        .map(|ndt| ndt.and_utc())
        .unwrap_or_else(|_| Utc::now())
}

/// Helper: Parse data type.
fn parse_data_type(s: &str) -> TimeSeriesDataType {
    match s.to_lowercase().as_str() {
        "instantaneous" => TimeSeriesDataType::Instantaneous,
        "accumulated" => TimeSeriesDataType::Accumulated,
        "average" => TimeSeriesDataType::Average,
        "total" => TimeSeriesDataType::Total,
        "boolean" => TimeSeriesDataType::Boolean,
        "enum" => TimeSeriesDataType::Enum,
        _ => TimeSeriesDataType::Instantaneous,
    }
}

/// Helper: Parse source type.
fn parse_source_type(s: &str) -> TimeSeriesSourceType {
    match s.to_lowercase().as_str() {
        "fews" => TimeSeriesSourceType::Fews,
        "hydronet" => TimeSeriesSourceType::Hydronet,
        "dhydro" => TimeSeriesSourceType::DHydro,
        "energyzero" => TimeSeriesSourceType::EnergyZero,
        "manual" => TimeSeriesSourceType::Manual,
        "calculated" => TimeSeriesSourceType::Calculated,
        other => TimeSeriesSourceType::Custom(other.to_string()),
    }
}

/// Helper: Convert aggregation level to string.
fn level_to_str(level: AggregationLevel) -> &'static str {
    match level {
        AggregationLevel::Raw => "raw",
        AggregationLevel::Min1 => "1m",
        AggregationLevel::Min5 => "5m",
        AggregationLevel::Min15 => "15m",
        AggregationLevel::Hour1 => "1h",
        AggregationLevel::Hour6 => "6h",
        AggregationLevel::Day1 => "1d",
        AggregationLevel::Week1 => "1w",
        AggregationLevel::Month1 => "1mo",
    }
}

/// Helper: Parse catalog row.
fn parse_catalog_row(row: &duckdb::Row) -> duckdb::Result<TimeSeriesCatalogEntry> {
    Ok(TimeSeriesCatalogEntry {
        id: TimeSeriesId {
            location_id: row.get(0)?,
            parameter: row.get(1)?,
            qualifier: row.get(2)?,
        },
        display_name: row.get(3)?,
        units: row.get(4)?,
        source: row.get(5)?,
        has_raw_data: true, // We assume true if in catalog
        first_timestamp: row.get::<_, Option<String>>(6)?.map(|s| parse_datetime(&s)),
        last_timestamp: row.get::<_, Option<String>>(7)?.map(|s| parse_datetime(&s)),
        point_count: row.get::<_, Option<i64>>(8)?.unwrap_or(0) as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeseries_id() {
        let id = TimeSeriesId::new("LOC_001", "water_level");
        assert_eq!(id.key(), "LOC_001|water_level");

        let id_with_q = TimeSeriesId::with_qualifier("LOC_001", "water_level", "inlet");
        assert_eq!(id_with_q.key(), "LOC_001|water_level|inlet");
    }

    #[test]
    fn test_quality_flag() {
        assert!(QualityFlag::Good.is_valid());
        assert!(!QualityFlag::Bad.is_valid());
        assert_eq!(QualityFlag::Missing.as_str(), "missing");
    }

    #[test]
    fn test_data_point() {
        let p = TimeSeriesDataPoint::new(Utc::now(), 10.0);
        assert!(p.is_valid());

        let p_bad = TimeSeriesDataPoint::with_flag(Utc::now(), 10.0, QualityFlag::Bad);
        assert!(!p_bad.is_valid());
    }

    #[test]
    fn test_aggregation_level() {
        assert_eq!(AggregationLevel::Min1.duration(), Duration::seconds(60));
        assert_eq!(AggregationLevel::Hour1.interval_seconds(), 3600);
    }

    #[test]
    fn test_query_validation() {
        let id = TimeSeriesId::new("test", "value");
        let start = Utc::now();
        let end = start + Duration::hours(1);

        let valid = TimeSeriesQuery::new(id.clone(), start, end);
        assert!(valid.validate().is_ok());

        let invalid = TimeSeriesQuery::new(id, end, start);
        assert!(invalid.validate().is_err());
    }
}
