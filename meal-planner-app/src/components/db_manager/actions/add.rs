use dioxus::prelude::*;

use crate::components::product_related::Product;
use meal_planner_lib::data_types as data;

#[component]
pub fn add() -> Element {
    let product_signal: Signal<Option<data::Product>> = use_signal(|| None);
    rsx!(
        div {
            "Add Component"
            Product { product_signal, editable: true }
        }
    )
}
