use dioxus::prelude::*;

#[component]
pub fn MealPlanView() -> Element {
	rsx! {
		div { class: "view-content", "Meal Plan View" }
	}
}
