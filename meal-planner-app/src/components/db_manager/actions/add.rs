use dioxus::prelude::*;

use super::db_operation_helper::{operation_triggered, DbOperation};
use super::popup::DbActionPopup;
use crate::components::product_related::Product;
use dioxus_i18n::t;
use meal_planner_lib::data_types as data;

#[component]
pub fn Add() -> Element {
    let product_signal: Signal<Option<data::Product>> = use_signal(|| None);
    let result_signal: Signal<Option<Result<(), String>>> = use_signal(|| None);

    let show_popup = result_signal().is_some();

    rsx!(
        div {
            span { {t!("add-product-sentence")} }
            Product { product_signal, editable: true }
            button {
                class: "db-button",
                onclick: move |_| operation_triggered(product_signal, result_signal, DbOperation::Add),
                {t!("action-add")}
            }
            if show_popup {
                DbActionPopup { result_signal }
            }
        }
    )
}
