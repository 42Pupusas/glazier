//! [`Checkbox`] — a checkbox, mirroring shadcn's `<Checkbox>`.
//!
//! A rounded 16px square: bordered/transparent when off, `primary`-filled with a
//! contrasting check glyph when on. Borrows `&mut bool`, optional trailing label.

use egui::{Response, Sense, Stroke, Ui, Vec2, Widget};

use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry/timing for [`Checkbox`] — reach in via
/// [`Checkbox::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct CheckboxMetrics {
    /// Box edge length (shadcn `size-4`).
    pub box_size: f32,
    /// Gap between the box and the trailing label.
    pub label_gap: f32,
    /// Seconds for the on/off fill + check transition.
    pub toggle_time: f32,
}

impl Default for CheckboxMetrics {
    fn default() -> Self {
        Self {
            box_size: 16.0,
            label_gap: 8.0,
            toggle_time: 0.15,
        }
    }
}

/// A boolean checkbox with an optional label.
///
/// ```no_run
/// use glazier::checkbox::Checkbox;
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// let mut checked = false;
/// Checkbox::new(&mut checked).label("Accept terms").ui(ui);
/// # });
/// ```
#[must_use = "checkboxes do nothing unless you add them to a Ui"]
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    label: Option<String>,
    sizing_hook: SizingHook<CheckboxMetrics>,
}

impl<'a> Checkbox<'a> {
    /// Create a checkbox bound to `checked`.
    pub const fn new(checked: &'a mut bool) -> Self {
        Self {
            checked,
            label: None,
            sizing_hook: SizingHook::new(),
        }
    }

    /// Add a trailing label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl Sizeable<CheckboxMetrics> for Checkbox<'_> {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<CheckboxMetrics> {
        &mut self.sizing_hook
    }
}

impl Widget for Checkbox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(self.sizing_hook);

        // Lay out the label (if any) to size the click target.
        let gap = m.label_gap;
        let galley = self.label.as_ref().map(|text| {
            ui.painter().layout_no_wrap(
                text.clone(),
                egui::TextStyle::Body.resolve(ui.style()),
                tokens.foreground,
            )
        });
        let label_w = galley.as_ref().map_or(0.0, |g| gap + g.size().x);
        let height = galley
            .as_ref()
            .map_or(m.box_size, |g| g.size().y.max(m.box_size));

        let (rect, mut response) =
            ui.allocate_at_least(Vec2::new(m.box_size + label_w, height), Sense::click());

        if response.clicked() {
            *self.checked = !*self.checked;
            response.mark_changed();
        }

        // Eased on/off value so the fill, border and check glide in.
        let t =
            ui.ctx()
                .animate_bool_with_time(response.id.with("on"), *self.checked, m.toggle_time);

        if ui.is_rect_visible(rect) {
            let box_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left(), rect.center().y - m.box_size / 2.0),
                Vec2::splat(m.box_size),
            );
            let radius = tokens.radius_sm();
            let painter = ui.painter();
            // Border fades out as the fill fades in; fill lerps from the empty
            // (opaque `widget`) surface to `primary` — not `background`, which
            // may be translucent under a user theme.
            let fill = tokens.widget.lerp_to_gamma(tokens.primary, t);
            let border = Stroke::new(1.0, tokens.input.gamma_multiply(1.0 - t));
            painter.rect(box_rect, radius, fill, border, egui::StrokeKind::Inside);
            if t > 0.01 {
                // Check mark draws in proportionally; colour fades with `t`.
                let center = box_rect.center();
                let s = m.box_size * 0.28;
                let col = tokens.primary_foreground.gamma_multiply(t);
                let stroke = Stroke::new(2.0, col);
                let p0 = egui::pos2(center.x - s, center.y + s * 0.1);
                let elbow = egui::pos2(center.x - s * 0.2, center.y + s * 0.8);
                let p1 = egui::pos2(center.x + s, center.y - s * 0.7);
                // Reveal the two strokes across the first/second half of `t`.
                let short = (t * 2.0).min(1.0);
                let long = t.mul_add(2.0, -1.0).clamp(0.0, 1.0);
                painter.line_segment([p0, p0.lerp(elbow, short)], stroke);
                if long > 0.0 {
                    painter.line_segment([elbow, elbow.lerp(p1, long)], stroke);
                }
            }

            if let Some(galley) = galley {
                let text_pos = egui::pos2(
                    box_rect.right() + gap,
                    rect.center().y - galley.size().y / 2.0,
                );
                ui.painter().galley(text_pos, galley, tokens.foreground);
            }
        }

        response
    }
}
