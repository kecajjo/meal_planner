use dioxus::prelude::*;
use dioxus_i18n::prelude::{use_init_i18n, I18nConfig};
use dioxus_i18n::unic_langid::langid;

/// Define a components module that contains all shared components for our app.
mod components;

use components::{
    action_bar::ActionBar,
    main_view::{MainView, ViewKind},
};

// The asset macro also minifies some assets like CSS and JS to make bundled smaller
const SIDE_BAR_CSS: Asset = asset!("/assets/styling/side_bar.css");
const DB_MANAGER_CSS: Asset = asset!("/assets/styling/db_manager.css");
const PRODUCT_RELATED_CSS: Asset = asset!("/assets/styling/product_related.css");
const MAIN_CSS: Asset = asset!("/assets/styling/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

const EN_US_FTL: &str = include_str!("../assets/locales/en-US/main.ftl");
const PL_PL_FTL: &str = include_str!("../assets/locales/pl-PL/main.ftl");

fn main() {
    // The `launch` function is the main entry point for a dioxus app. It takes a component and renders it with the platform feature
    // you have enabled
    dioxus::launch(App);
}

/// App is the main component of our app. Components are the building blocks of dioxus apps. Each component is a function
/// that takes some props and returns an Element. In this case, App takes no props because it is the root of our app.
///
/// Components should be annotated with `#[component]` to support props, better error messages, and autocomplete
#[component]
fn App() -> Element {
    let _i18n = use_init_i18n(|| {
        I18nConfig::new(langid!("en-US"))
            .with_fallback(langid!("en-US"))
            .with_locale((langid!("en-US"), EN_US_FTL))
            .with_locale((langid!("pl-PL"), PL_PL_FTL))
    });
    let selection = use_signal(|| ViewKind::MealPlan);
    let sidebar_open = use_signal(|| false);

    // The `rsx!` macro lets us define HTML inside of rust. It expands to an Element with all of our HTML inside.
    rsx! {
        // In addition to element and text (which we will see later), rsx can contain other components. In this case,
        // we are using the `document::Link` component to add a link to our favicon and main CSS file into the head of our app.
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        document::Link { rel: "stylesheet", href: SIDE_BAR_CSS }
        document::Link { rel: "stylesheet", href: DB_MANAGER_CSS }
        document::Link { rel: "stylesheet", href: PRODUCT_RELATED_CSS }

        div { class: "app-shell text-slate-900",
            ActionBar { selection, sidebar_open }
            MainView { selection }
        }
    }
}
