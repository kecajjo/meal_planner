use super::actions::{add, copy, modify};
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum DbActionKinds {
    Add,
    Modify,
    Copy,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ButtonData {
    label: &'static str,
    action: DbActionKinds,
}

static BUTTONS: &[ButtonData] = &[
    ButtonData {
        label: "Add",
        action: DbActionKinds::Add,
    },
    ButtonData {
        label: "Modify",
        action: DbActionKinds::Modify,
    },
    ButtonData {
        label: "Copy",
        action: DbActionKinds::Copy,
    },
];

#[component]
pub fn DbManagerView() -> Element {
    let selected_action = use_signal(|| DbActionKinds::Add);

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
                "{btn_data.label}"
            }
        )
    });

    rsx! {
        nav { class: "view-content",
            "Add or modify products"
            nav { class: "navigation-button-bar", {buttons_elems} }
            div {
                match curr_action {
                    DbActionKinds::Add => rsx! {
                        add::add {}
                    },
                    DbActionKinds::Modify => rsx! {
                        modify::modify {}
                    },
                    DbActionKinds::Copy => rsx! {
                        copy::copy {}
                    },
                }
            }
        }
    }
}
