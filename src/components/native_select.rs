//! [`NativeSelect`] — a bordered dropdown picker, mirroring shadcn's
//! `<NativeSelect>` (their thin wrapper over the browser's `<select>`).
//!
//! Where [`Select`](crate::Select) is the fully-custom listbox with a filled
//! `bg-input/50` trigger and a check-marked popover, `NativeSelect` is the
//! plainer control: an **outlined** input-style row (`h-9`, hairline `input`
//! border, `radius-md`) with a trailing chevron. It binds an index into a fixed
//! option list and opens a lightweight menu popup to pick.
//!
//! ```no_run
//! use glazier::native_select::NativeSelect;
//! # egui::__run_test_ui(|ui| {
//! let mut size = 1usize;
//! NativeSelect::new(&mut size, ["Small", "Medium", "Large"]).show(ui);
//! # });
//! ```

use egui::{Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::components::dropdown_menu::DropdownMenu;
use crate::components::icon::Icon;
use crate::tokens::Tokens;

/// `chevron-down` (lucide) — the trailing affordance.
const CHEVRON_DOWN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#;

/// Trigger height: `h-9` (36px).
const HEIGHT: f32 = 36.0;
/// Trigger horizontal padding (`px-3`).
const PAD_X: f32 = 12.0;
/// Value / option text size (`text-sm`).
const TEXT: f32 = 14.0;
/// Chevron edge length, plus the gap reserved before it.
const CHEVRON: f32 = 16.0;
const CHEVRON_GAP: f32 = 6.0;
/// Option row padding inside the popover.
const ITEM_PAD_X: f32 = 8.0;
const ITEM_PAD_Y: f32 = 6.0;
/// Minimum option height (`min-h-8`).
const ITEM_MIN_H: f32 = 32.0;

/// A bordered, native-style dropdown picker bound to an index.
#[must_use = "selects do nothing unless you show them"]
pub struct NativeSelect<'a> {
    selected: &'a mut usize,
    options: Vec<String>,
    placeholder: Option<String>,
    width: Option<f32>,
}

impl<'a> NativeSelect<'a> {
    /// Create a native select bound to `selected`, over the given `options`.
    pub fn new<I, S>(selected: &'a mut usize, options: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            selected,
            options: options.into_iter().map(Into::into).collect(),
            placeholder: None,
            width: None,
        }
    }

    /// Text shown when `selected` is out of range (no valid option).
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set an explicit width; defaults to the available width.
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Render the trigger and (when open) the option popover. Returns the
    /// trigger [`Response`]; the chosen index is written back into `selected`.
    pub fn show(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let width = self.width.unwrap_or_else(|| ui.available_width());

        let value = self
            .options
            .get(*self.selected)
            .map(String::as_str)
            .or(self.placeholder.as_deref())
            .unwrap_or("");
        let is_placeholder = self.options.get(*self.selected).is_none();

        let (rect, response) = ui.allocate_at_least(Vec2::new(width, HEIGHT), Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

        if ui.is_rect_visible(rect) {
            // Outlined input chrome: background fill + hairline border; the
            // border tightens to the ring while the popup is focused/hovered.
            let stroke = if response.hovered() {
                Stroke::new(1.0, tokens.ring)
            } else {
                Stroke::new(1.0, tokens.input)
            };
            ui.painter().rect(
                rect,
                tokens.radius_md(),
                tokens.background,
                stroke,
                StrokeKind::Inside,
            );

            // Trailing chevron.
            let chevron_rect = egui::Rect::from_min_size(
                egui::pos2(
                    rect.right() - PAD_X - CHEVRON,
                    rect.center().y - CHEVRON / 2.0,
                ),
                Vec2::splat(CHEVRON),
            );
            Icon::new(CHEVRON_DOWN)
                .color(tokens.muted_foreground)
                .image(tokens)
                .paint_at(ui, chevron_rect);

            // Leading value, truncated so it never collides with the chevron.
            let color = if is_placeholder {
                tokens.muted_foreground
            } else {
                tokens.foreground
            };
            let avail = (PAD_X.mul_add(-2.0, rect.width()) - CHEVRON - CHEVRON_GAP).max(0.0);
            let mut job = egui::text::LayoutJob::simple(
                value.to_owned(),
                egui::FontId::proportional(TEXT),
                color,
                avail,
            );
            job.wrap = egui::text::TextWrapping::truncate_at_width(avail);
            let galley = ui.painter().layout_job(job);
            let pos = egui::pos2(rect.left() + PAD_X, rect.center().y - galley.size().y / 2.0);
            ui.painter().galley(pos, galley, color);
        }

        // Option popover, hung off the trigger.
        let trigger_w = response.rect.width();
        egui::Popup::menu(&response)
            .frame(DropdownMenu::frame(tokens))
            .gap(4.0)
            .show(|ui| {
                ui.set_min_width(trigger_w);
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                for (i, option) in self.options.iter().enumerate() {
                    if option_row(ui, tokens, trigger_w, option, i == *self.selected).clicked() {
                        *self.selected = i;
                        ui.close();
                    }
                }
            });

        response
    }
}

/// One option row: accent-on-hover, the active row tinted to the accent fill.
fn option_row(ui: &mut Ui, tokens: Tokens, width: f32, text: &str, active: bool) -> Response {
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        egui::FontId::proportional(TEXT),
        tokens.foreground,
    );
    let height = 2.0_f32.mul_add(ITEM_PAD_Y, galley.size().y).max(ITEM_MIN_H);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());

    if ui.is_rect_visible(rect) {
        let hovered = response.hovered();
        if hovered || active {
            ui.painter().rect(
                rect,
                tokens.radius_xl(),
                tokens.accent,
                Stroke::NONE,
                StrokeKind::Inside,
            );
        }
        let text_color = if hovered || active {
            tokens.accent_foreground
        } else {
            tokens.foreground
        };
        let galley = ui.painter().layout_no_wrap(
            text.to_owned(),
            egui::FontId::proportional(TEXT),
            text_color,
        );
        let pos = egui::pos2(
            rect.left() + ITEM_PAD_X,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(pos, galley, text_color);
    }
    response
}
