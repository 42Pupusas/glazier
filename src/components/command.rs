//! [`Command`] — a command palette, mirroring shadcn's `<Command>`.
//!
//! A search field over a grouped, **keyboard-navigable** action list — the
//! `⌘K` menu. shadcn builds it on `cmdk`; glazier ports the surface directly: a
//! borderless search header (magnifier + input) above a hairline divider, then
//! a scrollable body of [groups](CommandGroup) (muted heading + items). Each
//! [item](CommandItem) carries an optional leading [`Icon`] and a trailing
//! shortcut [`Kbd`] chip. Type to filter case-insensitively; ↑/↓ move the
//! highlight (wrapping, auto-scrolling into view); Enter (or a click) activates.
//!
//! This is the same engine shadcn drops inside a Popover to make a Combobox, or
//! inside a Dialog to make the `⌘K` palette — here it's the standalone surface.
//! [`show`](Command::show) returns the **id of the activated item**, if any:
//!
//! ```no_run
//! use glazier::command::{Command, CommandGroup, CommandItem};
//! # egui::__run_test_ui(|ui| {
//! let activated = Command::new("cmd-demo")
//!     .placeholder("Type a command or search…")
//!     .group(CommandGroup::new("Suggestions").items([
//!         CommandItem::new("calendar", "Calendar"),
//!         CommandItem::new("search", "Search Emoji"),
//!     ]))
//!     .group(CommandGroup::new("Settings").items([
//!         CommandItem::new("profile", "Profile").shortcut("Ctrl P"),
//!         CommandItem::new("settings", "Settings").shortcut("Ctrl S"),
//!     ]))
//!     .show(ui);
//! if let Some(id) = activated {
//!     // route on `id`…
//! }
//! # });
//! ```

use egui::{Key, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2};

use crate::components::icon::Icon;
use crate::components::kbd::Kbd;
use crate::components::scroll_area::ScrollArea;
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;
use egui::Widget as _;

/// `search` (lucide) — the leading glyph in the search header.
const SEARCH: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>"#;

/// Overridable geometry for [`Command`] — reach in via [`Command::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct CommandMetrics {
    /// Item / input text size (`text-sm`).
    pub text: f32,
    /// Group heading size (`text-xs`).
    pub heading: f32,
    /// Search header height.
    pub header_h: f32,
    /// Magnifier edge.
    pub search_icon: f32,
    /// Horizontal padding inside the surface (`px-2`/`px-3`).
    pub pad_x: f32,
    /// Item row left/right padding.
    pub item_pad_x: f32,
    /// Minimum item height (`min-h-8`).
    pub item_min_h: f32,
    /// Leading icon column width inside an item row (16px icon + 12px gap).
    pub icon_col: f32,
    /// Max height of the scrolling list before it scrolls.
    pub list_max_h: f32,
    /// Default surface width.
    pub default_width: f32,
}

impl Default for CommandMetrics {
    fn default() -> Self {
        Self {
            text: 14.0,
            heading: 12.0,
            header_h: 40.0,
            search_icon: 16.0,
            pad_x: 10.0,
            item_pad_x: 8.0,
            item_min_h: 36.0,
            icon_col: 28.0,
            list_max_h: 300.0,
            default_width: 420.0,
        }
    }
}

/// One activatable command — a leading icon, a label, an optional shortcut.
pub struct CommandItem {
    id: String,
    label: String,
    icon: Option<Icon>,
    shortcut: Option<String>,
    keywords: Vec<String>,
}

impl CommandItem {
    /// Create an item with a stable `id` (returned when activated) and a label.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            shortcut: None,
            keywords: Vec::new(),
        }
    }

    /// Add a leading [`Icon`].
    #[must_use]
    pub const fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Add a trailing shortcut hint, rendered as a [`Kbd`] chip.
    ///
    /// Spell it in words (`"Ctrl P"`) for cross-platform legibility — the
    /// Apple modifier glyphs (⌘, ⇧, …) aren't in egui's bundled fonts and show
    /// as tofu ▯. See [`Kbd`] for the font-coverage caveat.
    #[must_use]
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Add extra search keywords that match this item beyond its label.
    #[must_use]
    pub fn keywords<I, S>(mut self, keywords: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.keywords = keywords.into_iter().map(Into::into).collect();
        self
    }

    /// Does this item match `needle` (already lowercased)?
    fn matches(&self, needle: &str) -> bool {
        if needle.is_empty() {
            return true;
        }
        self.label.to_lowercase().contains(needle)
            || self
                .keywords
                .iter()
                .any(|k| k.to_lowercase().contains(needle))
    }
}

/// A titled group of [items](CommandItem), shadcn's `CommandGroup`.
pub struct CommandGroup {
    heading: Option<String>,
    items: Vec<CommandItem>,
}

impl CommandGroup {
    /// Create a group with a muted heading.
    pub fn new(heading: impl Into<String>) -> Self {
        Self {
            heading: Some(heading.into()),
            items: Vec::new(),
        }
    }

    /// Create a group with no heading.
    #[must_use]
    pub const fn untitled() -> Self {
        Self {
            heading: None,
            items: Vec::new(),
        }
    }

    /// Append one item.
    #[must_use]
    pub fn item(mut self, item: CommandItem) -> Self {
        self.items.push(item);
        self
    }

    /// Append several items.
    #[must_use]
    pub fn items<I>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = CommandItem>,
    {
        self.items.extend(items);
        self
    }
}

/// A command palette surface.
#[must_use = "commands do nothing unless you show them"]
pub struct Command {
    groups: Vec<CommandGroup>,
    placeholder: String,
    empty_text: String,
    width: Option<f32>,
    id_salt: egui::Id,
    sizing_hook: SizingHook<CommandMetrics>,
}

impl Sizeable<CommandMetrics> for Command {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<CommandMetrics> {
        &mut self.sizing_hook
    }
}

impl Command {
    /// Create an empty palette. `id_salt` keys its persisted filter/highlight
    /// state, so give sibling palettes distinct salts.
    pub fn new(id_salt: impl std::hash::Hash) -> Self {
        Self {
            groups: Vec::new(),
            placeholder: "Type a command or search…".to_owned(),
            empty_text: "No results found.".to_owned(),
            width: None,
            id_salt: egui::Id::new(("glazier-command", egui::Id::new(id_salt))),
            sizing_hook: SizingHook::default(),
        }
    }

    /// Set the search field's placeholder.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the "no results" text shown when the filter matches nothing.
    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty_text = text.into();
        self
    }

    /// Set an explicit surface width (defaults to `DEFAULT_WIDTH`, capped at the
    /// available width).
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Append a group.
    pub fn group(mut self, group: CommandGroup) -> Self {
        self.groups.push(group);
        self
    }

    /// Render the palette. Returns the id of the activated item, if any
    /// (Enter on the highlighted row, or a click).
    #[allow(clippy::too_many_lines)] // the search + nav + grouped body as one
    pub fn show(mut self, ui: &mut Ui) -> Option<String> {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let width = self
            .width
            .unwrap_or(m.default_width)
            .min(ui.available_width());

        let filter_id = self.id_salt.with("filter");
        let focus_id = self.id_salt.with("needs-focus");
        let hi_id = self.id_salt.with("highlight");

        let mut filter: String = ui.data_mut(|d| d.get_temp(filter_id).unwrap_or_default());
        let prev_filter = filter.clone();

        let mut activated: Option<String> = None;

        ui.allocate_ui(Vec2::new(width, 0.0), |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = Vec2::ZERO;

            // ---- Search header: magnifier + frameless input + divider. ----
            let field_id = self.id_salt.with("search-field");
            let field = search_header(
                ui,
                tokens,
                width,
                &mut filter,
                &self.placeholder,
                focus_id,
                field_id,
                m,
            );

            // ---- Capture the navigation keys while the field is focused. ----
            // A focused singleline `TextEdit` surrenders focus on up/down (egui's
            // keyboard focus-navigation) and treats Enter as commit+blur — so
            // without intervention our nav code never sees them. Mirror the
            // proven TimePicker approach: lock the vertical arrows to the field,
            // then `consume_key` them (and Enter) before anyone else acts on the
            // press. Gated on the field's *actual* focus, so we never eat keys
            // globally when the user has clicked elsewhere.
            let (mut down, mut up, mut enter) = (false, false, false);
            if field.has_focus() {
                ui.memory_mut(|m| {
                    m.set_focus_lock_filter(
                        field_id,
                        egui::EventFilter {
                            tab: false,
                            horizontal_arrows: false,
                            vertical_arrows: true,
                            escape: false,
                        },
                    );
                });
                ui.input_mut(|i| {
                    down = i.consume_key(egui::Modifiers::NONE, Key::ArrowDown);
                    up = i.consume_key(egui::Modifiers::NONE, Key::ArrowUp);
                    enter = i.consume_key(egui::Modifiers::NONE, Key::Enter);
                });
            }

            divider(ui, tokens, width);

            // Reset the highlight to the top whenever the query changes.
            if filter != prev_filter {
                ui.data_mut(|d| d.insert_temp(hi_id, 0usize));
            }

            // ---- Flatten visible items for keyboard navigation. ----
            let needle = filter.trim().to_lowercase();
            let visible: Vec<(usize, usize)> = self
                .groups
                .iter()
                .enumerate()
                .flat_map(|(gi, g)| {
                    g.items
                        .iter()
                        .enumerate()
                        .filter(|(_, it)| it.matches(&needle))
                        .map(move |(ii, _)| (gi, ii))
                })
                .collect();

            // ---- Keyboard navigation over the flat visible list. ----
            let mut highlight: usize = ui.data_mut(|d| d.get_temp(hi_id).unwrap_or(0));
            let mut nav_moved = false;
            let n = visible.len();
            if n == 0 {
                highlight = 0;
            } else {
                highlight = highlight.min(n - 1);
                if down {
                    highlight = (highlight + 1) % n;
                }
                if up {
                    highlight = (highlight + n - 1) % n;
                }
                // Only nudge the row into view when the keyboard *moved* the
                // highlight — never every frame (that bubbles up to the page
                // scroll area and drags the whole gallery down).
                nav_moved = down || up;
                if enter {
                    let (gi, ii) = visible[highlight];
                    activated = Some(self.groups[gi].items[ii].id.clone());
                }
            }

            // ---- Real accelerators: fire an item when its shortcut is pressed.
            // Each item's `shortcut` string (e.g. "Ctrl P") is parsed into a
            // chord and consumed globally, so pressing it activates the command
            // even while the search field holds focus.
            if activated.is_none() {
                'accel: for group in &self.groups {
                    for item in &group.items {
                        let Some(text) = &item.shortcut else { continue };
                        let Some(chord) = parse_shortcut(text) else {
                            continue;
                        };
                        if ui.input_mut(|i| i.consume_shortcut(&chord)) {
                            activated = Some(item.id.clone());
                            break 'accel;
                        }
                    }
                }
            }

            // ---- Body: grouped, scrollable item list. ----
            if n == 0 {
                empty_row(ui, tokens, width, &self.empty_text, m);
            } else {
                // `scroll` is true only inside our own ScrollArea; when the
                // list fits we render the rows directly and must NEVER call
                // scroll_to_me (it would propagate to the page scroll area).
                let mut body = |ui: &mut Ui, scroll: bool| {
                    let mut flat = 0usize;
                    for group in &self.groups {
                        let any = group.items.iter().any(|it| it.matches(&needle));
                        if !any {
                            continue;
                        }
                        if let Some(heading) = &group.heading {
                            group_heading(ui, tokens, width, heading, m);
                        }
                        for item in &group.items {
                            if !item.matches(&needle) {
                                continue;
                            }
                            let active = flat == highlight;
                            let resp = item_row(ui, tokens, width, item, active, m);
                            if resp.clicked() {
                                activated = Some(item.id.clone());
                            }
                            // Only when a keypress moved the highlight, and only
                            // inside our own scroll viewport.
                            if active && scroll && nav_moved {
                                resp.scroll_to_me(None);
                            }
                            flat += 1;
                        }
                    }
                };

                #[allow(clippy::cast_precision_loss)] // tiny item counts
                let content_h = n as f32 * m.item_min_h;
                if content_h > m.list_max_h {
                    ScrollArea::new()
                        .max_height(m.list_max_h)
                        .show(ui, |ui| body(ui, true));
                } else {
                    body(ui, false);
                }
            }

            ui.data_mut(|d| {
                d.insert_temp(hi_id, highlight);
                d.insert_temp(filter_id, filter.clone());
            });
        });

        activated
    }
}

/// Parse a human shortcut string (`"Ctrl P"`, `"Ctrl Shift K"`) into an egui
/// [`KeyboardShortcut`]. Modifier words are case-insensitive (`ctrl`/`control`,
/// `cmd`/`command`/`super`/`win`, `alt`/`option`, `shift`); the final token is
/// the key, parsed by [`Key::from_name`]. Returns `None` when there is no
/// recognisable key, so unparseable hints stay purely decorative.
fn parse_shortcut(text: &str) -> Option<egui::KeyboardShortcut> {
    let mut modifiers = egui::Modifiers::NONE;
    let mut key = None;
    for token in text
        .split(|c: char| c.is_whitespace() || c == '+')
        .filter(|t| !t.is_empty())
    {
        match token.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers.ctrl = true,
            "cmd" | "command" | "super" | "win" | "meta" => modifiers.mac_cmd = true,
            "alt" | "option" => modifiers.alt = true,
            "shift" => modifiers.shift = true,
            _ => key = Key::from_name(token),
        }
    }
    key.map(|k| egui::KeyboardShortcut::new(modifiers, k))
}

/// The borderless search header: a magnifier glyph and a frameless text field.
/// Returns the field's [`egui::Response`] so the caller can read its focus.
#[allow(clippy::too_many_arguments)]
fn search_header(
    ui: &mut Ui,
    tokens: Tokens,
    width: f32,
    filter: &mut String,
    placeholder: &str,
    focus_id: egui::Id,
    field_id: egui::Id,
    m: CommandMetrics,
) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, m.header_h), Sense::hover());
    let icon_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + m.pad_x, rect.center().y - m.search_icon / 2.0),
        Vec2::splat(m.search_icon),
    );
    Icon::new(SEARCH)
        .color(tokens.muted_foreground)
        .image(tokens)
        .paint_at(ui, icon_rect);

    let field_rect = egui::Rect::from_min_max(
        egui::pos2(icon_rect.right() + 8.0, rect.top()),
        egui::pos2(rect.right() - m.pad_x, rect.bottom()),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(field_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let output = TextEdit::singleline(filter)
        .id(field_id)
        .frame(egui::Frame::NONE)
        .desired_width(field_rect.width())
        .hint_text(placeholder)
        .font(egui::FontId::proportional(m.text))
        .margin(egui::Margin::ZERO)
        .show(&mut child);

    // Auto-focus the field the frame the palette first appears.
    if ui.data_mut(|d| d.get_temp::<bool>(focus_id).unwrap_or(true)) {
        output.response.request_focus();
        ui.data_mut(|d| d.insert_temp(focus_id, false));
    }
    // `AtomLayoutResponse` derefs to the real `Response`; hand that back.
    output.response.response
}

/// A full-width hairline divider under the search header.
fn divider(ui: &mut Ui, tokens: Tokens, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 1.0), Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        Stroke::new(1.0, tokens.border),
    );
    ui.add_space(4.0);
}

/// A muted group heading row (`text-xs`, `font-medium`).
fn group_heading(ui: &mut Ui, tokens: Tokens, width: f32, text: &str, m: CommandMetrics) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 24.0), Sense::hover());
    ui.painter().text(
        egui::pos2(rect.left() + m.item_pad_x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(m.heading),
        tokens.muted_foreground,
    );
}

/// One command row: accent-on-highlight, leading icon, trailing shortcut chip.
fn item_row(
    ui: &mut Ui,
    tokens: Tokens,
    width: f32,
    item: &CommandItem,
    active: bool,
    m: CommandMetrics,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, m.item_min_h), Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let lit = active || response.hovered();

    if ui.is_rect_visible(rect) {
        if lit {
            ui.painter().rect(
                rect,
                tokens.radius_xl(),
                tokens.accent,
                Stroke::NONE,
                StrokeKind::Inside,
            );
        }
        let text_color = if lit {
            tokens.accent_foreground
        } else {
            tokens.foreground
        };

        // Leading icon.
        let mut text_left = rect.left() + m.item_pad_x;
        if let Some(icon) = item.icon {
            let icon_rect = egui::Rect::from_min_size(
                egui::pos2(text_left, rect.center().y - 8.0),
                Vec2::splat(16.0),
            );
            icon.color(text_color).image(tokens).paint_at(ui, icon_rect);
            text_left += m.icon_col;
        }

        // Label.
        let galley = ui.painter().layout_no_wrap(
            item.label.clone(),
            egui::FontId::proportional(m.text),
            text_color,
        );
        ui.painter().galley(
            egui::pos2(text_left, rect.center().y - galley.size().y / 2.0),
            galley,
            text_color,
        );

        // Trailing shortcut chip.
        if let Some(shortcut) = &item.shortcut {
            // Give the chip the right edge up to the label; the right-to-left
            // layout sizes the Kbd to its text, so a generous slot just sets the
            // left clip bound (wide enough for multi-key hints like "Ctrl P").
            let chip_w = 96.0;
            let chip_rect = egui::Rect::from_min_max(
                egui::pos2(rect.right() - m.item_pad_x - chip_w, rect.top()),
                egui::pos2(rect.right() - m.item_pad_x, rect.bottom()),
            );
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(chip_rect)
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
            );
            Kbd::new(shortcut.clone()).ui(&mut child);
        }
    }
    response
}

/// The muted, centered "no results" row (shadcn's `CommandEmpty`).
fn empty_row(ui: &mut Ui, tokens: Tokens, width: f32, text: &str, m: CommandMetrics) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 64.0), Sense::hover());
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(m.text),
        tokens.muted_foreground,
    );
}
