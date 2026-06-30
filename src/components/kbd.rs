//! [`Kbd`] — an inline keyboard-key chip, e.g. `⌘K`.
//!
//! shadcn renders shortcut hints as small bordered mono chips; this is the
//! standalone widget version. Colors follow the active visuals.
//!
//! Text is laid out in the **monospace** family. egui's bundled font stack
//! (Hack + Ubuntu-Light + Noto-emoji) covers ASCII and common symbols but *not*
//! the Apple modifier glyphs (⌘ U+2318, ⌥, ⇧, ⌃), which render as tofu ▯. Spell
//! cross-platform shortcuts in words (`"Ctrl P"`) unless you've installed a font
//! that covers those codepoints.

use egui::{Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget};

use crate::tokens::Tokens;

/// A small chip showing a keyboard key or shortcut.
///
/// ```no_run
/// use glazier::kbd::Kbd;
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// Kbd::new("⌘K").ui(ui);
/// # });
/// ```
#[must_use = "kbd chips do nothing unless you add them to a Ui"]
pub struct Kbd {
    keys: String,
}

impl Kbd {
    /// Create a chip with the given key text.
    pub fn new(keys: impl Into<String>) -> Self {
        Self { keys: keys.into() }
    }
}

impl Widget for Kbd {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let fill = tokens.muted;
        let stroke = Stroke::new(1.0, tokens.border);
        let text_color = tokens.muted_foreground;

        let padding = Vec2::new(7.0, 2.0);
        let galley =
            ui.painter()
                .layout_no_wrap(self.keys, egui::FontId::monospace(11.0), text_color);
        let size = galley.size() + padding * 2.0;
        let (rect, response) = ui.allocate_at_least(size, Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            painter.rect(rect, tokens.radius_sm(), fill, stroke, StrokeKind::Inside);
            let text_pos = rect.center() - galley.size() / 2.0;
            painter.galley(text_pos, galley, text_color);
        }

        response
    }
}
