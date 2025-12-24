use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn DbActionPopup(result_signal: Signal<Option<Result<(), String>>>) -> Element {
    let popup_message = match result_signal() {
        Some(Ok(())) => Some(t!("success-message")),
        Some(Err(err)) => Some(format!("{}: {err}", t!("popup-error"))),
        None => None,
    };
    rsx! {
        if let Some(message) = popup_message.clone() {
            div {
                class: "db-popup",
                onclick: move |_| {
                    result_signal.set(None);
                },
                {message}
            }
        }
    }
}
