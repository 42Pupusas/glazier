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
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// `chevron-down` (lucide) — the trailing affordance.
const CHEVRON_DOWN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#;

/// Overridable geometry for [`NativeSelect`] — reach in via
/// [`NativeSelect::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct NativeSelectMetrics {
    /// Trigger height: `h-9` (36px).
    pub height: f32,
    /// Trigger horizontal padding (`px-3`).
    pub pad_x: f32,
    /// Value / option text size (`text-sm`).
    pub text: f32,
    /// Chevron edge length.
    pub chevron: f32,
    /// Gap reserved before the chevron.
    pub chevron_gap: f32,
    /// Option row horizontal padding inside the popover.
    pub item_pad_x: f32,
    /// Option row vertical padding inside the popover.
    pub item_pad_y: f32,
    /// Minimum option height (`min-h-8`).
    pub item_min_h: f32,
}

impl Default for NativeSelectMetrics {
    fn default() -> Self {
        Self {
            height: 36.0,
            pad_x: 12.0,
            text: 14.0,
            chevron: 16.0,
            chevron_gap: 6.0,
            item_pad_x: 8.0,
            item_pad_y: 6.0,
            item_min_h: 32.0,
        }
    }
}

/// A bordered, native-style dropdown picker bound to an index.
#[must_use = "selects do nothing unless you show them"]
pub struct NativeSelect<'a> {
    selected: &'a mut usize,
    options: Vec<String>,
    placeholder: Option<String>,
    width: Option<f32>,
    sizing_hook: SizingHook<NativeSelectMetrics>,
}

impl Sizeable<NativeSelectMetrics> for NativeSelect<'_> {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<NativeSelectMetrics> {
        &mut self.sizing_hook
    }
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
            sizing_hook: SizingHook::default(),
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
    pub fn show(mut self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let width = self.width.unwrap_or_else(|| ui.available_width());

        let value = self
            .options
            .get(*self.selected)
            .map(String::as_str)
            .or(self.placeholder.as_deref())
            .unwrap_or("");
        let is_placeholder = self.options.get(*self.selected).is_none();

        let (rect, response) = ui.allocate_at_least(Vec2::new(width, m.height), Sense::click());
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
                    rect.right() - m.pad_x - m.chevron,
                    rect.center().y - m.chevron / 2.0,
                ),
                Vec2::splat(m.chevron),
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
            let avail =
                (m.pad_x.mul_add(-2.0, rect.width()) - m.chevron - m.chevron_gap).max(0.0);
            let mut job = egui::text::LayoutJob::simple(
                value.to_owned(),
                egui::FontId::proportional(m.text),
                color,
                avail,
            );
            job.wrap = egui::text::TextWrapping::truncate_at_width(avail);
            let galley = ui.painter().layout_job(job);
            let pos = egui::pos2(rect.left() + m.pad_x, rect.center().y - galley.size().y / 2.0);
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
                    if option_row(ui, tokens, trigger_w, option, i == *self.selected, m)
                        .clicked()
                    {
                        *self.selected = i;
                        ui.close();
                    }
                }
            });

        response
    }
}

/// One option row: accent-on-hover, the active row tinted to the accent fill.
fn option_row(
    ui: &mut Ui,
    tokens: Tokens,
    width: f32,
    text: &str,
    active: bool,
    m: NativeSelectMetrics,
) -> Response {
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        egui::FontId::proportional(m.text),
        tokens.foreground,
    );
    let height = 2.0_f32
        .mul_add(m.item_pad_y, galley.size().y)
        .max(m.item_min_h);
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
            egui::FontId::proportional(m.text),
            text_color,
        );
        let pos = egui::pos2(
            rect.left() + m.item_pad_x,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(pos, galley, text_color);
    }
    response
}
