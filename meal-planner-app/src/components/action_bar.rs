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
#[cfg(not(target_os = "android"))]
mod non_android_constants {
    pub(super) const SWIPE_THRESHOLD: f32 = 60.0;
    pub(super) const CLICK_TIME_DELTA_MS: u128 = 600;
}
#[cfg(target_os = "android")]
mod android_constants {
    pub(super) const SWIPE_THRESHOLD: f32 = 15.0;
    pub(super) const CLICK_TIME_DELTA_MS: u128 = 1000;
}

#[allow(clippy::wildcard_imports)]
#[cfg(target_os = "android")]
use android_constants::*;
#[allow(clippy::wildcard_imports)]
#[cfg(not(target_os = "android"))]
use non_android_constants::*;

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
        // for small screen - suggests there is a side bar which is closed
        div {
            class: "sidebar-handle",
            role: "button",
            aria_label: "Open navigation",
            onclick: move |_| *sidebar_open.write() = true,
            onpointerdown: move |evt| {
                record_pointer_down(pointer_down_time);
                begin_swipe(open_swipe, &evt);
            },
            span { class: "sidebar-handle__hint", ">>" }
        }
        // after user starts to open side bar, overlay is needed to keep tracking of pointer events
        if open_swipe().is_some() {
            div {
                class: "action-bar__overlay",
                onpointerup: move |evt| {
                    if was_click(pointer_down_time) {
                        cancel_swipe(open_swipe);
                        sidebar_open.set(true);
                    }
                    calc_swipe(SwipeDirection::Opening, open_swipe, &evt, sidebar_open, true);
                    cancel_swipe(open_swipe);
                },
                // if pointer was down already check if delta is over threshold
                onpointermove: move |evt| {
                    calc_swipe(SwipeDirection::Opening, open_swipe, &evt, sidebar_open, true);
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
                begin_swipe(close_swipe, &evt);
            },
            onpointerup: move |evt| {
                calc_swipe(SwipeDirection::Closing, close_swipe, &evt, sidebar_open, false);
                cancel_swipe(close_swipe);
            },
            onpointermove: move |evt| {
                calc_swipe(SwipeDirection::Closing, close_swipe, &evt, sidebar_open, false);
            },
            // visible only on small screens when side bar is open
            button {
                class: "action-bar__close",
                aria_label: "Close navigation",
                onclick: move |_| sidebar_open.set(false),
                "x"
            }
            // real side bar content
            button {
                class: "action-bar__button",
                onclick: move |_| {
                    selection.set(ViewKind::MealPlan);
                    sidebar_open.set(false);
                },
                "Meal Plan"
            }
            button {
                class: "action-bar__button",
                onclick: move |_| {
                    selection.set(ViewKind::SwapFood);
                    sidebar_open.set(false);
                },
                "Swap Foods"
            }
            button {
                class: "action-bar__button",
                onclick: move |_| {
                    selection.set(ViewKind::DbManager);
                    sidebar_open.set(false);
                },
                "DB Manager"
            }
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
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
        ptr_down_time.set(None);
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
        ptr_down_time.set(None);
        if elapsed < CLICK_TIME_DELTA_MS as f64 {
            return true;
        }
    }

    false
}

fn calc_swipe(
    direction: SwipeDirection,
    mut swipe: Signal<Option<SwipeSession>>,
    evt: &PointerEvent,
    mut sidebar_open: Signal<bool>,
    target_state: bool,
) {
    if let Some(session) = swipe() {
        let delta = direction.delta(session.start_x, pointer_x(evt));
        if delta > SWIPE_THRESHOLD {
            sidebar_open.set(target_state);
            swipe.set(None);
        }
    }
}

fn cancel_swipe(mut swipe: Signal<Option<SwipeSession>>) {
    swipe.set(None);
}

#[cfg(not(target_arch = "wasm32"))]
fn record_pointer_down(mut ptr_down_time: Signal<Option<time::Instant>>) {
    *ptr_down_time.write() = Some(time::Instant::now());
}

#[cfg(target_arch = "wasm32")]
fn record_pointer_down(mut ptr_down_time: Signal<Option<js_sys::Date>>) {
    *ptr_down_time.write() = Some(js_sys::Date::new_0());
}
