use crate::i18n::t;
use dioxus::html::geometry::WheelDelta;
use dioxus::prelude::*;
use meal_planner_lib::data_types::{AllowedUnits as DataAllowedUnits, AllowedUnitsType, UnitData};
use std::collections::HashSet;
use strum::IntoEnumIterator;

const DEFAULT_UNIT: (AllowedUnitsType, UnitData) = (
    AllowedUnitsType::Gram,
    UnitData {
        amount: 1,
        divider: 1,
    },
);

fn unit_from_value(value: &str) -> Option<AllowedUnitsType> {
    AllowedUnitsType::iter().find(|candidate| format!("{candidate:?}") == value)
}

fn unit_label(unit: AllowedUnitsType) -> String {
    match unit {
        AllowedUnitsType::Gram => t("au-unit-gram"),
        AllowedUnitsType::Piece => t("au-unit-piece"),
        AllowedUnitsType::Cup => t("au-unit-cup"),
        AllowedUnitsType::Tablespoon => t("au-unit-tablespoon"),
        AllowedUnitsType::Teaspoon => t("au-unit-teaspoon"),
        AllowedUnitsType::Box => t("au-unit-box"),
        AllowedUnitsType::Custom => t("au-unit-custom"),
    }
}

fn ordered_units(units: &DataAllowedUnits) -> Vec<(AllowedUnitsType, UnitData)> {
    let mut list: Vec<(AllowedUnitsType, UnitData)> = Vec::new();
    for unit in AllowedUnitsType::iter() {
        if unit == AllowedUnitsType::Gram {
            continue;
        }
        if let Some(data) = units.get(&unit) {
            list.push((unit, *data));
        }
    }
    list
}

#[component]
pub fn AllowedUnits(ad_signal: Signal<DataAllowedUnits>, editable: bool) -> Element {
    let mut rows_signal = use_signal(|| ordered_units(&ad_signal()));
    let mut last_seen_map = use_signal(|| ad_signal());

    // Keep local rows in sync when parent signal changes (e.g., when a new product loads).
    use_effect(move || {
        let incoming = ad_signal();
        if incoming != last_seen_map() {
            last_seen_map.set(incoming.clone());

            let mut updated_rows = Vec::new();
            let mut seen = HashSet::new();

            for (unit, _) in rows_signal().iter() {
                if let Some(data) = incoming.get(unit) {
                    updated_rows.push((*unit, *data));
                    seen.insert(*unit);
                }
            }

            for unit in AllowedUnitsType::iter() {
                if unit == AllowedUnitsType::Gram || seen.contains(&unit) {
                    continue;
                }
                if let Some(data) = incoming.get(&unit) {
                    updated_rows.push((unit, *data));
                }
            }

            rows_signal.set(updated_rows);
        }
    });

    // Push local edits back to the parent signal.
    use_effect(move || {
        let mut map = DataAllowedUnits::default();
        for (unit, data) in rows_signal().iter().copied() {
            map.insert(unit, data);
        }
        if map.is_empty() {
            map.insert(DEFAULT_UNIT.0, DEFAULT_UNIT.1);
        }
        if ad_signal() != map {
            last_seen_map.set(map.clone());
            ad_signal.set(map);
        }
    });

    let rows = rows_signal();
    let used_units: HashSet<_> = rows.iter().map(|(unit, _)| *unit).collect();
    let add_target = AllowedUnitsType::iter()
        .filter(|unit| *unit != AllowedUnitsType::Gram)
        .find(|unit| !used_units.contains(unit));
    let disable_remove = rows.len() <= 1;

    rsx! {
        div { class: "allowed-section",
            h3 { class: "allowed-title", {t("label-allowed-units")} }
            for (row_index , (unit , data)) in rows.iter().copied().enumerate() {
                div { class: "allowed-row",
                    span { class: "allowed-label", {t("au-unit")} }
                    if editable {
                        select {
                            class: "allowed-select",
                            value: format!("{unit:?}"),
                            onchange: move |e| {
                                if let Some(new_unit) = unit_from_value(&e.value()) {
                                    let mut rows = rows_signal();
                                    let duplicate = rows
                                        .iter()
                                        .enumerate()
                                        .any(|(idx, (existing, _))| idx != row_index && *existing == new_unit);
                                    if duplicate {
                                        return;
                                    }
                                    if let Some((unit_slot, _)) = rows.get_mut(row_index) {
                                        *unit_slot = new_unit;
                                    }
                                    rows_signal.set(rows);
                                }
                            },
                            for option in AllowedUnitsType::iter().filter(|u| *u != AllowedUnitsType::Gram) {
                                option {
                                    value: format!("{option:?}"),
                                    disabled: option != unit && used_units.contains(&option),
                                    {unit_label(option)}
                                }
                            }
                        }
                    } else {
                        span { class: "allowed-value", {unit_label(unit)} }
                    }
                    span { class: "allowed-label", {t("au-amount")} }
                    if editable {
                        input {
                            class: "allowed-input nutrient-input",
                            r#type: "number",
                            min: "0",
                            value: data.amount.to_string(),
                            onwheel: move |e| {
                                e.prevent_default();
                                e.stop_propagation();
                                let delta_y = match e.delta() {
                                    WheelDelta::Pixels(v) => v.y,
                                    WheelDelta::Lines(v) => v.y,
                                    WheelDelta::Pages(v) => v.y,
                                };
                                let step = if delta_y < 0.0 { 1_i32 } else { -1_i32 };
                                let mut rows = rows_signal();
                                if let Some((_, data)) = rows.get_mut(row_index) {
                                    let current = i32::from(data.amount);
                                    let next = (current + step).max(0) as u16;
                                    data.amount = next;
                                }
                                rows_signal.set(rows);
                            },
                            onchange: move |e| {
                                let mut rows = rows_signal();
                                if let Some((_, data)) = rows.get_mut(row_index) {
                                    if let Ok(val) = e.value().parse::<i32>() {
                                        data.amount = val.max(0) as u16;
                                    }
                                }
                                rows_signal.set(rows);
                            },
                        }
                    } else {
                        span { class: "allowed-value", {data.amount.to_string()} }
                    }
                    if editable {
                        button {
                            class: "allowed-remove",
                            disabled: disable_remove,
                            onclick: move |_| {
                                let mut rows = rows_signal();
                                if rows.len() > 1 && row_index < rows.len() {
                                    rows.remove(row_index);
                                    rows_signal.set(rows);
                                }
                            },
                            {t("au-delete")}
                        }
                    }
                }
            }
            if editable {
                button {
                    class: "allowed-add",
                    disabled: add_target.is_none(),
                    onclick: move |_| {
                        let mut rows = rows_signal();
                        let used: HashSet<_> = rows.iter().map(|(unit, _)| *unit).collect();
                        if let Some(next_unit) = AllowedUnitsType::iter()
                            .filter(|unit| *unit != AllowedUnitsType::Gram)
                            .find(|unit| !used.contains(unit))
                        {
                            rows.push((next_unit, UnitData { amount: 1, divider: 1 }));
                            rows_signal.set(rows);
                        }
                    },
                    {t("au-add")}
                }
            }
        }
    }
}
