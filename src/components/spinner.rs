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
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Default edge length in points — shadcn's `size-4`.
const DEFAULT_SIZE: f32 = 16.0;

/// Overridable geometry/timing for [`Spinner`] — reach in via
/// [`Spinner::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct SpinnerMetrics {
    /// Revolutions per second (CSS `animate-spin` is 1s per turn).
    pub speed: f64,
    /// Fraction of the circle left open as the trailing gap.
    pub gap: f32,
    /// Number of straight segments approximating the arc.
    pub steps: usize,
}

impl Default for SpinnerMetrics {
    fn default() -> Self {
        Self {
            speed: 1.0,
            gap: 0.25,
            steps: 48,
        }
    }
}

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
    sizing_hook: SizingHook<SpinnerMetrics>,
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
            sizing_hook: SizingHook::default(),
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

impl Sizeable<SpinnerMetrics> for Spinner {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<SpinnerMetrics> {
        &mut self.sizing_hook
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
        let m = crate::sizing::resolve(self.sizing_hook);

        let (rect, response) = ui.allocate_at_least(Vec2::splat(self.size), Sense::hover());

        if ui.is_rect_visible(rect) {
            let time = ui.input(|i| i.time);
            #[allow(clippy::cast_possible_truncation)]
            let phase = (time * m.speed).rem_euclid(1.0) as f32; // 0..1 turn
            let start = phase * std::f32::consts::TAU;

            let center = rect.center();
            let radius = (self.size - thickness) / 2.0;
            let stroke = Stroke::new(thickness, color);

            // Draw the arc as a fan of short segments (egui has no arc
            // primitive). Leave `m.gap` of the circle open as the tail.
            let sweep = (1.0 - m.gap) * std::f32::consts::TAU;
            let mut prev = None;
            for i in 0..=m.steps {
                #[allow(clippy::cast_precision_loss)]
                let t = i as f32 / m.steps as f32;
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
