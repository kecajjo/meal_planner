use dioxus::prelude::*;

#[component]
pub fn DbManagerView() -> Element {
    rsx! {
        div { class: "view-content", "DB Manager View" }
    }
}
