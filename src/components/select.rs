//! [`Select`] — a dropdown picker, mirroring shadcn's `<Select>`.
//!
//! A select is a filled, `rounded-2xl` trigger row (`bg-input/50`) showing the
//! current value on the left and a muted chevron-down on the right. Clicking it
//! opens a popover listing the options; picking one writes its index back into
//! the bound `selected` and closes the popup.
//!
//! ```no_run
//! use glazier::select::Select;
//! # egui::__run_test_ui(|ui| {
//! let mut account = 0usize;
//! Select::new(&mut account, ["Checking", "Savings"])
//!     .placeholder("Pick an account")
//!     .show(ui);
//! # });
//! ```

use egui::{Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::components::dropdown_menu::DropdownMenu;
use crate::components::icon::Icon;
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// `chevron-down` (lucide) — the trailing affordance.
const CHEVRON_DOWN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#;
/// `check` (lucide) — marks the active option in the popup.
const CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>"#;

/// Overridable geometry for [`Select`] — reach in via [`Select::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct SelectMetrics {
    /// Text size for group/section labels inside the popup (`text-xs`).
    pub group_label_text: f32,
    /// The popover frame's inner padding (mirrors [`DropdownMenu::frame`]'s
    /// `Margin::same(4)`) — used to bleed the separator to the frame edge.
    pub popup_pad: f32,
    /// Trigger inner height: `h-8` (32px) minus the frame's 12px vertical
    /// padding.
    pub row_height: f32,
    /// Trigger horizontal padding (`px-3`).
    pub pad_x: f32,
    /// Trigger vertical padding (`py-2`).
    pub pad_y: f32,
    /// Option row horizontal padding inside the popover.
    pub item_pad_x: f32,
    /// Option row vertical padding inside the popover.
    pub item_pad_y: f32,
    /// Minimum option height (`min-h-8`).
    pub item_min_h: f32,
    /// Value / option text size (`text-sm`).
    pub text: f32,
    /// Leading check column width inside an option row.
    pub check_col: f32,
}

impl Default for SelectMetrics {
    fn default() -> Self {
        Self {
            group_label_text: 11.0,
            popup_pad: 4.0,
            row_height: 20.0,
            pad_x: 12.0,
            pad_y: 6.0,
            item_pad_x: 8.0,
            item_pad_y: 6.0,
            item_min_h: 32.0,
            text: 13.0,
            check_col: 22.0,
        }
    }
}

/// A dropdown picker bound to an index into a fixed option list.
#[must_use = "selects do nothing unless you show them"]
pub struct Select<'a> {
    selected: &'a mut usize,
    options: Vec<String>,
    placeholder: Option<String>,
    width: Option<f32>,
    /// Named groups of option indices. When non-empty the popup renders
    /// section headers + separators between groups instead of a flat list.
    /// Indices are into `options`.
    groups: Vec<(String, Vec<usize>)>,
    sizing_hook: SizingHook<SelectMetrics>,
}

impl Sizeable<SelectMetrics> for Select<'_> {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<SelectMetrics> {
        &mut self.sizing_hook
    }
}

impl<'a> Select<'a> {
    /// Create a select bound to `selected`, over the given `options`.
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
            groups: Vec::new(),
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

    /// Group the options under named section headers in the popup.
    ///
    /// `groups` is an ordered list of `(label, indices)` pairs where each
    /// index refers to a position in the `options` list passed to [`Self::new`].
    /// Groups are separated by a hairline divider. When the list has only one
    /// group the header is still rendered (it names the provider), so only
    /// call this when grouping is meaningful (≥ 2 providers).
    pub fn groups(
        mut self,
        groups: impl IntoIterator<Item = (impl Into<String>, impl IntoIterator<Item = usize>)>,
    ) -> Self {
        self.groups = groups
            .into_iter()
            .map(|(label, indices)| (label.into(), indices.into_iter().collect()))
            .collect();
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

        // Allocate the whole trigger as ONE clickable rect (mirrors Button), so
        // the hover cursor + click land reliably — then paint into it.
        let height = m.pad_y.mul_add(2.0, m.row_height); // h-8
        let (rect, response) = ui.allocate_at_least(Vec2::new(width, height), Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

        if ui.is_rect_visible(rect) {
            let hovered = response.hovered();
            // bg-input/50, deepening slightly on hover.
            let mut fill = tokens.input.lerp_to_gamma(tokens.background, 0.5);
            if hovered {
                fill = fill.gamma_multiply(0.92);
            }
            ui.painter().rect(
                rect,
                tokens.radius_2xl(),
                fill,
                Stroke::NONE,
                StrokeKind::Inside,
            );

            let pad_x = m.pad_x;
            // Trailing chevron.
            let chevron_rect = egui::Rect::from_min_size(
                egui::pos2(rect.right() - pad_x - 16.0, rect.center().y - 8.0),
                Vec2::splat(16.0),
            );
            Icon::new(CHEVRON_DOWN)
                .color(tokens.muted_foreground)
                .image(tokens)
                .paint_at(ui, chevron_rect);

            // Leading value text: a single line, truncated with `…` (shadcn's
            // `truncate`/`line-clamp-1`) so a long value never wraps or spills
            // into the trailing chevron.
            let color = if is_placeholder {
                tokens.muted_foreground
            } else {
                tokens.foreground
            };
            let avail = (pad_x.mul_add(-2.0, rect.width()) - 22.0).max(0.0);
            let mut job = egui::text::LayoutJob::simple(
                value.to_owned(),
                egui::FontId::proportional(m.text),
                color,
                avail,
            );
            job.wrap = egui::text::TextWrapping::truncate_at_width(avail);
            let galley = ui.painter().layout_job(job);
            let pos = egui::pos2(rect.left() + pad_x, rect.center().y - galley.size().y / 2.0);
            ui.painter().galley(pos, galley, color);
        }

        // Option popover, hung off the trigger. `Popup::menu` toggles its own
        // open-state from the trigger's click — don't toggle it ourselves, and
        // don't override its id (the toggle is keyed to the default id).
        let trigger_w = response.rect.width();
        let groups = self.groups; // move out before the closure borrows selected
        egui::Popup::menu(&response)
            .frame(DropdownMenu::frame(tokens))
            .gap(4.0)
            .show(|ui| {
                ui.set_min_width(trigger_w);
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                if groups.is_empty() {
                    // Flat list — original behaviour.
                    for (i, option) in self.options.iter().enumerate() {
                        if option_row(ui, tokens, trigger_w, option, i == *self.selected, m)
                            .clicked()
                        {
                            *self.selected = i;
                            ui.close();
                        }
                    }
                } else {
                    // Grouped list: section header → option rows → separator.
                    for (g_idx, (label, indices)) in groups.iter().enumerate() {
                        if g_idx > 0 {
                            group_separator_row(ui, tokens, trigger_w, m);
                        }
                        group_label_row(ui, tokens, trigger_w, label, m);
                        for &opt_idx in indices {
                            if let Some(option) = self.options.get(opt_idx) {
                                let active = opt_idx == *self.selected;
                                if option_row(ui, tokens, trigger_w, option, active, m).clicked() {
                                    *self.selected = opt_idx;
                                    ui.close();
                                }
                            }
                        }
                    }
                }
            });

        response
    }
}

/// A muted, xs-weight section label row — mirrors `DropdownMenuEntry::Label`.
fn group_label_row(ui: &mut Ui, tokens: Tokens, width: f32, text: &str, m: SelectMetrics) {
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        egui::FontId::proportional(m.group_label_text),
        tokens.muted_foreground,
    );
    // py-1 top + text height + a touch extra so it breathes.
    let height = (galley.size().y + 10.0).max(20.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let pos = egui::pos2(
            rect.left() + m.item_pad_x,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(pos, galley, tokens.muted_foreground);
    }
}

/// A hairline separator between groups — bleeds to the popup frame edge.
fn group_separator_row(ui: &mut Ui, tokens: Tokens, width: f32, m: SelectMetrics) {
    // 9px tall (4px margin + 1px line + 4px margin), matching `DropdownMenu`.
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 9.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let y = rect.center().y.round();
        ui.painter().hline(
            (rect.left() - m.popup_pad)..=(rect.right() + m.popup_pad),
            y,
            egui::Stroke::new(1.0, tokens.border.gamma_multiply(0.5)),
        );
    }
}

/// One option row: an accent-on-hover command with a leading check when active.
fn option_row(
    ui: &mut Ui,
    tokens: Tokens,
    width: f32,
    text: &str,
    active: bool,
    m: SelectMetrics,
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
        if hovered {
            ui.painter().rect(
                rect,
                tokens.radius_xl(),
                tokens.accent,
                Stroke::NONE,
                StrokeKind::Inside,
            );
        }
        let text_color = if hovered {
            tokens.accent_foreground
        } else {
            tokens.foreground
        };
        // Leading check column (drawn only when active).
        if active {
            let icon_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left() + m.item_pad_x, rect.center().y - 8.0),
                Vec2::splat(16.0),
            );
            Icon::new(CHECK)
                .color(text_color)
                .image(tokens)
                .paint_at(ui, icon_rect);
        }
        let galley = ui.painter().layout_no_wrap(
            text.to_owned(),
            egui::FontId::proportional(m.text),
            text_color,
        );
        let pos = egui::pos2(
            rect.left() + m.item_pad_x + m.check_col,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(pos, galley, text_color);
    }
    response
}
