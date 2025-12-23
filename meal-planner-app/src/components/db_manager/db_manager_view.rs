use super::actions::{add, search};
use dioxus::prelude::*;
use dioxus_i18n::t;

#[derive(Clone, Copy, PartialEq, Eq)]
enum DbActionKinds {
    Add,
    Search,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ButtonData {
    label: &'static str,
    action: DbActionKinds,
}

static BUTTONS: &[ButtonData] = &[ButtonData {
    label: "action-add",
    action: DbActionKinds::Add,
}];

#[component]
pub fn DbManagerView() -> Element {
    let selected_action = use_signal(|| DbActionKinds::Search);

    let curr_action = *selected_action.read();
    let buttons_elems = BUTTONS.iter().map(|btn_data| {
        let cls = if btn_data.action == curr_action {
            "navigation-button--selected"
        } else {
            "navigation-button"
        };

        rsx!(
            button {
                class: "{cls}",
                onclick: move |_| {
                    let mut selected_action = selected_action;
                    selected_action.set(btn_data.action);
                },
                {t!(btn_data.label)}
            }
        )
    });

    match curr_action {
        DbActionKinds::Add => rsx! {
            // Arrow-like button to go back to Search
            div {
                button {
                    class: "arrow-back-button",
                    onclick: move |_| {
                        let mut selected_action = selected_action;
                        selected_action.set(DbActionKinds::Search);
                    },
                    "← Back"
                }
            }
            div { class: "view-content", add::Add {} }
        },
        DbActionKinds::Search => rsx! {
            nav { class: "view-content",
                nav { class: "navigation-button-bar", {buttons_elems} }
                div { search::Search {} }
            }
        },
    }
}
