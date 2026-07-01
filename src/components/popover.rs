//! [`Popover`] — a click-anchored floating panel, mirroring shadcn's
//! `<Popover>`.
//!
//! Unlike a [tooltip](crate::tooltip::Tooltip) (hover, tiny, non-interactive), a
//! popover opens on click and hosts arbitrary interactive content in a
//! `rounded-2xl` card on the popover surface, with a hairline ring and a soft
//! shadow. It dismisses on click-outside or Escape.
//!
//! Drive it from any trigger [`Response`]; the panel's open-state lives in
//! egui's popup memory keyed to that trigger:
//!
//! ```no_run
//! use glazier::popover::Popover;
//! use egui::Widget as _;
//! # egui::__run_test_ui(|ui| {
//! let trigger = egui::Button::new("Open").ui(ui);
//! Popover::new().width(260.0).show(ui, &trigger, |ui| {
//!     ui.label("Dimensions");
//! });
//! # });
//! ```

use egui::{Frame, Margin, PopupCloseBehavior, RectAlign, Response, Stroke, Ui};

use crate::customize::{Customize, StyleHook};
use crate::tokens::Tokens;

/// A click-anchored floating content panel.
#[must_use = "popovers do nothing unless you show them"]
pub struct Popover {
    width: Option<f32>,
    gap: f32,
    align: Option<RectAlign>,
    close_behavior: PopupCloseBehavior,
    style_hook: StyleHook<Frame>,
}

impl Customize<Frame> for Popover {
    fn style_hook_mut(&mut self) -> &mut StyleHook<Frame> {
        &mut self.style_hook
    }
}

impl Default for Popover {
    fn default() -> Self {
        Self::new()
    }
}

impl Popover {
    /// Create a popover with default sizing (content-width, 6px gap).
    pub const fn new() -> Self {
        Self {
            width: None,
            gap: 6.0,
            align: None,
            close_behavior: PopupCloseBehavior::CloseOnClick,
            style_hook: StyleHook::new(),
        }
    }

    /// Set an explicit content width (shadcn's default popover is `w-72`).
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the gap between the trigger and the panel.
    pub const fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Set the side the panel opens towards, as a [`RectAlign`] relative to the
    /// trigger (e.g. [`RectAlign::TOP`], [`RectAlign::RIGHT`]). By default egui
    /// picks the best fit; setting this makes that side the preferred position
    /// (egui still falls back to alternatives if it would clip off-screen).
    pub const fn align(mut self, align: RectAlign) -> Self {
        self.align = Some(align);
        self
    }

    /// Set when the popover dismisses on a click. Defaults to
    /// [`PopupCloseBehavior::CloseOnClick`] (any click closes it, like a menu);
    /// use [`CloseOnClickOutside`](PopupCloseBehavior::CloseOnClickOutside) for
    /// panels with interactive content (a calendar, a form) that should only
    /// close on an outside click, Escape, or an explicit [`Ui::close`].
    pub const fn close_behavior(mut self, behavior: PopupCloseBehavior) -> Self {
        self.close_behavior = behavior;
        self
    }

    /// The popover [`Frame`]: popover surface, `rounded-2xl`, `p-4`, a hairline
    /// ring and a soft shadow.
    pub fn frame(tokens: Tokens) -> Frame {
        Frame::new()
            .fill(tokens.background)
            .stroke(Stroke::new(1.0, tokens.border))
            .corner_radius(tokens.radius_2xl())
            .inner_margin(Margin::same(16)) // p-4
            .shadow(egui::epaint::Shadow {
                offset: [0, 8],
                blur: 32,
                spread: 0,
                color: egui::Color32::from_black_alpha(45),
            })
    }

    /// Toggle the panel from `trigger`'s clicks and, while open, render
    /// `content` inside the floating card. Dismisses on click-outside or Escape.
    pub fn show<R>(
        mut self,
        ui: &Ui,
        trigger: &Response,
        content: impl FnOnce(&mut Ui) -> R,
    ) -> Option<R> {
        let tokens = Tokens::get(ui);
        let width = self.width;
        let align = self.align;
        let mut frame = Self::frame(tokens);
        std::mem::take(&mut self.style_hook).apply(&mut frame);
        // `Popup::menu` toggles its own open-state off the trigger's click and
        // dismisses on click-outside — exactly shadcn's behaviour. We only
        // restyle the frame, set the close behaviour, and host content.
        let mut popup = egui::Popup::menu(trigger)
            .frame(frame)
            .close_behavior(self.close_behavior)
            .gap(self.gap);
        if let Some(align) = align {
            popup = popup.align(align);
        }
        popup
            .show(|ui| {
                if let Some(w) = width {
                    ui.set_width(w);
                }
                content(ui)
            })
            .map(|inner| inner.inner)
    }
}
