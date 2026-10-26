//! [`Combobox`] — a searchable single-select, mirroring shadcn's `<Combobox>`.
//!
//! shadcn builds the combobox by dropping a `Command` (a search field over a
//! filtered list) inside a `Popover`, triggered by an outline button. glazier
//! follows the same recipe: an **outlined** trigger row (`h-9`, hairline
//! `input` border) showing the current value or a placeholder, opening a
//! [`Popover`](crate::Popover) whose panel hosts a search [`Input`] above a
//! check-marked, case-insensitively filtered list of options.
//!
//! Like [`Select`](crate::Select) it binds an index into a fixed option list,
//! but it adds a type-to-filter field — the right control once the list grows
//! past a handful of entries.
//!
//! ```no_run
//! use glazier::combobox::Combobox;
//! # egui::__run_test_ui(|ui| {
//! let mut framework = 0usize;
//! Combobox::new(&mut framework, ["Next.js", "SvelteKit", "Remix", "Astro"])
//!     .placeholder("Select framework\u{2026}")
//!     .search_placeholder("Search framework\u{2026}")
//!     .show(ui);
//! # });
//! ```

use egui::{Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget};

use super::picker::{Mark, OptionRow, ValueLine};
use crate::components::dropdown_menu::DropdownMenu;
use crate::components::input::Input;
use crate::components::scroll_area::ScrollArea;
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry for [`Combobox`] — reach in via [`Combobox::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct ComboboxMetrics {
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
    /// Leading check column width inside an option row.
    pub check_col: f32,
    /// Maximum height of the scrolling option list (`max-h-...`).
    pub list_max_h: f32,
}

impl Default for ComboboxMetrics {
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
            check_col: 22.0,
            list_max_h: 240.0,
        }
    }
}

/// A searchable dropdown picker bound to an index into a fixed option list.
#[must_use = "comboboxes do nothing unless you show them"]
pub struct Combobox<'a> {
    selected: &'a mut usize,
    options: Vec<String>,
    placeholder: String,
    search_placeholder: String,
    empty_text: String,
    width: Option<f32>,
    id_salt: egui::Id,
    /// Use compact (reduced-padding) sizing for the search input.
    compact: bool,
    sizing_hook: SizingHook<ComboboxMetrics>,
}

impl Sizeable<ComboboxMetrics> for Combobox<'_> {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<ComboboxMetrics> {
        &mut self.sizing_hook
    }
}

impl<'a> Combobox<'a> {
    /// Create a combobox bound to `selected`, over the given `options`.
    pub fn new<I, S>(selected: &'a mut usize, options: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            selected,
            options: options.into_iter().map(Into::into).collect(),
            placeholder: "Select option\u{2026}".to_owned(),
            search_placeholder: "Search\u{2026}".to_owned(),
            empty_text: "No results found.".to_owned(),
            width: None,
            id_salt: egui::Id::unique("glazier-combobox"),
            compact: false,
            sizing_hook: SizingHook::default(),
        }
    }

    /// Text shown on the trigger when `selected` is out of range.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Hint text for the in-popup search field.
    pub fn search_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.search_placeholder = placeholder.into();
        self
    }

    /// Message shown when the filter matches no options.
    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty_text = text.into();
        self
    }

    /// Set an explicit width; defaults to the available width.
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Use compact (reduced-padding) sizing for the search input inside the
    /// popover. Defaults to `false`.
    pub const fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    /// Disambiguate multiple comboboxes sharing one view (keeps each one's
    /// search filter + focus state separate).
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = egui::Id::unique(("glazier-combobox", egui::Id::unique(salt)));
        self
    }

    /// Render the trigger and (when open) the searchable popover. Returns the
    /// trigger [`Response`]; the chosen index is written back into `selected`.
    pub fn show(mut self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let width = self.width.unwrap_or_else(|| ui.available_width());

        // ---- Trigger: an outline row showing the value or placeholder. ----
        let value = self.options.get(*self.selected).map(String::as_str);
        let label = value.unwrap_or(&self.placeholder);
        let response = Self::trigger(ui, tokens, width, m).paint(label, value.is_none());

        // ---- Popover: a search field over the filtered option list. ----
        // The panel width matches the trigger (with a sensible floor), and
        // every inner widget is pinned to it so the Area never stretches to the
        // available width (the bug that made it balloon).
        let panel_w = response.rect.width().max(220.0);
        let filter_id = self.id_salt.with("filter");
        let focus_id = self.id_salt.with("needs-focus");

        // Drive `egui::Popup` directly with the tight DropdownMenu frame (p-1),
        // not the rich-content Popover (p-4). It toggles its own open-state off
        // the trigger click; we only set CloseOnClickOutside so typing into the
        // search field doesn't dismiss it, and close ourselves on a pick.
        let opened = egui::Popup::menu(&response)
            .frame(DropdownMenu::frame(tokens))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .gap(4.0)
            .width(panel_w)
            .show(|ui| {
                ui.set_width(panel_w);
                ui.spacing_mut().item_spacing = Vec2::new(0.0, 6.0);

                // Persisted search filter (per combobox id).
                let mut filter: String = ui.data_mut(|d| d.get_temp(filter_id).unwrap_or_default());
                let search = Input::new(&mut filter)
                    .placeholder(self.search_placeholder.clone())
                    .width(panel_w)
                    .compact(self.compact)
                    .ui(ui);
                // Auto-focus the search field the frame the popover opens.
                if ui.data_mut(|d| d.get_temp::<bool>(focus_id).unwrap_or(true)) {
                    search.request_focus();
                    ui.data_mut(|d| d.insert_temp(focus_id, false));
                }
                ui.data_mut(|d| d.insert_temp(filter_id, filter.clone()));

                // Case-insensitive substring filter, preserving option order.
                let needle = filter.trim().to_lowercase();
                let matches: Vec<usize> = self
                    .options
                    .iter()
                    .enumerate()
                    .filter(|(_, opt)| needle.is_empty() || opt.to_lowercase().contains(&needle))
                    .map(|(i, _)| i)
                    .collect();

                if matches.is_empty() {
                    Self::empty_row(ui, tokens, panel_w, &self.empty_text, m);
                } else {
                    let row = OptionRow {
                        tokens,
                        width: panel_w,
                        text: m.text,
                        pad_x: m.item_pad_x,
                        pad_y: m.item_pad_y,
                        min_h: m.item_min_h,
                        mark: Mark::Check(m.check_col),
                    };
                    // Render the rows, optionally inside a ScrollArea. We only
                    // wrap when the list *actually* overflows the cap: egui's
                    // ScrollArea latches its offset/content-size in memory, so
                    // an always-on area keeps a stale scrolled state after the
                    // filter shrinks then restores the list (the bug where a
                    // fully-fitting list still scrolled). When it fits, no area
                    // exists, so there's nothing to latch.
                    let mut rows = |ui: &mut Ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        for i in &matches {
                            let active = *i == *self.selected;
                            if row.show(ui, &self.options[*i], active).clicked() {
                                *self.selected = *i;
                                ui.close();
                            }
                        }
                    };

                    let count = u16::try_from(matches.len()).unwrap_or(u16::MAX);
                    let content_h = f32::from(count) * m.item_min_h;
                    if content_h > m.list_max_h {
                        ScrollArea::new()
                            .max_height(m.list_max_h)
                            .show(ui, &mut rows);
                    } else {
                        rows(ui);
                    }
                }
            });

        // When closed, prime the search field to clear + refocus on next open.
        if opened.is_none() {
            ui.data_mut(|d| {
                d.insert_temp(filter_id, String::new());
                d.insert_temp(focus_id, true);
            });
        }

        response
    }
}

impl Combobox<'_> {
    /// Allocate the outlined trigger row.
    fn trigger(ui: &mut Ui, tokens: Tokens, width: f32, m: ComboboxMetrics) -> Trigger<'_> {
        let (rect, response) = ui.allocate_at_least(Vec2::new(width, m.height), Sense::click());
        Trigger {
            ui,
            tokens,
            m,
            rect,
            response: response.on_hover_cursor(egui::CursorIcon::PointingHand),
        }
    }

    /// The muted "no results" row, centred like shadcn's `CommandEmpty`.
    fn empty_row(ui: &mut Ui, tokens: Tokens, width: f32, text: &str, m: ComboboxMetrics) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, m.item_min_h), Sense::hover());
        if ui.is_rect_visible(rect) {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(m.text),
                tokens.muted_foreground,
            );
        }
    }
}

/// The allocated trigger row, ready to paint.
struct Trigger<'a> {
    ui: &'a Ui,
    tokens: Tokens,
    m: ComboboxMetrics,
    rect: egui::Rect,
    response: Response,
}

impl Trigger<'_> {
    /// Outlined input chrome, its border tightening to the ring on hover,
    /// around the value line. Returns the click response.
    fn paint(self, label: &str, is_placeholder: bool) -> Response {
        let (ui, tokens, m, rect) = (self.ui, self.tokens, self.m, self.rect);
        if ui.is_rect_visible(rect) {
            let border = if self.response.hovered() {
                tokens.ring
            } else {
                tokens.input
            };
            // Opaque `widget` surface — `background` is the app-canvas token
            // and may be translucent under a user theme.
            ui.painter().rect(
                rect,
                tokens.radius_md(),
                tokens.widget,
                Stroke::new(1.0, border),
                StrokeKind::Inside,
            );
            ValueLine {
                pad_x: m.pad_x,
                text: m.text,
                chevron: m.chevron,
                chevron_gap: m.chevron_gap,
            }
            .paint(ui, tokens, rect, label, is_placeholder);
        }
        self.response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;

    const FRAMEWORKS: [&str; 4] = ["Next.js", "SvelteKit", "Remix", "Astro"];

    /// Open, filter, and pick; an unmatched filter shows the empty text.
    #[test]
    fn filtering_then_picking_writes_the_index() {
        let ui = TestUi::new();
        let mut selected = 9;
        let mut build = |ui: &mut Ui| {
            Combobox::new(&mut selected, FRAMEWORKS)
                .placeholder("Pick")
                .empty_text("None")
                .id_salt("fw")
                .compact(true)
                .width(240.0)
                .show(ui);
        };
        let out = ui.frame(&mut build);
        ui.click(TestUi::text_rect(&out, "Pick").center(), &mut build);
        ui.frame(&mut build);
        ui.frame_with(vec![egui::Event::Text("zz".to_owned())], &mut build);
        let out = ui.frame(&mut build);
        assert!(TestUi::text_x(&out, "None") > 0.0);
        let erase = TestUi::key(egui::Key::Backspace);
        ui.frame_with(vec![erase.clone(), erase], &mut build);
        ui.frame_with(vec![egui::Event::Text("rem".to_owned())], &mut build);
        let out = ui.frame(&mut build);
        ui.click(TestUi::text_rect(&out, "Remix").center(), &mut build);
        drop(build);
        assert_eq!(selected, 2);
    }
}
