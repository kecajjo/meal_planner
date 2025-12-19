use dioxus::prelude::*;
use meal_planner_lib::data_types::MicroNutrients as DataMicroNutrients;

#[component]
pub fn MicroNutrients(mn_signal: Signal<DataMicroNutrients>, editable: bool) -> Element {
    rsx! {
        div { "MicroNutrients component" }
    }
}
