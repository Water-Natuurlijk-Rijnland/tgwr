use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use chrono::{DateTime, NaiveDateTime, Utc};
use duckdb::{params, Connection};

use peilbeheer_core::asset::AssetRegistratie;
use peilbeheer_core::gemaal::{GemaalSnapshot, GemaalStatus, GemaalTrends};
use peilbeheer_core::hydronet::GeoJsonGemaal;
use peilbeheer_core::peilgebied::PeilgebiedInfo;

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
    cached_peilgebieden_geojson: Mutex<Option<String>>,
}

impl Database {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;

        conn.execute_batch("INSTALL spatial; LOAD spatial;")?;
        tracing::info!("Spatial extension loaded");

        Ok(Self {
            conn: Mutex::new(conn),
            cached_peilgebieden_geojson: Mutex::new(None),
        })
    }

    /// Check if a table exists in the database.
    pub fn table_exists(&self, table_name: &str) -> bool {
        let conn = self.conn.lock().unwrap();
        // Try to query the table - if it fails, it doesn't exist
        let result = conn.prepare(&format!("SELECT 1 FROM {}", table_name));
        result.is_ok()
    }

    /// Initialiseer het database schema vanuit de migratie SQL-bestanden.
    pub fn initialize_schema(&self) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();

        let migrations = &[
            include_str!("../../../migrations/001_initial_schema.sql"),
            include_str!("../../../migrations/002_asset_registratie.sql"),
            include_str!("../../../migrations/003_peilgebieden.sql"),
            include_str!("../../../migrations/004_scenarios.sql"),
            include_str!("../../../migrations/005_scenario_results.sql"),
            include_str!("../../../migrations/006_users.sql"),
            include_str!("../../../migrations/007_alerts.sql"),
            include_str!("../../../migrations/008_timeseries.sql"),
        ];

        for schema in migrations {
            for statement in schema.split(';') {
                // Strip comment lines before checking if statement is empty
                let stmt: String = statement
                    .lines()
                    .filter(|line| !line.trim_start().starts_with("--"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let stmt = stmt.trim();
                if !stmt.is_empty()
                    && let Err(e) = conn.execute(stmt, []) {
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

    /// Schrijf gemaal registraties (bulk upsert).
    pub fn write_gemaal_registraties(&self, gemalen: &[GeoJsonGemaal]) -> anyhow::Result<usize> {
        let conn = self.conn.lock().unwrap();
        let now = datetime_to_string(&Utc::now());
        let mut count = 0;

        for g in gemalen {
            conn.execute(
                r#"
                INSERT INTO gemaal_registratie (code, naam, latitude, longitude, capaciteit, functie, soort, plaats, gemeente, fetched_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (code) DO UPDATE SET
                    naam = excluded.naam,
                    latitude = excluded.latitude,
                    longitude = excluded.longitude,
                    capaciteit = excluded.capaciteit,
                    functie = excluded.functie,
                    soort = excluded.soort,
                    plaats = excluded.plaats,
                    gemeente = excluded.gemeente,
                    fetched_at = excluded.fetched_at
                "#,
                params![
                    g.code,
                    g.naam,
                    g.lat,
                    g.lon,
                    g.capaciteit,
                    g.functie,
                    g.soort,
                    g.plaats,
                    g.gemeente,
                    now,
                ],
            )?;
            count += 1;
        }

        tracing::info!("Gemaal registraties geschreven: {count}");
        Ok(count)
    }

    /// Lees alle gemaal registraties.
    pub fn get_all_registraties(&self) -> anyhow::Result<Vec<GeoJsonGemaal>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT code, naam, latitude, longitude, capaciteit, functie, soort, plaats, gemeente FROM gemaal_registratie ORDER BY code",
        )?;

        let mut gemalen = Vec::new();
        let rows = stmt.query_map([], |row| {
            Ok(GeoJsonGemaal {
                code: row.get(0)?,
                naam: row.get(1)?,
                lat: row.get(2)?,
                lon: row.get(3)?,
                capaciteit: row.get(4)?,
                functie: row.get(5)?,
                soort: row.get(6)?,
                plaats: row.get(7)?,
                gemeente: row.get(8)?,
            })
        })?;

        for row in rows {
            gemalen.push(row?);
        }

        Ok(gemalen)
    }

    /// Tel het aantal gemaal registraties.
    /// Returns 0 if the table doesn't exist yet.
    pub fn get_registratie_count(&self) -> anyhow::Result<usize> {
        let conn = self.conn.lock().unwrap();

        // Try to get count, return 0 if table doesn't exist or query fails
        let result: Result<i64, _> = conn.query_row(
            "SELECT COUNT(*) FROM gemaal_registratie",
            [],
            |row| row.get(0),
        );
        Ok(result.unwrap_or(0) as usize)
    }

    /// Schrijf asset registraties (bulk upsert).
    pub fn write_asset_registraties(&self, assets: &[AssetRegistratie]) -> anyhow::Result<usize> {
        let conn = self.conn.lock().unwrap();
        let now = datetime_to_string(&Utc::now());
        let mut count = 0;

        for a in assets {
            let extra = a
                .extra_properties
                .as_ref()
                .map(|v| serde_json::to_string(v).unwrap_or_default());

            conn.execute(
                r#"
                INSERT INTO asset_registratie (layer_type, code, naam, latitude, longitude, extra_properties, fetched_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (layer_type, code) DO UPDATE SET
                    naam = excluded.naam,
                    latitude = excluded.latitude,
                    longitude = excluded.longitude,
                    extra_properties = excluded.extra_properties,
                    fetched_at = excluded.fetched_at
                "#,
                params![a.layer_type, a.code, a.naam, a.lat, a.lon, extra, now],
            )?;
            count += 1;
        }

        tracing::info!("Asset registraties geschreven: {count} ({})", assets.first().map(|a| a.layer_type.as_str()).unwrap_or("-"));
        Ok(count)
    }

    /// Lees assets per laagtype.
    pub fn get_assets_by_layer(&self, layer_type: &str) -> anyhow::Result<Vec<AssetRegistratie>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT layer_type, code, naam, latitude, longitude, extra_properties FROM asset_registratie WHERE layer_type = ? ORDER BY code",
        )?;

        let mut assets = Vec::new();
        let rows = stmt.query_map(params![layer_type], |row| {
            let extra_str: Option<String> = row.get(5)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<f64>>(3)?,
                row.get::<_, Option<f64>>(4)?,
                extra_str,
            ))
        })?;

        for row in rows {
            let (layer_type, code, naam, lat, lon, extra_str) = row?;
            let extra_properties = extra_str.and_then(|s| serde_json::from_str(&s).ok());
            assets.push(AssetRegistratie {
                layer_type,
                code,
                naam,
                lat,
                lon,
                extra_properties,
            });
        }

        Ok(assets)
    }

    /// Lees alle assets (optioneel gefilterd op meerdere laagtypen).
    pub fn get_all_assets(&self, layer_types: Option<&[&str]>) -> anyhow::Result<Vec<AssetRegistratie>> {
        let conn = self.conn.lock().unwrap();

        let (query, filter_types) = match layer_types {
            Some(types) if !types.is_empty() => {
                let placeholders: Vec<&str> = types.iter().map(|_| "?").collect();
                (
                    format!(
                        "SELECT layer_type, code, naam, latitude, longitude, extra_properties FROM asset_registratie WHERE layer_type IN ({}) ORDER BY layer_type, code",
                        placeholders.join(", ")
                    ),
                    Some(types),
                )
            }
            _ => (
                "SELECT layer_type, code, naam, latitude, longitude, extra_properties FROM asset_registratie ORDER BY layer_type, code".to_string(),
                None,
            ),
        };

        let mut stmt = conn.prepare(&query)?;
        let mut assets = Vec::new();

        #[allow(clippy::type_complexity)]
        let row_mapper = |row: &duckdb::Row| -> duckdb::Result<(String, String, Option<String>, Option<f64>, Option<f64>, Option<String>)> {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        };

        if let Some(types) = filter_types {
            let params: Vec<&dyn duckdb::ToSql> = types.iter().map(|t| t as &dyn duckdb::ToSql).collect();
            let rows = stmt.query_map(params.as_slice(), row_mapper)?;
            for row in rows {
                let (layer_type, code, naam, lat, lon, extra_str) = row?;
                let extra_properties: Option<serde_json::Value> = extra_str.and_then(|s| serde_json::from_str(&s).ok());
                assets.push(AssetRegistratie { layer_type, code, naam, lat, lon, extra_properties });
            }
        } else {
            let rows = stmt.query_map([], row_mapper)?;
            for row in rows {
                let (layer_type, code, naam, lat, lon, extra_str) = row?;
                let extra_properties: Option<serde_json::Value> = extra_str.and_then(|s| serde_json::from_str(&s).ok());
                assets.push(AssetRegistratie { layer_type, code, naam, lat, lon, extra_properties });
            }
        }

        Ok(assets)
    }

    /// Tel het totaal aantal asset registraties.
    /// Returns 0 if the table doesn't exist yet.
    pub fn get_total_asset_count(&self) -> anyhow::Result<usize> {
        let conn = self.conn.lock().unwrap();

        // Try to get count, return 0 if table doesn't exist or query fails
        let result: Result<i64, _> = conn.query_row(
            "SELECT COUNT(*) FROM asset_registratie",
            [],
            |row| row.get(0),
        );
        Ok(result.unwrap_or(0) as usize)
    }

    // ── Peilgebieden ──

    /// Tel het aantal peilgebieden.
    pub fn get_peilgebied_count(&self) -> anyhow::Result<usize> {
        let conn = self.conn.lock().unwrap();
        // Try to get count, return 0 if table doesn't exist or query fails
        let result: Result<i64, _> =
            conn.query_row("SELECT COUNT(*) FROM peilgebied", [], |row| row.get(0));
        Ok(result.unwrap_or(0) as usize)
    }

    /// Laad peilgebieden vanuit een GeoJSON-bestand via ST_Read.
    pub fn load_peilgebieden_from_geojson(&self, path: &str) -> anyhow::Result<usize> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            r#"
            INSERT INTO peilgebied
            SELECT CODE, NAAM, ZOMERPEIL, WINTERPEIL, VASTPEIL, OPPERVLAKTE,
                   SOORTAFWATERING, SOORTPEILGEBIED, geom
            FROM ST_Read(?)
            "#,
            params![path],
        )?;

        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM peilgebied", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Herlaad peilgebieden: leeg tabel, laad opnieuw vanuit GeoJSON, invalideer cache.
    pub fn reload_peilgebieden_from_geojson(&self, path: &str) -> anyhow::Result<usize> {
        {
            let conn = self.conn.lock().unwrap();
            conn.execute("DELETE FROM peilgebied", [])?;
        }
        let count = self.load_peilgebieden_from_geojson(path)?;
        // Invalideer de cache zodat het volgende GET verse data teruggeeft
        let mut cache = self.cached_peilgebieden_geojson.lock().unwrap();
        *cache = None;
        Ok(count)
    }

    /// Alle peilgebieden als GeoJSON FeatureCollection string (cached).
    pub fn get_all_peilgebieden_geojson(&self) -> anyhow::Result<String> {
        let mut cache = self.cached_peilgebieden_geojson.lock().unwrap();
        if let Some(ref cached) = *cache {
            return Ok(cached.clone());
        }
        let geojson = self.build_peilgebieden_geojson()?;
        *cache = Some(geojson.clone());
        Ok(geojson)
    }

    fn build_peilgebieden_geojson(&self) -> anyhow::Result<String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT code, naam, zomerpeil, winterpeil, vastpeil, oppervlakte,
                   soortafwatering, soortpeilgebied, ST_AsGeoJSON(geometry) AS geojson
            FROM peilgebied
            ORDER BY code
            "#,
        )?;

        let mut features = Vec::new();
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<f64>>(2)?,
                row.get::<_, Option<f64>>(3)?,
                row.get::<_, Option<f64>>(4)?,
                row.get::<_, Option<f64>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, String>(8)?,
            ))
        })?;

        for row in rows {
            let (code, naam, zomerpeil, winterpeil, vastpeil, oppervlakte, soortafwatering, soortpeilgebied, geojson) = row?;

            let properties = serde_json::json!({
                "CODE": code,
                "NAAM": naam,
                "ZOMERPEIL": zomerpeil,
                "WINTERPEIL": winterpeil,
                "VASTPEIL": vastpeil,
                "OPPERVLAKTE": oppervlakte,
                "SOORTAFWATERING": soortafwatering,
                "SOORTPEILGEBIED": soortpeilgebied,
            });

            let geometry: serde_json::Value = serde_json::from_str(&geojson)?;

            let feature = serde_json::json!({
                "type": "Feature",
                "properties": properties,
                "geometry": geometry,
            });

            features.push(feature);
        }

        let collection = serde_json::json!({
            "type": "FeatureCollection",
            "features": features,
        });

        Ok(serde_json::to_string(&collection)?)
    }

    /// Zoek peilgebied bij een punt (lon, lat).
    #[allow(dead_code)]
    pub fn find_peilgebied_for_point(
        &self,
        lon: f64,
        lat: f64,
    ) -> anyhow::Result<Option<PeilgebiedInfo>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            r#"
            SELECT code, naam, zomerpeil, winterpeil, vastpeil, oppervlakte, soortafwatering
            FROM peilgebied
            WHERE ST_Contains(geometry, ST_Point(?, ?))
            LIMIT 1
            "#,
            params![lon, lat],
            |row| {
                Ok(PeilgebiedInfo {
                    code: row.get(0)?,
                    naam: row.get(1)?,
                    zomerpeil: row.get(2)?,
                    winterpeil: row.get(3)?,
                    vastpeil: row.get(4)?,
                    oppervlakte: row.get(5)?,
                    soortafwatering: row.get(6)?,
                })
            },
        );

        match result {
            Ok(info) => Ok(Some(info)),
            Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Bulk koppeling: gemaal_code → peilgebied_code via spatial join.
    pub fn get_gemaal_peilgebied_mapping(&self) -> anyhow::Result<HashMap<String, String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT g.code, p.code
            FROM gemaal_registratie g, peilgebied p
            WHERE ST_Contains(p.geometry, ST_Point(g.longitude, g.latitude))
            "#,
        )?;

        let mut mapping = HashMap::new();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows {
            let (gemaal_code, peilgebied_code) = row?;
            mapping.insert(gemaal_code, peilgebied_code);
        }

        Ok(mapping)
    }

    // ═══════════════════════════════════════════════════════════════
    // Scenario Management helper methods
    // ═══════════════════════════════════════════════════════════════

    /// Execute a SQL statement with parameters.
    pub fn execute(&self, sql: &str, params: &[&dyn duckdb::ToSql]) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(sql, params)?;
        Ok(())
    }

    /// Query and map rows using a closure.
    pub fn query<T, F>(
        &self,
        sql: &str,
        params: &[&dyn duckdb::ToSql],
        mapper: F,
    ) -> anyhow::Result<Vec<T>>
    where
        F: FnMut(&duckdb::Row<'_>) -> duckdb::Result<T>,
    {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params.as_ref(), mapper)?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Query a single row.
    pub fn query_row<T, F>(
        &self,
        sql: &str,
        params: &[&dyn duckdb::ToSql],
        mapper: F,
    ) -> anyhow::Result<T>
    where
        F: FnOnce(&duckdb::Row<'_>) -> duckdb::Result<T>,
    {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(sql, params, mapper)?;
        Ok(result)
    }
}
