//! [`Progress`] — a determinate progress bar, mirroring shadcn's `<Progress>`.
//!
//! A rounded track (`bg-primary/20`-style, here `muted`) with a `primary`
//! indicator filling `value` of the width. Stateless: pass a `0.0..=1.0`
//! fraction.

use egui::{Response, Sense, Ui, Vec2, Widget};

use crate::tokens::Tokens;

/// A determinate progress bar.
///
/// ```no_run
/// use glazier::progress::Progress;
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// Progress::new(0.6).ui(ui);
/// Progress::new(0.3).height(6.0).ui(ui);
/// # });
/// ```
#[must_use = "progress bars do nothing unless you add them to a Ui"]
pub struct Progress {
    value: f32,
    height: f32,
    width: Option<f32>,
}

impl Progress {
    /// Create a progress bar filled to `value` (clamped to `0.0..=1.0`).
    pub const fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            height: 8.0,
            width: None,
        }
    }

    /// Set the track height in points (default 8).
    pub const fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Set an explicit width; defaults to the available width.
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
}

impl Widget for Progress {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let (rect, response) = ui.allocate_at_least(Vec2::new(width, self.height), Sense::hover());

        if ui.is_rect_visible(rect) {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let radius = (self.height / 2.0) as u8;
            let painter = ui.painter();
            painter.rect_filled(rect, radius, tokens.muted);
            if self.value > 0.0 {
                let mut fill = rect;
                fill.set_width(rect.width() * self.value);
                painter.rect_filled(fill, radius, tokens.primary);
            }
        }

        response
    }
}
