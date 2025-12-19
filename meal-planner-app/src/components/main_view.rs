use dioxus::prelude::*;

use crate::components::{
    db_manager::DbManagerView, food_swapper::SwapFoodView, meal_planner::MealPlanView,
};

#[derive(Clone, Copy, PartialEq)]
pub enum ViewKind {
    MealPlan,
    SwapFood,
    DbManager,
}

#[component]
pub fn MainView(selection: Signal<ViewKind>) -> Element {
    let locale_rev = use_context::<Signal<u64>>();
    let _locale_rev = locale_rev();
    rsx! {
        main { class: "flex-1 p-4 overflow-y-auto min-h-0", role: "main",
            match selection() {
                ViewKind::MealPlan => rsx! {
                    MealPlanView {}
                },
                ViewKind::SwapFood => rsx! {
                    SwapFoodView {}
                },
                ViewKind::DbManager => rsx! {
                    DbManagerView {}
                },
            }
        }
    }
}
