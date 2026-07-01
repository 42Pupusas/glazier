//! [`Sizeable`] — overridable geometry, sibling to [`Customize`](crate::Customize)
//! (paint) and [`Decorate`](crate::Decorate) (structural wrapping).
//!
//! Components are full of small pixel constants — row heights, paddings,
//! gaps, icon sizes, animation durations — picked to match shadcn's Tailwind
//! scale. They're sane defaults, not hard limits, so they shouldn't be
//! `const` dead ends a caller can't reach. [`Sizeable<T>`] gives a
//! metrics-heavy component **one** struct enumerating its own constants as
//! public, `Copy` fields (`XxxMetrics`, with a `Default` reproducing the
//! shipped numbers) and **one** builder method, `.sizing(...)`, that hands
//! you `&mut T` before layout runs — same shape as `Customize`, just for
//! geometry instead of paint.
//!
//! ```no_run
//! use egui::Widget as _;
//! use glazier::badge::{Badge, BadgeMetrics};
//! use glazier::Sizeable as _;
//! # egui::__run_test_ui(|ui| {
//! Badge::new("New")
//!     .sizing(|m: &mut BadgeMetrics| m.icon_gap = 8.0)
//!     .ui(ui);
//! # });
//! ```
//!
//! Implementing this for a component is one field + one trait impl + one
//! `resolve` call:
//!
//! ```ignore
//! #[derive(Clone, Copy, Debug)]
//! pub struct MyWidgetMetrics { pub height: f32, pub pad_x: f32 }
//!
//! impl Default for MyWidgetMetrics {
//!     fn default() -> Self { Self { height: 36.0, pad_x: 12.0 } }
//! }
//!
//! pub struct MyWidget { sizing_hook: SizingHook<MyWidgetMetrics>, /* … */ }
//!
//! impl Sizeable<MyWidgetMetrics> for MyWidget {
//!     fn sizing_hook_mut(&mut self) -> &mut SizingHook<MyWidgetMetrics> {
//!         &mut self.sizing_hook
//!     }
//! }
//! // then, at the top of Widget::ui / show, before any layout math:
//! let metrics = crate::sizing::resolve(self.sizing_hook);
//! // … use metrics.height, metrics.pad_x, … instead of the old consts.
//! ```

/// A boxed `FnOnce(&mut T)`, applied once to a resolved metrics value.
type SizingFn<T> = Box<dyn FnOnce(&mut T)>;

/// A one-shot closure, applied to a component's default-resolved metrics `T`
/// before it lays out. Stored as a plain field; [`Sizeable::sizing`] is the
/// only way callers touch it.
pub struct SizingHook<T>(Option<SizingFn<T>>);

impl<T> Default for SizingHook<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SizingHook<T> {
    /// Create an empty hook (no closure recorded). `const` so it can seed a
    /// component's `const fn new()`.
    #[must_use]
    pub const fn new() -> Self {
        Self(None)
    }
}

impl<T> SizingHook<T> {
    /// Record a closure to run against the resolved metrics, replacing any
    /// previously set one.
    pub fn set(&mut self, f: impl FnOnce(&mut T) + 'static) {
        self.0 = Some(Box::new(f));
    }

    /// Run the recorded closure (if any) against `value`. Call this once,
    /// after building the default metrics and before laying out with it.
    pub fn apply(self, value: &mut T) {
        if let Some(f) = self.0 {
            f(value);
        }
    }
}

/// Build `T::default()`, apply a pending [`SizingHook`], and return the
/// metrics to lay out with. The one-liner every component's `ui`/`show`
/// calls first.
#[must_use]
pub fn resolve<T: Default>(hook: SizingHook<T>) -> T {
    let mut metrics = T::default();
    hook.apply(&mut metrics);
    metrics
}

/// Implemented by a component for the concrete metrics type `T` it resolves
/// before laying out.
///
/// `T` is a plain `Copy` struct of the component's own geometry constants,
/// each field defaulting to the shipped shadcn-matched value. Provides the
/// single `.sizing(...)` builder method.
pub trait Sizeable<T: Default>: Sized {
    /// Mutable access to the pending sizing hook.
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<T>;

    /// Reach into this component's default geometry and change anything
    /// about it — heights, paddings, gaps, icon sizes, animation durations.
    #[must_use]
    fn sizing(mut self, f: impl FnOnce(&mut T) + 'static) -> Self {
        self.sizing_hook_mut().set(f);
        self
    }
}
