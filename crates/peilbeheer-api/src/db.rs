use std::path::Path;
use std::sync::Mutex;

use chrono::{DateTime, NaiveDateTime, Utc};
use duckdb::{params, Connection};

use peilbeheer_core::gemaal::{GemaalSnapshot, GemaalStatus, GemaalTrends};

#[allow(dead_code)]
fn datetime_to_string(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S%.6f").to_string()
}

fn parse_datetime(s: &str) -> DateTime<Utc> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.6f")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.6f"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
        .map(|ndt| ndt.and_utc())
        .unwrap_or_else(|_| Utc::now())
}

fn parse_optional_datetime(s: Option<String>) -> Option<DateTime<Utc>> {
    s.map(|ds| parse_datetime(&ds))
}

/// Database wrapper met thread-safe connection.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Initialiseer het database schema vanuit het migratie SQL-bestand.
    pub fn initialize_schema(&self) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        let schema = include_str!("../../../migrations/001_initial_schema.sql");

        for statement in schema.split(';') {
            let stmt = statement.trim();
            if !stmt.is_empty() && !stmt.starts_with("--") {
                if let Err(e) = conn.execute(stmt, []) {
                    let err_str = e.to_string();
                    if !err_str.contains("already exists") {
                        tracing::warn!("Schema statement failed: {}", err_str);
                    }
                }
            }
        }

        tracing::info!("Database schema initialized");
        Ok(())
    }

    /// Schrijf of update een gemaal status snapshot.
    #[allow(dead_code)]
    pub fn write_snapshot(
        &self,
        gemaal_code: &str,
        status: &str,
        debiet: f64,
        last_update: Option<DateTime<Utc>>,
        generated_at: DateTime<Utc>,
        trends_json: Option<&str>,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        let last_update_str = last_update.map(|dt| datetime_to_string(&dt));
        let generated_at_str = datetime_to_string(&generated_at);

        conn.execute(
            r#"
            INSERT INTO gemaal_status_snapshot (gemaal_code, status, debiet, last_update, generated_at, trends_json)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT (gemaal_code) DO UPDATE SET
                status = excluded.status,
                debiet = excluded.debiet,
                last_update = excluded.last_update,
                generated_at = excluded.generated_at,
                trends_json = excluded.trends_json
            "#,
            params![
                gemaal_code,
                status,
                debiet,
                last_update_str,
                generated_at_str,
                trends_json,
            ],
        )?;

        Ok(())
    }

    /// Schrijf uur-gemiddelden voor een gemaal.
    #[allow(dead_code)]
    pub fn write_hourly_averages(
        &self,
        gemaal_code: &str,
        hour_utc: DateTime<Utc>,
        avg_debiet: f64,
        n_metingen: i32,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        let hour_str = datetime_to_string(&hour_utc);

        conn.execute(
            r#"
            INSERT INTO gemaal_debiet_per_uur (gemaal_code, hour_utc, avg_debiet, n_metingen)
            VALUES (?, ?, ?, ?)
            ON CONFLICT (gemaal_code, hour_utc) DO UPDATE SET
                avg_debiet = excluded.avg_debiet,
                n_metingen = excluded.n_metingen
            "#,
            params![gemaal_code, hour_str, avg_debiet, n_metingen],
        )?;

        Ok(())
    }

    /// Verwijder uur-records ouder dan `days` dagen.
    #[allow(dead_code)]
    pub fn cleanup_old_hourly(&self, days: i64) -> anyhow::Result<u64> {
        let conn = self.conn.lock().unwrap();
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let cutoff_str = datetime_to_string(&cutoff);

        let count = conn.execute(
            "DELETE FROM gemaal_debiet_per_uur WHERE hour_utc < ?",
            params![cutoff_str],
        )?;

        tracing::info!(
            "Gemaal_Debiet_Per_Uur: opschonen tot {} dagen behouden ({} verwijderd)",
            days,
            count
        );
        Ok(count as u64)
    }

    /// Lees alle gemaal snapshots.
    pub fn get_all_snapshots(&self) -> anyhow::Result<Vec<GemaalSnapshot>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT gemaal_code, status, debiet, last_update, generated_at, trends_json
            FROM gemaal_status_snapshot
            ORDER BY gemaal_code
            "#,
        )?;

        let mut snapshots = Vec::new();
        let rows = stmt.query_map([], |row| {
            let gemaal_code: String = row.get(0)?;
            let status_str: String = row.get(1)?;
            let debiet: f64 = row.get(2)?;
            let last_update: Option<String> = row.get(3)?;
            let generated_at: Option<String> = row.get(4)?;
            let trends_json: Option<String> = row.get(5)?;

            Ok((
                gemaal_code,
                status_str,
                debiet,
                last_update,
                generated_at,
                trends_json,
            ))
        })?;

        for row in rows {
            let (gemaal_code, status_str, debiet, last_update, generated_at, trends_json) = row?;

            let trends: Option<GemaalTrends> =
                trends_json.and_then(|json| serde_json::from_str(&json).ok());

            snapshots.push(GemaalSnapshot {
                gemaal_code,
                status: GemaalStatus::from_str_loose(&status_str),
                debiet,
                last_update: parse_optional_datetime(last_update),
                generated_at: parse_optional_datetime(generated_at),
                trends,
                error: None,
            });
        }

        Ok(snapshots)
    }

    /// Lees een specifiek gemaal snapshot.
    pub fn get_snapshot(&self, gemaal_code: &str) -> anyhow::Result<Option<GemaalSnapshot>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT gemaal_code, status, debiet, last_update, generated_at, trends_json
            FROM gemaal_status_snapshot
            WHERE gemaal_code = ?
            "#,
        )?;

        let result = stmt.query_row(params![gemaal_code], |row| {
            let gemaal_code: String = row.get(0)?;
            let status_str: String = row.get(1)?;
            let debiet: f64 = row.get(2)?;
            let last_update: Option<String> = row.get(3)?;
            let generated_at: Option<String> = row.get(4)?;
            let trends_json: Option<String> = row.get(5)?;

            Ok((
                gemaal_code,
                status_str,
                debiet,
                last_update,
                generated_at,
                trends_json,
            ))
        });

        match result {
            Ok((gemaal_code, status_str, debiet, last_update, generated_at, trends_json)) => {
                let trends: Option<GemaalTrends> =
                    trends_json.and_then(|json| serde_json::from_str(&json).ok());

                Ok(Some(GemaalSnapshot {
                    gemaal_code,
                    status: GemaalStatus::from_str_loose(&status_str),
                    debiet,
                    last_update: parse_optional_datetime(last_update),
                    generated_at: parse_optional_datetime(generated_at),
                    trends,
                    error: None,
                }))
            }
            Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
