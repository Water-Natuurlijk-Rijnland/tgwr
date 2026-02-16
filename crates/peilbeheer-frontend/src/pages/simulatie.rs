use dioxus::prelude::*;
use dioxus_charts::LineChart;

use crate::api::{self, SimulatieParams, SimulatieResponse};

/// Standalone page wrapper (kept for backwards compatibility).
#[component]
pub fn Simulatie() -> Element {
    rsx! {
        div { class: "page",
            SimulatieSection {}
        }
    }
}

/// Reusable simulatie form + results, without page wrapper.
#[component]
pub fn SimulatieSection() -> Element {
    let mut start_waterstand = use_signal(|| "-0.60".to_string());
    let mut regen_intensiteit = use_signal(|| "10.0".to_string());
    let mut regen_duur = use_signal(|| "60.0".to_string());
    let mut oppervlakte = use_signal(|| "100000.0".to_string());
    let mut gemaal_debiet = use_signal(|| "0.5".to_string());
    let mut verdamping = use_signal(|| "0.0".to_string());
    let mut infiltratie = use_signal(|| "0.0".to_string());
    let mut na_regen_duur = use_signal(|| "30.0".to_string());
    let mut tijd_stap = use_signal(|| "1.0".to_string());
    let mut smart_control = use_signal(|| false);
    let mut streefpeil = use_signal(|| "-0.60".to_string());
    let mut marge = use_signal(|| "5.0".to_string());
    let mut maaiveld_niveau = use_signal(|| "0.0".to_string());

    let mut result: Signal<Option<Result<SimulatieResponse, String>>> = use_signal(|| None);
    let mut loading = use_signal(|| false);

    let on_submit = move |_| {
        let params = SimulatieParams {
            start_waterstand: start_waterstand().parse().unwrap_or(-0.60),
            regen_intensiteit: regen_intensiteit().parse().unwrap_or(10.0),
            regen_duur: regen_duur().parse().unwrap_or(60.0),
            oppervlakte: oppervlakte().parse().unwrap_or(100000.0),
            gemaal_debiet: gemaal_debiet().parse().unwrap_or(0.5),
            verdamping: verdamping().parse().unwrap_or(0.0),
            infiltratie: infiltratie().parse().unwrap_or(0.0),
            na_regen_duur: na_regen_duur().parse().unwrap_or(30.0),
            tijd_stap: tijd_stap().parse().unwrap_or(1.0),
            smart_control: smart_control(),
            streefpeil: streefpeil().parse().unwrap_or(-0.60),
            marge: marge().parse().unwrap_or(5.0),
            maaiveld_niveau: maaiveld_niveau().parse().unwrap_or(0.0),
        };

        spawn(async move {
            loading.set(true);
            let res = api::run_simulatie(&params).await;
            result.set(Some(res));
            loading.set(false);
        });
    };

    rsx! {
        h2 { class: "section-title", "Waterbalans Simulatie" }

        div { class: "form-card",
                div { class: "form-section-title", "Basisparameters" }
                div { class: "form-grid",
                    FormField { label: "Startwaterstand", unit: "m NAP", value: start_waterstand, on_change: move |v| start_waterstand.set(v) }
                    FormField { label: "Regenintensiteit", unit: "mm/uur", value: regen_intensiteit, on_change: move |v| regen_intensiteit.set(v) }
                    FormField { label: "Regenduur", unit: "minuten", value: regen_duur, on_change: move |v| regen_duur.set(v) }
                    FormField { label: "Oppervlakte", unit: "m\u{00B2}", value: oppervlakte, on_change: move |v| oppervlakte.set(v) }
                    FormField { label: "Gemaal debiet", unit: "m\u{00B3}/s", value: gemaal_debiet, on_change: move |v| gemaal_debiet.set(v) }
                }

                div { class: "form-section-title", "Verliezen" }
                div { class: "form-grid",
                    FormField { label: "Verdamping", unit: "mm/uur", value: verdamping, on_change: move |v| verdamping.set(v) }
                    FormField { label: "Infiltratie", unit: "mm/uur", value: infiltratie, on_change: move |v| infiltratie.set(v) }
                }

                div { class: "form-section-title", "Simulatie-instellingen" }
                div { class: "form-grid",
                    FormField { label: "Duur na regen", unit: "minuten", value: na_regen_duur, on_change: move |v| na_regen_duur.set(v) }
                    FormField { label: "Tijdstap", unit: "minuten", value: tijd_stap, on_change: move |v| tijd_stap.set(v) }
                }

                div { class: "form-section-title", "Smart Control (PID)" }
                div { class: "form-grid",
                    div { class: "form-group",
                        label { "Smart control" }
                        input {
                            r#type: "checkbox",
                            checked: smart_control(),
                            onchange: move |e: Event<FormData>| smart_control.set(e.checked()),
                        }
                    }
                    FormField { label: "Streefpeil", unit: "m NAP", value: streefpeil, on_change: move |v| streefpeil.set(v) }
                    FormField { label: "Marge", unit: "cm", value: marge, on_change: move |v| marge.set(v) }
                    FormField { label: "Maaiveld niveau", unit: "m NAP", value: maaiveld_niveau, on_change: move |v| maaiveld_niveau.set(v) }
                }

                div { class: "form-actions",
                    button {
                        class: "btn btn-primary",
                        disabled: loading(),
                        onclick: on_submit,
                        if loading() { "Berekenen..." } else { "Simuleer" }
                    }
                }
            }

        // Results
        if let Some(ref res) = *result.read() {
            match res {
                Ok(data) => rsx! { SimulatieResult { data: data.clone() } },
                Err(e) => rsx! {
                    div { class: "error-message", "Fout: {e}" }
                },
            }
        }
    }
}

#[component]
fn FormField(
    label: &'static str,
    unit: &'static str,
    value: Signal<String>,
    on_change: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "form-group",
            label { "{label}" }
            input {
                r#type: "number",
                step: "any",
                value: "{value}",
                oninput: move |e: Event<FormData>| on_change(e.value()),
            }
            span { class: "unit", "{unit}" }
        }
    }
}

#[component]
fn SimulatieResult(data: SimulatieResponse) -> Element {
    // Prepare chart data: sample if too many points
    let stappen = &data.tijdstappen;
    let step = if stappen.len() > 200 {
        stappen.len() / 200
    } else {
        1
    };

    let labels: Vec<String> = stappen
        .iter()
        .step_by(step)
        .map(|s| format!("{:.0}", s.tijd))
        .collect();

    let waterstand_series: Vec<f32> = stappen
        .iter()
        .step_by(step)
        .map(|s| s.waterstand as f32)
        .collect();

    rsx! {
        div { class: "result-section",
            h2 { class: "page-title", "Resultaat" }

            div { class: "result-summary",
                div { class: "card",
                    div { class: "card-label", "Max waterstand" }
                    div { class: "card-value",
                        "{data.samenvatting.max_waterstand:.3}"
                        span { class: "card-unit", " m NAP" }
                    }
                }
                div { class: "card",
                    div { class: "card-label", "Min waterstand" }
                    div { class: "card-value",
                        "{data.samenvatting.min_waterstand:.3}"
                        span { class: "card-unit", " m NAP" }
                    }
                }
                div { class: "card",
                    div { class: "card-label", "Aantal stappen" }
                    div { class: "card-value", "{data.samenvatting.aantal_stappen}" }
                }
            }

            if let Some(ref drooglegging) = data.drooglegging {
                div { class: "result-summary",
                    div { class: "card",
                        div { class: "card-label", "Drooglegging" }
                        div { class: "card-value",
                            "{drooglegging.drooglegging:.3}"
                            span { class: "card-unit", " m" }
                        }
                    }
                    div { class: "card",
                        div { class: "card-label", "Overschrijding" }
                        div { class: "card-value",
                            "{drooglegging.overschrijding_cm:.1}"
                            span { class: "card-unit", " cm" }
                        }
                    }
                }
            }

            div { class: "chart-container",
                h3 { "Waterstand verloop (m NAP)" }
                LineChart {
                    labels: labels,
                    series: vec![waterstand_series],
                    series_labels: vec!["Waterstand".to_string()],
                    width: "100%",
                    height: "300px",
                    label_interpolation: (|v: f32| format!("{v:.3}")) as fn(f32) -> String,
                    max_ticks: 10,
                }
            }
        }
    }
}
