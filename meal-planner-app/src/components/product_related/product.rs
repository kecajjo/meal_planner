use super::{AllowedUnits, MacroElements, MicroNutrients};
use dioxus::prelude::*;
use meal_planner_lib::data_types as data;
use std::rc::Rc;

#[component]
fn EditableTextInput(
    label: &'static str,
    signal: Signal<String>,
    product_signal: Signal<Option<data::Product>>,
    input_ref: Signal<Option<MountedData>>,
    editable: bool,
) -> Element {
    rsx! {
        div {
            "{label}: "
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
                            signal
                                .set(
                                    match product_signal() {
                                        Some(prod) => {
                                            match label {
                                                "Name" => prod.name().to_string(),
                                                "Brand" => prod.brand().unwrap_or("").to_string(),
                                                _ => "".to_string(),
                                            }
                                        }
                                        None => "".to_string(),
                                    },
                                );
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

    use_effect(move || {
        tracing::debug!("Updating name and brand signal state");
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
        tracing::debug!("Updating Product component state");
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
                label: "Name",
                signal: name_signal,
                product_signal,
                input_ref: name_input_ref,
                editable,
            }
            EditableTextInput {
                label: "Brand",
                signal: brand_signal,
                product_signal,
                input_ref: brand_input_ref,
                editable,
            }
            MacroElements { me_signal: macro_elements_signal, editable }
            MicroNutrients { mn_signal: micro_nutrients_signal, editable }
            AllowedUnits { ad_signal: allowed_units_signal, editable }
        }
    }
}
