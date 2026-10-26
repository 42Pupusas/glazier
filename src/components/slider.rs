//! [`Slider`] — a value slider, mirroring shadcn's `<Slider>`.
//!
//! A horizontal `bg-muted` track with a `primary` filled range from the start to
//! the thumb, and a circular `background` thumb ringed in `primary`. Borrows
//! `&mut f32` and clamps it to an inclusive range.

use std::ops::RangeInclusive;

use egui::{Response, Sense, Stroke, Ui, Vec2, Widget};

use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry + timing for [`Slider`] — reach in via [`Slider::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct SliderMetrics {
    /// Track thickness (shadcn: 6px).
    pub track_h: f32,
    /// Track corner radius (shadcn: half `track_h`).
    pub track_radius: u8,
    /// Thumb diameter (shadcn: 16px).
    pub thumb: f32,
    /// Default width when the caller doesn't stretch it.
    pub default_w: f32,
    /// Seconds for the hover thumb-grow transition.
    pub hover_time: f32,
}

impl Default for SliderMetrics {
    fn default() -> Self {
        Self {
            track_h: 6.0,
            track_radius: 3,
            thumb: 16.0,
            default_w: 180.0,
            hover_time: 0.15,
        }
    }
}

/// A horizontal value slider.
///
/// ```no_run
/// use glazier::slider::Slider;
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// let mut volume = 0.5;
/// Slider::new(&mut volume, 0.0..=1.0).ui(ui);
/// # });
/// ```
#[must_use = "sliders do nothing unless you add them to a Ui"]
pub struct Slider<'a> {
    value: &'a mut f32,
    range: RangeInclusive<f32>,
    step: Option<f32>,
    width: Option<f32>,
    sizing_hook: SizingHook<SliderMetrics>,
}

impl<'a> Slider<'a> {
    /// Create a slider bound to `value`, constrained to `range`.
    pub const fn new(value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        Self {
            value,
            range,
            step: None,
            width: None,
            sizing_hook: SizingHook::new(),
        }
    }

    /// Snap the value to multiples of `step`.
    pub const fn step(mut self, step: f32) -> Self {
        self.step = Some(step);
        self
    }

    /// Fix the track width (default: stretch to available, min 180).
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
}

impl Sizeable<SliderMetrics> for Slider<'_> {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<SliderMetrics> {
        &mut self.sizing_hook
    }
}

impl Widget for Slider<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(self.sizing_hook);
        let (min, max) = (*self.range.start(), *self.range.end());
        let span = (max - min).max(f32::EPSILON);

        let width = self
            .width
            .unwrap_or_else(|| ui.available_width().max(m.default_w));
        let desired = Vec2::new(width, m.thumb);
        let (rect, mut response) = ui.allocate_at_least(desired, Sense::click_and_drag());

        // The thumb centre travels between these x bounds (inset by its radius).
        let x0 = rect.left() + m.thumb / 2.0;
        let x1 = rect.right() - m.thumb / 2.0;
        let travel = (x1 - x0).max(f32::EPSILON);

        // Drag / click sets the value from the pointer position.
        let picked = response.interact_pointer_pos().map(|pos| {
            let raw = ((pos.x - x0) / travel).clamp(0.0, 1.0);
            let v = raw.mul_add(span, min);
            let snapped = self
                .step
                .filter(|s| *s > 0.0)
                .map_or(v, |s| (v / s).round() * s);
            snapped.clamp(min, max)
        });
        if let Some(v) = picked.filter(|v| (v - *self.value).abs() > f32::EPSILON) {
            *self.value = v;
            response.mark_changed();
        }

        let t = ((*self.value - min) / span).clamp(0.0, 1.0);

        if ui.is_rect_visible(rect) {
            let cy = rect.center().y;
            let thumb_x = f32::mul_add(travel, t, x0);
            let painter = ui.painter();

            // Track (full width) + filled range (start → thumb).
            let track = egui::Rect::from_center_size(
                egui::pos2(rect.center().x, cy),
                Vec2::new(rect.width(), m.track_h),
            );
            let radius = m.track_radius;
            painter.rect_filled(track, radius, tokens.muted);
            let filled =
                egui::Rect::from_min_max(track.left_top(), egui::pos2(thumb_x, track.bottom()));
            painter.rect_filled(filled, radius, tokens.primary);

            // Thumb grows slightly on hover/drag.
            let grow = ui.ctx().animate_bool_with_time(
                response.id.with("hover"),
                response.hovered() || response.dragged(),
                m.hover_time,
            );
            let r = 1.0_f32.mul_add(grow, m.thumb / 2.0);
            // Opaque `widget` fill, not `background` — the app canvas token
            // can be translucent, which would make this thumb see-through.
            painter.circle(
                egui::pos2(thumb_x, cy),
                r,
                tokens.widget,
                Stroke::new(2.0, tokens.primary),
            );
        }

        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;

    /// Click `fraction` of the way along a 200pt slider; return the value.
    fn click_at(fraction: f32, step: Option<f32>) -> f32 {
        let ui = TestUi::new();
        let mut value = 0.0;
        let rect = std::cell::Cell::new(egui::Rect::NOTHING);
        let mut build = |ui: &mut Ui| {
            let s = Slider::new(&mut value, 0.0..=100.0).width(200.0);
            let s = match step {
                Some(step) => s.step(step),
                None => s,
            };
            rect.set(ui.add(s).rect);
        };
        ui.frame(&mut build);
        let r = rect.get();
        ui.click(
            egui::pos2(r.width().mul_add(fraction, r.left()), r.center().y),
            &mut build,
        );
        drop(build);
        value
    }

    #[test]
    fn clicking_sets_the_value() {
        assert!((click_at(0.5, None) - 50.0).abs() < 1.0);
        assert!(click_at(0.0, None).abs() < f32::EPSILON);
        assert!((click_at(1.0, None) - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn a_step_snaps_the_value() {
        assert!((click_at(0.43, Some(25.0)) - 50.0).abs() < f32::EPSILON);
        assert!((click_at(0.43, Some(0.0)) - 43.0).abs() < 1.5);
    }
}
