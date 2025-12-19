use dioxus::prelude::*;

/// Define a components module that contains all shared components for our app.
mod components;
mod i18n;

use components::{
    action_bar::ActionBar,
    main_view::{MainView, ViewKind},
};

// The asset macro also minifies some assets like CSS and JS to make bundled smaller
const SIDE_BAR_CSS: Asset = asset!("/assets/styling/side_bar.css");
const DB_MANAGER_CSS: Asset = asset!("/assets/styling/db_manager.css");
const MAIN_CSS: Asset = asset!("/assets/styling/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

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

        div { class: "app-shell text-slate-900",
            ActionBar { selection, sidebar_open }
            MainView { selection }
        }
    }
}
