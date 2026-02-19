use dioxus::prelude::*;

use crate::api::{self, GemaalStatus, StatusResponse};
use crate::components::status_badge::StatusBadge;
use crate::Route;

#[component]
pub fn Dashboard() -> Element {
    let status = use_resource(api::fetch_status);

    match &*status.read() {
        Some(Ok(data)) => rsx! { DashboardContent { data: data.clone() } },
        Some(Err(e)) => rsx! {
            div { class: "page",
                h1 { class: "page-title", "Dashboard" }
                div { class: "error-message", "Fout bij laden: {e}" }
            }
        },
        None => rsx! {
            div { class: "page",
                h1 { class: "page-title", "Dashboard" }
                div { class: "loading", "Laden..." }
            }
        },
    }
}

#[component]
fn DashboardContent(data: StatusResponse) -> Element {
    let active_gemalen: Vec<_> = data
        .stations
        .iter()
        .filter(|s| s.status == GemaalStatus::Aan)
        .collect();

    rsx! {
        div { class: "page",
            h1 { class: "page-title", "Dashboard" }

            div { class: "card-grid",
                div { class: "card",
                    div { class: "card-label", "Geregistreerde gemalen" }
                    div { class: "card-value", "{data.registered_gemalen}" }
                }
                div { class: "card",
                    div { class: "card-label", "Actieve gemalen" }
                    div { class: "card-value", "{data.active_stations}" }
                }
                div { class: "card",
                    div { class: "card-label", "Totaal debiet" }
                    div { class: "card-value",
                        "{data.total_debiet_m3s:.3}"
                        span { class: "card-unit", " m\u{00B3}/s" }
                    }
                }
            }

            // Link naar de kaart
            div { class: "card",
                style: "text-align: center; padding: 2rem;",
                Link {
                    to: Route::KaartPage {},
                    class: "btn btn-primary",
                    "Bekijk kaart met alle gemalen"
                }
            }

            if !active_gemalen.is_empty() {
                h2 { class: "page-title", "Actieve gemalen" }
                div { class: "table-container",
                    table {
                        thead {
                            tr {
                                th { "Code" }
                                th { "Status" }
                                th { "Debiet (m\u{00B3}/s)" }
                            }
                        }
                        tbody {
                            for gemaal in &active_gemalen {
                                tr {
                                    td {
                                        Link {
                                            to: Route::GemaalDetail { code: gemaal.gemaal_code.clone() },
                                            "{gemaal.gemaal_code}"
                                        }
                                    }
                                    td { StatusBadge { status: gemaal.status } }
                                    td { "{gemaal.debiet:.4}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
