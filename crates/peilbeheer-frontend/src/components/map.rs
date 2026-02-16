use std::collections::HashMap;

use dioxus::prelude::*;
use serde::Deserialize;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use crate::api::{
    self, AssetFeature, AssetFeatureCollection, GemaalDetailResponse, LayerConfig,
};

/// Geselecteerd asset voor het zijpaneel.
#[derive(Debug, Clone, PartialEq, Deserialize)]
struct SelectedAsset {
    code: String,
    naam: String,
    layer_type: String,
    display_label: String,
    color: String,
    extra_properties: Option<serde_json::Value>,
}

impl SelectedAsset {
    fn from_feature(f: &AssetFeature) -> Self {
        Self {
            code: f.properties.code.clone(),
            naam: f
                .properties
                .naam
                .clone()
                .unwrap_or_else(|| "Onbekend".to_string()),
            layer_type: f.properties.layer_type.clone(),
            display_label: f.properties.display_label.clone(),
            color: f.properties.color.clone(),
            extra_properties: f.properties.extra_properties.clone(),
        }
    }
}

/// Build a legend item HTML string for one layer.
fn legend_item_html(layer: &LayerConfig, count: usize, visible: bool) -> String {
    let checked = if visible { "checked" } else { "" };
    let icon_display = format!(
        r#"<span style="display:inline-flex;width:18px;height:18px;align-items:center;justify-content:center;flex-shrink:0">{}</span>"#,
        layer.icon_svg
    );

    format!(
        r#"<label class="legend-item" style="display:flex;align-items:center;gap:6px;cursor:pointer;padding:3px 4px;border-radius:4px;font-size:12px" onmouseover="this.style.background='rgba(0,0,0,0.04)'" onmouseout="this.style.background='none'">
  <input type="checkbox" {checked} onchange="window._peilbeheerToggleLayer('{lt}',this.checked)" style="width:14px;height:14px;cursor:pointer;accent-color:{color}" />
  {icon_display}
  <span style="flex:1;color:#374151;font-weight:500">{label}</span>
  <span style="font-size:10px;color:#9ca3af;background:#f3f4f6;padding:1px 6px;border-radius:8px;font-weight:600">{count}</span>
</label>"#,
        lt = layer.layer_type,
        color = layer.color,
        label = layer.display_label,
    )
}

fn build_map_js(
    assets: &AssetFeatureCollection,
    layers: &[LayerConfig],
    visibility: &HashMap<String, bool>,
    peilgebieden_geojson: &Option<String>,
) -> String {
    // Group features by layer_type
    let mut layer_markers: HashMap<&str, Vec<String>> = HashMap::new();
    let mut layer_counts: HashMap<&str, usize> = HashMap::new();

    for feature in &assets.features {
        if feature.geometry.coordinates.len() < 2 {
            continue;
        }
        let lon = feature.geometry.coordinates[0];
        let lat = feature.geometry.coordinates[1];
        let code = &feature.properties.code;
        let layer_type = &feature.properties.layer_type;
        let naam = feature
            .properties
            .naam
            .as_deref()
            .unwrap_or("Onbekend")
            .replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        let display_label = feature
            .properties
            .display_label
            .replace('\\', "\\\\")
            .replace('\'', "\\'");
        let icon_svg = feature
            .properties
            .icon_svg
            .replace('\\', "\\\\")
            .replace('\'', "\\'");

        let marker_js = format!(
            r#"(function() {{
                var icon = L.divIcon({{
                    html: '{icon_svg}',
                    className: 'asset-icon',
                    iconSize: [28, 28],
                    iconAnchor: [14, 14]
                }});
                var m = L.marker([{lat}, {lon}], {{ icon: icon }});
                m.bindTooltip('<b>{naam}</b><br>{display_label}', {{ direction: 'top', offset: [0, -10] }});
                m.on('click', function() {{
                    window._peilbeheerSelectAsset('{code}');
                }});
                return m;
            }})()"#
        );

        layer_markers
            .entry(layer_type.as_str())
            .or_default()
            .push(marker_js);
        *layer_counts.entry(layer_type.as_str()).or_insert(0) += 1;
    }

    // Build layer groups JS
    let mut layer_group_js = String::new();
    for layer in layers {
        let markers = layer_markers
            .get(layer.layer_type.as_str())
            .cloned()
            .unwrap_or_default();

        let visible = visibility
            .get(&layer.layer_type)
            .copied()
            .unwrap_or(layer.default_visible);

        layer_group_js.push_str(&format!(
            r#"
            (function() {{
                var markers = [{}];
                var group = L.layerGroup(markers);
                window._peilbeheerLayers['{}'] = group;
                if ({}) {{
                    group.addTo(map);
                }}
            }})();
            "#,
            markers.join(",\n"),
            layer.layer_type,
            if visible { "true" } else { "false" },
        ));
    }

    // Build legend HTML
    let mut legend_items = String::new();
    let total: usize = layer_counts.values().sum();
    let visible_count = layers
        .iter()
        .filter(|l| {
            visibility
                .get(&l.layer_type)
                .copied()
                .unwrap_or(l.default_visible)
        })
        .count()
        + if peilgebieden_geojson.is_some() { 1 } else { 0 };

    for layer in layers {
        let count = layer_counts
            .get(layer.layer_type.as_str())
            .copied()
            .unwrap_or(0);
        let visible = visibility
            .get(&layer.layer_type)
            .copied()
            .unwrap_or(layer.default_visible);
        legend_items.push_str(&legend_item_html(layer, count, visible));
    }

    // Peilgebieden polygon layer JS + legend entry
    // Note: the actual GeoJSON data is pre-set on window._kaartPgData by Rust
    // to avoid embedding the huge (16 MB) JSON in the eval string.
    let peilgebieden_js = if peilgebieden_geojson.is_some() {
        let pg_count = peilgebieden_geojson.as_ref()
            .map(|g| g.matches("\"type\":\"Feature\"").count())
            .unwrap_or(0);
        legend_items.push_str(&format!(
            r##"<label class="legend-item" style="display:flex;align-items:center;gap:6px;cursor:pointer;padding:3px 4px;border-radius:4px;font-size:12px" onmouseover="this.style.background='rgba(0,0,0,0.04)'" onmouseout="this.style.background='none'">
  <input type="checkbox" checked onchange="window._peilbeheerToggleLayer('peilgebieden',this.checked)" style="width:14px;height:14px;cursor:pointer;accent-color:#3b82f6" />
  <span style="display:inline-flex;width:18px;height:18px;align-items:center;justify-content:center;flex-shrink:0"><svg width="16" height="16" viewBox="0 0 16 16"><rect x="1" y="1" width="14" height="14" rx="2" fill="#3b82f6" fill-opacity="0.3" stroke="#2563eb" stroke-width="1.5"/></svg></span>
  <span style="flex:1;color:#374151;font-weight:500">Peilgebieden</span>
  <span style="font-size:10px;color:#9ca3af;background:#f3f4f6;padding:1px 6px;border-radius:8px;font-weight:600">{pg_count}</span>
</label>"##
        ));

        r#"
            (function() {
                var peilgebiedenData = window._kaartPgData;
                if (!peilgebiedenData) return;
                var peilgebiedenGroup = L.geoJSON(peilgebiedenData, {
                    style: function(feature) {
                        return {
                            color: '#2563eb',
                            weight: 1.5,
                            fillColor: '#3b82f6',
                            fillOpacity: 0.15
                        };
                    },
                    onEachFeature: function(feature, layer) {
                        var p = feature.properties;
                        var naam = p.NAAM || 'Onbekend';
                        var code = p.CODE || '';
                        var peil = '';
                        if (p.VASTPEIL != null) peil = 'Vastpeil: ' + p.VASTPEIL.toFixed(2) + ' m';
                        else {
                            var parts = [];
                            if (p.ZOMERPEIL != null) parts.push('Zomer: ' + p.ZOMERPEIL.toFixed(2));
                            if (p.WINTERPEIL != null) parts.push('Winter: ' + p.WINTERPEIL.toFixed(2));
                            if (parts.length) peil = parts.join(' / ') + ' m';
                        }
                        layer.bindTooltip('<b>' + naam + '</b><br>' + code + (peil ? '<br>' + peil : ''));
                    }
                });
                window._peilbeheerLayers['peilgebieden'] = peilgebiedenGroup;
                peilgebiedenGroup.addTo(map);
            })();
        "#.to_string()
    } else {
        String::new()
    };

    let total_layers = layers.len() + if peilgebieden_geojson.is_some() { 1 } else { 0 };

    let legend_html = format!(
        r##"<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:8px;padding-bottom:8px;border-bottom:1px solid #e5e7eb">
  <h4 style="font-size:14px;font-weight:700;color:#1f2937;margin:0">Legenda</h4>
  <button onclick="var c=this.parentElement.nextElementSibling;var s=this.parentElement.nextElementSibling.nextElementSibling;if(c.style.display==='none'){{c.style.display='block';s.style.display='block';this.textContent='\u2212'}}else{{c.style.display='none';s.style.display='none';this.textContent='+'}}" style="background:none;border:none;font-size:18px;color:#9ca3af;cursor:pointer;padding:0 4px;line-height:1">&minus;</button>
</div>
<div class="legend-items" style="display:flex;flex-direction:column;gap:1px">{legend_items}</div>
<div class="legend-summary" style="margin-top:10px;padding-top:8px;border-top:1px solid #e5e7eb;font-size:11px;color:#6b7280">
  <div>Zichtbaar: <strong>{visible_count}</strong> / {total_layers}</div>
  <div>Objecten: <strong>{total}</strong></div>
</div>"##,
    );

    // Escape for JS string
    let legend_html_escaped = legend_html
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n");

    format!(
        r#"
        setTimeout(function() {{
            try {{
                if (window._peilbeheerMap) {{
                    window._peilbeheerMap.remove();
                    window._peilbeheerMap = null;
                }}
                var el = document.getElementById('gemalen-map');
                if (!el) {{ console.error('gemalen-map not found'); return; }}

                var map = L.map('gemalen-map').setView([52.16, 4.49], 11);
                L.tileLayer('https://{{s}}.tile.openstreetmap.org/{{z}}/{{x}}/{{y}}.png', {{
                    attribution: '&copy; OpenStreetMap',
                    maxZoom: 18
                }}).addTo(map);

                window._peilbeheerLayers = {{}};
                window._peilbeheerMap = map;

                window._peilbeheerToggleLayer = function(layerType, visible) {{
                    var group = window._peilbeheerLayers[layerType];
                    if (!group) return;
                    if (visible) {{
                        group.addTo(map);
                    }} else {{
                        group.removeFrom(map);
                    }}
                }};

                {layer_group_js}

                {peilgebieden_js}

                // Add legend as Leaflet control
                var LegendControl = L.Control.extend({{
                    options: {{ position: 'topleft' }},
                    onAdd: function(map) {{
                        var div = L.DomUtil.create('div', 'leaflet-legend-control');
                        div.innerHTML = '{legend_html_escaped}';
                        div.style.cssText = 'background:white;border-radius:8px;padding:12px;box-shadow:0 2px 12px rgba(0,0,0,0.15);border:1px solid #e5e7eb;min-width:200px;max-width:280px;max-height:calc(100vh - 10rem);overflow-y:auto;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif';
                        L.DomEvent.disableClickPropagation(div);
                        L.DomEvent.disableScrollPropagation(div);
                        return div;
                    }}
                }});
                new LegendControl().addTo(map);
            }} catch(e) {{
                console.error('Leaflet init fout:', e);
            }}
        }}, 100);
        "#
    )
}

#[component]
pub fn KaartPage() -> Element {
    let layers_res = use_resource(|| api::fetch_layers());
    let assets_res = use_resource(|| async { api::fetch_assets_geojson(None).await });
    let peilgebieden_res = use_resource(|| api::fetch_peilgebieden_geojson());

    // Peilgebieden are optional – don't block the map if they fail,
    // but DO wait until the request has completed (success or error)
    // so the effect gets the final value on first run.
    let peilgebieden_geojson = match &*peilgebieden_res.read() {
        Some(Ok(geojson)) => Some(geojson.clone()),
        Some(Err(_)) => None, // failed, proceed without polygons
        None => {
            // Still loading — show spinner until all three requests have settled
            return rsx! {
                div { class: "kaart-page",
                    div { class: "kaart-loading",
                        div { class: "loading", "Kaart laden..." }
                    }
                }
            };
        }
    };

    match (&*layers_res.read(), &*assets_res.read()) {
        (Some(Ok(layers)), Some(Ok(assets))) => {
            let layers = layers.clone();
            let assets = assets.clone();
            rsx! {
                KaartView { layers, assets, peilgebieden_geojson }
            }
        }
        (Some(Err(e)), _) | (_, Some(Err(e))) => rsx! {
            div { class: "kaart-page",
                div { class: "kaart-loading",
                    div { class: "error-message", "Kaart laden mislukt: {e}" }
                }
            }
        },
        _ => rsx! {
            div { class: "kaart-page",
                div { class: "kaart-loading",
                    div { class: "loading", "Kaart laden..." }
                }
            }
        },
    }
}

#[component]
fn KaartView(layers: Vec<LayerConfig>, assets: AssetFeatureCollection, peilgebieden_geojson: Option<String>) -> Element {
    let total_count = assets.features.len();
    let mut selected = use_signal::<Option<SelectedAsset>>(|| None);

    // Initial visibility from layer defaults
    let initial_vis: HashMap<String, bool> = layers
        .iter()
        .map(|l| (l.layer_type.clone(), l.default_visible))
        .collect();

    // Set up JS->Rust callback for marker clicks and build Leaflet map once
    use_effect(move || {
        let features = assets.features.clone();

        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |code: String| {
            if let Some(f) = features.iter().find(|f| f.properties.code == code) {
                selected.set(Some(SelectedAsset::from_feature(f)));
            }
        }) as Box<dyn FnMut(String)>);

        let window = web_sys::window().unwrap();
        js_sys::Reflect::set(
            &window,
            &JsValue::from_str("_peilbeheerSelectAsset"),
            closure.as_ref().unchecked_ref(),
        )
        .ok();
        closure.forget();

        // Parse peilgebieden GeoJSON to JS object and set on window
        // (avoids embedding 16 MB JSON in eval string)
        if let Some(ref pg_json) = peilgebieden_geojson {
            if let Ok(js_val) = js_sys::JSON::parse(pg_json) {
                js_sys::Reflect::set(&window, &JsValue::from_str("_kaartPgData"), &js_val).ok();
            }
        } else {
            js_sys::Reflect::set(&window, &JsValue::from_str("_kaartPgData"), &JsValue::NULL).ok();
        }

        let js = build_map_js(&assets, &layers, &initial_vis, &peilgebieden_geojson);
        let _ = js_sys::eval(&js);
    });

    let sel = selected.read();

    rsx! {
        div { class: "kaart-page",
            div {
                id: "gemalen-map",
                class: "kaart-map",
            }

            div { class: "kaart-count-badge",
                "{total_count} objecten"
            }

            if let Some(asset) = &*sel {
                AssetDetailPanel {
                    key: "{asset.code}-{asset.layer_type}",
                    asset: asset.clone(),
                    on_close: move |_| selected.set(None),
                }
            }
        }
    }
}

#[component]
fn AssetDetailPanel(asset: SelectedAsset, on_close: EventHandler) -> Element {
    let is_gemaal = asset.layer_type == "gemaal";
    let code = asset.code.clone();

    let live = if is_gemaal {
        Some(use_resource(move || {
            let code = code.clone();
            async move { api::fetch_gemaal(&code).await }
        }))
    } else {
        None
    };

    rsx! {
        div { class: "kaart-panel",
            div { class: "kaart-panel-header",
                div {
                    h3 { "{asset.naam}" }
                    span {
                        class: "kaart-panel-type",
                        style: "color: {asset.color};",
                        "{asset.display_label}"
                    }
                }
                button {
                    class: "kaart-panel-close",
                    onclick: move |_| on_close.call(()),
                    "\u{00D7}"
                }
            }
            div { class: "kaart-panel-body",
                ExtraPropertiesSection {
                    code: asset.code.clone(),
                    properties: asset.extra_properties.clone(),
                }
            }

            // Live data section for gemalen
            if is_gemaal {
                div { class: "kaart-panel-live",
                    match live.as_ref().map(|l| l.read().clone()) {
                        Some(Some(Ok(detail))) => rsx! { LiveDataSection { detail } },
                        Some(Some(Err(_))) => rsx! {
                            div { class: "kaart-panel-status", "Geen live data beschikbaar" }
                        },
                        _ => rsx! {
                            div { class: "kaart-panel-status", "Live data laden..." }
                        },
                    }
                }
            }
        }
    }
}

/// Tijdvenster voor de grafiek.
#[derive(Debug, Clone, Copy, PartialEq)]
enum TimeWindow {
    ThreeHours,
    SevenDays,
}

#[component]
fn LiveDataSection(detail: GemaalDetailResponse) -> Element {
    let live = match &detail.live_data {
        Some(ld) if !ld.series.is_empty() && !ld.series[0].data.is_empty() => ld,
        _ => {
            return rsx! {
                div { class: "kaart-panel-status", "Geen meetdata beschikbaar" }
            }
        }
    };

    let mut window = use_signal(|| TimeWindow::ThreeHours);

    let series = &live.series[0];
    let all_data = &series.data;

    let last = &all_data[all_data.len() - 1];
    let current_debiet = last.value;
    let status = if current_debiet > 0.001 { "AAN" } else { "UIT" };
    let status_class = if current_debiet > 0.001 {
        "kaart-status-aan"
    } else {
        "kaart-status-uit"
    };

    let last_time = chrono::DateTime::from_timestamp_millis(last.timestamp_ms)
        .map(|dt| dt.format("%d-%m-%Y %H:%M").to_string())
        .unwrap_or_default();

    let last_ts = last.timestamp_ms;
    let win = *window.read();

    let (chart_points, tick_labels) = match win {
        TimeWindow::ThreeHours => {
            let cutoff = last_ts - 3 * 3600 * 1000;
            let filtered: Vec<_> = all_data.iter().filter(|d| d.timestamp_ms >= cutoff).collect();
            let points: Vec<(f64, i64)> =
                filtered.iter().map(|d| (d.value, d.timestamp_ms)).collect();
            let ticks = make_tick_labels(&points, 4, "%H:%M");
            (points, ticks)
        }
        TimeWindow::SevenDays => {
            let cutoff = last_ts - 7 * 24 * 3600 * 1000;
            let filtered: Vec<_> = all_data.iter().filter(|d| d.timestamp_ms >= cutoff).collect();
            let hourly = hourly_averages(&filtered);
            let ticks = make_tick_labels(&hourly, 5, "%d/%m");
            (hourly, ticks)
        }
    };

    let max_debiet = chart_points
        .iter()
        .map(|(v, _)| *v)
        .fold(0.0f64, f64::max);
    let nonzero: Vec<f64> = chart_points
        .iter()
        .map(|(v, _)| *v)
        .filter(|v| *v > 0.0)
        .collect();
    let avg_debiet = if nonzero.is_empty() {
        0.0
    } else {
        nonzero.iter().sum::<f64>() / nonzero.len() as f64
    };

    let sparkline = build_sparkline(&chart_points, 300, 80);

    let active_3h = if win == TimeWindow::ThreeHours {
        " active"
    } else {
        ""
    };
    let active_7d = if win == TimeWindow::SevenDays {
        " active"
    } else {
        ""
    };

    rsx! {
        div { class: "kaart-live-header",
            span { class: "kaart-status-badge {status_class}", "{status}" }
            span { class: "kaart-live-debiet", "{current_debiet:.3} m\u{00B3}/s" }
        }
        div { class: "kaart-live-time", "Laatste meting: {last_time}" }

        div { class: "kaart-live-stats",
            div { class: "kaart-live-stat",
                span { class: "kaart-live-stat-label", "Max" }
                span { class: "kaart-live-stat-value", "{max_debiet:.3}" }
            }
            div { class: "kaart-live-stat",
                span { class: "kaart-live-stat-label", "Gem" }
                span { class: "kaart-live-stat-value", "{avg_debiet:.3}" }
            }
        }

        div { class: "kaart-toggle",
            button {
                class: "kaart-toggle-btn{active_3h}",
                onclick: move |_| window.set(TimeWindow::ThreeHours),
                "3 uur"
            }
            button {
                class: "kaart-toggle-btn{active_7d}",
                onclick: move |_| window.set(TimeWindow::SevenDays),
                "7 dagen"
            }
        }

        if !chart_points.is_empty() {
            div { class: "kaart-chart",
                dangerous_inner_html: "{sparkline}",
            }
            div { class: "kaart-chart-ticks",
                for label in &tick_labels {
                    span { "{label}" }
                }
            }
        }
    }
}

fn hourly_averages(data: &[&api::LiveDataPoint]) -> Vec<(f64, i64)> {
    use std::collections::BTreeMap;
    let mut buckets: BTreeMap<i64, (f64, usize)> = BTreeMap::new();

    for d in data {
        let hour = d.timestamp_ms / (3600 * 1000) * (3600 * 1000);
        let entry = buckets.entry(hour).or_insert((0.0, 0));
        entry.0 += d.value;
        entry.1 += 1;
    }

    buckets
        .into_iter()
        .map(|(ts, (sum, count))| (sum / count as f64, ts))
        .collect()
}

fn make_tick_labels(points: &[(f64, i64)], n: usize, fmt: &str) -> Vec<String> {
    if points.is_empty() || n == 0 {
        return vec![];
    }
    let len = points.len();
    let step = if len <= n { 1 } else { len / (n - 1) };
    let mut labels = Vec::new();
    let mut i = 0;
    while i < len && labels.len() < n - 1 {
        let ts = points[i].1;
        let label = chrono::DateTime::from_timestamp_millis(ts)
            .map(|dt| dt.format(fmt).to_string())
            .unwrap_or_default();
        labels.push(label);
        i += step;
    }
    let last_label = chrono::DateTime::from_timestamp_millis(points[len - 1].1)
        .map(|dt| dt.format(fmt).to_string())
        .unwrap_or_default();
    labels.push(last_label);
    labels
}

fn build_sparkline(points: &[(f64, i64)], width: u32, height: u32) -> String {
    if points.is_empty() {
        return String::new();
    }

    let pad = 4.0;
    let w = width as f64 - 2.0 * pad;
    let h = height as f64 - 2.0 * pad;

    let max_val = points
        .iter()
        .map(|(v, _)| *v)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_val = points
        .iter()
        .map(|(v, _)| *v)
        .fold(f64::INFINITY, f64::min);
    let range = if (max_val - min_val).abs() < 1e-9 {
        1.0
    } else {
        max_val - min_val
    };

    let n = points.len() as f64;
    let coords: Vec<String> = points
        .iter()
        .enumerate()
        .map(|(i, (v, _))| {
            let x = pad + (i as f64 / (n - 1.0).max(1.0)) * w;
            let y = pad + h - ((v - min_val) / range) * h;
            format!("{x:.1},{y:.1}")
        })
        .collect();

    let first_x = pad;
    let last_x = pad + w;
    let bottom = pad + h;

    format!(
        r##"<svg width="100%" height="{height}" viewBox="0 0 {width} {height}" preserveAspectRatio="none" xmlns="http://www.w3.org/2000/svg">
  <polygon points="{first_x:.1},{bottom:.1} {all_coords} {last_x:.1},{bottom:.1}" fill="rgba(26,82,118,0.12)" stroke="none"/>
  <polyline points="{all_coords}" fill="none" stroke="rgb(26,82,118)" stroke-width="1.5" stroke-linejoin="round"/>
  <text x="{width}" y="12" text-anchor="end" font-size="10" fill="rgb(100,100,100)">{max_label}</text>
  <text x="{width}" y="{bottom:.0}" text-anchor="end" font-size="10" fill="rgb(100,100,100)">{min_label}</text>
</svg>"##,
        all_coords = coords.join(" "),
        max_label = format!("{max_val:.3}"),
        min_label = format!("{min_val:.3}"),
    )
}

#[component]
fn ExtraPropertiesSection(code: String, properties: Option<serde_json::Value>) -> Element {
    let mut expanded = use_signal(|| false);
    let is_open = *expanded.read();

    let has_props = properties
        .as_ref()
        .and_then(|v| v.as_object())
        .is_some_and(|o| o.values().any(|v| !v.is_null()));

    let arrow = if is_open { " \u{25BE}" } else { " \u{25B8}" };

    rsx! {
        div { class: "kaart-detail-row",
            span { class: "kaart-detail-label", "Code" }
            if has_props {
                a {
                    class: "kaart-code-link",
                    href: "#",
                    onclick: move |e| { e.prevent_default(); expanded.set(!is_open); },
                    "{code}{arrow}"
                }
            } else {
                span { class: "kaart-detail-value", "{code}" }
            }
        }
        if is_open {
            if let Some(ref extra) = properties {
                if let Some(obj) = extra.as_object() {
                    for (key, val) in obj.iter() {
                        if !val.is_null() {
                            {
                                let display_val = match val {
                                    serde_json::Value::String(s) => s.clone(),
                                    serde_json::Value::Number(n) => {
                                        if let Some(f) = n.as_f64() {
                                            format!("{f:.1}")
                                        } else {
                                            n.to_string()
                                        }
                                    }
                                    other => other.to_string(),
                                };
                                rsx! {
                                    DetailRow {
                                        label: key.to_string(),
                                        value: display_val,
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DetailRow(label: String, value: String) -> Element {
    rsx! {
        div { class: "kaart-detail-row",
            span { class: "kaart-detail-label", "{label}" }
            span { class: "kaart-detail-value", "{value}" }
        }
    }
}
