//! [`Drawer`] — an edge-anchored modal panel that slides in, mirroring shadcn's
//! `Drawer` (Vaul). Defaults to the bottom edge.
//!
//! A `Drawer` is a [`Sheet`](crate::sheet::Sheet) with a **drag-handle grip**
//! and **inner-edge rounded corners** — the signature Vaul look. Pick the edge
//! with [`side`](Drawer::side) (bottom by default). It shares the modal chrome
//! (dimmed backdrop, optional header, arbitrary body).
//!
//! Clicking the backdrop, pressing Escape, or the × button dismisses it.
//!
//! Drive it from a `bool` you own: flip it `true` to open, and call
//! [`show`](Drawer::show) **every frame** — it animates in and out and paints
//! nothing while fully closed, clearing the `bool` when dismissed.
//!
//! ```no_run
//! use glazier::drawer::Drawer;
//! use glazier::button::Button;
//! use egui::Widget as _;
//! # let ctx = egui::Context::default();
//! # let mut open = true;
//! Drawer::new("Move goal")
//!     .description("Set your daily activity goal.")
//!     .show(&ctx, &mut open, |ui| {
//!         if Button::new("Submit").ui(ui).clicked() {
//!             // …
//!         }
//!     });
//! ```

use egui::{Sense, Vec2};

pub use crate::components::dialog::Side;
use crate::components::dialog::{close_button, modal_shell, ModalStyle};
use crate::fonts;
use crate::tokens::Tokens;

/// Long-axis length of the drag-handle grip pill (Vaul's `w-[100px]`-ish).
const GRIP_LONG: f32 = 48.0;
/// Short-axis thickness of the drag-handle grip pill.
const GRIP_THICK: f32 = 5.0;
/// Default cross-axis size for a left/right drawer (`max-w-sm`, 384px).
const DEFAULT_WIDTH: f32 = 384.0;

/// An edge-anchored modal panel that slides in, with a drag-handle grip.
#[must_use = "drawers do nothing unless you show them"]
pub struct Drawer {
    title: Option<String>,
    description: Option<String>,
    side: Side,
    width: f32,
    show_close: bool,
    show_grip: bool,
}

impl Drawer {
    /// Create a drawer with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            description: None,
            side: Side::Bottom,
            width: DEFAULT_WIDTH,
            show_close: false,
            show_grip: true,
        }
    }

    /// Create a drawer with no header title (grip + body only).
    pub const fn untitled() -> Self {
        Self {
            title: None,
            description: None,
            side: Side::Bottom,
            width: DEFAULT_WIDTH,
            show_close: false,
            show_grip: true,
        }
    }

    /// Set the edge the drawer slides in from (default [`Side::Bottom`]).
    pub const fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// Override the cross-axis size for a left/right drawer (its width).
    /// Ignored for top/bottom drawers, which span the full viewport width.
    pub const fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Set the muted description line below the title.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// Show a top-right close (×) button (off by default — drawers usually
    /// dismiss via the grip, backdrop, or Escape).
    pub const fn show_close(mut self, show: bool) -> Self {
        self.show_close = show;
        self
    }

    /// Show the centered drag-handle grip above the content (on by default).
    pub const fn show_grip(mut self, show: bool) -> Self {
        self.show_grip = show;
        self
    }

    /// Show the drawer. Call this **every frame** — it animates in and out and
    /// paints nothing while fully closed. `content` draws the body; its return
    /// value is forwarded as `Some(R)` while visible (`None` once fully closed).
    /// `open` is cleared to `false` when dismissed (grip, × button, backdrop
    /// click, or Escape).
    pub fn show<R>(
        self,
        ctx: &egui::Context,
        open: &mut bool,
        content: impl FnOnce(&mut egui::Ui) -> R,
    ) -> Option<R> {
        let id = egui::Id::new("glazier-drawer").with(self.title.as_deref().unwrap_or("untitled"));
        let side = self.side;

        let mut closed = false;
        let out = modal_shell(
            ctx,
            id,
            *open,
            ModalStyle::Drawer(side),
            self.width,
            |ui, tokens, _width| {
                ui.spacing_mut().item_spacing = Vec2::new(0.0, 16.0); // gap-4

                // Drag-handle grip on the hugged edge (clicking it dismisses).
                // It only reads naturally on the vertical axis: above the body
                // for a bottom drawer, below it for a top drawer.
                let grip_here = self.show_grip && matches!(side, Side::Top | Side::Bottom);
                if grip_here && side == Side::Bottom && grip(ui, tokens).clicked() {
                    closed = true;
                }

                self.header(ui, tokens, &mut closed);
                let out = content(ui);

                if grip_here && side == Side::Top && grip(ui, tokens).clicked() {
                    closed = true;
                }
                out
            },
        );

        let Some((inner, backdrop_close)) = out else {
            return None; // fully closed
        };

        if *open && (closed || backdrop_close) {
            *open = false; // begins the exit animation on the next frame
        }
        Some(inner)
    }

    /// Paint the header: an optional title row with the top-right close (×)
    /// button, then the muted description beneath.
    fn header(&self, ui: &mut egui::Ui, tokens: Tokens, closed: &mut bool) {
        if self.title.is_none() && self.description.is_none() {
            return;
        }
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0; // gap-1.5

            if let Some(title) = &self.title {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(title)
                            .font(fonts::semibold(ui, 18.0)) // text-lg
                            .color(tokens.foreground),
                    );
                    if self.show_close {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            if close_button(ui, tokens).clicked() {
                                *closed = true;
                            }
                        });
                    }
                });
            }

            if let Some(desc) = &self.description {
                ui.label(
                    egui::RichText::new(desc)
                        .size(14.0) // text-sm
                        .color(tokens.muted_foreground),
                );
            }
        });
    }
}

/// Paint the centered horizontal drag-handle grip: a small rounded `muted`
/// pill, the signature Vaul grabber. Returns its clickable response.
fn grip(ui: &mut egui::Ui, tokens: Tokens) -> egui::Response {
    let (rect, response) =
        ui.allocate_at_least(Vec2::new(ui.available_width(), GRIP_THICK), Sense::click());
    if ui.is_rect_visible(rect) {
        let pill = egui::Rect::from_center_size(rect.center(), Vec2::new(GRIP_LONG, GRIP_THICK));
        ui.painter()
            .rect_filled(pill, GRIP_THICK / 2.0, tokens.muted);
    }
    response.on_hover_cursor(egui::CursorIcon::Grab)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Driving a closed drawer must paint nothing and forward `None`.
    #[test]
    fn closed_drawer_is_inert() {
        let ctx = egui::Context::default();
        let mut open = false;
        let mut ran = false;
        let mut out = Some(0);
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            out = Drawer::new("Move goal").show(ui.ctx(), &mut open, |_| {
                ran = true;
                42
            });
        });
        assert_eq!(out, None);
        assert!(!ran, "content closure must not run while fully closed");
    }

    /// An open drawer runs its content and forwards the closure's value.
    #[test]
    fn open_drawer_forwards_value() {
        let ctx = egui::Context::default();
        let mut open = true;
        let mut out = None;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            out = Drawer::new("Move goal")
                .description("Set your daily goal.")
                .show(ui.ctx(), &mut open, |_| 7);
        });
        assert_eq!(out, Some(7));
        assert!(open, "drawer stays open until dismissed");
    }
}
