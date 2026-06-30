//! [`Switch`] — a toggle, mirroring shadcn's `<Switch>`.
//!
//! A pill track that is `primary` when on and `input` when off, with a circular
//! `background` thumb that slides between the ends. Borrows `&mut bool`.

use egui::{Response, Sense, Ui, Vec2, Widget};

use crate::tokens::Tokens;

/// shadcn switch dimensions: track 36×20, thumb 16, 2px inset.
const TRACK: Vec2 = Vec2::new(36.0, 20.0);
const THUMB: f32 = 16.0;
const INSET: f32 = 2.0;

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
}

impl<'a> Switch<'a> {
    /// Create a switch bound to `on`.
    pub const fn new(on: &'a mut bool) -> Self {
        Self { on }
    }
}

impl Widget for Switch<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let (rect, mut response) = ui.allocate_at_least(TRACK, Sense::click());

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

            let travel = INSET.mul_add(-2.0, rect.width() - THUMB);
            let cx = rect.left() + INSET + THUMB / 2.0 + travel * t;
            let cy = rect.center().y;
            painter.circle_filled(egui::pos2(cx, cy), THUMB / 2.0, tokens.background);
        }

        response
    }
}
