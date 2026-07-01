//! [`InputGroup`] — a text field wrapped with inline addon slots, mirroring
//! shadcn's `<InputGroup>` / `<InputGroupAddon>` / `<InputGroupInput>`.
//!
//! The whole group is one filled, `rounded-2xl` surface (`bg-input/50`,
//! transparent border) with an `h-8` height. A frameless text field sits in the
//! middle; short addon strings (icons, kbd hints, units) can be pinned to the
//! inline-start and/or inline-end, rendered in `muted-foreground`.

use egui::{Color32, Frame, Margin, Response, RichText, Stroke, TextEdit, Ui, Vec2, Widget};

use crate::components::icon::Icon;
use crate::customize::{Customize, StyleHook};
use crate::tokens::Tokens;

/// Inner content height: `h-8` (32px) minus the frame's 4px vertical padding.
const ROW_HEIGHT: f32 = 24.0;

/// A text input with optional leading/trailing addon slots.
///
/// ```no_run
/// use glazier::input_group::InputGroup;
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// let mut q = String::new();
/// InputGroup::new(&mut q).placeholder("Name").addon_end("\u{1F50D}").ui(ui);
/// # });
/// ```
#[must_use = "input groups do nothing unless you add them to a Ui"]
pub struct InputGroup<'a> {
    text: &'a mut String,
    placeholder: Option<String>,
    addon_start: Option<String>,
    addon_end: Option<String>,
    icon_start: Option<Icon>,
    icon_end: Option<Icon>,
    width: Option<f32>,
    style_hook: StyleHook<Frame>,
}

impl Customize<Frame> for InputGroup<'_> {
    fn style_hook_mut(&mut self) -> &mut StyleHook<Frame> {
        &mut self.style_hook
    }
}

impl<'a> InputGroup<'a> {
    /// Create an input group bound to `text`.
    pub const fn new(text: &'a mut String) -> Self {
        Self {
            text,
            placeholder: None,
            addon_start: None,
            addon_end: None,
            icon_start: None,
            icon_end: None,
            width: None,
            style_hook: StyleHook::new(),
        }
    }

    /// Set placeholder (hint) text shown when empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Pin an addon (icon / short text) to the inline-start.
    pub fn addon_start(mut self, addon: impl Into<String>) -> Self {
        self.addon_start = Some(addon.into());
        self
    }

    /// Pin an addon (icon / short text) to the inline-end.
    pub fn addon_end(mut self, addon: impl Into<String>) -> Self {
        self.addon_end = Some(addon.into());
        self
    }

    /// Pin an [`Icon`] to the inline-start.
    pub const fn icon_start(mut self, icon: Icon) -> Self {
        self.icon_start = Some(icon);
        self
    }

    /// Pin an [`Icon`] to the inline-end (e.g. a lucide search glyph).
    pub const fn icon_end(mut self, icon: Icon) -> Self {
        self.icon_end = Some(icon);
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

impl Widget for InputGroup<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let width = self.width.unwrap_or_else(|| ui.available_width());

        let mut frame = Frame::new()
            .fill(filled_input(tokens))
            .corner_radius(tokens.radius_2xl())
            .stroke(Stroke::NONE)
            .inner_margin(Margin::symmetric(10, 4));
        std::mem::take(&mut self.style_hook).apply(&mut frame);

        frame
            .show(ui, |ui| {
                // Bound the content to a fixed-height strip so nested layouts
                // can't claim the column's full remaining height (which would
                // stretch the field vertically and overflow the card).
                let inner_w = width - 20.0; // frame inner_margin: 10 each side
                ui.allocate_ui_with_layout(
                    Vec2::new(inner_w, ROW_HEIGHT),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;

                        // right_to_left lays items from the right edge inward:
                        // trailing addon first, then leading addon, then the
                        // field fills whatever width is left.
                        if let Some(icon) = self.icon_end {
                            ui.add(icon.color(tokens.muted_foreground).image(tokens));
                        }
                        if let Some(addon) = &self.addon_end {
                            ui.label(
                                RichText::new(addon)
                                    .color(tokens.muted_foreground)
                                    .size(14.0),
                            );
                        }
                        let lead = self.addon_start.clone();
                        let icon_lead = self.icon_start;
                        let field_w = if lead.is_some() || icon_lead.is_some() {
                            ui.available_width() - 22.0
                        } else {
                            ui.available_width()
                        };
                        // Field fills remaining width (drawn left-aligned so
                        // the caret sits beside any leading addon).
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            if let Some(icon) = icon_lead {
                                ui.add(icon.color(tokens.muted_foreground).image(tokens));
                            }
                            if let Some(addon) = &lead {
                                ui.label(
                                    RichText::new(addon)
                                        .color(tokens.muted_foreground)
                                        .size(14.0),
                                );
                            }
                            let mut edit = TextEdit::singleline(self.text)
                                .frame(Frame::NONE)
                                .desired_width(field_w)
                                .margin(Margin::ZERO)
                                .background_color(Color32::TRANSPARENT)
                                .text_color(tokens.foreground);
                            if let Some(hint) = &self.placeholder {
                                edit = edit.hint_text(hint.clone());
                            }
                            edit.ui(ui);
                        });
                    },
                );
            })
            .response
    }
}
