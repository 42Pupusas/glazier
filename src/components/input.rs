//! [`Input`] — a single-line text field, mirroring shadcn's `<Input>`.
//!
//! Wraps egui's [`TextEdit`](egui::TextEdit) but paints shadcn's chrome: a
//! `background` fill, hairline `input` border, `radius-md` corners, and a 36px
//! (`h-9`) height with comfortable padding. Borrows `&mut String`.
//!
//! Supports the three interactive states from the shadcn spec: a `focus-visible`
//! ring ([`Input::ui`] draws it live), [`disabled`](Input::disabled) (muted,
//! non-interactive), and [`invalid`](Input::invalid) (`aria-invalid` —
//! destructive border + ring).
//!
//! Use [`Input::compact`] for dense contexts (sidebars, toolbars) where the
//! default `h-9` (36 px) is disproportionate to surrounding rows.

use egui::{Frame, Margin, Response, Stroke, StrokeKind, TextEdit, Ui, Vec2, Widget};

use crate::customize::{Customize, StyleHook};
use crate::tokens::Tokens;

/// Full height (`h-9`) — matches shadcn default.
const H_FULL: f32 = 36.0;
/// Compact height (`h-7`) — for sidebar / dense-list contexts.
const H_COMPACT: f32 = 28.0;

/// A single-line text input.
///
/// ```no_run
/// use glazier::input::Input;
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// let mut name = String::new();
/// Input::new(&mut name).placeholder("Email").ui(ui);
/// # });
/// ```
#[must_use = "inputs do nothing unless you add them to a Ui"]
#[allow(clippy::struct_excessive_bools)] // each is an independent, orthogonal input mode
pub struct Input<'a> {
    text: &'a mut String,
    placeholder: Option<String>,
    icon_start: Option<crate::icon::Icon>,
    password: bool,
    disabled: bool,
    invalid: bool,
    width: Option<f32>,
    /// `h-7` (28 px) with tighter vertical padding — use inside dense lists
    /// and sidebar panels where the default `h-9` is too tall relative to
    /// surrounding 32 px rows.
    compact: bool,
    style_hook: StyleHook<Frame>,
}

impl Customize<Frame> for Input<'_> {
    fn style_hook_mut(&mut self) -> &mut StyleHook<Frame> {
        &mut self.style_hook
    }
}

impl<'a> Input<'a> {
    /// Create an input bound to `text`.
    pub const fn new(text: &'a mut String) -> Self {
        Self {
            text,
            placeholder: None,
            icon_start: None,
            password: false,
            disabled: false,
            invalid: false,
            width: None,
            compact: false,
            style_hook: StyleHook::new(),
        }
    }

    /// Prefix the input with a leading icon, inset into the left side of the
    /// field. The text is shifted right to make room; the icon is painted in
    /// `muted_foreground` so it reads as a hint, not an action.
    pub const fn icon_start(mut self, icon: crate::icon::Icon) -> Self {
        self.icon_start = Some(icon);
        self
    }

    /// Set placeholder (hint) text shown when empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Mask the contents (password field).
    pub const fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// Disable the input: muted fill, non-interactive, no focus ring.
    ///
    /// Mirrors shadcn's `disabled` prop (`disabled:opacity-50`,
    /// `disabled:cursor-not-allowed`).
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Mark the input invalid (`aria-invalid`): destructive border and ring.
    ///
    /// Pair with a `Field` error message so meaning is not conveyed by
    /// colour alone.
    pub const fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Set an explicit width; defaults to the available width.
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Use compact (`h-7`, 28 px) sizing.
    ///
    /// Suitable for sidebar search bars and any dense-list context where the
    /// default `h-9` (36 px) feels too tall relative to 32 px nav rows.
    pub const fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }
}

impl Widget for Input<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let width = self.width.unwrap_or_else(|| ui.available_width());

        let h = if self.compact { H_COMPACT } else { H_FULL };
        // Tighter vertical padding in compact mode (4 px top/bottom vs 8 px).
        let v_pad: i8 = if self.compact { 4 } else { 8 };

        let fill = if self.disabled {
            tokens.muted
        } else {
            tokens.background
        };
        let text_color = if self.disabled {
            tokens.muted_foreground
        } else {
            tokens.foreground
        };

        // When an icon is present shift the left margin right to make room
        // (10 px gap + 16 px icon + 10 px gap to text = 36 px total).
        let left_margin: i8 = if self.icon_start.is_some() { 36 } else { 12 };

        let mut frame = Frame::new()
            .fill(fill)
            .corner_radius(tokens.radius_md())
            .inner_margin(Margin {
                left: left_margin,
                right: 12,
                top: v_pad,
                bottom: v_pad,
            });
        std::mem::take(&mut self.style_hook).apply(&mut frame);

        let mut edit = TextEdit::singleline(self.text)
            .frame(frame)
            .desired_width(width)
            .min_size(Vec2::new(width, h))
            .background_color(fill)
            .text_color(text_color)
            .interactive(!self.disabled)
            .password(self.password);
        if let Some(hint) = self.placeholder {
            edit = edit.hint_text(hint);
        }

        let response = edit.ui(ui);

        // Paint the leading icon inside the left margin, vertically centred.
        if let Some(icon) = self.icon_start {
            let icon_size = 16.0;
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(
                    response.rect.left() + 10.0 + icon_size / 2.0,
                    response.rect.center().y,
                ),
                egui::Vec2::splat(icon_size),
            );
            icon.size(icon_size)
                .color(tokens.muted_foreground)
                .image(tokens)
                .paint_at(ui, icon_rect);
        }

        // Border + focus ring painted over the laid-out rect.
        let rect = response.rect;
        let radius = tokens.radius_md();
        let focused = response.has_focus() && !self.disabled;

        let border = if self.invalid {
            tokens.destructive
        } else if focused {
            tokens.ring
        } else {
            tokens.input
        };
        let painter = ui.painter();
        painter.rect_stroke(rect, radius, Stroke::new(1.0, border), StrokeKind::Inside);

        if focused || self.invalid {
            let ring = if self.invalid {
                tokens.destructive.gamma_multiply(0.2)
            } else {
                tokens.ring.gamma_multiply(0.5)
            };
            painter.rect_stroke(rect, radius, Stroke::new(3.0, ring), StrokeKind::Outside);
        }

        response
    }
}
