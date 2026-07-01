//! [`Empty`] — a centered empty-state placeholder, mirroring shadcn's
//! `<Empty>`.
//!
//! Used when a list, search, or panel has nothing to show. A vertically
//! centered column: an optional media [`Icon`] in a rounded tile, a semibold
//! title, a muted description, and an optional `content` slot for actions
//! (buttons, links). Matches shadcn's `Empty` / `EmptyHeader` / `EmptyMedia` /
//! `EmptyTitle` / `EmptyDescription` / `EmptyContent` composition.
//!
//! ```no_run
//! use glazier::empty::Empty;
//! use glazier::icon::Icon;
//! use egui::Widget as _;
//! # const FILES: &str = "<svg/>";
//! # egui::__run_test_ui(|ui| {
//! Empty::new("No projects yet")
//!     .icon(Icon::new(FILES))
//!     .description("Create your first project to get started.")
//!     .show(ui, |ui| {
//!         let _ = ui.button("New project");
//!     });
//! # });
//! ```

use egui::{Frame, Response, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::components::icon::Icon;
use crate::customize::{Customize, StyleHook};
use crate::tokens::Tokens;

/// Edge length of the rounded media tile (shadcn `size-10`).
const MEDIA_TILE: f32 = 40.0;

/// A centered empty-state placeholder.
#[must_use = "empty states do nothing unless you show them"]
pub struct Empty {
    title: String,
    description: Option<String>,
    icon: Option<Icon>,
    style_hook: StyleHook<Frame>,
}

impl Customize<Frame> for Empty {
    fn style_hook_mut(&mut self) -> &mut StyleHook<Frame> {
        &mut self.style_hook
    }
}

impl Empty {
    /// Create an empty state with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            icon: None,
            style_hook: StyleHook::new(),
        }
    }

    /// Set the muted description line(s) below the title.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a leading media [`Icon`], shown in a rounded tile above the title.
    pub const fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Render the empty state, with an optional `content` slot beneath the text
    /// for actions (buttons, links). The closure runs centered.
    pub fn show<R>(mut self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let tokens = Tokens::get(ui);
        let mut out = None;

        let mut frame = Frame::new().inner_margin(24.0); // p-6
        std::mem::take(&mut self.style_hook).apply(&mut frame);

        frame
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;

                    if let Some(icon) = self.icon {
                        media_tile(ui, tokens, icon);
                        ui.add_space(4.0);
                    }

                    ui.label(
                        RichText::new(&self.title)
                            .font(crate::fonts::semibold(ui, 16.0))
                            .color(tokens.foreground),
                    );

                    if let Some(description) = &self.description {
                        ui.label(
                            RichText::new(description)
                                .color(tokens.muted_foreground)
                                .size(13.0),
                        );
                    }

                    out = Some(content(ui));
                });
            });

        out
    }
}

impl egui::Widget for Empty {
    /// Render the empty state with no action content.
    fn ui(self, ui: &mut Ui) -> Response {
        Frame::new()
            .inner_margin(24.0)
            .show(ui, |ui| {
                self.show(ui, |_| {});
            })
            .response
    }
}

/// Paint the rounded media tile holding the icon (shadcn `bg-muted` `rounded-lg`).
fn media_tile(ui: &mut Ui, tokens: Tokens, icon: Icon) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(MEDIA_TILE), Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect(
            rect,
            tokens.radius_lg(),
            tokens.muted,
            Stroke::NONE,
            StrokeKind::Inside,
        );
        let image = icon.color(tokens.muted_foreground).size(20.0).image(tokens);
        image.paint_at(
            ui,
            egui::Rect::from_center_size(rect.center(), Vec2::splat(20.0)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Widget as _;

    /// The action slot's return value is forwarded.
    #[test]
    fn forwards_content_value() {
        let ctx = egui::Context::default();
        let mut got = None;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            got = Empty::new("Nothing here").show(ui, |_| 7);
        });
        assert_eq!(got, Some(7));
    }

    /// The plain `Widget` form renders without an action slot.
    #[test]
    fn widget_form_renders() {
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let _ = Empty::new("Empty").description("nada").ui(ui);
        });
    }
}
