//! [`Field`] — a labelled form-field wrapper, mirroring shadcn's `<Field>`.
//!
//! Composes the vertical stack shadcn assembles from `Field` / `FieldLabel` /
//! `FieldDescription` / `FieldError`: a medium-weight label, your control, and
//! either a muted description **or** a destructive error message beneath it.
//! When an [`error`](Field::error) is set it replaces the description and the
//! label is tinted destructive, matching shadcn's `data-[invalid]` styling.
//!
//! The control itself is whatever you draw in the `content` closure, so any
//! glazier input ([`Input`](crate::Input), [`Textarea`](crate::Textarea),
//! [`Select`](crate::Select), …) drops in.
//!
//! ```no_run
//! use glazier::field::Field;
//! use glazier::input::Input;
//! use egui::Widget as _;
//! # egui::__run_test_ui(|ui| {
//! # let mut email = String::new();
//! Field::new("Email")
//!     .description("We'll never share your email.")
//!     .show(ui, |ui| Input::new(&mut email).ui(ui));
//!
//! Field::new("Password")
//!     .error("Must be at least 8 characters.")
//!     .show(ui, |ui| Input::new(&mut email).ui(ui));
//! # });
//! ```

use egui::{RichText, Ui};

use crate::tokens::Tokens;

/// A labelled form-field wrapper.
#[must_use = "fields do nothing unless you show them"]
pub struct Field {
    label: String,
    description: Option<String>,
    error: Option<String>,
}

impl Field {
    /// Create a field with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
            error: None,
        }
    }

    /// Set the muted helper text shown beneath the control. Hidden when an
    /// [`error`](Self::error) is set.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set a destructive error message. Replaces the description and tints the
    /// label, marking the field invalid.
    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Render the field: label, the `content` control, then the description or
    /// error line. Returns the control closure's value.
    ///
    /// # Panics
    /// Never in practice: the `content` closure is invoked exactly once inside
    /// the layout, so its value is always captured before this returns.
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> R {
        let tokens = Tokens::get(ui);
        let invalid = self.error.is_some();
        let label_color = if invalid {
            tokens.destructive
        } else {
            tokens.foreground
        };

        let mut out = None;
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0; // gap-1.5

            ui.label(
                RichText::new(&self.label)
                    .font(crate::fonts::semibold(ui, 14.0))
                    .color(label_color),
            );

            out = Some(content(ui));

            if let Some(error) = &self.error {
                ui.label(RichText::new(error).color(tokens.destructive).size(13.0));
            } else if let Some(description) = &self.description {
                ui.label(
                    RichText::new(description)
                        .color(tokens.muted_foreground)
                        .size(13.0),
                );
            }
        });

        out.expect("content closure runs exactly once")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The control closure's value is forwarded.
    #[test]
    fn forwards_content_value() {
        let ctx = egui::Context::default();
        let mut got = 0;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            got = Field::new("Name").show(ui, |_| 42);
        });
        assert_eq!(got, 42);
    }

    /// An error replaces the description without panicking.
    #[test]
    fn error_renders() {
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            Field::new("Email")
                .description("hidden when invalid")
                .error("required")
                .show(ui, |_| {});
        });
    }
}
