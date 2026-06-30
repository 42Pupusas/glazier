//! [`Avatar`] — a circular user image with an initials fallback.
//!
//! Mirrors shadcn's `<Avatar>` / `<AvatarFallback>`. With no image (or before
//! one loads) it draws a filled circle with centered initials. Colors follow
//! the active visuals.

use egui::{Response, Sense, Ui, Vec2, Widget};

use crate::tokens::Tokens;

/// A circular avatar.
///
/// ```no_run
/// use glazier::avatar::Avatar;
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// Avatar::new("Ada Lovelace").ui(ui);            // initials "AL"
/// Avatar::new("Bob").diameter(48.0).ui(ui);
/// # });
/// ```
#[must_use = "avatars do nothing unless you add them to a Ui"]
pub struct Avatar {
    /// Full name; initials are derived from it.
    name: String,
    diameter: f32,
}

impl Avatar {
    /// Create an avatar from a name (used for the initials fallback).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            diameter: 40.0,
        }
    }

    /// Set the circle diameter in points.
    pub const fn diameter(mut self, diameter: f32) -> Self {
        self.diameter = diameter;
        self
    }

    /// Paint the avatar centred in `rect` (its diameter is taken from
    /// [`Self::diameter`], so size `rect` to match). Exposed so layout
    /// components like [`Message`](crate::Message) can anchor an avatar to a
    /// computed position instead of the normal top-aligned flow.
    pub fn paint_at(&self, ui: &Ui, rect: egui::Rect) {
        if !ui.is_rect_visible(rect) {
            return;
        }
        let tokens = Tokens::get(ui);
        let painter = ui.painter();
        painter.circle_filled(rect.center(), self.diameter / 2.0, tokens.muted);
        let galley = painter.layout_no_wrap(
            self.initials(),
            egui::FontId::proportional(self.diameter * 0.4),
            tokens.muted_foreground,
        );
        let pos = rect.center() - galley.size() / 2.0;
        painter.galley(pos, galley, tokens.muted_foreground);
    }

    /// The configured circle diameter in points.
    #[must_use]
    pub const fn diameter_value(&self) -> f32 {
        self.diameter
    }

    /// Derive up to two uppercase initials from the name.
    fn initials(&self) -> String {
        let mut out = String::new();
        for word in self.name.split_whitespace().take(2) {
            if let Some(c) = word.chars().next() {
                out.extend(c.to_uppercase());
            }
        }
        if out.is_empty() {
            out.push('?');
        }
        out
    }
}

impl Widget for Avatar {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = Vec2::splat(self.diameter);
        let (rect, response) = ui.allocate_at_least(size, Sense::hover());
        self.paint_at(ui, rect);
        response
    }
}
