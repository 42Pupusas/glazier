//! [`Spinner`] — an indeterminate loading indicator, mirroring shadcn's
//! `<Spinner>`.
//!
//! shadcn's spinner is the lucide `loader-circle` glyph spun with CSS
//! `animate-spin`. Rather than rasterise and rotate an SVG, we paint the same
//! shape directly: a `currentColor` ring with a gap, rotating on wall-clock
//! time. The widget requests repaints to keep the animation running.
//!
//! ```no_run
//! use glazier::spinner::Spinner;
//! use egui::Widget as _;
//! # egui::__run_test_ui(|ui| {
//! Spinner::new().ui(ui);                       // size-4, foreground
//! Spinner::new().size(24.0).ui(ui);            // larger
//! # });
//! ```

use egui::{Color32, Response, Sense, Stroke, Ui, Vec2, Widget};

use crate::customize::{Customize, StyleHook};
use crate::tokens::Tokens;

/// Default edge length in points — shadcn's `size-4`.
const DEFAULT_SIZE: f32 = 16.0;
/// Revolutions per second (CSS `animate-spin` is 1s per turn).
const SPEED: f64 = 1.0;
/// Fraction of the circle left open as the trailing gap.
const GAP: f32 = 0.25;
/// Number of straight segments approximating the arc.
const STEPS: usize = 48;

/// [`Spinner`]'s resolved paint — stroke colour and thickness. The real value
/// [`Spinner`] paints with; reach in via [`Spinner::style`].
#[derive(Clone, Copy, Debug)]
pub struct SpinnerStyle {
    /// Arc stroke colour (defaults to `foreground`).
    pub color: Color32,
    /// Stroke width in points (defaults to ~`size / 8`).
    pub thickness: f32,
}

/// An indeterminate spinning loader.
#[must_use = "spinners do nothing unless you add them to a Ui"]
pub struct Spinner {
    size: f32,
    style_hook: StyleHook<SpinnerStyle>,
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Spinner {
    /// Create a spinner at the default `size-4` (16px), tinted `foreground`.
    pub fn new() -> Self {
        Self {
            size: DEFAULT_SIZE,
            style_hook: StyleHook::default(),
        }
    }

    /// Set the square edge length in points (default 16 = `size-4`).
    pub const fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
}

impl Customize<SpinnerStyle> for Spinner {
    fn style_hook_mut(&mut self) -> &mut StyleHook<SpinnerStyle> {
        &mut self.style_hook
    }
}

impl Widget for Spinner {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let mut style = SpinnerStyle {
            color: tokens.foreground,
            thickness: self.size / 8.0,
        };
        self.style_hook.apply(&mut style);
        let SpinnerStyle { color, thickness } = style;

        let (rect, response) = ui.allocate_at_least(Vec2::splat(self.size), Sense::hover());

        if ui.is_rect_visible(rect) {
            let time = ui.input(|i| i.time);
            #[allow(clippy::cast_possible_truncation)]
            let phase = (time * SPEED).rem_euclid(1.0) as f32; // 0..1 turn
            let start = phase * std::f32::consts::TAU;

            let center = rect.center();
            let radius = (self.size - thickness) / 2.0;
            let stroke = Stroke::new(thickness, color);

            // Draw the arc as a fan of short segments (egui has no arc
            // primitive). Leave `GAP` of the circle open as the tail.
            let sweep = (1.0 - GAP) * std::f32::consts::TAU;
            let mut prev = None;
            for i in 0..=STEPS {
                #[allow(clippy::cast_precision_loss)]
                let t = i as f32 / STEPS as f32;
                let a = t.mul_add(sweep, start);
                let p = center + radius * Vec2::angled(a);
                if let Some(prev) = prev {
                    ui.painter().line_segment([prev, p], stroke);
                }
                prev = Some(p);
            }

            ui.ctx().request_repaint(); // keep spinning
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A spinner allocates its requested square and is hover-only.
    #[test]
    fn allocates_requested_size() {
        let ctx = egui::Context::default();
        let mut size = Vec2::ZERO;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let r = Spinner::new().size(24.0).ui(ui);
            size = r.rect.size();
        });
        assert!((size.x - 24.0).abs() < 0.5);
        assert!((size.y - 24.0).abs() < 0.5);
    }
}
