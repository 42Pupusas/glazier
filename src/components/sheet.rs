//! [`Sheet`] — an edge-anchored modal panel that slides in from a screen edge,
//! mirroring shadcn's `Sheet`.
//!
//! Where [`Dialog`](crate::dialog::Dialog) zooms a card into the center,
//! `Sheet` slides a full-height (or full-width) panel in from a chosen
//! [`Side`] — right by default, like shadcn. It shares the same modal chrome
//! (dimmed backdrop, header with title + muted description + × close button,
//! arbitrary body) but anchors to an edge instead of floating.
//!
//! Clicking the backdrop, pressing Escape, or the × button dismisses it.
//!
//! Drive it from a `bool` you own: flip it `true` to open, and call
//! [`show`](Sheet::show) **every frame** — it animates in and out and paints
//! nothing while fully closed, clearing the `bool` when dismissed.
//!
//! ```no_run
//! use glazier::sheet::Sheet;
//! use glazier::dialog::Side;
//! use glazier::button::Button;
//! use egui::Widget as _;
//! # let ctx = egui::Context::default();
//! # let mut open = true;
//! # let mut name = String::new();
//! Sheet::new("Edit profile")
//!     .side(Side::Right)
//!     .description("Make changes to your profile here.")
//!     .show(&ctx, &mut open, |ui| {
//!         ui.text_edit_singleline(&mut name);
//!         if Button::new("Save changes").ui(ui).clicked() {
//!             // persist…
//!         }
//!     });
//! ```

use egui::{Frame, Vec2};

use crate::components::dialog::{close_button, modal_shell, ModalStyle, Side};
use crate::customize::{Customize, StyleHook};
use crate::fonts;
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry for [`Sheet`] — reach in via [`Sheet::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct SheetMetrics {
    /// shadcn `sm:max-w-sm`: a side sheet's cross-axis size (panel width for
    /// left/right, height for top/bottom).
    pub default_size: f32,
    /// Header title text size (`text-lg`).
    pub title_text: f32,
    /// Header description text size (`text-sm`).
    pub description_text: f32,
    /// Gap between the header column's title and description (`gap-1.5`).
    pub header_gap: f32,
    /// Gap between the header and the body content (`gap-4`).
    pub body_gap: f32,
}

impl Default for SheetMetrics {
    fn default() -> Self {
        Self {
            default_size: 384.0,
            title_text: 18.0,
            description_text: 14.0,
            header_gap: 6.0,
            body_gap: 16.0,
        }
    }
}

/// An edge-anchored modal panel that slides in from a screen [`Side`].
#[must_use = "sheets do nothing unless you show them"]
pub struct Sheet {
    title: String,
    description: Option<String>,
    side: Side,
    size: Option<f32>,
    show_close: bool,
    style_hook: StyleHook<Frame>,
    sizing_hook: SizingHook<SheetMetrics>,
}

impl Customize<Frame> for Sheet {
    fn style_hook_mut(&mut self) -> &mut StyleHook<Frame> {
        &mut self.style_hook
    }
}

impl Sizeable<SheetMetrics> for Sheet {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<SheetMetrics> {
        &mut self.sizing_hook
    }
}

impl Sheet {
    /// Create a sheet with the given title. Slides in from the right by default.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            side: Side::Right,
            size: None,
            show_close: true,
            style_hook: StyleHook::new(),
            sizing_hook: SizingHook::new(),
        }
    }

    /// Choose which edge the sheet slides in from.
    pub const fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// Set the muted description line below the title.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// Override the panel's cross-axis size (width for left/right, height for
    /// top/bottom). shadcn's default is `max-w-sm`, 384px.
    pub const fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// Hide the top-right close (×) button. The backdrop and Escape still
    /// dismiss the sheet.
    pub const fn show_close(mut self, show: bool) -> Self {
        self.show_close = show;
        self
    }

    /// Show the sheet. Call this **every frame** — it animates in and out and
    /// paints nothing while fully closed. `content` draws the body; its return
    /// value is forwarded as `Some(R)` while visible (`None` once fully closed).
    /// `open` is cleared to `false` when dismissed (× button, backdrop click,
    /// or Escape).
    pub fn show<R>(
        mut self,
        ctx: &egui::Context,
        open: &mut bool,
        content: impl FnOnce(&mut egui::Ui) -> R,
    ) -> Option<R> {
        let id = egui::Id::new("glazier-sheet").with(&self.title);
        let style_hook = std::mem::take(&mut self.style_hook);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let size = self.size.unwrap_or(m.default_size);

        let mut closed = false;
        let out = modal_shell(
            ctx,
            id,
            *open,
            ModalStyle::Sheet(self.side),
            size,
            style_hook,
            |ui, tokens, _width| {
                ui.spacing_mut().item_spacing = Vec2::new(0.0, m.body_gap);
                self.header(ui, tokens, m, &mut closed);
                content(ui)
            },
        );

        let Some((inner, backdrop_close)) = out else {
            return None; // fully closed
        };

        // Like `Dialog`, a `Sheet` dismisses on ×, backdrop click, or Escape.
        if *open && (closed || backdrop_close) {
            *open = false; // begins the exit animation on the next frame
        }
        Some(inner)
    }

    /// Paint the header: a title row with the top-right close (×) button, then
    /// the muted description beneath (shadcn's `gap-1.5` header column).
    fn header(&self, ui: &mut egui::Ui, tokens: Tokens, m: SheetMetrics, closed: &mut bool) {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = m.header_gap;

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(&self.title)
                        .font(fonts::semibold(ui, m.title_text))
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

            if let Some(desc) = &self.description {
                ui.label(
                    egui::RichText::new(desc)
                        .size(m.description_text)
                        .color(tokens.muted_foreground),
                );
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Driving a closed sheet must paint nothing and forward `None`.
    #[test]
    fn closed_sheet_is_inert() {
        let ctx = egui::Context::default();
        let mut open = false;
        let mut ran = false;
        let mut out = Some(0);
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            out = Sheet::new("Edit profile").show(ui.ctx(), &mut open, |_| {
                ran = true;
                42
            });
        });
        assert_eq!(out, None);
        assert!(!ran, "content closure must not run while fully closed");
    }

    /// An open sheet runs its content and forwards the closure's value.
    #[test]
    fn open_sheet_forwards_value() {
        let ctx = egui::Context::default();
        let mut open = true;
        let mut out = None;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            out = Sheet::new("Edit profile")
                .side(Side::Left)
                .show(ui.ctx(), &mut open, |_| 7);
        });
        assert_eq!(out, Some(7));
        assert!(open, "sheet stays open until dismissed");
    }
}
