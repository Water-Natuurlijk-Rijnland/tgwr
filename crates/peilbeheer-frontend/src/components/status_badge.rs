use dioxus::prelude::*;

use crate::api::GemaalStatus;

#[component]
pub fn StatusBadge(status: GemaalStatus) -> Element {
    let (class, label) = match status {
        GemaalStatus::Aan => ("badge badge-aan", "Aan"),
        GemaalStatus::Uit => ("badge badge-uit", "Uit"),
        GemaalStatus::Onbekend => ("badge badge-onbekend", "Onbekend"),
        GemaalStatus::Error => ("badge badge-error", "Error"),
    };

    rsx! {
        span { class: "{class}", "{label}" }
    }
}
