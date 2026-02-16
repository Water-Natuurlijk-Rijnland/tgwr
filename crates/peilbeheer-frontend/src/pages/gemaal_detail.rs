use dioxus::prelude::*;

use crate::api::{self, GemaalSnapshot, TrendDirection, TrendInfo};
use crate::components::status_badge::StatusBadge;

#[component]
pub fn GemaalDetail(code: String) -> Element {
    let code_clone = code.clone();
    let detail = use_resource(move || {
        let c = code_clone.clone();
        async move { api::fetch_gemaal(&c).await }
    });

    match &*detail.read() {
        Some(Ok(data)) => {
            if let Some(snapshot) = &data.snapshot {
                rsx! { GemaalDetailContent { code: code.clone(), snapshot: snapshot.clone() } }
            } else {
                rsx! {
                    div { class: "page",
                        h1 { class: "page-title", "Gemaal {code}" }
                        div { class: "empty-state", "Geen data beschikbaar voor dit gemaal." }
                    }
                }
            }
        }
        Some(Err(e)) => rsx! {
            div { class: "page",
                h1 { class: "page-title", "Gemaal {code}" }
                div { class: "error-message", "Fout bij laden: {e}" }
            }
        },
        None => rsx! {
            div { class: "page",
                h1 { class: "page-title", "Gemaal {code}" }
                div { class: "loading", "Laden..." }
            }
        },
    }
}

#[component]
fn GemaalDetailContent(code: String, snapshot: GemaalSnapshot) -> Element {
    rsx! {
        div { class: "page",
            div { class: "detail-header",
                h1 { class: "page-title", "Gemaal {code}" }
                StatusBadge { status: snapshot.status }
            }

            div { class: "detail-grid",
                // Huidige status
                div { class: "detail-card",
                    h3 { "Huidige status" }
                    div { class: "detail-row",
                        span { class: "detail-label", "Debiet" }
                        span { class: "detail-value", "{snapshot.debiet:.4} m\u{00B3}/s" }
                    }
                    if let Some(ref lu) = snapshot.last_update {
                        div { class: "detail-row",
                            span { class: "detail-label", "Laatste update" }
                            span { class: "detail-value", "{lu}" }
                        }
                    }
                    if let Some(ref ga) = snapshot.generated_at {
                        div { class: "detail-row",
                            span { class: "detail-label", "Gegenereerd" }
                            span { class: "detail-value", "{ga}" }
                        }
                    }
                    if let Some(ref err) = snapshot.error {
                        div { class: "detail-row",
                            span { class: "detail-label", "Fout" }
                            span { class: "detail-value", style: "color: var(--danger)", "{err}" }
                        }
                    }
                }

                // Trends
                if let Some(ref trends) = snapshot.trends {
                    div { class: "detail-card",
                        h3 { "Trends" }
                        if let Some(ref t) = trends.min_30 {
                            TrendRow { label: "30 min", trend: t.clone() }
                        }
                        if let Some(ref t) = trends.min_60 {
                            TrendRow { label: "60 min", trend: t.clone() }
                        }
                        if let Some(ref t) = trends.min_180 {
                            TrendRow { label: "180 min", trend: t.clone() }
                        }
                        if trends.min_30.is_none() && trends.min_60.is_none() && trends.min_180.is_none() {
                            div { class: "empty-state", "Geen trenddata beschikbaar" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TrendRow(label: &'static str, trend: TrendInfo) -> Element {
    let (arrow, class) = match trend.direction {
        TrendDirection::Increasing => ("\u{2191}", "trend-up"),
        TrendDirection::Decreasing => ("\u{2193}", "trend-down"),
        TrendDirection::Stable => ("\u{2192}", "trend-stable"),
    };

    let strength = match trend.strength {
        crate::api::TrendStrength::Strong => "sterk",
        crate::api::TrendStrength::Moderate => "matig",
        crate::api::TrendStrength::Weak => "zwak",
    };

    rsx! {
        div { class: "detail-row",
            span { class: "detail-label", "{label}" }
            span { class: "detail-value {class}",
                "{arrow} {trend.slope_per_hour:.4}/u ({strength}, R\u{00B2}={trend.r_squared:.2})"
            }
        }
    }
}
