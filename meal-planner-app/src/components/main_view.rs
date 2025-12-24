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
    rsx! {
        main { class: "content-shell app-theme", role: "main",
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
