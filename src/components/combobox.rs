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
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// `chevron-down` (lucide) — the trailing affordance on the trigger.
const CHEVRON_DOWN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#;
/// `check` (lucide) — marks the active option in the popup.
const CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>"#;

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
            id_salt: egui::Id::new("glazier-combobox"),
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
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.id_salt = egui::Id::new(("glazier-combobox", egui::Id::new(salt)));
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
        let response = trigger(ui, tokens, width, label, value.is_none(), m);

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
                    empty_row(ui, tokens, panel_w, &self.empty_text, m);
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
                            if option_row(ui, tokens, panel_w, &self.options[*i], active, m)
                                .clicked()
                            {
                                *self.selected = *i;
                                ui.close();
                            }
                        }
                    };

                    #[allow(clippy::cast_precision_loss)] // tiny option counts
                    let content_h = matches.len() as f32 * m.item_min_h;
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

/// Paint the outline trigger row (value/placeholder + trailing chevron) and
/// return its click [`Response`].
fn trigger(
    ui: &mut Ui,
    tokens: Tokens,
    width: f32,
    label: &str,
    is_placeholder: bool,
    m: ComboboxMetrics,
) -> Response {
    let (rect, response) = ui.allocate_at_least(Vec2::new(width, m.height), Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

    if ui.is_rect_visible(rect) {
        // Outlined input chrome; border tightens to the ring on hover.
        let stroke = if response.hovered() {
            Stroke::new(1.0, tokens.ring)
        } else {
            Stroke::new(1.0, tokens.input)
        };
        // Opaque `widget` surface — `background` is the app-canvas token and
        // may be translucent under a user theme.
        ui.painter().rect(
            rect,
            tokens.radius_md(),
            tokens.widget,
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
        let avail = (m.pad_x.mul_add(-2.0, rect.width()) - m.chevron - m.chevron_gap).max(0.0);
        let mut job = egui::text::LayoutJob::simple(
            label.to_owned(),
            egui::FontId::proportional(m.text),
            color,
            avail,
        );
        job.wrap = egui::text::TextWrapping::truncate_at_width(avail);
        let galley = ui.painter().layout_job(job);
        let pos = egui::pos2(
            rect.left() + m.pad_x,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(pos, galley, color);
    }
    response
}

/// One option row: an accent-on-hover command with a leading check when active.
fn option_row(
    ui: &mut Ui,
    tokens: Tokens,
    width: f32,
    text: &str,
    active: bool,
    m: ComboboxMetrics,
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

/// The muted "no results" placeholder row, centered like shadcn's `CommandEmpty`.
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
