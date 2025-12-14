use dioxus::prelude::*;
use meal_planner_lib::data_types::{MacroElementsType, MicroNutrientsType, Product};
use strum::IntoEnumIterator;

#[component]
pub fn product(product: Product, editable: bool) -> Element {
    let product_signal = use_signal(|| product);
    let product = product_signal.read().clone();
    rsx! {
        div {
            // Name
            div {
                "Name: "
                if editable {
                    input { value: product.name() }
                } else {
                    "{product.name()}"
                }
            }
            // Brand
            div {
                "Brand: "
                if editable {
                    input { value: product.brand().unwrap_or("") }
                } else {
                    {product.brand().unwrap_or("")}
                }
            }
            // Macro nutrients
            for macro_type in MacroElementsType::iter() {
                div {
                    "{macro_type}: "
                    if editable {
                        input { value: product.macro_elements[macro_type] }
                    } else {
                        "{product.macro_elements[macro_type]}"
                    }
                }
            }
            // Micro nutrients
            for micro_type in MicroNutrientsType::iter() {
                div {
                    "{micro_type}: "
                    if editable {
                        input { value: product.micro_nutrients[micro_type].unwrap_or(0.0) }
                    } else {
                        {
                            product
                                .micro_nutrients[micro_type]
                                .map(|v| v.to_string())
                                .unwrap_or("None".to_string())
                        }
                    }
                }
            }
            // Allowed units
            for (unit , data) in &product.allowed_units {
                div {
                    "{unit}: "
                    if editable {
                        input { value: data.amount }
                        " / "
                        input { value: data.divider }
                    } else {
                        "{data.amount} / {data.divider}"
                    }
                }
            }
        }
    }
}
