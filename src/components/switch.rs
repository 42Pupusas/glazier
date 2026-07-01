//! [`Switch`] — a toggle, mirroring shadcn's `<Switch>`.
//!
//! A pill track that is `primary` when on and `input` when off, with a circular
//! `card` thumb that slides between the ends. Borrows `&mut bool`.

use egui::{Response, Sense, Ui, Vec2, Widget};

use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry for [`Switch`] — reach in via [`Switch::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct SwitchMetrics {
    /// Track size (shadcn switch is 36×20).
    pub track: Vec2,
    /// Thumb diameter.
    pub thumb: f32,
    /// Inset between the track edge and the thumb at rest.
    pub inset: f32,
}

impl Default for SwitchMetrics {
    fn default() -> Self {
        Self {
            track: Vec2::new(36.0, 20.0),
            thumb: 16.0,
            inset: 2.0,
        }
    }
}

/// A boolean toggle switch.
///
/// ```no_run
/// use glazier::switch::Switch;
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// let mut on = true;
/// Switch::new(&mut on).ui(ui);
/// # });
/// ```
#[must_use = "switches do nothing unless you add them to a Ui"]
pub struct Switch<'a> {
    on: &'a mut bool,
    sizing_hook: SizingHook<SwitchMetrics>,
}

impl<'a> Switch<'a> {
    /// Create a switch bound to `on`.
    pub const fn new(on: &'a mut bool) -> Self {
        Self {
            on,
            sizing_hook: SizingHook::new(),
        }
    }
}

impl Sizeable<SwitchMetrics> for Switch<'_> {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<SwitchMetrics> {
        &mut self.sizing_hook
    }
}

impl Widget for Switch<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(self.sizing_hook);
        let (rect, mut response) = ui.allocate_at_least(m.track, Sense::click());

        if response.clicked() {
            *self.on = !*self.on;
            response.mark_changed();
        }

        if ui.is_rect_visible(rect) {
            // Animate the thumb position on toggle.
            let t = ui.ctx().animate_bool(response.id, *self.on);
            let track_color = tokens.input.lerp_to_gamma(tokens.primary, t);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let radius = (rect.height() / 2.0) as u8;
            let painter = ui.painter();
            painter.rect_filled(rect, radius, track_color);

            let travel = m.inset.mul_add(-2.0, rect.width() - m.thumb);
            let cx = rect.left() + m.inset + m.thumb / 2.0 + travel * t;
            let cy = rect.center().y;
            // Opaque `card` fill — `background` is the app-canvas token and
            // may be translucent under a user theme, which would make the
            // thumb see-through against the track.
            painter.circle_filled(egui::pos2(cx, cy), m.thumb / 2.0, tokens.card);
        }

        response
    }
}
