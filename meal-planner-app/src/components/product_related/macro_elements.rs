use dioxus::html::geometry::WheelDelta;
use dioxus::prelude::*;
use dioxus_i18n::t;
use meal_planner_lib::data_types::{
    MacroElements as DataMacroElements, MacroElementsType as DataMEType,
};
use std::rc::Rc;

#[component]
fn MacroElementSingleInputField(
    label_key: &'static str,
    macro_type: DataMEType,
    signal: Signal<f32>,
    macro_signal: Signal<DataMacroElements>,
    input_ref: Signal<Option<MountedData>>,
    editable: bool,
) -> Element {
    rsx! {
        div {
            {format!("{}: ", t!(label_key))}
            if editable {
                input {
                    class: "nutrient-input",
                    r#type: "number",
                    step: "0.01",
                    value: format!("{:.2}", signal()),
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
                        let next = (signal() + delta).max(0.0);
                        signal.set(next);
                    },
                    onchange: move |e| {
                        if let Ok(val) = e.value().parse::<f32>() {
                            signal.set(val.max(0.0));
                        } else {
                            signal.set(signal());
                        }
                    },
                    onmounted: move |e| {
                        let Event { data, .. } = e;
                        let mounted = Rc::try_unwrap(data)
                            .expect("Element not mounted properly in EditableTextInput");
                        input_ref.set(Some(mounted));
                    },
                    onkeydown: move |e| {
                        if e.key() == Key::Enter || e.key() == Key::Escape {
                            #[allow(clippy::let_underscore_future)]
                            if let Some(input) = input_ref.read().as_ref() {
                                let _ = input.set_focus(false);
                            }
                        }
                        if e.key() == Key::Escape {
                            let macro_elements = macro_signal();
                            signal.set(macro_elements[macro_type]);
                        }
                    },
                }
            } else {
                {format!("{:.2}", signal())}
            }
        }
    }
}

#[component]
pub fn MacroElements(me_signal: Signal<DataMacroElements>, editable: bool) -> Element {
    // Signals for input refs
    let fat_input_ref = use_signal(|| None);
    let saturated_fat_input_ref = use_signal(|| None);
    let carbs_input_ref = use_signal(|| None);
    let sugar_input_ref = use_signal(|| None);
    let protein_input_ref = use_signal(|| None);
    // Signals for each editable field
    let mut fat_signal = use_signal(|| me_signal()[DataMEType::Fat]);
    let mut saturated_fat_signal = use_signal(|| me_signal()[DataMEType::SaturatedFat]);
    let mut carbs_signal = use_signal(|| me_signal()[DataMEType::Carbs]);
    let mut sugar_signal = use_signal(|| me_signal()[DataMEType::Sugar]);
    let mut protein_signal = use_signal(|| me_signal()[DataMEType::Protein]);

    use_effect(move || {
        let me = me_signal();
        fat_signal.set(me[DataMEType::Fat]);
        saturated_fat_signal.set(me[DataMEType::SaturatedFat]);
        carbs_signal.set(me[DataMEType::Carbs]);
        sugar_signal.set(me[DataMEType::Sugar]);
        protein_signal.set(me[DataMEType::Protein]);
    });

    use_effect(move || {
        let mut new_me = me_signal().clone();
        new_me
            .set(DataMEType::Fat, fat_signal())
            .expect("Failed to set Fat inside GUI");
        new_me
            .set(DataMEType::SaturatedFat, saturated_fat_signal())
            .expect("Failed to set Saturated Fat inside GUI");
        new_me
            .set(DataMEType::Carbs, carbs_signal())
            .expect("Failed to set Carbs inside GUI");
        new_me
            .set(DataMEType::Sugar, sugar_signal())
            .expect("Failed to set Sugar inside GUI");
        new_me
            .set(DataMEType::Protein, protein_signal())
            .expect("Failed to set Protein inside GUI");
        if me_signal() != new_me {
            me_signal.set(new_me);
        }
    });

    let calories = me_signal()[DataMEType::Calories];

    rsx! {
        div {
            MacroElementSingleInputField {
                label_key: "label-fat",
                macro_type: DataMEType::Fat,
                signal: fat_signal,
                macro_signal: me_signal,
                input_ref: fat_input_ref,
                editable,
            }
            MacroElementSingleInputField {
                label_key: "label-saturated-fat",
                macro_type: DataMEType::SaturatedFat,
                signal: saturated_fat_signal,
                macro_signal: me_signal,
                input_ref: saturated_fat_input_ref,
                editable,
            }
            MacroElementSingleInputField {
                label_key: "label-carbohydrates",
                macro_type: DataMEType::Carbs,
                signal: carbs_signal,
                macro_signal: me_signal,
                input_ref: carbs_input_ref,
                editable,
            }
            MacroElementSingleInputField {
                label_key: "label-sugar",
                macro_type: DataMEType::Sugar,
                signal: sugar_signal,
                macro_signal: me_signal,
                input_ref: sugar_input_ref,
                editable,
            }
            MacroElementSingleInputField {
                label_key: "label-protein",
                macro_type: DataMEType::Protein,
                signal: protein_signal,
                macro_signal: me_signal,
                input_ref: protein_input_ref,
                editable,
            }
            div {
                {format!("{}: ", t!("label-calories"))}
                {format!("{calories:.2}")}
            }
        }
    }
}
