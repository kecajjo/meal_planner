use super::{AllowedUnits, MacroElements, MicroNutrients};
use dioxus::prelude::*;
use dioxus_i18n::t;
use meal_planner_lib::data_types as data;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq)]
enum ProductField {
    Name,
    Brand,
}

#[component]
fn EditableTextInput(
    label_key: &'static str,
    field: ProductField,
    signal: Signal<String>,
    product_signal: Signal<Option<data::Product>>,
    input_ref: Signal<Option<MountedData>>,
    editable: bool,
) -> Element {
    rsx! {
        div {
            {format!("{}: ", t!(label_key))}
            if editable {
                input {
                    value: signal(),
                    onchange: move |e| { signal.set(e.value()) },
                    onmounted: move |e| {
                        let Event { data, .. } = e;
                        match Rc::try_unwrap(data) {
                            Ok(mounted) => input_ref.set(Some(mounted)),
                            Err(_) => panic!("Element not mounted properly in EditableTextInput"),
                        }
                    },
                    onkeydown: move |e| {
                        if e.key() == Key::Enter || e.key() == Key::Escape {
                            if let Some(input) = input_ref.read().as_ref() {
                                let _ = input.set_focus(false);
                            }
                        }
                        if e.key() == Key::Escape {
                            let reset_value = match (product_signal(), field) {
                                (Some(prod), ProductField::Name) => prod.name().to_string(),
                                (Some(prod), ProductField::Brand) => {
                                    prod.brand().unwrap_or("").to_string()
                                }
                                _ => "".to_string(),
                            };
                            signal.set(reset_value);
                        }
                    },
                }
            } else {
                {signal()}
            }
        }
    }
}

#[component]
pub fn Product(product_signal: Signal<Option<data::Product>>, editable: bool) -> Element {
    let name_input_ref = use_signal(|| None);
    let brand_input_ref = use_signal(|| None);
    let mut name_signal = use_signal(|| "".to_string());
    let mut brand_signal = use_signal(|| "".to_string());
    let mut macro_elements_signal =
        use_signal(|| data::MacroElements::new(0.0, 0.0, 0.0, 0.0, 0.0));
    let mut micro_nutrients_signal = use_signal(|| data::MicroNutrients::default());
    let mut allowed_units_signal = use_signal(|| data::AllowedUnits::default());
    let mut macro_open = use_signal(|| true);
    let mut micro_open = use_signal(|| true);
    let mut allowed_units_open = use_signal(|| true);

    use_effect(move || {
        let Some(product) = product_signal() else {
            name_signal.set("".to_string());
            brand_signal.set("".to_string());
            return;
        };

        let name = product.name().to_string();
        name_signal.set(name);

        let brand = product
            .brand()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "".to_string());
        brand_signal.set(brand);

        macro_elements_signal.set(product.macro_elements.as_ref().clone());
        micro_nutrients_signal.set(product.micro_nutrients.as_ref().clone());
        allowed_units_signal.set(product.allowed_units.clone());
    });

    use_effect(move || {
        let new_product = Some(data::Product::new(
            name_signal(),
            if brand_signal().is_empty() {
                None
            } else {
                Some(brand_signal())
            },
            Box::new(macro_elements_signal().clone()),
            Box::new(micro_nutrients_signal().clone()),
            allowed_units_signal().clone(),
        ));
        if new_product != product_signal() {
            product_signal.set(new_product);
        }
    });

    rsx! {
        div {
            EditableTextInput {
                label_key: "label-name",
                field: ProductField::Name,
                signal: name_signal,
                product_signal,
                input_ref: name_input_ref,
                editable,
            }
            EditableTextInput {
                label_key: "label-brand",
                field: ProductField::Brand,
                signal: brand_signal,
                product_signal,
                input_ref: brand_input_ref,
                editable,
            }
            div { class: "collapsible",
                button {
                    class: "collapsible__header",
                    onclick: move |_| macro_open.set(!macro_open()),
                    span { class: "collapsible__chevron", {if macro_open() { "▾" } else { "▸" }} }
                    span { class: "collapsible__title", {t!("label-macro-elements")} }
                }
                if macro_open() {
                    div { class: "collapsible__content",
                        MacroElements { me_signal: macro_elements_signal, editable }
                    }
                }
            }
            div { class: "collapsible",
                button {
                    class: "collapsible__header",
                    onclick: move |_| micro_open.set(!micro_open()),
                    span { class: "collapsible__chevron", {if micro_open() { "▾" } else { "▸" }} }
                    span { class: "collapsible__title", {t!("label-micro-nutrients")} }
                }
                if micro_open() {
                    div { class: "collapsible__content",
                        MicroNutrients { mn_signal: micro_nutrients_signal, editable }
                    }
                }
            }
            div { class: "collapsible",
                button {
                    class: "collapsible__header",
                    onclick: move |_| allowed_units_open.set(!allowed_units_open()),
                    span { class: "collapsible__chevron",
                        {if allowed_units_open() { "▾" } else { "▸" }}
                    }
                    span { class: "collapsible__title", {t!("label-allowed-units")} }
                }
                if allowed_units_open() {
                    div { class: "collapsible__content",
                        AllowedUnits { ad_signal: allowed_units_signal, editable }
                    }
                }
            }
        }
    }
}
