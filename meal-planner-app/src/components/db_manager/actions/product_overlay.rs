use std::vec;

use super::db_operation_helper::{operation_triggered, DbOperation};
use crate::components::{layout::use_sidebar_width, product_related::Product};
use dioxus::prelude::*;
use dioxus_i18n::t;
use meal_planner_lib::data_types::Product as ProductData;
use meal_planner_lib::database_access as db_access;

pub fn create_product_overlay(
    on_close: EventHandler<()>,
    selected_product: Signal<Option<ProductData>>,
    db_type: db_access::DataBaseTypes,
    operation_results: Signal<Option<Result<(), String>>>,
) -> Element {
    let mut current_operation = use_signal(|| DbOperation::None);
    let mut editable = false;

    // Helper to reset operation and close overlay
    let mut on_close_and_reset = move || {
        current_operation.set(DbOperation::None);
        on_close.call(());
    };

    let buttons = if DbOperation::None == current_operation() {
        if db_type.supports_writing() {
            vec![
                rsx!(
                    button {
                        class: "button db-button",
                        onclick: {
                            move |_| {
                                current_operation.set(DbOperation::Edit);
                            }
                        },
                        {t!("edit-label")}
                    }
                ),
                rsx!(
                    button {
                        class: "button db-button button--danger",
                        onclick: {
                            move |_| {
                                operation_triggered(selected_product, operation_results, DbOperation::Delete);
                                on_close_and_reset();
                            }
                        },
                        {t!("delete-label")}
                    }
                ),
            ]
        } else {
            vec![
                rsx!(
                    button {
                        class: "button db-button",
                        onclick: {
                            move |_| {
                                current_operation.set(DbOperation::Add);
                            }
                        },
                        {t!("edit-label")}
                    }
                ),
                rsx!(
                    button {
                        class: "button db-button",
                        onclick: {
                            move |_| {
                                operation_triggered(selected_product, operation_results, DbOperation::Add);
                                on_close_and_reset();
                            }
                        },
                        {t!("add-label")}
                    }
                ),
            ]
        }
    } else {
        editable = true;
        vec![rsx!(
            button {
                class: "button db-button",
                onclick: {
                    move |_| {
                        operation_triggered(
                            selected_product,
                            operation_results,
                            current_operation(),
                        );
                        on_close_and_reset();
                    }
                },
                {t!("save-label")}
            }
        )]
    };

    rsx! {
        ProductOverlay {
            on_close: on_close_and_reset,
            selected_product,
            editable,
            buttons,
        }
    }
}

#[component]
pub fn ProductOverlay(
    on_close: EventHandler<()>,
    selected_product: Signal<Option<ProductData>>,
    editable: bool,
    buttons: Vec<Element>,
) -> Element {
    let sidebar_width = use_sidebar_width()();
    let style = format!("--action-bar-width: {sidebar_width}px;");
    rsx! {
        div { class: "product-overlay", style,
            div {
                class: "product-overlay__backdrop",
                onclick: move |_| on_close.call(()),
            }
            div { class: "product-overlay__panel",
                button {
                    class: "arrow-back-button",
                    style: "margin-bottom: 1rem; font-size: 1.5rem; background: none; border: none; color: var(--color-text); cursor: pointer;",
                    onclick: move |_| on_close.call(()),
                    "← Back"
                }
                div { class: "view-content view-content--overlay", {t!("search-product-details")} }
                Product { product_signal: selected_product, editable }
                div { style: "display: flex; flex-direction: row; gap: 0.5rem;",
                    {buttons.into_iter()}
                }
            }
        }
    }
}
