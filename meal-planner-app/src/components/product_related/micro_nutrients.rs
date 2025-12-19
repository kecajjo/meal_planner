use crate::i18n::t;
use dioxus::html::geometry::WheelDelta;
use dioxus::prelude::*;
use meal_planner_lib::data_types::{
    MicroNutrients as DataMicroNutrients, MicroNutrientsType as DataMNType,
};

#[component]
fn MicroNutrientInput(
    label_key: &'static str,
    mn_type: DataMNType,
    signal: Signal<Option<f32>>,
    mn_signal: Signal<DataMicroNutrients>,
    editable: bool,
) -> Element {
    let none_label = t("mn-none");
    let no_data_label = t("mn-no-data");
    let is_none = signal().is_none();
    let disabled_key = format!("mn-{mn_type:?}-disabled");
    let enabled_key = format!("mn-{mn_type:?}-enabled");

    rsx! {
        div { class: "micro-row",
            span { class: "micro-label", {format!("{}:", t(label_key))} }
            if editable {
                span { class: "micro-controls",
                    if is_none {
                        input {
                            key: "{disabled_key}",
                            class: "micro-input nutrient-input nutrient-input--disabled",
                            r#type: "number",
                            step: "0.01",
                            disabled: true,
                            value: "",
                            placeholder: none_label.clone(),
                        }
                    } else {
                        input {
                            key: "{enabled_key}",
                            class: "micro-input nutrient-input",
                            r#type: "number",
                            step: "0.01",
                            value: signal().map(|v| format!("{:.2}", v)).unwrap_or_default(),
                            placeholder: none_label.clone(),
                            onwheel: move |e| {
                                e.prevent_default();
                                e.stop_propagation();
                                let step = 0.1_f32;
                                let delta_y = match e.delta() {
                                    WheelDelta::Pixels(v) => v.y,
                                    WheelDelta::Lines(v) => v.y,
                                    WheelDelta::Pages(v) => v.y,
                                };
                                let delta = if delta_y < 0.0 { step } else { -step };
                                let next = (signal().unwrap_or(0.0) + delta).max(0.0);
                                signal.set(Some(next));
                                let mut new_mn = mn_signal().clone();
                                new_mn[mn_type] = Some(next);
                                if mn_signal() != new_mn {
                                    mn_signal.set(new_mn);
                                }
                            },
                            onchange: move |e| {
                                if let Ok(val) = e.value().parse::<f32>() {
                                    signal.set(Some(val.max(0.0)));
                                }
                                let mut new_mn = mn_signal().clone();
                                new_mn[mn_type] = signal();
                                if mn_signal() != new_mn {
                                    mn_signal.set(new_mn);
                                }
                            },
                        }
                    }
                    label { class: "micro-toggle",
                        input {
                            r#type: "checkbox",
                            checked: is_none,
                            onchange: move |e| {
                                if e.checked() {
                                    signal.set(None);
                                } else {
                                    signal.set(Some(0.0));
                                }
                                let mut new_mn = mn_signal().clone();
                                new_mn[mn_type] = signal();
                                if mn_signal() != new_mn {
                                    mn_signal.set(new_mn);
                                }
                            },
                        }
                        span { class: "micro-checkbox__label", {no_data_label.clone()} }
                    }
                }
            } else {
                if let Some(val) = signal() {
                    span { class: "micro-value", {format!("{:.2}", val)} }
                }
            }
        }
    }
}

#[component]
pub fn MicroNutrients(mn_signal: Signal<DataMicroNutrients>, editable: bool) -> Element {
    // Signals per micro nutrient
    let mut fiber_signal = use_signal(|| mn_signal()[DataMNType::Fiber]);
    let mut zinc_signal = use_signal(|| mn_signal()[DataMNType::Zinc]);
    let mut sodium_signal = use_signal(|| mn_signal()[DataMNType::Sodium]);
    let mut alcohol_signal = use_signal(|| mn_signal()[DataMNType::Alcohol]);

    // Keep local signals in sync with parent
    use_effect(move || {
        let mn = mn_signal();
        fiber_signal.set(mn[DataMNType::Fiber]);
        zinc_signal.set(mn[DataMNType::Zinc]);
        sodium_signal.set(mn[DataMNType::Sodium]);
        alcohol_signal.set(mn[DataMNType::Alcohol]);
    });

    // Push local changes back to parent
    use_effect(move || {
        let mut new_mn = mn_signal().clone();
        new_mn[DataMNType::Fiber] = fiber_signal();
        new_mn[DataMNType::Zinc] = zinc_signal();
        new_mn[DataMNType::Sodium] = sodium_signal();
        new_mn[DataMNType::Alcohol] = alcohol_signal();
        if mn_signal() != new_mn {
            mn_signal.set(new_mn);
        }
    });

    rsx! {
        div { class: "micro-section",
            if editable || fiber_signal().is_some() {
                MicroNutrientInput {
                    label_key: "mn-fiber",
                    mn_type: DataMNType::Fiber,
                    signal: fiber_signal,
                    mn_signal,
                    editable,
                }
            }
            if editable || zinc_signal().is_some() {
                MicroNutrientInput {
                    label_key: "mn-zinc",
                    mn_type: DataMNType::Zinc,
                    signal: zinc_signal,
                    mn_signal,
                    editable,
                }
            }
            if editable || sodium_signal().is_some() {
                MicroNutrientInput {
                    label_key: "mn-sodium",
                    mn_type: DataMNType::Sodium,
                    signal: sodium_signal,
                    mn_signal,
                    editable,
                }
            }
            if editable || alcohol_signal().is_some() {
                MicroNutrientInput {
                    label_key: "mn-alcohol",
                    mn_type: DataMNType::Alcohol,
                    signal: alcohol_signal,
                    mn_signal,
                    editable,
                }
            }
        }
    }
}
