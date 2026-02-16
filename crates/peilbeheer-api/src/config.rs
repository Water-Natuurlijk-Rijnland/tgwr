use std::env;

use serde::{Deserialize, Serialize};

/// Configuratie voor een ArcGIS-laag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcgisLayerConfig {
    pub service_name: String,
    pub layer_id: u32,
    pub display_label: String,
    pub layer_type: String,
    pub icon_svg: String,
    pub color: String,
    pub default_visible: bool,
}

/// Server configuratie.
#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_path: String,
    pub hydronet_chart_id: String,
    pub arcgis_layers: Vec<ArcgisLayerConfig>,
    pub peilgebieden_geojson_path: String,
    pub peilgebieden_arcgis_service: String,
    pub peilgebieden_arcgis_layer_id: u32,
}

impl Config {
    /// Laad configuratie uit omgevingsvariabelen.
    pub fn from_env() -> anyhow::Result<Self> {
        let arcgis_layers = match env::var("ARCGIS_LAYERS") {
            Ok(json) => serde_json::from_str(&json)?,
            Err(_) => default_arcgis_layers(),
        };

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
            arcgis_layers,
            peilgebieden_geojson_path: env::var("PEILGEBIEDEN_GEOJSON_PATH")
                .unwrap_or_else(|_| "data/peilgebieden_rijnland.geojson".to_string()),
            peilgebieden_arcgis_service: env::var("PEILGEBIEDEN_ARCGIS_SERVICE")
                .unwrap_or_else(|_| "Peilgebied_vigerend_besluit".to_string()),
            peilgebieden_arcgis_layer_id: env::var("PEILGEBIEDEN_ARCGIS_LAYER_ID")
                .unwrap_or_else(|_| "0".to_string())
                .parse()
                .unwrap_or(0),
        })
    }
}

fn default_arcgis_layers() -> Vec<ArcgisLayerConfig> {
    vec![
        ArcgisLayerConfig {
            service_name: "Gemaal".to_string(),
            layer_id: 0,
            display_label: "Gemalen".to_string(),
            layer_type: "gemaal".to_string(),
            icon_svg: r##"<svg width="28" height="28" viewBox="0 0 28 28"><circle cx="14" cy="14" r="12" fill="#1a5276" stroke="white" stroke-width="2"/><path d="M9 17v-3a5 5 0 0 1 10 0v3" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round"/><line x1="14" y1="9" x2="14" y2="12" stroke="white" stroke-width="1.8" stroke-linecap="round"/><line x1="10" y1="17" x2="18" y2="17" stroke="white" stroke-width="1.8" stroke-linecap="round"/></svg>"##.to_string(),
            color: "#1a5276".to_string(),
            default_visible: true,
        },
        ArcgisLayerConfig {
            service_name: "Stuw".to_string(),
            layer_id: 0,
            display_label: "Stuwen".to_string(),
            layer_type: "stuw".to_string(),
            icon_svg: r##"<svg width="28" height="28" viewBox="0 0 28 28"><circle cx="14" cy="14" r="12" fill="#8e44ad" stroke="white" stroke-width="2"/><rect x="9" y="10" width="10" height="8" rx="1" fill="none" stroke="white" stroke-width="1.8"/><line x1="9" y1="14" x2="19" y2="14" stroke="white" stroke-width="1.8"/></svg>"##.to_string(),
            color: "#8e44ad".to_string(),
            default_visible: true,
        },
        ArcgisLayerConfig {
            service_name: "Sluis".to_string(),
            layer_id: 0,
            display_label: "Sluizen".to_string(),
            layer_type: "sluis".to_string(),
            icon_svg: r##"<svg width="28" height="28" viewBox="0 0 28 28"><circle cx="14" cy="14" r="12" fill="#2980b9" stroke="white" stroke-width="2"/><rect x="8" y="11" width="5" height="6" fill="none" stroke="white" stroke-width="1.5"/><rect x="15" y="11" width="5" height="6" fill="none" stroke="white" stroke-width="1.5"/><line x1="13" y1="13" x2="15" y2="13" stroke="white" stroke-width="1.5"/></svg>"##.to_string(),
            color: "#2980b9".to_string(),
            default_visible: true,
        },
        ArcgisLayerConfig {
            service_name: "Inlaat".to_string(),
            layer_id: 0,
            display_label: "Inlaten".to_string(),
            layer_type: "inlaat".to_string(),
            icon_svg: r##"<svg width="28" height="28" viewBox="0 0 28 28"><circle cx="14" cy="14" r="12" fill="#27ae60" stroke="white" stroke-width="2"/><path d="M10 14h8M15 11l3 3-3 3" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>"##.to_string(),
            color: "#27ae60".to_string(),
            default_visible: false,
        },
        ArcgisLayerConfig {
            service_name: "Duiker".to_string(),
            layer_id: 0,
            display_label: "Duikers".to_string(),
            layer_type: "duiker".to_string(),
            icon_svg: r##"<svg width="28" height="28" viewBox="0 0 28 28"><circle cx="14" cy="14" r="12" fill="#d35400" stroke="white" stroke-width="2"/><ellipse cx="14" cy="14" rx="5" ry="3" fill="none" stroke="white" stroke-width="1.8"/></svg>"##.to_string(),
            color: "#d35400".to_string(),
            default_visible: false,
        },
        ArcgisLayerConfig {
            service_name: "Dam".to_string(),
            layer_id: 0,
            display_label: "Dammen".to_string(),
            layer_type: "dam".to_string(),
            icon_svg: r##"<svg width="28" height="28" viewBox="0 0 28 28"><circle cx="14" cy="14" r="12" fill="#7f8c8d" stroke="white" stroke-width="2"/><line x1="8" y1="14" x2="20" y2="14" stroke="white" stroke-width="2.5" stroke-linecap="round"/><line x1="14" y1="10" x2="14" y2="18" stroke="white" stroke-width="1.5" stroke-linecap="round"/></svg>"##.to_string(),
            color: "#7f8c8d".to_string(),
            default_visible: false,
        },
    ]
}
