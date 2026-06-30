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

use crate::components::dropdown_menu::DropdownMenu;
use crate::components::icon::Icon;
use crate::components::input::Input;
use crate::components::scroll_area::ScrollArea;
use crate::tokens::Tokens;

/// `chevron-down` (lucide) — the trailing affordance on the trigger.
const CHEVRON_DOWN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#;
/// `check` (lucide) — marks the active option in the popup.
const CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>"#;

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
/// Leading check column width inside an option row.
const CHECK_COL: f32 = 22.0;
/// Maximum height of the scrolling option list (`max-h-...`).
const LIST_MAX_H: f32 = 240.0;

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
            id_salt: egui::Id::new("glazier-combobox"),
            compact: false,
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
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.id_salt = egui::Id::new(("glazier-combobox", egui::Id::new(salt)));
        self
    }

    /// Render the trigger and (when open) the searchable popover. Returns the
    /// trigger [`Response`]; the chosen index is written back into `selected`.
    pub fn show(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let width = self.width.unwrap_or_else(|| ui.available_width());

        // ---- Trigger: an outline row showing the value or placeholder. ----
        let value = self.options.get(*self.selected).map(String::as_str);
        let label = value.unwrap_or(&self.placeholder);
        let response = trigger(ui, tokens, width, label, value.is_none());

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
                    empty_row(ui, tokens, panel_w, &self.empty_text);
                } else {
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
                            if option_row(ui, tokens, panel_w, &self.options[*i], active).clicked()
                            {
                                *self.selected = *i;
                                ui.close();
                            }
                        }
                    };

                    #[allow(clippy::cast_precision_loss)] // tiny option counts
                    let content_h = matches.len() as f32 * ITEM_MIN_H;
                    if content_h > LIST_MAX_H {
                        ScrollArea::new().max_height(LIST_MAX_H).show(ui, &mut rows);
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

/// Paint the outline trigger row (value/placeholder + trailing chevron) and
/// return its click [`Response`].
fn trigger(ui: &mut Ui, tokens: Tokens, width: f32, label: &str, is_placeholder: bool) -> Response {
    let (rect, response) = ui.allocate_at_least(Vec2::new(width, HEIGHT), Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

    if ui.is_rect_visible(rect) {
        // Outlined input chrome; border tightens to the ring on hover.
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
            label.to_owned(),
            egui::FontId::proportional(TEXT),
            color,
            avail,
        );
        job.wrap = egui::text::TextWrapping::truncate_at_width(avail);
        let galley = ui.painter().layout_job(job);
        let pos = egui::pos2(rect.left() + PAD_X, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(pos, galley, color);
    }
    response
}

/// One option row: an accent-on-hover command with a leading check when active.
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
                egui::pos2(rect.left() + ITEM_PAD_X, rect.center().y - 8.0),
                Vec2::splat(16.0),
            );
            Icon::new(CHECK)
                .color(text_color)
                .image(tokens)
                .paint_at(ui, icon_rect);
        }
        let galley = ui.painter().layout_no_wrap(
            text.to_owned(),
            egui::FontId::proportional(TEXT),
            text_color,
        );
        let pos = egui::pos2(
            rect.left() + ITEM_PAD_X + CHECK_COL,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(pos, galley, text_color);
    }
    response
}

/// The muted "no results" placeholder row, centered like shadcn's `CommandEmpty`.
fn empty_row(ui: &mut Ui, tokens: Tokens, width: f32, text: &str) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, ITEM_MIN_H), Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(TEXT),
            tokens.muted_foreground,
        );
    }
}
