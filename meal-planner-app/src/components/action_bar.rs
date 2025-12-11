use dioxus::prelude::*;

use crate::components::main_view::ViewKind;

const SWIPE_THRESHOLD: f32 = 60.0;

#[component]
pub fn ActionBar(mut selection: Signal<ViewKind>, mut sidebar_open: Signal<bool>) -> Element {
    let mut open_swipe_start = use_signal(|| None::<f32>);
    let mut close_swipe_start = use_signal(|| None::<f32>);

    let nav_class = if sidebar_open() {
        "action-bar action-bar--open"
    } else {
        "action-bar"
    };

    rsx! {
        div {
            class: "sidebar-handle",
            role: "button",
            aria_label: "Open navigation",
            onpointerdown: move |evt| {
                *open_swipe_start.write() = Some(pointer_x(&evt));
            },
            onpointerleave: move |_| {
                *open_swipe_start.write() = None;
            },
            onpointerup: move |evt| {
                if let Some(start) = open_swipe_start() {
                    let delta = pointer_x(&evt) - start;
                    if delta > SWIPE_THRESHOLD {
                        *sidebar_open.write() = true;
                    }
                }
                *open_swipe_start.write() = None;
            },
            onclick: move |_| *sidebar_open.write() = true,
            span { class: "sidebar-handle__hint", "›" }

            // Debug button for non-release builds (mobile only)
            {cfg!(debug_assertions).then(|| rsx! {
                button {
                    class: "sidebar-debug-btn",
                    onclick: move |_| *sidebar_open.write() = true,
                    style: "position: absolute; bottom: 1.5rem; left: 0.25rem; z-index: 100; background: #64748b; color: #fff; border: none; border-radius: 0.375rem; padding: 0.5rem 0.75rem; font-size: 1rem;",
                    "Open Sidebar (Debug)"
                }
            })}
        }
        nav {
            class: nav_class,
            onpointerdown: move |evt| {
                if !sidebar_open() {
                    return;
                }
                *close_swipe_start.write() = Some(pointer_x(&evt));
            },
            onpointerleave: move |_| {
                *close_swipe_start.write() = None;
            },
            onpointerup: move |evt| {
                if let Some(start) = close_swipe_start() {
                    let delta = start - pointer_x(&evt);
                    if delta > SWIPE_THRESHOLD {
                        *sidebar_open.write() = false;
                    }
                }
                *close_swipe_start.write() = None;
            },
            button {
                class: "action-bar__close",
                aria_label: "Close navigation",
                onclick: move |_| *sidebar_open.write() = false,
                "×"
            }
            button {
                class: "action-bar__button",
                onclick: move |_| {
                    *selection.write() = ViewKind::MealPlan;
                    *sidebar_open.write() = false;
                },
                "Meal Plan"
            }
            button {
                class: "action-bar__button",
                onclick: move |_| {
                    *selection.write() = ViewKind::SwapFood;
                    *sidebar_open.write() = false;
                },
                "Swap Foods"
            }
            button {
                class: "action-bar__button",
                onclick: move |_| {
                    *selection.write() = ViewKind::DbManager;
                    *sidebar_open.write() = false;
                },
                "DB Manager"
            }
        }
    }
}

fn pointer_x(evt: &PointerEvent) -> f32 {
    evt.client_coordinates().x as f32
}
