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

use super::picker::{Mark, OptionRow, ValueLine};
use crate::components::dropdown_menu::DropdownMenu;
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

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
            // bg-input/50, deepening slightly on hover. Blends toward the
            // opaque `widget` surface, not `background` — the app-canvas
            // token can be translucent, which would leak through this
            // trigger.
            let mut fill = tokens.input.lerp_to_gamma(tokens.widget, 0.5);
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

            // A single line, truncated with `…` (shadcn's `truncate`) so a
            // long value never wraps or runs into the 16pt chevron.
            ValueLine {
                pad_x: m.pad_x,
                text: m.text,
                chevron: 16.0,
                chevron_gap: 6.0,
            }
            .paint(ui, tokens, rect, value, is_placeholder);
        }

        // Option popover, hung off the trigger. `Popup::menu` toggles its own
        // open-state from the trigger's click — don't toggle it ourselves, and
        // don't override its id (the toggle is keyed to the default id).
        let list = OptionList {
            row: OptionRow {
                tokens,
                width: response.rect.width(),
                text: m.text,
                pad_x: m.item_pad_x,
                pad_y: m.item_pad_y,
                min_h: m.item_min_h,
                mark: Mark::Check(m.check_col),
            },
            m,
            options: &self.options,
        };
        let groups = &self.groups;
        let selected = &mut *self.selected;
        egui::Popup::menu(&response)
            .frame(DropdownMenu::frame(tokens))
            .gap(4.0)
            .show(|ui| {
                ui.set_min_width(list.row.width);
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let picked = if groups.is_empty() {
                    list.rows(ui, 0..list.options.len(), *selected)
                } else {
                    list.groups(ui, groups, *selected)
                };
                if let Some(i) = picked {
                    *selected = i;
                    ui.close();
                }
            });

        response
    }
}

/// The popup's option list.
struct OptionList<'a> {
    row: OptionRow,
    m: SelectMetrics,
    options: &'a [String],
}

impl OptionList<'_> {
    /// Rows for the options at `indices`. Returns the clicked option.
    fn rows(
        &self,
        ui: &mut Ui,
        indices: impl IntoIterator<Item = usize>,
        selected: usize,
    ) -> Option<usize> {
        let mut clicked = None;
        for i in indices {
            let Some(option) = self.options.get(i) else {
                continue;
            };
            if self.row.show(ui, option, i == selected).clicked() {
                clicked = Some(i);
            }
        }
        clicked
    }

    /// Each group's heading and rows, separated by hairlines.
    fn groups(
        &self,
        ui: &mut Ui,
        groups: &[(String, Vec<usize>)],
        selected: usize,
    ) -> Option<usize> {
        let mut clicked = None;
        for (i, (label, indices)) in groups.iter().enumerate() {
            if i > 0 {
                self.separator(ui);
            }
            self.label(ui, label);
            clicked = self.rows(ui, indices.iter().copied(), selected).or(clicked);
        }
        clicked
    }

    /// A muted, xs section label — mirrors `DropdownMenuEntry::Label`.
    fn label(&self, ui: &mut Ui, text: &str) {
        let tokens = self.row.tokens;
        let galley = ui.painter().layout_no_wrap(
            text.to_owned(),
            egui::FontId::proportional(self.m.group_label_text),
            tokens.muted_foreground,
        );
        // py-1 plus a touch extra so it breathes.
        let height = (galley.size().y + 10.0).max(20.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(self.row.width, height), Sense::hover());
        let pos = egui::pos2(
            rect.left() + self.m.item_pad_x,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(pos, galley, tokens.muted_foreground);
    }

    /// A hairline between groups, 9pt tall like `DropdownMenu`'s, bleeding
    /// to the popup frame's edge.
    fn separator(&self, ui: &mut Ui) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(self.row.width, 9.0), Sense::hover());
        let pad = self.m.popup_pad;
        ui.painter().hline(
            (rect.left() - pad)..=(rect.right() + pad),
            rect.center().y.round(),
            Stroke::new(1.0, self.row.tokens.border.gamma_multiply(0.5)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;

    /// Open the popup, then click `option`; returns the bound index.
    fn pick(grouped: bool, option: &str) -> usize {
        let ui = TestUi::new();
        let mut selected = 0;
        let mut build = |ui: &mut Ui| {
            let s = Select::new(&mut selected, ["Checking", "Savings", "Brokerage"]).width(200.0);
            let s = if grouped {
                s.groups([("Banking", vec![0, 1, 7]), ("Investing", vec![2])])
            } else {
                s
            };
            s.show(ui);
        };
        let out = ui.frame(&mut build);
        ui.click(TestUi::text_rect(&out, "Checking").center(), &mut build);
        let out = ui.frame(&mut build);
        if grouped {
            assert!(TestUi::text_x(&out, "Investing") > 0.0);
        }
        // The popup repeats the value; the last painted copy is the option.
        let pos = out
            .shapes
            .iter()
            .rev()
            .find_map(|c| match &c.shape {
                egui::Shape::Text(t) if t.galley.job.text == option => {
                    Some(egui::Rect::from_min_size(t.pos, t.galley.size()).center())
                }
                _ => None,
            })
            .expect("option painted");
        ui.click(pos, &mut build);
        drop(build);
        selected
    }

    #[test]
    fn picking_writes_the_index() {
        assert_eq!(pick(false, "Savings"), 1);
        assert_eq!(pick(true, "Brokerage"), 2);
    }

    #[test]
    fn an_out_of_range_selection_shows_the_placeholder() {
        let mut selected = 9;
        let out = TestUi::new().frame(|ui| {
            Select::new(&mut selected, ["A"])
                .placeholder("Pick one")
                .show(ui);
        });
        assert!(TestUi::text_x(&out, "Pick one") > 0.0);
    }
}
