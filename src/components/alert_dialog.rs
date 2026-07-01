//! [`AlertDialog`] — a centered modal confirmation, mirroring shadcn's
//! `AlertDialog`.
//!
//! A dimmed backdrop covers the app; a `rounded-4xl` popover card floats in the
//! center with a heading, a muted description, and a two-button footer (a
//! neutral *cancel* and a primary *action*). Unlike a [`Dialog`](crate::dialog::Dialog),
//! it is **button-only** — clicking the backdrop or pressing Escape does *not*
//! dismiss it (these are destructive confirmations you shouldn't wave away by
//! reflex), matching shadcn's `AlertDialog`.
//!
//! Drive it from a `bool` you own: flip it `true` to open, and call
//! [`show`](AlertDialog::show) **every frame** — it animates in and out and
//! paints nothing while fully closed, clearing the `bool` when dismissed.
//!
//! ```no_run
//! use glazier::alert_dialog::{AlertDialog, AlertChoice};
//! # let ctx = egui::Context::default();
//! # let mut open = true;
//! match AlertDialog::new("Allow accessory to connect?")
//!     .description("Do you want to allow the USB accessory to connect?")
//!     .cancel("Don't allow")
//!     .action("Allow")
//!     .show(&ctx, &mut open)
//! {
//!     Some(AlertChoice::Action) => { /* allowed */ }
//!     Some(AlertChoice::Cancel) => { /* dismissed */ }
//!     None => {}
//! }
//! ```

use egui::{Frame, Vec2, Widget as _};

use crate::components::button::{Button, Size, Variant};
use crate::components::dialog::{modal_shell, ModalStyle};
use crate::customize::{Customize, StyleHook};
use crate::fonts;
use crate::tokens::Tokens;

/// shadcn `max-w-xs`: the dialog's content width target.
const CONTENT_WIDTH: f32 = 320.0;
/// Height of a `Size::Small` button (`h-8`) — the footer row's fixed height.
const FOOTER_ROW_HEIGHT: f32 = 32.0;

/// Which control the user activated to dismiss an [`AlertDialog`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertChoice {
    /// The primary action was confirmed.
    Action,
    /// The dialog was cancelled (cancel button, backdrop click, or Escape).
    Cancel,
}

/// A centered modal confirmation dialog.
#[must_use = "alert dialogs do nothing unless you show them"]
pub struct AlertDialog {
    title: String,
    description: Option<String>,
    cancel: String,
    action: String,
    style_hook: StyleHook<Frame>,
}

impl Customize<Frame> for AlertDialog {
    fn style_hook_mut(&mut self) -> &mut StyleHook<Frame> {
        &mut self.style_hook
    }
}

impl AlertDialog {
    /// Create a dialog with the given title; default footer is
    /// *Cancel* / *Continue*.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            cancel: "Cancel".to_owned(),
            action: "Continue".to_owned(),
            style_hook: StyleHook::new(),
        }
    }

    /// Set the muted description line below the title.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// Set the neutral cancel button's label.
    pub fn cancel(mut self, label: impl Into<String>) -> Self {
        self.cancel = label.into();
        self
    }

    /// Set the primary action button's label.
    pub fn action(mut self, label: impl Into<String>) -> Self {
        self.action = label.into();
        self
    }

    /// Show the dialog. Call this **every frame** — it animates in and out and
    /// paints nothing while fully closed. `open` is cleared to `false` when the
    /// dialog should close; the returned [`AlertChoice`] reports *how* it closed
    /// (or `None` if it stayed open / was already shut this frame).
    pub fn show(mut self, ctx: &egui::Context, open: &mut bool) -> Option<AlertChoice> {
        let id = egui::Id::new("glazier-alert-dialog");
        let style_hook = std::mem::take(&mut self.style_hook);

        let mut choice = None;
        let out = modal_shell(
            ctx,
            id,
            *open,
            ModalStyle::Center,
            CONTENT_WIDTH,
            style_hook,
            |ui, tokens, width| {
                ui.spacing_mut().item_spacing = Vec2::new(0.0, 24.0); // gap-6
                self.body(ui, tokens, width, &mut choice);
                #[cfg(test)]
                {
                    let card = ui.min_rect().size();
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(id.with("card_size"), card));
                }
            },
        );

        // Unlike `Dialog`, a shadcn `AlertDialog` is *button-only*: clicking the
        // backdrop or pressing Escape does NOT dismiss it (these are destructive
        // confirmations you shouldn't be able to wave away by reflex). So we
        // deliberately ignore `backdrop_close` and only react to a button.
        out?;
        if choice.is_some() {
            *open = false; // begins the exit animation on the next frame
        }
        choice
    }

    /// Paint the header (title + description) and the footer buttons. `width` is
    /// the deterministic content width, so layout decisions never depend on the
    /// area's cached/stale measured width.
    fn body(
        &self,
        ui: &mut egui::Ui,
        tokens: Tokens,
        width: f32,
        choice: &mut Option<AlertChoice>,
    ) {
        // Header: centered title + muted description (gap-1.5).
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.label(
                egui::RichText::new(&self.title)
                    .font(fonts::semibold(ui, 18.0)) // text-lg
                    .color(tokens.foreground),
            );
            if let Some(desc) = &self.description {
                ui.label(
                    egui::RichText::new(desc)
                        .size(14.0) // text-sm
                        .color(tokens.muted_foreground),
                );
            }
        });

        // Footer: shadcn's `flex-col-reverse sm:flex-row sm:justify-end`. When
        // the two buttons + gap fit on one row they sit right-justified;
        // otherwise they stack full-width with the action on top.
        if self.footer_fits_row(ui, width) {
            // Allocate the row at a *fixed* height. A plain `with_layout` takes
            // up all available space, which on a modal means the Area's cached
            // (possibly stale, taller) height — so the row would balloon to fill
            // it and the card would never shrink back after the window grows.
            // A bounded box keeps the row content-sized in both directions.
            ui.allocate_ui_with_layout(
                Vec2::new(width, FOOTER_ROW_HEIGHT),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0; // gap-2
                                                           // right_to_left adds the action first so it sits on the right.
                    if Button::new(&self.action)
                        .variant(Variant::Default)
                        .size(Size::Small)
                        .ui(ui)
                        .clicked()
                    {
                        *choice = Some(AlertChoice::Action);
                    }
                    if Button::new(&self.cancel)
                        .variant(Variant::Outline)
                        .size(Size::Small)
                        .ui(ui)
                        .clicked()
                    {
                        *choice = Some(AlertChoice::Cancel);
                    }
                },
            );
        } else {
            // `flex-col-reverse`: action on top, both full-width (gap-2).
            ui.spacing_mut().item_spacing.y = 8.0;
            if Button::new(&self.action)
                .variant(Variant::Default)
                .size(Size::Small)
                .full_width(true)
                .ui(ui)
                .clicked()
            {
                *choice = Some(AlertChoice::Action);
            }
            if Button::new(&self.cancel)
                .variant(Variant::Outline)
                .size(Size::Small)
                .full_width(true)
                .ui(ui)
                .clicked()
            {
                *choice = Some(AlertChoice::Cancel);
            }
        }
    }

    /// Whether both footer buttons fit side-by-side within `width`. Measures
    /// each label at the button font and adds `px-3` padding (Small) plus the
    /// `gap-2` between them.
    fn footer_fits_row(&self, ui: &egui::Ui, width: f32) -> bool {
        let label_w = |text: &str| {
            ui.painter()
                .layout_no_wrap(
                    text.to_owned(),
                    egui::TextStyle::Button.resolve(ui.style()),
                    egui::Color32::PLACEHOLDER,
                )
                .size()
                .x
        };
        // Small button: px-3 (12px) each side = 24px padding per button, plus
        // the gap-2 (8px) between the two.
        let needed = 24.0_f32.mul_add(2.0, label_w(&self.action) + label_w(&self.cancel)) + 8.0;
        needed <= width
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run one frame at the given screen width, returning the card's measured
    /// content size (height included), which the dialog stashes for us.
    fn frame_card_size(ctx: &egui::Context, width: f32, open: &mut bool) -> Vec2 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 800.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run_ui(input, |ui| {
            AlertDialog::new("Allow accessory to connect?")
                .description(
                    "Do you want to allow the USB accessory to connect? \
                     This will let it exchange data with your device.",
                )
                .cancel("Don't allow")
                .action("Allow")
                .show(ui.ctx(), open);
        });
        let card_id = egui::Id::new("glazier-alert-dialog").with("card_size");
        ctx.data(|d| d.get_temp::<Vec2>(card_id))
            .unwrap_or(Vec2::ZERO)
    }

    /// The card must return to (approximately) its original height after the
    /// window is shrunk and then grown back: it must not get stuck tall. This
    /// guards the footer-row regression where a `with_layout` row expanded to
    /// fill the modal Area's stale cached height.
    #[test]
    fn card_height_recovers_after_resize() {
        let ctx = egui::Context::default();
        let mut open = true;

        let mut wide = Vec2::ZERO;
        for _ in 0..6 {
            wide = frame_card_size(&ctx, 900.0, &mut open);
        }

        // Shrink hard so text wraps and the footer stacks (card grows taller).
        let mut narrow = Vec2::ZERO;
        for _ in 0..4 {
            narrow = frame_card_size(&ctx, 200.0, &mut open);
        }
        assert!(
            narrow.y > wide.y,
            "expected a taller card when narrow: wide={wide:?} narrow={narrow:?}"
        );

        // Grow back: the card must converge to its original height.
        let mut grown = Vec2::ZERO;
        for _ in 0..4 {
            grown = frame_card_size(&ctx, 900.0, &mut open);
        }
        assert!(
            (grown.y - wide.y).abs() < 1.0,
            "card stayed deformed after resize-back: wide={wide:?} grown={grown:?}"
        );
    }
}
