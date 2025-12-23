use crate::components::{layout::use_sidebar_width, product_related::Product};
use dioxus::prelude::*;
use meal_planner_lib::data_types::Product as ProductData;
use meal_planner_lib::database_access as db_access;

use dioxus_i18n::t;

fn db_type_to_string(db_type: &db_access::DataBaseTypes) -> String {
    match db_type {
        db_access::DataBaseTypes::Local(_) => t!("db-type-local"),
        _ => todo!(),
    }
}

#[component]
pub fn Search() -> Element {
    let sidebar_width = use_sidebar_width();
    let mut query = use_signal(|| "".to_string());
    let mut input_value = use_signal(|| "".to_string());
    let mut selected_db_type = use_signal(|| None as Option<db_access::DataBaseTypes>);
    let mut selected_product = use_signal(|| None as Option<ProductData>);

    let results = use_resource(move || {
        let search_text = query();

        async move {
            let trimmed = search_text.trim();
            if trimmed.is_empty() {
                return Vec::<(String, ProductData, db_access::DataBaseTypes)>::new();
            }

            // TODO: change to iterating over the enum once everything is implemented
            let db_types = vec![db_access::DataBaseTypes::Local(
                db_access::LOCAL_DB_DEFAULT_FILE.to_string(),
            )];
            let mut aggregated: Vec<(String, ProductData, db_access::DataBaseTypes)> = Vec::new();

            for db_type in db_types {
                if let Some(db) = db_access::get_db(db_type.clone()).await {
                    let map = db
                        .get_products_matching_criteria(&[db_access::DbSearchCriteria::ById(
                            search_text.clone(),
                        )])
                        .await;
                    aggregated.extend(
                        map.into_iter()
                            .map(|(id, product)| (id, product, db_type.clone())),
                    );
                }
            }

            aggregated
        }
    });

    rsx! {
        div {
            class: "search-panel",
            style: "display: flex; flex-direction: column; gap: 1rem; max-width: 40rem;",
            div { style: "display: flex; gap: 0.5rem; align-items: center;",
                input {
                    class: "navigation-button",
                    r#type: "text",
                    placeholder: t!("search-placeholder"),
                    value: input_value(),
                    oninput: move |e| input_value.set(e.value()),
                    onkeydown: move |e| {
                        if e.key() == Key::Enter {
                            query.set(input_value.peek().clone());
                        }
                    },
                    style: "flex: 1; min-width: 12rem;",
                }
                button {
                    class: "navigation-button navigation-button--selected",
                    onclick: move |_| {
                        query.set(input_value.peek().clone());
                    },
                    {t!("search-button")}
                }
            }

            match results() {
                None => rsx! {
                    div { class: "view-content", {t!("search-loading")} }
                },
                Some(ref list) => {
                    if query().trim().is_empty() {
                        rsx! {
                            div { class: "view-content", {t!("search-empty-prompt")} }
                        }
                    } else if list.is_empty() {
                        rsx! {
                            div { class: "view-content", {t!("search-no-results")} }
                        }
                    } else {
                        rsx! {
                            div {
                                class: "search-results",
                                style: "display: flex; flex-direction: column; gap: 0.5rem;",
                                for (id , product , db_type) in list.iter().cloned() {
                                    button {
                                        class: "navigation-button",
                                        style: "justify-content: space-between; display: flex; align-items: center;",
                                        onclick: move |_| {
                                            selected_db_type.set(Some(db_type.clone()));
                                            selected_product.set(Some(product.clone()));
                                        },
                                        span { "{id}" }
                                        span { style: "font-weight: 600; color: var(--color-highlight);",
                                            {db_type_to_string(&db_type)}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if selected_db_type().is_some() {
                div {
                    class: "product-overlay",
                    style: format!("--action-bar-width: {}px;", sidebar_width()),
                    div {
                        class: "product-overlay__backdrop",
                        onclick: move |_| {
                            selected_db_type.set(None);
                            selected_product.set(None);
                        }
                    }
                    div {
                        class: "product-overlay__panel",
                        button {
                            class: "arrow-back-button",
                            style: "margin-bottom: 1rem; font-size: 1.5rem; background: none; border: none; color: var(--color-text); cursor: pointer;",
                            onclick: move |_| {
                                selected_db_type.set(None);
                                selected_product.set(None);
                            },
                            "← Back"
                        }
                        div {
                            class: "view-content view-content--overlay",
                            {t!("search-product-details")}
                        }
                        Product { product_signal: selected_product, editable: false }
                    }
                }
            }
        }
    }
}
