use dioxus::prelude::*;
use meal_planner_lib::data_types::{
    MacroElements as DataMacroElements, MacroElementsType as DataMEType,
};

#[component]
fn MacroElementSingleInputField(
    label: &'static str,
    signal: Signal<f32>,
    editable: bool,
) -> Element {
    rsx! {
        div {
            {format!("{label}: ")}
            if editable {
                input {
                    r#type: "number",
                    step: "0.01",
                    value: signal().to_string(),
                    onchange: move |e| {
                        if let Ok(val) = e.value().parse::<f32>() {
                            signal.set(val);
                        } else {
                            signal.set(signal());
                        }
                    },
                }
            } else {
                {signal().to_string()}
            }
        }
    }
}

#[component]
pub fn MacroElements(me_signal: Signal<DataMacroElements>, editable: bool) -> Element {
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
            "Macro Elements:"
            MacroElementSingleInputField { label: "Fat", signal: fat_signal, editable }
            MacroElementSingleInputField {
                label: "Saturated Fat",
                signal: saturated_fat_signal,
                editable,
            }
            MacroElementSingleInputField { label: "Carbohydrates", signal: carbs_signal, editable }
            MacroElementSingleInputField { label: "Sugar", signal: sugar_signal, editable }
            MacroElementSingleInputField { label: "Protein", signal: protein_signal, editable }
            div {
                "Calories: "
                {((calories * 100.0).round() / 100.0).to_string()}
            }
        }
    }
}
