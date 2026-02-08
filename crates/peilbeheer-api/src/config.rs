use std::env;

/// Server configuratie.
#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_path: String,
    pub hydronet_chart_id: String,
}

impl Config {
    /// Laad configuratie uit omgevingsvariabelen.
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()?,
            database_path: env::var("DATABASE_PATH")
                .unwrap_or_else(|_| "data/peilbeheer.duckdb".to_string()),
            hydronet_chart_id: env::var("HYDRONET_CHART_ID").unwrap_or_else(|_| {
                "e743fb87-2a02-4f3e-ac6c-03d03401aab8".to_string()
            }),
        })
    }
}
