use dioxus::prelude::*;

use crate::components::product_related::Product;
use crate::i18n::t;
use meal_planner_lib::data_types as data;
use meal_planner_lib::database_access as db_access;

fn add_trigerred(
    input: Signal<Option<data::Product>>,
    mut result_signal: Signal<Option<Result<(), String>>>,
) {
    let product = match input() {
        Some(prod) => prod,
        None => {
            result_signal.set(Some(Err("No product to add".to_string())));
            return;
        }
    };
    tracing::info!("Creating DB access");
    let mut db = db_access::get_mutable_db(db_access::DataBaseTypes::Local("local_db.sqlite"))
        .expect("Couldnt access local Database");
    tracing::info!("DB Accessed");
    let product_id = product.id();
    result_signal.set(Some(db.add_product(&product_id, product)));
}

#[component]
pub fn Add() -> Element {
    let product_signal: Signal<Option<data::Product>> = use_signal(|| None);
    let mut result_signal: Signal<Option<Result<(), String>>> = use_signal(|| None);
    let mut show_popup_signal = use_signal(|| false);

    use_effect(move || {
        if let Some(_) = result_signal() {
            show_popup_signal.set(true);
        } else {
            show_popup_signal.set(false);
        }
    });

    let popup_message = match result_signal() {
        Some(Ok(_)) => Some("Product added successfully".to_string()),
        Some(Err(err)) => Some(format!("Error: {err}")),
        None => None,
    };
    let show_popup = show_popup_signal();

    rsx!(
        div {
            "Add Component:"
            Product { product_signal, editable: true }
            button {
                class: "db-add-btn",
                onclick: move |_| add_trigerred(product_signal, result_signal),
                {t("action-add")}
            }
            if show_popup {
                if let Some(message) = popup_message.clone() {
                    div {
                        class: "db-popup",
                        onclick: move |_| {
                            show_popup_signal.set(false);
                            result_signal.set(None);
                        },
                        {message}
                    }
                }
            }
        }
    )
}
