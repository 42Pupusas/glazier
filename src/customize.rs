//! [`Customize`] — one hook to reach into a component's *real* resolved style,
//! sibling to [`Decorate`](crate::Decorate)'s structural wrapping.
//!
//! The wrong way to do this (tried it, reverted it) is bolting an
//! `Option<Color32>` field onto every component for every property someone
//! might want to override — `.fill_override(...)`, `.text_color(...)`,
//! `.outline(...)`, each only covering the handful of fields the library
//! author happened to guess. That's an escape hatch per property, and it
//! still doesn't cover the property nobody guessed.
//!
//! What every component already has is a **real** resolved style value it
//! computes from its `Variant` + the active [`Tokens`](crate::tokens::Tokens)
//! before painting — [`Button`](crate::Button) resolves a
//! [`ButtonStyle`](crate::button::ButtonStyle) `{ fill, stroke, text,
//! underline, radius }`, and most boxed/framed components
//! ([`Card`](crate::Card), [`Alert`](crate::Alert), [`Popover`](crate::Popover),
//! …) build the surface straight out of an [`egui::Frame`] — which is already
//! a public, field-complete struct (`fill`, `stroke`, `corner_radius`,
//! `inner_margin`, `shadow`, …). [`Customize<T>`] gives every component **one**
//! builder method, `.style(...)`, that hands you a `&mut T` to that exact
//! value, right before it's used to paint. No enumeration, no guessing, no
//! gaps — whatever fields the real style record has, you can set.
//!
//! ```no_run
//! use egui::{Color32, CornerRadius, Widget as _};
//! use glazier::button::{Button, ButtonStyle};
//! use glazier::card::Card;
//! use glazier::Customize as _;
//! # egui::__run_test_ui(|ui| {
//! // Hand-painted widget: T is the component's own style struct.
//! Button::new("Save")
//!     .style(|s: &mut ButtonStyle| s.fill = Color32::from_rgb(0x22, 0x88, 0x44))
//!     .ui(ui);
//!
//! // Frame-backed widget: T is egui::Frame itself — its real native style type.
//! Card::new()
//!     .style(|f: &mut egui::Frame| f.corner_radius = CornerRadius::ZERO)
//!     .show(ui, |ui| { ui.label("body"); });
//! # });
//! ```
//!
//! Implementing this for a component is one field + one trait impl:
//!
//! ```ignore
//! pub struct MyWidget { style_hook: StyleHook<MyWidgetStyle>, /* … */ }
//!
//! impl Customize<MyWidgetStyle> for MyWidget {
//!     fn style_hook_mut(&mut self) -> &mut StyleHook<MyWidgetStyle> {
//!         &mut self.style_hook
//!     }
//! }
//! // then, right before painting:
//! let mut resolved = self.variant.resolve(tokens);
//! self.style_hook.apply(&mut resolved);
//! ```

/// A boxed `FnOnce(&mut T)`, applied once to a resolved style value.
type StyleFn<T> = Box<dyn FnOnce(&mut T)>;

/// A one-shot closure, applied to a resolved style value `T` right before a
/// component paints. Stored as a plain field; [`Customize::style`] is the only
/// way callers touch it.
pub struct StyleHook<T>(Option<StyleFn<T>>);

impl<T> Default for StyleHook<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<T> StyleHook<T> {
    /// Record a closure to run against the resolved style, replacing any
    /// previously set one.
    pub fn set(&mut self, f: impl FnOnce(&mut T) + 'static) {
        self.0 = Some(Box::new(f));
    }

    /// Run the recorded closure (if any) against `value`. Call this once,
    /// after computing the default style and before painting with it.
    pub fn apply(self, value: &mut T) {
        if let Some(f) = self.0 {
            f(value);
        }
    }
}

/// Implemented by a component for the concrete style type `T` it resolves
/// before painting.
///
/// `T` is the component's own style struct, or [`egui::Frame`] directly for
/// anything Frame-backed. Provides the single `.style(...)` builder method.
pub trait Customize<T>: Sized {
    /// Mutable access to the pending style hook.
    fn style_hook_mut(&mut self) -> &mut StyleHook<T>;

    /// Reach into this component's resolved style (its own style struct, or
    /// the [`egui::Frame`] it paints with) and change anything about it.
    #[must_use]
    fn style(mut self, f: impl FnOnce(&mut T) + 'static) -> Self {
        self.style_hook_mut().set(f);
        self
    }
}
