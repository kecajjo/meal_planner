use dioxus::prelude::*;
use meal_planner_lib::data_types::AllowedUnits as DataAllowedUnits;

#[component]
pub fn AllowedUnits(ad_signal: Signal<DataAllowedUnits>, editable: bool) -> Element {
    rsx! {
        div { "AllowedUnits component" }
    }
}
