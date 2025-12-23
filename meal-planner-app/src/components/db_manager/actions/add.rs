use dioxus::prelude::*;

use crate::components::product_related::Product;
use dioxus_i18n::t;
use meal_planner_lib::data_types as data;
use meal_planner_lib::database_access as db_access;

fn add_trigerred(
    input: Signal<Option<data::Product>>,
    mut result_signal: Signal<Option<Result<(), String>>>,
) {
    let product = match input() {
        Some(prod) => prod,
        None => {
            result_signal.set(Some(Err(t!("error-no-product"))));
            return;
        }
    };
    let product_id = product.id();

    spawn({
        let mut result_signal = result_signal.clone();
        async move {
            tracing::info!("Creating DB access");
            let Some(mut db) = db_access::get_mutable_db(db_access::DataBaseTypes::Local(
                "local_db.sqlite3".to_string(),
            ))
            .await
            else {
                result_signal.set(Some(Err(t!("error-db-access"))));
                return;
            };
            tracing::info!("DB Accessed");
            let res = db.add_product(&product_id, product).await;
            result_signal.set(Some(res));
        }
    });
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
        Some(Ok(_)) => Some(t!("popup-product-added")),
        Some(Err(err)) => Some(format!("{}: {err}", t!("popup-error"))),
        None => None,
    };
    let show_popup = show_popup_signal();

    rsx!(
        div {
            span { {t!("add-product-sentence")} }
            Product { product_signal, editable: true }
            button {
                class: "db-add-btn",
                onclick: move |_| add_trigerred(product_signal, result_signal),
                {t!("action-add")}
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
