use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub struct SidebarLayoutContext {
    pub sidebar_width: Signal<f32>,
}

/// Convenience hook to access the current sidebar width context.
pub fn use_sidebar_width() -> Signal<f32> {
    use_context::<SidebarLayoutContext>().sidebar_width
}
