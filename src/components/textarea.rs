//! [`Textarea`] — a multi-line text field, mirroring shadcn's `<Textarea>`.
//!
//! The newer shadcn form style: a *filled* surface (`bg-input/50`) with a
//! transparent border, `rounded-2xl` corners, and a `min-h-16` (64px) minimum
//! height. Borrows `&mut String`.

use egui::{Color32, Frame, Margin, Response, Stroke, TextEdit, Ui, Vec2, Widget};

use crate::customize::{Customize, StyleHook};
use crate::tokens::Tokens;

/// A multi-line text input.
///
/// ```no_run
/// use glazier::textarea::Textarea;
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// let mut message = String::new();
/// Textarea::new(&mut message).placeholder("Message").ui(ui);
/// # });
/// ```
#[must_use = "textareas do nothing unless you add them to a Ui"]
pub struct Textarea<'a> {
    text: &'a mut String,
    placeholder: Option<String>,
    rows: usize,
    width: Option<f32>,
    style_hook: StyleHook<Frame>,
}

impl Customize<Frame> for Textarea<'_> {
    fn style_hook_mut(&mut self) -> &mut StyleHook<Frame> {
        &mut self.style_hook
    }
}

impl<'a> Textarea<'a> {
    /// Create a textarea bound to `text`.
    pub const fn new(text: &'a mut String) -> Self {
        Self {
            text,
            placeholder: None,
            rows: 3,
            width: None,
            style_hook: StyleHook::new(),
        }
    }

    /// Set placeholder (hint) text shown when empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set the number of visible text rows (default 3).
    pub const fn rows(mut self, rows: usize) -> Self {
        self.rows = rows;
        self
    }

    /// Set an explicit width; defaults to the available width.
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
}

/// shadcn's `bg-input/50`: the input color blended halfway toward the surface.
fn filled_input(tokens: Tokens) -> Color32 {
    tokens.input.lerp_to_gamma(tokens.background, 0.5)
}

impl Widget for Textarea<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let width = self.width.unwrap_or_else(|| ui.available_width());

        // Filled surface, transparent border, rounded-2xl.
        let mut frame = Frame::new()
            .fill(filled_input(tokens))
            .corner_radius(tokens.radius_2xl())
            .stroke(Stroke::NONE)
            .inner_margin(Margin::symmetric(10, 8));
        std::mem::take(&mut self.style_hook).apply(&mut frame);

        let mut edit = TextEdit::multiline(self.text)
            .frame(frame)
            .desired_width(width)
            .desired_rows(self.rows)
            .min_size(Vec2::new(width, 64.0))
            .background_color(filled_input(tokens))
            .text_color(tokens.foreground);
        if let Some(hint) = self.placeholder {
            edit = edit.hint_text(hint);
        }

        edit.ui(ui)
    }
}
