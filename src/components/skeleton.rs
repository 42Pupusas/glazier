//! [`Skeleton`] — a pulsing placeholder block, mirroring shadcn's `<Skeleton>`.
//!
//! Used while content loads. The fill gently pulses between two alpha levels;
//! the widget requests repaints to keep the animation running.

use egui::{Response, Sense, Ui, Vec2, Widget};

use crate::tokens::Tokens;

/// A pulsing placeholder rectangle.
///
/// ```no_run
/// use glazier::skeleton::Skeleton;
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// Skeleton::new([120.0, 16.0]).ui(ui);          // a line of text
/// Skeleton::new([40.0, 40.0]).rounded(20).ui(ui); // an avatar circle
/// # });
/// ```
#[must_use = "skeletons do nothing unless you add them to a Ui"]
pub struct Skeleton {
    size: Vec2,
    radius: u8,
}

impl Skeleton {
    /// Create a skeleton of the given `[width, height]`.
    pub fn new(size: impl Into<Vec2>) -> Self {
        Self {
            size: size.into(),
            radius: 6,
        }
    }

    /// Set the corner radius (use half the height for a pill / circle).
    pub const fn rounded(mut self, radius: u8) -> Self {
        self.radius = radius;
        self
    }
}

impl Widget for Skeleton {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_at_least(self.size, Sense::hover());

        if ui.is_rect_visible(rect) {
            // Pulse alpha with a sine wave on wall-clock time.
            let time = ui.input(|i| i.time);
            let phase = (time * std::f64::consts::PI).sin().mul_add(0.5, 0.5); // 0..1
            #[allow(clippy::cast_possible_truncation)]
            let t = phase as f32;

            let base = Tokens::get(ui).muted;
            let fill = base.gamma_multiply(0.4f32.mul_add(t, 0.6));
            ui.painter().rect_filled(rect, self.radius, fill);

            ui.ctx().request_repaint(); // keep the pulse going
        }

        response
    }
}
