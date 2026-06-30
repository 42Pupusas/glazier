//! Concrete glazier components. These are plain [`egui::Widget`]s — they take
//! a `&mut Ui` and the small slice of state they need, never a `*mut App`.

use egui::{Response, Ui, Widget};

/// A simple chat message bubble. Compose decorators for the standard look:
/// `ChatBubble::new(text).carded().rounded().ui(ui)`.
pub struct ChatBubble {
    text: String,
}

impl ChatBubble {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl Widget for ChatBubble {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.label(self.text)
    }
}
