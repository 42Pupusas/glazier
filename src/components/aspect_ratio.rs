//! [`AspectRatio`] — a fixed-ratio container, mirroring shadcn's `<AspectRatio>`.
//!
//! Allocates a box whose height is derived from the available width and the
//! requested ratio, then renders child content clipped to that box.

use egui::{Response, Sense, Ui, Vec2, Widget};

/// A container that locks its content to a fixed width : height ratio.
///
/// ```no_run
/// use glazier::aspect_ratio::AspectRatio;
/// # egui::__run_test_ui(|ui| {
/// AspectRatio::new(16.0 / 9.0).show(ui, |ui| {
///     ui.label("16:9 content");
/// });
/// # });
/// ```
#[must_use = "aspect-ratio containers do nothing unless shown"]
pub struct AspectRatio {
    ratio: f32,
    width: Option<f32>,
}

impl AspectRatio {
    /// Create a container with the given `width / height` ratio.
    pub const fn new(ratio: f32) -> Self {
        Self { ratio, width: None }
    }

    /// Override the width (defaults to the available width).
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Show child content inside the ratio-locked box.
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> Response {
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let height = width / self.ratio.max(f32::EPSILON);
        let (rect, response) = ui.allocate_at_least(Vec2::new(width, height), Sense::hover());

        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(*ui.layout()));
        child.set_clip_rect(rect);
        content(&mut child);

        response
    }
}

impl Widget for AspectRatio {
    /// Renders an empty ratio-locked box. Use [`AspectRatio::show`] to fill it.
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui, |_| {})
    }
}
