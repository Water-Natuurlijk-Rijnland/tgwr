use dioxus::prelude::*;

mod api;
mod components;
mod pages;

use components::map::KaartPage;
use components::navbar::Navbar;
use pages::dashboard::Dashboard;
use pages::gemaal_detail::GemaalDetail;
use pages::gemalen::Gemalen;

#[derive(Debug, Clone, PartialEq, Routable)]
enum Route {
    #[layout(Layout)]
    #[route("/")]
    Dashboard {},
    #[route("/kaart")]
    KaartPage {},
    #[route("/gemalen")]
    Gemalen {},
    #[route("/gemalen/{code}")]
    GemaalDetail { code: String },
}

#[component]
fn Layout() -> Element {
    rsx! {
        Navbar {}
        Outlet::<Route> {}
    }
}

fn main() {
    dioxus::launch(|| {
        rsx! { Router::<Route> {} }
    });
}
