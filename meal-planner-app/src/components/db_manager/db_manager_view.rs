use super::actions::{add, modify};
use dioxus::prelude::*;
use dioxus_i18n::t;

#[derive(Clone, Copy, PartialEq, Eq)]
enum DbActionKinds {
    Add,
    Modify,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ButtonData {
    label: &'static str,
    action: DbActionKinds,
}

static BUTTONS: &[ButtonData] = &[
    ButtonData {
        label: "action-add",
        action: DbActionKinds::Add,
    },
    ButtonData {
        label: "action-modify",
        action: DbActionKinds::Modify,
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
                {t!(btn_data.label)}
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
                        add::Add {}
                    },
                    DbActionKinds::Modify => rsx! {
                        modify::Modify {}
                    },
                }
            }
        }
    }
}
