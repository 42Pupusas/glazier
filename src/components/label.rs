//! [`Label`] — a form-style caption, mirroring shadcn's `<Label>`.
//!
//! A small, medium-weight piece of text intended to caption an input. Resolves
//! its color from the active visuals; dims when the surrounding [`Ui`] is
//! disabled, matching shadcn's `peer-disabled` styling.

use egui::{Response, RichText, Ui, Widget};

/// A form field label.
///
/// Text is **non-selectable by default** — an opinionated stance for chrome and
/// prose. Opt back in with [`selectable`](Self::selectable) for data-ish labels
/// the user might want to copy (account numbers, balances, ids).
///
/// ```no_run
/// use glazier::label::Label;
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// Label::new("Email").ui(ui);
/// Label::new("acct-8841-0042").selectable(true).ui(ui); // copyable
/// # });
/// ```
#[must_use = "labels do nothing unless you add them to a Ui"]
pub struct Label {
    text: String,
    selectable: bool,
}

impl Label {
    /// Create a label with the given text.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            selectable: false,
        }
    }

    /// Allow the user to select (and copy) the label's text. Off by default.
    pub const fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }
}

impl Widget for Label {
    fn ui(self, ui: &mut Ui) -> Response {
        let color = if ui.is_enabled() {
            ui.visuals().text_color()
        } else {
            ui.visuals().weak_text_color()
        };
        ui.add(
            egui::Label::new(RichText::new(self.text).color(color).strong())
                .selectable(self.selectable),
        )
    }
}
