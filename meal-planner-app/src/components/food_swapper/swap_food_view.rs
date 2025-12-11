use dioxus::prelude::*;

#[component]
pub fn SwapFoodView() -> Element {
	rsx! {
		div { class: "view-content", "Swap Food View" }
	}
}
