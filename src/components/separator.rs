//! [`Separator`] — a hairline divider, mirroring shadcn's `<Separator>`.

use egui::{Response, Sense, Stroke, Ui, Vec2, Widget};

/// Orientation of a [`Separator`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Orientation {
    /// A full-width horizontal rule.
    #[default]
    Horizontal,
    /// A full-height vertical rule.
    Vertical,
}

/// A thin divider line.
///
/// ```no_run
/// use glazier::separator::{Separator, Orientation};
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// Separator::new().ui(ui);
/// Separator::new().orientation(Orientation::Vertical).ui(ui);
/// # });
/// ```
#[must_use = "separators do nothing unless you add them to a Ui"]
#[derive(Default)]
pub struct Separator {
    orientation: Orientation,
}

impl Separator {
    /// Create a horizontal separator.
    pub const fn new() -> Self {
        Self {
            orientation: Orientation::Horizontal,
        }
    }

    /// Set the [`Orientation`].
    pub const fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }
}

impl Widget for Separator {
    fn ui(self, ui: &mut Ui) -> Response {
        let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
        let thickness = stroke.width.max(1.0);
        let avail = ui.available_size();
        let size = match self.orientation {
            Orientation::Horizontal => Vec2::new(avail.x, thickness),
            Orientation::Vertical => Vec2::new(thickness, avail.y),
        };
        let (rect, response) = ui.allocate_at_least(size, Sense::hover());
        // A line through the rect's centre along its long side.
        let c = rect.center();
        let ends = match self.orientation {
            Orientation::Horizontal => {
                [egui::pos2(rect.left(), c.y), egui::pos2(rect.right(), c.y)]
            }
            Orientation::Vertical => [egui::pos2(c.x, rect.top()), egui::pos2(c.x, rect.bottom())],
        };
        if ui.is_rect_visible(rect) {
            ui.painter()
                .line_segment(ends, Stroke::new(thickness, stroke.color));
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;

    #[test]
    fn each_orientation_spans_its_axis() {
        let mut sizes = Vec::new();
        TestUi::new().frame(|ui| {
            sizes.push(Separator::new().ui(ui).rect.size());
            ui.horizontal(|ui| {
                ui.set_height(40.0);
                let s = Separator::default().orientation(Orientation::Vertical);
                sizes.push(s.ui(ui).rect.size());
            });
        });
        assert!(sizes[0].x > 100.0 && sizes[0].y <= 1.0 + f32::EPSILON);
        assert!(sizes[1].y >= 40.0 && sizes[1].x <= 1.0 + f32::EPSILON);
    }
}
