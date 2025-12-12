use dioxus::prelude::*;

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

const SWIPE_THRESHOLD: f32 = 60.0;
const HAS_POINTER_CAPTURE: bool = cfg!(all(feature = "web", target_arch = "wasm32"));

#[component]
pub fn ActionBar(mut selection: Signal<ViewKind>, mut sidebar_open: Signal<bool>) -> Element {
    let open_swipe = use_signal(|| None::<SwipeSession>);
    let close_swipe = use_signal(|| None::<SwipeSession>);

    let nav_class = if sidebar_open() {
        "action-bar action-bar--open"
    } else {
        "action-bar"
    };

    let overlay_active =
        !HAS_POINTER_CAPTURE && (open_swipe().is_some() || close_swipe().is_some());

    rsx! {
        if overlay_active {
            div {
                class: "swipe-overlay",
                style: "position: fixed; inset: 0; z-index: 24; touch-action: none; background: transparent;",
                onpointermove: move |evt| {
                    let mut handled = false;
                    if process_swipe_move(
                        SwipeDirection::Opening,
                        open_swipe.clone(),
                        &evt,
                        sidebar_open.clone(),
                        true,
                    ) {
                        handled = true;
                    }
                    if process_swipe_move(
                        SwipeDirection::Closing,
                        close_swipe.clone(),
                        &evt,
                        sidebar_open.clone(),
                        false,
                    ) {
                        handled = true;
                    }
                    if handled {
                        release_pointer(&evt);
                    }
                },
                onpointerup: move |evt| {
                    let mut handled = false;
                    if finalize_swipe(
                        SwipeDirection::Opening,
                        open_swipe.clone(),
                        &evt,
                        sidebar_open.clone(),
                        true,
                    ) {
                        handled = true;
                    }
                    if finalize_swipe(
                        SwipeDirection::Closing,
                        close_swipe.clone(),
                        &evt,
                        sidebar_open.clone(),
                        false,
                    ) {
                        handled = true;
                    }
                    if handled {
                        release_pointer(&evt);
                    }
                },
                onpointercancel: move |evt| {
                    let cancelled_open = cancel_swipe(open_swipe.clone());
                    let cancelled_close = cancel_swipe(close_swipe.clone());
                    if cancelled_open || cancelled_close {
                        release_pointer(&evt);
                    }
                },
            }
        }
        div {
            class: "sidebar-handle",
            role: "button",
            aria_label: "Open navigation",
            onpointerdown: move |evt| {
                begin_swipe(open_swipe.clone(), &evt);
                capture_pointer(&evt);
            },
            onpointermove: move |evt| {
                if process_swipe_move(
                    SwipeDirection::Opening,
                    open_swipe.clone(),
                    &evt,
                    sidebar_open.clone(),
                    true,
                ) {
                    release_pointer(&evt);
                }
            },
            onpointerup: move |evt| {
                if finalize_swipe(
                    SwipeDirection::Opening,
                    open_swipe.clone(),
                    &evt,
                    sidebar_open.clone(),
                    true,
                ) {
                    release_pointer(&evt);
                }
            },
            onpointercancel: move |evt| {
                if cancel_swipe(open_swipe.clone()) {
                    release_pointer(&evt);
                }
            },
            onclick: move |_| *sidebar_open.write() = true,
            span { class: "sidebar-handle__hint", "›" }
        }
        nav {
            class: nav_class,
            onpointerdown: move |evt| {
                if !sidebar_open() {
                    return;
                }
                begin_swipe(close_swipe.clone(), &evt);
                capture_pointer(&evt);
            },
            onpointermove: move |evt| {
                if process_swipe_move(
                    SwipeDirection::Closing,
                    close_swipe.clone(),
                    &evt,
                    sidebar_open.clone(),
                    false,
                ) {
                    release_pointer(&evt);
                }
            },
            onpointerup: move |evt| {
                if finalize_swipe(
                    SwipeDirection::Closing,
                    close_swipe.clone(),
                    &evt,
                    sidebar_open.clone(),
                    false,
                ) {
                    release_pointer(&evt);
                }
            },
            onpointercancel: move |evt| {
                if cancel_swipe(close_swipe.clone()) {
                    release_pointer(&evt);
                }
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

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn capture_pointer(evt: &PointerEvent) {
    use wasm_bindgen::JsCast;

    if let Some(web_event) = evt.data().downcast::<web_sys::PointerEvent>() {
        if let Some(target) = web_event
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let _ = target.set_pointer_capture(web_event.pointer_id());
        }
    }
}

#[cfg(not(all(feature = "web", target_arch = "wasm32")))]
fn capture_pointer(_: &PointerEvent) {}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn release_pointer(evt: &PointerEvent) {
    use wasm_bindgen::JsCast;

    if let Some(web_event) = evt.data().downcast::<web_sys::PointerEvent>() {
        if let Some(target) = web_event
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let _ = target.release_pointer_capture(web_event.pointer_id());
        }
    }
}

#[cfg(not(all(feature = "web", target_arch = "wasm32")))]
fn release_pointer(_: &PointerEvent) {}

fn begin_swipe(swipe: Signal<Option<SwipeSession>>, evt: &PointerEvent) {
    let mut swipe_state = swipe;
    *swipe_state.write() = Some(SwipeSession {
        start_x: pointer_x(evt),
    });
}

fn process_swipe_move(
    direction: SwipeDirection,
    swipe: Signal<Option<SwipeSession>>,
    evt: &PointerEvent,
    sidebar_open: Signal<bool>,
    target_state: bool,
) -> bool {
    if let Some(session) = swipe() {
        let delta = direction.delta(session.start_x, pointer_x(evt));
        if delta > SWIPE_THRESHOLD {
            let mut sidebar_state = sidebar_open;
            *sidebar_state.write() = target_state;
            let mut swipe_state = swipe;
            *swipe_state.write() = None;
            return true;
        }
    }

    false
}

fn finalize_swipe(
    direction: SwipeDirection,
    swipe: Signal<Option<SwipeSession>>,
    evt: &PointerEvent,
    sidebar_open: Signal<bool>,
    target_state: bool,
) -> bool {
    if let Some(session) = swipe() {
        let delta = direction.delta(session.start_x, pointer_x(evt));
        if delta > SWIPE_THRESHOLD {
            let mut sidebar_state = sidebar_open;
            *sidebar_state.write() = target_state;
        }
        let mut swipe_state = swipe;
        *swipe_state.write() = None;
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
