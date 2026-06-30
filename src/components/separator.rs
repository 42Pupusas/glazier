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

        let (rect, response) = match self.orientation {
            Orientation::Horizontal => {
                let width = ui.available_width();
                ui.allocate_at_least(Vec2::new(width, thickness), Sense::hover())
            }
            Orientation::Vertical => {
                let height = ui.available_height();
                ui.allocate_at_least(Vec2::new(thickness, height), Sense::hover())
            }
        };

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let stroke = Stroke::new(thickness, stroke.color);
            match self.orientation {
                Orientation::Horizontal => {
                    let y = rect.center().y;
                    painter.hline(rect.x_range(), y, stroke);
                }
                Orientation::Vertical => {
                    let x = rect.center().x;
                    painter.vline(x, rect.y_range(), stroke);
                }
            }
        }

        response
    }
}
