use dioxus::prelude::*;
use serde::Deserialize;
use wasm_bindgen::JsValue;

use crate::api::{self, OptimalisatieParams, OptimalisatieResultaat, UurPrijs};

/// Geselecteerd peilgebied (vanuit JS map click).
#[derive(Debug, Clone, PartialEq, Deserialize)]
struct SelectedPeilgebied {
    code: String,
    naam: String,
    oppervlakte: Option<f64>,
    zomerpeil: Option<f64>,
    winterpeil: Option<f64>,
    vastpeil: Option<f64>,
    gemaal_naam: Option<String>,
    gemaal_capaciteit: Option<f64>,
}

#[component]
pub fn Gemalen() -> Element {
    let peilgebieden_res = use_resource(|| api::fetch_peilgebieden_geojson());
    let gemalen_res =
        use_resource(|| async { api::fetch_assets_geojson_raw(Some("gemaal")).await });

    let pg = match &*peilgebieden_res.read() {
        Some(Ok(g)) => {
            web_sys::console::log_1(&format!("[Gemalen] pg loaded: {} bytes", g.len()).into());
            Some(g.clone())
        }
        Some(Err(e)) => {
            web_sys::console::log_1(&format!("[Gemalen] pg error: {e}").into());
            None
        }
        None => {
            return rsx! {
                div { class: "kaart-page",
                    div { class: "kaart-loading",
                        div { class: "loading", "Kaartdata laden..." }
                    }
                }
            };
        }
    };
    let gm = match &*gemalen_res.read() {
        Some(Ok(g)) => {
            web_sys::console::log_1(&format!("[Gemalen] gm loaded: {} bytes", g.len()).into());
            Some(g.clone())
        }
        Some(Err(e)) => {
            web_sys::console::log_1(&format!("[Gemalen] gm error: {e}").into());
            None
        }
        None => {
            return rsx! {
                div { class: "kaart-page",
                    div { class: "kaart-loading",
                        div { class: "loading", "Kaartdata laden..." }
                    }
                }
            };
        }
    };

    rsx! {
        GemalenMapView { peilgebieden_geojson: pg, gemalen_geojson: gm }
    }
}

#[component]
fn GemalenMapView(
    peilgebieden_geojson: Option<String>,
    gemalen_geojson: Option<String>,
) -> Element {
    let mut selected = use_signal::<Option<SelectedPeilgebied>>(|| None);

    // Receive peilgebied clicks via Dioxus eval channel (runs on Dioxus event loop,
    // so signal.set() properly triggers re-renders — unlike wasm_bindgen closures).
    use_future(move || async move {
        loop {
            let result = document::eval(
                r#"
                if (window._bufferedPgClick) {
                    var b = window._bufferedPgClick;
                    window._bufferedPgClick = null;
                    return b;
                }
                var data = await new Promise(function(resolve) {
                    window._resolvePgClick = resolve;
                });
                return data;
                "#,
            )
            .await;
            match result {
                Ok(val) => {
                    // val is serde_json::Value; the JS sends a JSON string
                    let json_str = match val.as_str() {
                        Some(s) => s.to_string(),
                        None => val.to_string(),
                    };
                    web_sys::console::log_1(
                        &format!(
                            "[Gemalen] click received: {}",
                            &json_str[..json_str.len().min(200)]
                        )
                        .into(),
                    );
                    match serde_json::from_str::<SelectedPeilgebied>(&json_str) {
                        Ok(pg) => {
                            web_sys::console::log_1(
                                &format!("[Gemalen] parsed OK: {}", pg.naam).into(),
                            );
                            selected.set(Some(pg));
                        }
                        Err(e) => {
                            web_sys::console::log_1(
                                &format!("[Gemalen] parse FAILED: {e}").into(),
                            );
                        }
                    }
                }
                Err(e) => {
                    web_sys::console::log_1(
                        &format!("[Gemalen] eval error: {e:?}").into(),
                    );
                    break;
                }
            }
        }
    });

    // Set up map with GeoJSON data
    use_effect(move || {
        let window = web_sys::window().unwrap();

        // Parse GeoJSON to JS objects and store on window (avoids huge eval strings)
        if let Some(ref pg_json) = peilgebieden_geojson {
            if let Ok(js_val) = js_sys::JSON::parse(pg_json) {
                js_sys::Reflect::set(&window, &JsValue::from_str("_pgData"), &js_val).ok();
            }
        } else {
            js_sys::Reflect::set(&window, &JsValue::from_str("_pgData"), &JsValue::NULL).ok();
        }

        if let Some(ref gm_json) = gemalen_geojson {
            if let Ok(js_val) = js_sys::JSON::parse(gm_json) {
                js_sys::Reflect::set(&window, &JsValue::from_str("_gmData"), &js_val).ok();
            }
        } else {
            js_sys::Reflect::set(&window, &JsValue::from_str("_gmData"), &JsValue::NULL).ok();
        }

        // Build and run the map JS (references window._pgData / window._gmData)
        // NOTE: js_sys::eval is used here because Dioxus WASM requires dynamic JS
        // execution for Leaflet map initialization. The JS is a static string literal,
        // not user input, so there is no injection risk.
        let js = build_gemalen_map_js();
        let _ = js_sys::eval(&js);
    });

    let sel = selected.read();

    rsx! {
        div { class: "kaart-page",
            div {
                id: "gemalen-sim-map",
                class: "kaart-map",
            }

            if sel.is_none() {
                div { class: "sim-hint", "Klik op een peilgebied om de simulatie te starten" }
            }

            if let Some(pg) = &*sel {
                SimulatiePanel {
                    key: "{pg.code}",
                    peilgebied: pg.clone(),
                    on_close: move |_| selected.set(None),
                }
            }
        }
    }
}

#[component]
fn SimulatiePanel(peilgebied: SelectedPeilgebied, on_close: EventHandler) -> Element {
    let opp_ha = peilgebied.oppervlakte.map(|o| o / 10000.0);
    let opp_m2 = peilgebied.oppervlakte.unwrap_or(100_000.0);
    let cap = peilgebied.gemaal_capaciteit.unwrap_or(0.0);
    let has_cap = cap > 0.001;

    let mut peil_options: Vec<(&str, f64)> = Vec::new();
    if let Some(z) = peilgebied.zomerpeil {
        peil_options.push(("Zomerpeil", z));
    }
    if let Some(w) = peilgebied.winterpeil {
        peil_options.push(("Winterpeil", w));
    }
    if let Some(v) = peilgebied.vastpeil {
        peil_options.push(("Vastpeil", v));
    }
    let default_peil = peil_options.first().map(|(_, v)| *v).unwrap_or(-0.60);

    let peil_display = if let Some(v) = peilgebied.vastpeil {
        format!("Vast: {v:.2} m NAP")
    } else {
        let z = peilgebied
            .zomerpeil
            .map(|v| format!("{v:.2}"))
            .unwrap_or("-".into());
        let w = peilgebied
            .winterpeil
            .map(|v| format!("{v:.2}"))
            .unwrap_or("-".into());
        format!("Zomer: {z} / Winter: {w} m NAP")
    };

    let opp = opp_m2;

    // Signals
    let mut streefpeil = use_signal(move || default_peil);
    let mut gemaal_debiet = use_signal(move || if has_cap { cap } else { 0.5 });
    let mut verdamping = use_signal(|| 0.5_f64);
    let mut infiltratie = use_signal(|| 0.2_f64);
    let mut opt_regen_per_uur: Signal<Vec<f64>> = use_signal(|| vec![0.0; 24]);
    let mut opt_opvoerhoogte = use_signal(|| 2.0_f64);
    let mut opt_efficiency = use_signal(|| 0.70_f64);
    let mut opt_marge_cm = use_signal(|| 20.0_f64);
    let mut opt_berging = use_signal(|| 0.10_f64);
    let mut opt_prijzen: Signal<Vec<UurPrijs>> = use_signal(Vec::new);
    let mut opt_prijzen_loading = use_signal(|| false);
    let mut opt_prijzen_error: Signal<Option<String>> = use_signal(|| None);
    let mut opt_result: Signal<Option<Result<OptimalisatieResultaat, String>>> = use_signal(|| None);
    let mut opt_loading = use_signal(|| false);
    let mut opt_count = use_signal(|| 0u32);

    // Rain signals
    let mut qf_intensiteit = use_signal(|| 10.0_f64);
    let mut qf_startuur = use_signal(|| 6_u8);
    let mut qf_duur = use_signal(|| 3_u8);

    // Auto-compute regen_per_uur whenever inputs change
    use_effect(move || {
        let mut regen = vec![0.0; 24];
        let start = qf_startuur() as usize;
        let duur = qf_duur() as usize;
        let intens = qf_intensiteit();
        for i in 0..duur {
            let h = (start + i) % 24;
            regen[h] = intens;
        }
        opt_regen_per_uur.set(regen);
    });

    // Auto-fetch prices on mount
    use_effect(move || {
        spawn(async move {
            opt_prijzen_loading.set(true);
            opt_prijzen_error.set(None);
            match api::fetch_energieprijzen().await {
                Ok(p) => opt_prijzen.set(p),
                Err(e) => opt_prijzen_error.set(Some(e)),
            }
            opt_prijzen_loading.set(false);
        });
    });

    rsx! {
        div { class: "sim-modal-overlay",
            div {
                class: "sim-modal-backdrop",
                onclick: move |_| on_close.call(()),
            }
            div { class: "sim-modal sim-modal-wide",
            div { class: "sim-modal-header",
                div {
                    h2 { class: "sim-modal-title", "Energieoptimalisatie" }
                    div { class: "sim-modal-subtitle",
                        "{peilgebied.naam} \u{2014} {peilgebied.code}"
                    }
                }
                button {
                    class: "sim-modal-close",
                    onclick: move |_| on_close.call(()),
                    "\u{00D7}"
                }
            }

            div { class: "sim-modal-body",
                div { class: "sim-modal-left",
                    // Peilgebied info
                    div { class: "sim-info-bar",
                        div { class: "sim-info-chip",
                            span { class: "sim-info-chip-label", "Oppervlakte" }
                            span { class: "sim-info-chip-value",
                                {opp_ha.map(|v| format!("{v:.1} ha")).unwrap_or("-".into())}
                            }
                        }
                        div { class: "sim-info-chip",
                            span { class: "sim-info-chip-label", "Peil" }
                            span { class: "sim-info-chip-value", "{peil_display}" }
                        }
                        if let Some(ref naam) = peilgebied.gemaal_naam {
                            div { class: "sim-info-chip",
                                span { class: "sim-info-chip-label", "Gemaal" }
                                span { class: "sim-info-chip-value", "{naam}" }
                            }
                            div { class: "sim-info-chip",
                                span { class: "sim-info-chip-label", "Capaciteit" }
                                span { class: "sim-info-chip-value",
                                    {format!("{cap:.2} m\u{00B3}/s")}
                                }
                            }
                        }
                    }

                    // Regen scenario
                    h3 { class: "sim-section-title", "Regenscenario (24 uur)" }
                    div { class: "sim-form-grid",
                        div { class: "sim-field",
                            label { "Intensiteit" }
                            div { class: "sim-input-wrap",
                                input {
                                    r#type: "number", step: "1", min: "0",
                                    value: "{qf_intensiteit}",
                                    onchange: move |e: Event<FormData>| {
                                        if let Ok(v) = e.value().parse::<f64>() { qf_intensiteit.set(v); }
                                    },
                                }
                                span { class: "sim-unit", "mm/uur" }
                            }
                        }
                        div { class: "sim-field",
                            label { "Duur" }
                            div { class: "sim-input-wrap",
                                input {
                                    r#type: "number", step: "1", min: "1", max: "24",
                                    value: "{qf_duur}",
                                    onchange: move |e: Event<FormData>| {
                                        if let Ok(v) = e.value().parse::<u8>() { qf_duur.set(v.clamp(1, 24)); }
                                    },
                                }
                                span { class: "sim-unit", "uren" }
                            }
                        }
                    }
                    div { class: "sim-field rain-slider-field",
                        label { "Start: {qf_startuur():02}:00" }
                        input {
                            r#type: "range", min: "0", max: "23", step: "1",
                            class: "rain-slider",
                            value: "{qf_startuur}",
                            oninput: move |e: Event<FormData>| {
                                if let Ok(v) = e.value().parse::<u8>() { qf_startuur.set(v.min(23)); }
                            },
                        }
                        div { class: "rain-slider-labels",
                            span { "00:00" }
                            span { "06:00" }
                            span { "12:00" }
                            span { "18:00" }
                            span { "23:00" }
                        }
                    }

                    // Gemaal & energie
                    h3 { class: "sim-section-title", "Gemaal & energie" }
                    div { class: "sim-form-grid",
                        div { class: "sim-field",
                            label { "Max debiet" }
                            div { class: "sim-input-wrap",
                                input {
                                    r#type: "number", step: "0.01",
                                    value: "{gemaal_debiet:.2}",
                                    onchange: move |e: Event<FormData>| {
                                        if let Ok(v) = e.value().parse::<f64>() { gemaal_debiet.set(v); }
                                    },
                                }
                                span { class: "sim-unit", "m\u{00B3}/s" }
                            }
                        }
                        div { class: "sim-field",
                            label { "Opvoerhoogte" }
                            div { class: "sim-input-wrap",
                                input {
                                    r#type: "number", step: "0.1",
                                    value: "{opt_opvoerhoogte:.1}",
                                    onchange: move |e: Event<FormData>| {
                                        if let Ok(v) = e.value().parse::<f64>() { opt_opvoerhoogte.set(v); }
                                    },
                                }
                                span { class: "sim-unit", "m" }
                            }
                        }
                        div { class: "sim-field",
                            label { "Efficiency" }
                            div { class: "sim-input-wrap",
                                input {
                                    r#type: "number", step: "0.05", min: "0.1", max: "1.0",
                                    value: "{opt_efficiency:.2}",
                                    onchange: move |e: Event<FormData>| {
                                        if let Ok(v) = e.value().parse::<f64>() { opt_efficiency.set(v.clamp(0.1, 1.0)); }
                                    },
                                }
                            }
                        }
                        div { class: "sim-field",
                            label { "Marge" }
                            div { class: "sim-input-wrap",
                                input {
                                    r#type: "number", step: "1", min: "1",
                                    value: "{opt_marge_cm:.0}",
                                    onchange: move |e: Event<FormData>| {
                                        if let Ok(v) = e.value().parse::<f64>() { opt_marge_cm.set(v.max(1.0)); }
                                    },
                                }
                                span { class: "sim-unit", "cm" }
                            }
                        }
                        if peil_options.len() > 1 {
                            div { class: "sim-field",
                                label { "Streefpeil" }
                                div { class: "sim-input-wrap",
                                    select {
                                        value: "{streefpeil:.2}",
                                        onchange: move |e: Event<FormData>| {
                                            if let Ok(v) = e.value().parse::<f64>() {
                                                streefpeil.set(v);
                                            }
                                        },
                                        for (lbl, val) in peil_options.iter() {
                                            option { value: "{val:.2}", "{lbl} ({val:.2} m)" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Advanced parameters
                    details { class: "sim-advanced",
                        summary { "Geavanceerde parameters" }
                        div { class: "sim-form-grid",
                            div { class: "sim-field",
                                label { "Open water" }
                                div { class: "sim-input-wrap",
                                    input {
                                        r#type: "number", step: "1", min: "1", max: "100",
                                        value: "{(opt_berging() * 100.0) as u32}",
                                        onchange: move |e: Event<FormData>| {
                                            if let Ok(v) = e.value().parse::<f64>() {
                                                opt_berging.set((v / 100.0).clamp(0.01, 1.0));
                                            }
                                        },
                                    }
                                    span { class: "sim-unit", "%" }
                                }
                            }
                            div { class: "sim-field",
                                label { "Verdamping" }
                                div { class: "sim-input-wrap",
                                    input {
                                        r#type: "number", step: "0.1",
                                        value: "{verdamping}",
                                        onchange: move |e: Event<FormData>| {
                                            if let Ok(v) = e.value().parse::<f64>() { verdamping.set(v); }
                                        },
                                    }
                                    span { class: "sim-unit", "mm/uur" }
                                }
                            }
                            div { class: "sim-field",
                                label { "Infiltratie" }
                                div { class: "sim-input-wrap",
                                    input {
                                        r#type: "number", step: "0.1",
                                        value: "{infiltratie}",
                                        onchange: move |e: Event<FormData>| {
                                            if let Ok(v) = e.value().parse::<f64>() { infiltratie.set(v); }
                                        },
                                    }
                                    span { class: "sim-unit", "mm/uur" }
                                }
                            }
                        }
                    }

                    // Stroomprijzen preview
                    h3 { class: "sim-section-title", "Stroomprijzen vandaag" }
                    if opt_prijzen_loading() {
                        div { class: "sim-placeholder", "Prijzen ophalen..." }
                    } else if let Some(ref err) = *opt_prijzen_error.read() {
                        div { class: "sim-error", "Fout: {err}" }
                    } else if !opt_prijzen.read().is_empty() {
                        PriceBarChart { prijzen: opt_prijzen.read().clone() }
                    }

                    // Optimize button
                    button {
                        class: "sim-btn",
                        disabled: opt_loading() || gemaal_debiet() < 0.001,
                        onclick: move |_| {
                            let params = OptimalisatieParams {
                                streefpeil: streefpeil(),
                                max_debiet: gemaal_debiet(),
                                oppervlakte: opp,
                                verdamping: verdamping(),
                                infiltratie: infiltratie(),
                                opvoerhoogte: opt_opvoerhoogte(),
                                efficiency: opt_efficiency(),
                                regen_per_uur: opt_regen_per_uur.read().clone(),
                                prijzen: opt_prijzen.read().clone(),
                                marge_cm: opt_marge_cm(),
                                berging_factor: opt_berging(),
                            };

                            spawn(async move {
                                opt_loading.set(true);
                                let res = api::run_optimalisatie(&params).await;
                                opt_result.set(Some(res));
                                opt_count += 1;
                                opt_loading.set(false);
                            });
                        },
                        if opt_loading() { "Optimaliseren..." } else { "Optimaliseer pompschema" }
                    }
                }

                // Right column: results
                div { class: "sim-modal-right",
                    if let Some(ref res) = *opt_result.read() {
                        match res {
                            Ok(data) => rsx! {
                                OptimalisatieResultDisplay {
                                    key: "{opt_count}",
                                    data: data.clone(),
                                    streefpeil: streefpeil(),
                                    marge_cm: opt_marge_cm(),
                                    max_debiet: gemaal_debiet(),
                                }
                            },
                            Err(e) => rsx! {
                                div { class: "sim-error", "Fout: {e}" }
                            },
                        }
                    } else {
                        div { class: "sim-placeholder",
                            div { class: "sim-placeholder-icon", "\u{26A1}" }
                            p { "Stel een regenscenario en pompparameters in en klik op " strong { "Optimaliseer pompschema" } " om kosten te vergelijken." }
                        }
                    }
                }
            }
        } // end sim-modal
        } // end sim-modal-overlay
    }
}

// ── Mini price bar chart (CSS-based) ──

#[component]
fn PriceBarChart(prijzen: Vec<UurPrijs>) -> Element {
    let max_prijs = prijzen
        .iter()
        .map(|p| p.prijs_eur_kwh)
        .fold(0.0_f64, f64::max)
        .max(0.01);
    let min_prijs = prijzen
        .iter()
        .map(|p| p.prijs_eur_kwh)
        .fold(f64::INFINITY, f64::min);

    rsx! {
        div { class: "price-chart-mini",
            for p in prijzen.iter() {
                {
                    let pct = (p.prijs_eur_kwh / max_prijs * 100.0).max(2.0);
                    let t = if max_prijs > min_prijs {
                        (p.prijs_eur_kwh - min_prijs) / (max_prijs - min_prijs)
                    } else {
                        0.5
                    };
                    // Green (low) -> Orange -> Red (high)
                    let r = (255.0 * t.min(1.0)) as u8;
                    let g = (200.0 * (1.0 - t).min(1.0)) as u8;
                    let color = format!("rgb({r},{g},50)");
                    let height = format!("{pct:.0}%");
                    let style = format!("height:{height};background:{color}");
                    let title = format!("{:02}:00 -- \u{20AC}{:.3}/kWh", p.uur, p.prijs_eur_kwh);
                    rsx! {
                        div {
                            class: "price-bar",
                            style: "{style}",
                            title: "{title}",
                        }
                    }
                }
            }
        }
        div { class: "price-chart-labels",
            span { "00:00" }
            span { "06:00" }
            span { "12:00" }
            span { "18:00" }
            span { "23:00" }
        }
    }
}

// ── Optimalisatie result display ──

#[component]
fn OptimalisatieResultDisplay(
    data: OptimalisatieResultaat,
    streefpeil: f64,
    marge_cm: f64,
    max_debiet: f64,
) -> Element {
    let besparing_positief = data.besparing_eur >= 0.0;

    // Push data to window for Chart.js
    let chart_data = data.clone();
    let chart_streef = streefpeil;
    let chart_marge = marge_cm;
    let chart_debiet = max_debiet;
    use_effect(move || {
        let window = web_sys::window().unwrap();

        // Sample every 5 minutes for chart (reduce from 1440 to 288 points)
        let sample_step = 5;
        let labels: Vec<f64> = chart_data.tijdstappen_optimaal.iter()
            .step_by(sample_step)
            .map(|s| s.tijd_minuten)
            .collect();
        let ws_opt: Vec<f64> = chart_data.tijdstappen_optimaal.iter()
            .step_by(sample_step)
            .map(|s| s.waterstand)
            .collect();
        let ws_naief: Vec<f64> = chart_data.tijdstappen_naief.iter()
            .step_by(sample_step)
            .map(|s| s.waterstand)
            .collect();

        let pump_opt: Vec<f64> = chart_data.tijdstappen_optimaal.iter()
            .step_by(sample_step)
            .map(|s| {
                if chart_debiet > 0.001 {
                    (s.water_afvoer / chart_debiet * 100.0).min(100.0)
                } else { 0.0 }
            })
            .collect();

        let prices: Vec<f64> = chart_data.tijdstappen_optimaal.iter()
            .step_by(sample_step)
            .map(|s| s.prijs_eur_kwh)
            .collect();

        fn vec_to_js_array(v: &[f64]) -> js_sys::Array {
            let arr = js_sys::Array::new();
            for val in v { arr.push(&JsValue::from_f64(*val)); }
            arr
        }

        js_sys::Reflect::set(&window, &JsValue::from_str("_optLabels"), &vec_to_js_array(&labels)).ok();
        js_sys::Reflect::set(&window, &JsValue::from_str("_optWsOpt"), &vec_to_js_array(&ws_opt)).ok();
        js_sys::Reflect::set(&window, &JsValue::from_str("_optWsNaief"), &vec_to_js_array(&ws_naief)).ok();
        js_sys::Reflect::set(&window, &JsValue::from_str("_optPump"), &vec_to_js_array(&pump_opt)).ok();
        js_sys::Reflect::set(&window, &JsValue::from_str("_optPrices"), &vec_to_js_array(&prices)).ok();
        js_sys::Reflect::set(&window, &JsValue::from_str("_optStreefpeil"), &JsValue::from_f64(chart_streef)).ok();

        let marge_m = chart_marge / 100.0;
        js_sys::Reflect::set(&window, &JsValue::from_str("_optBandMin"), &JsValue::from_f64(chart_streef - marge_m)).ok();
        js_sys::Reflect::set(&window, &JsValue::from_str("_optBandMax"), &JsValue::from_f64(chart_streef + marge_m)).ok();

        // NOTE: js_sys::eval is used here because Dioxus WASM requires dynamic JS
        // execution for Chart.js initialization. The JS is a static string literal,
        // not user input, so there is no injection risk.
        let js = build_opt_chart_js();
        let _ = js_sys::eval(&js);
    });

    rsx! {
        div { class: "sim-results",
            // Savings banner
            div {
                class: if besparing_positief { "opt-savings-banner opt-savings-positive" } else { "opt-savings-banner opt-savings-neutral" },
                div { class: "opt-savings-amount",
                    {format!("\u{20AC}{:.2}", data.besparing_eur.abs())}
                }
                div { class: "opt-savings-label",
                    if besparing_positief {
                        {format!("bespaard ({:.0}%)", data.besparing_pct)}
                    } else {
                        "geen besparing mogelijk"
                    }
                }
            }

            // Metrics grid
            div { class: "sim-metrics-grid",
                div { class: "sim-metric",
                    div { class: "sim-metric-value",
                        {format!("\u{20AC}{:.2}", data.totale_kosten_optimaal)}
                    }
                    div { class: "sim-metric-label", "Kosten optimaal" }
                }
                div { class: "sim-metric",
                    div { class: "sim-metric-value",
                        {format!("\u{20AC}{:.2}", data.totale_kosten_naief)}
                    }
                    div { class: "sim-metric-label", "Kosten na\u{00EF}ef" }
                }
                div { class: "sim-metric",
                    div { class: "sim-metric-value",
                        {format!("{:.1} cm", data.max_afwijking_optimaal_cm)}
                    }
                    div { class: "sim-metric-label", "Max afwijking opt." }
                }
                div { class: "sim-metric",
                    div { class: "sim-metric-value",
                        {format!("{:.1} cm", data.max_afwijking_naief_cm)}
                    }
                    div { class: "sim-metric-label", "Max afwijking na\u{00EF}ef" }
                }
            }

            // Chart
            div { class: "sim-chart-container",
                canvas { id: "opt-waterstand-chart", height: "280" }
            }

            // Hourly table
            h3 { class: "sim-section-title", "Uuroverzicht" }
            div { class: "opt-table-scroll",
                table { class: "opt-table",
                    thead {
                        tr {
                            th { "Uur" }
                            th { "Prijs" }
                            th { "Regen" }
                            th { "Pomp opt." }
                            th { "Pomp na\u{00EF}ef" }
                            th { "Kosten opt." }
                            th { "Kosten na\u{00EF}ef" }
                        }
                    }
                    tbody {
                        for u in data.uren.iter() {
                            tr {
                                td { "{u.uur:02}:00" }
                                td { {format!("\u{20AC}{:.3}", u.prijs_eur_kwh)} }
                                td {
                                    if u.regen_mm_uur > 0.0 {
                                        {format!("{:.1}", u.regen_mm_uur)}
                                    } else {
                                        "-"
                                    }
                                }
                                td { {format!("{:.0}%", u.pomp_fractie_optimaal * 100.0)} }
                                td { {format!("{:.0}%", u.pomp_fractie_naief * 100.0)} }
                                td { {format!("\u{20AC}{:.3}", u.kosten_optimaal)} }
                                td { {format!("\u{20AC}{:.3}", u.kosten_naief)} }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── Chart JS builders ──

fn build_opt_chart_js() -> String {
    // All data is pre-set on window._opt* properties by Rust.
    // Static JS string literal — no user input, safe to evaluate.
    r##"
    setTimeout(function() {
        try {
            console.log('[OptChart] building chart...');
            if (window._optChart) { window._optChart.destroy(); window._optChart = null; }
            var canvas = document.getElementById('opt-waterstand-chart');
            if (!canvas) { console.warn('[OptChart] canvas not found'); return; }
            if (typeof Chart === 'undefined') { console.error('[OptChart] Chart.js not loaded'); return; }

            var ctx = canvas.getContext('2d');
            var labels = Array.from(window._optLabels || []);
            var wsOpt = Array.from(window._optWsOpt || []);
            var wsNaief = Array.from(window._optWsNaief || []);
            var pumpOpt = Array.from(window._optPump || []);
            var prices = Array.from(window._optPrices || []);
            var streefpeil = window._optStreefpeil || 0;
            var bandMin = window._optBandMin || 0;
            var bandMax = window._optBandMax || 0;

            var hourLabels = labels.map(function(m) {
                var h = Math.floor(m / 60);
                var min = Math.round(m % 60);
                return h + ':' + (min < 10 ? '0' : '') + min;
            });

            var datasets = [
                {
                    label: 'Waterstand optimaal',
                    data: wsOpt,
                    borderColor: 'rgb(37, 99, 235)',
                    fill: false,
                    borderWidth: 2,
                    pointRadius: 0,
                    tension: 0.2,
                    yAxisID: 'y'
                },
                {
                    label: 'Waterstand naief',
                    data: wsNaief,
                    borderColor: 'rgb(37, 99, 235)',
                    borderDash: [6, 3],
                    fill: false,
                    borderWidth: 1.5,
                    pointRadius: 0,
                    tension: 0.2,
                    yAxisID: 'y'
                },
                {
                    label: 'Streefpeil',
                    data: labels.map(function() { return streefpeil; }),
                    borderColor: 'rgb(34, 197, 94)',
                    borderWidth: 1.5,
                    borderDash: [6, 3],
                    pointRadius: 0,
                    fill: false,
                    yAxisID: 'y'
                },
                {
                    label: 'Band boven',
                    data: labels.map(function() { return bandMax; }),
                    borderColor: 'rgba(34, 197, 94, 0.3)',
                    backgroundColor: 'rgba(34, 197, 94, 0.08)',
                    borderWidth: 1,
                    borderDash: [3, 3],
                    pointRadius: 0,
                    fill: '+1',
                    yAxisID: 'y'
                },
                {
                    label: 'Band onder',
                    data: labels.map(function() { return bandMin; }),
                    borderColor: 'rgba(34, 197, 94, 0.3)',
                    borderWidth: 1,
                    borderDash: [3, 3],
                    pointRadius: 0,
                    fill: false,
                    yAxisID: 'y'
                },
                {
                    label: 'Pompinzet optimaal (%)',
                    data: pumpOpt,
                    borderColor: 'rgba(220, 38, 38, 0.7)',
                    backgroundColor: 'rgba(220, 38, 38, 0.12)',
                    fill: true,
                    borderWidth: 1.5,
                    pointRadius: 0,
                    stepped: true,
                    yAxisID: 'y1'
                },
                {
                    label: 'Stroomprijs',
                    data: prices.map(function(p) { return p * 100; }),
                    borderColor: 'rgba(249, 115, 22, 0.8)',
                    borderWidth: 1.5,
                    pointRadius: 0,
                    stepped: true,
                    fill: false,
                    yAxisID: 'y2'
                }
            ];

            window._optChart = new Chart(ctx, {
                type: 'line',
                data: { labels: hourLabels, datasets: datasets },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    animation: false,
                    interaction: { mode: 'index', intersect: false },
                    plugins: {
                        legend: {
                            display: true,
                            position: 'bottom',
                            labels: {
                                boxWidth: 12, font: { size: 9 }, padding: 6,
                                filter: function(item) {
                                    return item.text !== 'Band boven' && item.text !== 'Band onder';
                                }
                            }
                        },
                        tooltip: {
                            callbacks: {
                                title: function(items) { return items[0].label; }
                            }
                        }
                    },
                    scales: {
                        x: {
                            title: { display: true, text: 'Tijd', font: { size: 10 } },
                            ticks: { maxTicksLimit: 12, font: { size: 8 } }
                        },
                        y: {
                            title: { display: true, text: 'm NAP', font: { size: 10 } },
                            ticks: { font: { size: 9 } }
                        },
                        y1: {
                            position: 'right',
                            title: { display: true, text: 'Pomp (%)', font: { size: 10 } },
                            min: 0, max: 100,
                            grid: { drawOnChartArea: false },
                            ticks: { font: { size: 9 } }
                        },
                        y2: {
                            position: 'right',
                            title: { display: true, text: 'ct/kWh', font: { size: 10 } },
                            grid: { drawOnChartArea: false },
                            ticks: { font: { size: 9 } },
                            offset: true
                        }
                    }
                }
            });
            console.log('[OptChart] chart created OK');
        } catch(e) {
            console.error('[OptChart] error:', e);
        }
    }, 100);
    "##.to_string()
}

#[allow(dead_code)]
fn build_chart_js() -> String {
    // All data is pre-set on window._sim* properties by Rust.
    // Static JS string literal — no user input, safe to evaluate.
    r##"
    setTimeout(function() {
        try {
            console.log('[SimChart] building chart...');
            if (window._simChart) { window._simChart.destroy(); window._simChart = null; }
            var canvas = document.getElementById('sim-waterstand-chart');
            if (!canvas) { console.warn('[SimChart] canvas not found'); return; }
            if (typeof Chart === 'undefined') { console.error('[SimChart] Chart.js not loaded'); return; }

            var ctx = canvas.getContext('2d');
            var labels = Array.from(window._simLabels || []);
            var waterstandData = Array.from(window._simWaterstand || []);
            var pumpData = Array.from(window._simPump || []);
            var streefpeil = window._simStreefpeil || 0;
            var maaiveld = window._simMaaiveld || 0;
            var margeLevel = window._simMargeLevel || 0;

            console.log('[SimChart] data points:', labels.length, 'streefpeil:', streefpeil);

            var datasets = [
                {
                    label: 'Waterstand',
                    data: waterstandData,
                    borderColor: 'rgb(37, 99, 235)',
                    backgroundColor: 'rgba(37, 99, 235, 0.1)',
                    fill: true,
                    borderWidth: 2,
                    pointRadius: 0,
                    tension: 0.2,
                    yAxisID: 'y'
                },
                {
                    label: 'Streefpeil',
                    data: labels.map(function() { return streefpeil; }),
                    borderColor: 'rgb(34, 197, 94)',
                    borderWidth: 1.5,
                    borderDash: [6, 3],
                    pointRadius: 0,
                    fill: false,
                    yAxisID: 'y'
                }
            ];

            if (maaiveld !== 0) {
                datasets.push({
                    label: 'Maaiveld',
                    data: labels.map(function() { return maaiveld; }),
                    borderColor: 'rgb(249, 115, 22)',
                    borderWidth: 1.5,
                    borderDash: [6, 3],
                    pointRadius: 0,
                    fill: false,
                    yAxisID: 'y'
                });
                datasets.push({
                    label: 'Drooglegging marge',
                    data: labels.map(function() { return margeLevel; }),
                    borderColor: 'rgba(234, 179, 8, 0.5)',
                    backgroundColor: 'rgba(234, 179, 8, 0.08)',
                    borderWidth: 1,
                    borderDash: [3, 3],
                    pointRadius: 0,
                    fill: '+1',
                    yAxisID: 'y'
                });
            }

            datasets.push({
                label: 'Gemaal inzet (%)',
                data: pumpData,
                borderColor: 'rgba(220, 38, 38, 0.7)',
                backgroundColor: 'rgba(220, 38, 38, 0.15)',
                fill: true,
                borderWidth: 1.5,
                pointRadius: 0,
                yAxisID: 'y1'
            });

            window._simChart = new Chart(ctx, {
                type: 'line',
                data: { labels: labels, datasets: datasets },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    animation: false,
                    interaction: { mode: 'index', intersect: false },
                    plugins: {
                        legend: {
                            display: true,
                            position: 'bottom',
                            labels: { boxWidth: 12, font: { size: 10 }, padding: 8 }
                        },
                        tooltip: {
                            callbacks: {
                                title: function(items) { return 't = ' + items[0].label + ' min'; }
                            }
                        }
                    },
                    scales: {
                        x: {
                            title: { display: true, text: 'Tijd (min)', font: { size: 10 } },
                            ticks: {
                                maxTicksLimit: 8,
                                font: { size: 9 }
                            }
                        },
                        y: {
                            title: { display: true, text: 'm NAP', font: { size: 10 } },
                            ticks: { font: { size: 9 } }
                        },
                        y1: {
                            position: 'right',
                            title: { display: true, text: 'Gemaal (%)', font: { size: 10 } },
                            min: 0,
                            max: 100,
                            grid: { drawOnChartArea: false },
                            ticks: { font: { size: 9 } }
                        }
                    }
                }
            });
            console.log('[SimChart] chart created OK');
        } catch(e) {
            console.error('[SimChart] error:', e);
        }
    }, 100);
    "##.to_string()
}

fn build_gemalen_map_js() -> String {
    // GeoJSON data is pre-set on window._pgData / window._gmData by Rust
    // to avoid embedding huge JSON strings.
    // Static JS string literal — no user input, safe to evaluate.
    r##"
        setTimeout(function() {
            try {
                if (window._gemalenSimMap) {
                    window._gemalenSimMap.remove();
                    window._gemalenSimMap = null;
                }
                var el = document.getElementById('gemalen-sim-map');
                if (!el) { console.error('[Gemalen] #gemalen-sim-map not found!'); return; }

                var map = L.map('gemalen-sim-map').setView([52.16, 4.49], 11);
                L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
                    attribution: '&copy; OpenStreetMap',
                    maxZoom: 18
                }).addTo(map);
                window._gemalenSimMap = map;

                var gemalenFeatures = [];
                var gmData = window._gmData;
                if (gmData && gmData.features) {
                    gmData.features.forEach(function(f) {
                        if (f.geometry && f.geometry.coordinates) {
                            gemalenFeatures.push(f);
                        }
                    });

                    L.geoJSON(gmData, {
                        pointToLayer: function(feature, latlng) {
                            var icon = L.divIcon({
                                html: '<svg width="28" height="28" viewBox="0 0 28 28"><circle cx="14" cy="14" r="12" fill="#1a5276" stroke="white" stroke-width="2"/><path d="M9 17v-3a5 5 0 0 1 10 0v3" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round"/><line x1="14" y1="9" x2="14" y2="12" stroke="white" stroke-width="1.8" stroke-linecap="round"/><line x1="10" y1="17" x2="18" y2="17" stroke="white" stroke-width="1.8" stroke-linecap="round"/></svg>',
                                className: 'asset-icon',
                                iconSize: [28, 28],
                                iconAnchor: [14, 14]
                            });
                            return L.marker(latlng, { icon: icon });
                        },
                        onEachFeature: function(feature, layer) {
                            var p = feature.properties;
                            var naam = p.naam || 'Onbekend';
                            layer.bindTooltip('<b>' + naam + '</b><br>Gemaal', { direction: 'top', offset: [0, -10] });
                        }
                    }).addTo(map);
                }

                function findNearestGemaal(lat, lon) {
                    var best = null;
                    var bestDist = Infinity;
                    gemalenFeatures.forEach(function(f) {
                        var c = f.geometry.coordinates;
                        var d = Math.sqrt(Math.pow(c[1] - lat, 2) + Math.pow(c[0] - lon, 2));
                        if (d < bestDist) {
                            bestDist = d;
                            best = f;
                        }
                    });
                    return best;
                }

                var pgData = window._pgData;
                if (pgData && pgData.features) {
                    var pgLayer = L.geoJSON(pgData, {
                        style: function() {
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

                            layer.on('click', function(e) {
                                pgLayer.resetStyle();
                                layer.setStyle({ fillColor: '#1d4ed8', fillOpacity: 0.35, weight: 2.5 });

                                var center = layer.getBounds().getCenter();
                                var nearest = findNearestGemaal(center.lat, center.lng);

                                var info = {
                                    code: code,
                                    naam: naam,
                                    oppervlakte: p.OPPERVLAKTE || null,
                                    zomerpeil: p.ZOMERPEIL || null,
                                    winterpeil: p.WINTERPEIL || null,
                                    vastpeil: p.VASTPEIL || null,
                                    gemaal_naam: nearest ? (nearest.properties.naam || null) : null,
                                    gemaal_capaciteit: nearest ? (nearest.properties.extra_properties ? nearest.properties.extra_properties.MAXIMALECAPACITEIT : null) : null
                                };

                                if (info.gemaal_capaciteit != null) {
                                    var c = Number(info.gemaal_capaciteit);
                                    if (!isNaN(c)) {
                                        info.gemaal_capaciteit = c / 60;
                                    }
                                }

                                var jsonStr = JSON.stringify(info);
                                console.log('[Gemalen] click:', jsonStr.substring(0, 200));
                                if (window._resolvePgClick) {
                                    var resolve = window._resolvePgClick;
                                    window._resolvePgClick = null;
                                    resolve(jsonStr);
                                } else {
                                    window._bufferedPgClick = jsonStr;
                                }
                            });
                        }
                    }).addTo(map);

                    var bounds = pgLayer.getBounds();
                    if (bounds.isValid()) map.fitBounds(bounds, { padding: [20, 20] });
                }
            } catch(e) {
                console.error('Gemalen kaart fout:', e);
            }
        }, 100);
    "##.to_string()
}
