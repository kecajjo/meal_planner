use dioxus::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use std::time;
#[cfg(target_arch = "wasm32")]
use web_sys::js_sys;

use crate::components::main_view::ViewKind;

#[derive(Clone, Copy)]
struct SwipeSession {
    start_x: f32,
}

#[derive(Clone, Copy, Debug)]
enum SwipeDirection {
    Opening,
    Closing,
}

impl SwipeDirection {
    fn delta(self, start: f32, current: f32) -> f32 {
        match self {
            SwipeDirection::Opening => current - start,
            SwipeDirection::Closing => start - current,
        }
    }
}

const SWIPE_THRESHOLD: f32 = 30.0;
const CLICK_TIME_DELTA_MS: u128 = 200;

#[component]
pub fn ActionBar(mut selection: Signal<ViewKind>, mut sidebar_open: Signal<bool>) -> Element {
    let open_swipe = use_signal(|| None::<SwipeSession>);
    let close_swipe = use_signal(|| None::<SwipeSession>);
    // Only track pointer_down_time on non-wasm targets
    #[cfg(not(target_arch = "wasm32"))]
    let pointer_down_time = use_signal(|| None::<time::Instant>);
    #[cfg(target_arch = "wasm32")]
    let pointer_down_time = use_signal(|| None::<js_sys::Date>);

    let nav_class = if sidebar_open() {
        "action-bar action-bar--open"
    } else {
        "action-bar"
    };

    rsx! {
        // for small screen - sigests there is a side bar which is closed
        div {
            class: "sidebar-handle",
            role: "button",
            aria_label: "Open navigation",
            onclick: move |_| *sidebar_open.write() = true,
            onpointerdown: move |evt| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let now = std::time::Instant::now();
                    let mut mut_ptr_down_time = pointer_down_time;
                    *mut_ptr_down_time.write() = Some(now);
                }
                #[cfg(target_arch = "wasm32")]
                {
                    let mut mut_ptr_down_time = pointer_down_time;
                    *mut_ptr_down_time.write() = Some(js_sys::Date::new_0());
                }
                begin_swipe(open_swipe.clone(), &evt);
            },
            onpointerup: move |evt| {
                if was_click(pointer_down_time.clone()) {
                    cancel_swipe(open_swipe);
                    sidebar_open.set(true);
                }
                finalize_swipe(
                    SwipeDirection::Opening,
                    open_swipe.clone(),
                    &evt,
                    sidebar_open.clone(),
                    true,
                );
            },
            onpointercancel: move |_| {
                cancel_swipe(open_swipe.clone());
            },
            span { class: "sidebar-handle__hint", ">>" }
        }
        // after user starts to open side bar, overlay is needed to keep tracking of pointer events
        if open_swipe().is_some() {
            div {
                class: "action-bar__overlay",
                onpointerup: move |evt| {
                    if was_click(pointer_down_time.clone()) {
                        cancel_swipe(open_swipe);
                        sidebar_open.set(true);
                    }
                    finalize_swipe(
                        SwipeDirection::Opening,
                        open_swipe.clone(),
                        &evt,
                        sidebar_open.clone(),
                        true,
                    );
                },
                onpointerleave: move |evt| {
                    finalize_swipe(
                        SwipeDirection::Opening,
                        open_swipe.clone(),
                        &evt,
                        sidebar_open.clone(),
                        true,
                    );
                },
                onpointercancel: move |_| {
                    cancel_swipe(open_swipe.clone());
                },
            }
        }
        // side bar
        nav {
            class: nav_class,
            onpointerdown: move |evt| {
                if !sidebar_open() {
                    return;
                }
                begin_swipe(close_swipe.clone(), &evt);
            },
            onpointerup: move |evt| {
                finalize_swipe(
                    SwipeDirection::Closing,
                    close_swipe.clone(),
                    &evt,
                    sidebar_open.clone(),
                    false,
                );
            },
            onpointerleave: move |evt| {
                finalize_swipe(
                    SwipeDirection::Closing,
                    close_swipe.clone(),
                    &evt,
                    sidebar_open.clone(),
                    false,
                );
            },
            onpointercancel: move |_| {
                cancel_swipe(close_swipe.clone());
            },
            // visible only on small screens when side bar is open
            button {
                class: "action-bar__close",
                aria_label: "Close navigation",
                onclick: move |_| *sidebar_open.write() = false,
                "x"
            }
            // real side bar content
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

fn begin_swipe(mut swipe: Signal<Option<SwipeSession>>, evt: &PointerEvent) {
    *swipe.write() = Some(SwipeSession {
        start_x: pointer_x(evt),
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn was_click(mut ptr_down_time: Signal<Option<time::Instant>>) -> bool {
    if let Some(down_time) = ptr_down_time() {
        // events dont have timestamp if they are not in web, so get time of handling this event
        let elapsed = down_time.elapsed().as_millis();
        *ptr_down_time.write() = None;
        if elapsed < CLICK_TIME_DELTA_MS {
            return true;
        }
    }

    false
}
#[cfg(target_arch = "wasm32")]
fn was_click(mut ptr_down_time: Signal<Option<js_sys::Date>>) -> bool {
    if let Some(down_time) = ptr_down_time() {
        let now = js_sys::Date::new_0();
        let elapsed = now.get_time() - down_time.get_time();
        *ptr_down_time.write() = None;
        if elapsed < CLICK_TIME_DELTA_MS as f64 {
            return true;
        }
    }

    false
}

fn finalize_swipe(
    direction: SwipeDirection,
    mut swipe: Signal<Option<SwipeSession>>,
    evt: &PointerEvent,
    mut sidebar_open: Signal<bool>,
    target_state: bool,
) -> bool {
    if let Some(session) = swipe() {
        let delta = direction.delta(session.start_x, pointer_x(evt));
        if delta > SWIPE_THRESHOLD {
            *sidebar_open.write() = target_state;
        }
        *swipe.write() = None;
        return true;
    }

    false
}

fn cancel_swipe(swipe: Signal<Option<SwipeSession>>) -> bool {
    if swipe().is_some() {
        let mut swipe_state = swipe;
        *swipe_state.write() = None;
        return true;
    }

    false
}
