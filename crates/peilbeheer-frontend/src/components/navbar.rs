use dioxus::prelude::*;

use crate::Route;

#[component]
pub fn Navbar() -> Element {
    let route: Route = use_route();

    let links = [
        (Route::Dashboard {}, "Dashboard"),
        (Route::KaartPage {}, "Kaart"),
        (Route::Gemalen {}, "Gemalen"),
    ];

    rsx! {
        nav { class: "navbar",
            span { class: "navbar-brand", "Peilbeheer HHVR" }
            div { class: "navbar-links",
                for (target, label) in links {
                    Link {
                        to: target.clone(),
                        class: if route == target { "active" } else { "" },
                        "{label}"
                    }
                }
            }
        }
    }
}
