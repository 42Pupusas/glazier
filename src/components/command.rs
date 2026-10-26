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
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            groups: Vec::new(),
            placeholder: "Type a command or search…".to_owned(),
            empty_text: "No results found.".to_owned(),
            width: None,
            id_salt: egui::Id::unique(("glazier-command", egui::Id::unique(id_salt))),
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
    pub fn show(mut self, ui: &mut Ui) -> Option<String> {
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let width = self
            .width
            .unwrap_or(m.default_width)
            .min(ui.available_width());
        let palette = Palette {
            tokens: Tokens::get(ui),
            m,
            width,
            id: self.id_salt,
        };
        ui.allocate_ui(Vec2::new(width, 0.0), |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            self.run(ui, &palette)
        })
        .inner
    }

    /// One frame: search, navigate, and list. Returns the activated item.
    fn run(&self, ui: &mut Ui, palette: &Palette) -> Option<String> {
        let filter_id = palette.id.with("filter");
        let hi_id = palette.id.with("highlight");
        let mut filter: String = ui.data_mut(|d| d.get_temp(filter_id).unwrap_or_default());
        let prev_filter = filter.clone();

        let nav = Nav::capture(ui, palette.field_id());
        palette.search_header(ui, &mut filter, &self.placeholder);
        palette.divider(ui);

        // A new query starts the highlight back at the top.
        let stored = if filter == prev_filter {
            ui.data(|d| d.get_temp(hi_id).unwrap_or(0))
        } else {
            0
        };
        let needle = filter.trim().to_lowercase();
        let visible = self.visible(&needle);
        let highlight = nav.step(stored, visible.len());
        let chosen = visible
            .get(highlight)
            .filter(|_| nav.enter)
            .map(|&(gi, ii)| self.groups[gi].items[ii].id.clone());
        let mut activated = chosen.or_else(|| self.accelerator(ui));

        if visible.is_empty() {
            palette.empty_row(ui, &self.empty_text);
        } else {
            let list = List {
                palette,
                groups: &self.groups,
                needle: &needle,
                highlight,
                // Only a keypress may scroll the highlight into view; doing it
                // every frame drags the surrounding page along.
                nav_moved: nav.moved(),
            };
            activated = list.show(ui, visible.len()).or(activated);
        }

        ui.data_mut(|d| {
            d.insert_temp(hi_id, highlight);
            d.insert_temp(filter_id, filter);
        });
        activated
    }

    /// The `(group, item)` indices matching `needle`, in display order.
    fn visible(&self, needle: &str) -> Vec<(usize, usize)> {
        self.groups
            .iter()
            .enumerate()
            .flat_map(|(gi, g)| {
                g.items
                    .iter()
                    .enumerate()
                    .filter(|(_, it)| it.matches(needle))
                    .map(move |(ii, _)| (gi, ii))
            })
            .collect()
    }

    /// The item whose shortcut was pressed this frame, consuming the chord so
    /// it fires even while the search field holds focus.
    fn accelerator(&self, ui: &Ui) -> Option<String> {
        self.groups
            .iter()
            .flat_map(|g| &g.items)
            .find(|item| {
                item.shortcut
                    .as_deref()
                    .and_then(Shortcut::parse)
                    .is_some_and(|chord| ui.input_mut(|i| i.consume_shortcut(&chord)))
            })
            .map(|item| item.id.clone())
    }
}

/// The navigation keys pressed while the search field had focus.
#[derive(Clone, Copy, Default)]
struct Nav {
    down: bool,
    up: bool,
    enter: bool,
}

impl Nav {
    /// Claim ↑/↓/Enter for a focused search field, before the field runs.
    ///
    /// A focused singleline `TextEdit` surrenders focus on ↑/↓ (egui's focus
    /// navigation) and treats Enter as commit-and-blur, so the palette would
    /// never see them. Locking the vertical arrows to the field and consuming
    /// all three before it is drawn keeps them ours. Only while the field
    /// really has focus, so keys are never eaten globally.
    fn capture(ui: &Ui, field: egui::Id) -> Self {
        if !ui.memory(|m| m.has_focus(field)) {
            return Self::default();
        }
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                field,
                egui::EventFilter {
                    tab: false,
                    horizontal_arrows: false,
                    vertical_arrows: true,
                    escape: false,
                },
            );
        });
        let [down, up, enter] = ui.input_mut(|i| {
            [Key::ArrowDown, Key::ArrowUp, Key::Enter]
                .map(|key| i.consume_key(egui::Modifiers::NONE, key))
        });
        Self { down, up, enter }
    }

    /// Move `highlight` within `n` rows, wrapping.
    const fn step(self, highlight: usize, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        let h = if highlight < n { highlight } else { n - 1 };
        match (self.down, self.up) {
            (true, false) => (h + 1) % n,
            (false, true) => (h + n - 1) % n,
            _ => h,
        }
    }

    const fn moved(self) -> bool {
        self.down || self.up
    }
}

/// One frame of the palette surface.
struct Palette {
    tokens: Tokens,
    m: CommandMetrics,
    width: f32,
    id: egui::Id,
}

impl Palette {
    fn field_id(&self) -> egui::Id {
        self.id.with("search-field")
    }

    /// The borderless search header: a magnifier glyph and a frameless text
    /// field, focused the frame the palette first appears.
    fn search_header(&self, ui: &mut Ui, filter: &mut String, placeholder: &str) {
        let (tokens, m) = (self.tokens, self.m);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(self.width, m.header_h), Sense::hover());
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
            .id(self.field_id())
            .frame(egui::Frame::NONE)
            .desired_width(field_rect.width())
            .hint_text(placeholder)
            .font(egui::FontId::proportional(m.text))
            .margin(egui::Margin::ZERO)
            .show(&mut child);

        let focus_id = self.id.with("needs-focus");
        if ui.data_mut(|d| d.get_temp::<bool>(focus_id).unwrap_or(true)) {
            output.response.request_focus();
            ui.data_mut(|d| d.insert_temp(focus_id, false));
        }
    }

    /// A full-width hairline divider under the search header.
    fn divider(&self, ui: &mut Ui) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(self.width, 1.0), Sense::hover());
        ui.painter().hline(
            rect.x_range(),
            rect.center().y,
            Stroke::new(1.0, self.tokens.border),
        );
        ui.add_space(4.0);
    }

    /// A muted group heading row (`text-xs`, `font-medium`).
    fn group_heading(&self, ui: &mut Ui, text: &str) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(self.width, 24.0), Sense::hover());
        ui.painter().text(
            egui::pos2(rect.left() + self.m.item_pad_x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            text,
            egui::FontId::proportional(self.m.heading),
            self.tokens.muted_foreground,
        );
    }

    /// The muted, centred "no results" row (shadcn's `CommandEmpty`).
    fn empty_row(&self, ui: &mut Ui, text: &str) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(self.width, 64.0), Sense::hover());
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(self.m.text),
            self.tokens.muted_foreground,
        );
    }

    /// One command row: accent while lit, leading icon, trailing shortcut chip.
    fn item_row(&self, ui: &mut Ui, item: &CommandItem, active: bool) -> egui::Response {
        let (tokens, m) = (self.tokens, self.m);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(self.width, m.item_min_h), Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let lit = active || response.hovered();
        let text_color = if lit {
            ui.painter().rect(
                rect,
                tokens.radius_xl(),
                tokens.accent,
                Stroke::NONE,
                StrokeKind::Inside,
            );
            tokens.accent_foreground
        } else {
            tokens.foreground
        };

        let mut text_left = rect.left() + m.item_pad_x;
        if let Some(icon) = item.icon {
            let icon_rect = egui::Rect::from_min_size(
                egui::pos2(text_left, rect.center().y - 8.0),
                Vec2::splat(16.0),
            );
            icon.color(text_color).image(tokens).paint_at(ui, icon_rect);
            text_left += m.icon_col;
        }
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
        if let Some(shortcut) = &item.shortcut {
            self.shortcut_chip(ui, rect, shortcut);
        }
        response
    }

    /// The shortcut [`Kbd`], right-aligned. The right-to-left layout sizes the
    /// chip to its text; the 96pt slot only bounds it on the left, wide
    /// enough for multi-key hints like "Ctrl P".
    fn shortcut_chip(&self, ui: &mut Ui, row: egui::Rect, shortcut: &str) {
        let right = row.right() - self.m.item_pad_x;
        let slot = egui::Rect::from_min_max(
            egui::pos2(right - 96.0, row.top()),
            egui::pos2(right, row.bottom()),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(slot)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        Kbd::new(shortcut).ui(&mut child);
    }
}

/// The grouped list of matching items.
struct List<'a> {
    palette: &'a Palette,
    groups: &'a [CommandGroup],
    needle: &'a str,
    highlight: usize,
    nav_moved: bool,
}

impl List<'_> {
    /// Render the rows, scrolling once they outgrow `list_max_h`. Returns the
    /// clicked item.
    fn show(&self, ui: &mut Ui, rows: usize) -> Option<String> {
        let m = self.palette.m;
        let content_h = m.item_min_h * f32::from(u16::try_from(rows).unwrap_or(u16::MAX));
        if content_h > m.list_max_h {
            let mut clicked = None;
            ScrollArea::new()
                .max_height(m.list_max_h)
                .show(ui, |ui| clicked = self.rows(ui, true));
            clicked
        } else {
            self.rows(ui, false)
        }
    }

    /// `scroll` is true only inside our own scroll area: outside it,
    /// `scroll_to_me` would scroll the page instead.
    fn rows(&self, ui: &mut Ui, scroll: bool) -> Option<String> {
        let mut flat = 0;
        let mut clicked = None;
        for group in self.groups {
            let mut items = group
                .items
                .iter()
                .filter(|it| it.matches(self.needle))
                .peekable();
            if items.peek().is_none() {
                continue;
            }
            if let Some(heading) = &group.heading {
                self.palette.group_heading(ui, heading);
            }
            for item in items {
                let active = flat == self.highlight;
                let resp = self.palette.item_row(ui, item, active);
                if resp.clicked() {
                    clicked = Some(item.id.clone());
                }
                if active && scroll && self.nav_moved {
                    resp.scroll_to_me(None);
                }
                flat += 1;
            }
        }
        clicked
    }
}

/// Parses human shortcut hints like `"Ctrl P"` or `"Ctrl+Shift+K"`.
struct Shortcut;

impl Shortcut {
    /// The chord for `text`, or `None` without a recognisable key, so an
    /// unparseable hint stays purely decorative. Modifier words are
    /// case-insensitive (`ctrl`/`control`, `cmd`/`command`/`super`/`win`/`meta`,
    /// `alt`/`option`, `shift`); the last other token is the key.
    fn parse(text: &str) -> Option<egui::KeyboardShortcut> {
        let mut modifiers = egui::Modifiers::NONE;
        let mut key = None;
        for token in text
            .split(|c: char| c.is_whitespace() || c == '+')
            .filter(|t| !t.is_empty())
        {
            match Modifier::named(&token.to_lowercase()) {
                Some(m) => modifiers |= m.flag(),
                None => key = Key::from_name(token),
            }
        }
        key.map(|k| egui::KeyboardShortcut::new(modifiers, k))
    }
}

/// A modifier key, as spelled in a shortcut hint.
#[derive(Clone, Copy)]
enum Modifier {
    Ctrl,
    Cmd,
    Alt,
    Shift,
}

impl Modifier {
    const ALL: [(Self, &'static [&'static str]); 4] = [
        (Self::Ctrl, &["ctrl", "control"]),
        (Self::Cmd, &["cmd", "command", "super", "win", "meta"]),
        (Self::Alt, &["alt", "option"]),
        (Self::Shift, &["shift"]),
    ];

    /// The modifier a lowercase `word` names, if any.
    fn named(word: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|(_, names)| names.contains(&word))
            .map(|(m, _)| m)
    }

    const fn flag(self) -> egui::Modifiers {
        match self {
            Self::Ctrl => egui::Modifiers::CTRL,
            Self::Cmd => egui::Modifiers::MAC_CMD,
            Self::Alt => egui::Modifiers::ALT,
            Self::Shift => egui::Modifiers::SHIFT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;
    use egui::{Event, Modifiers};

    #[test]
    fn shortcuts_parse_modifiers_and_key() {
        let s = Shortcut::parse("Ctrl Shift K").unwrap();
        assert_eq!(s.logical_key, Key::K);
        assert!(s.modifiers.ctrl && s.modifiers.shift && !s.modifiers.alt);
        let s = Shortcut::parse("cmd+option+P").unwrap();
        assert!(s.modifiers.mac_cmd && s.modifiers.alt);
        assert_eq!(Shortcut::parse("Ctrl"), None);
        assert_eq!(Shortcut::parse(""), None);
    }

    #[test]
    fn nav_wraps() {
        let down = Nav {
            down: true,
            ..Nav::default()
        };
        let up = Nav {
            up: true,
            ..Nav::default()
        };
        assert_eq!(down.step(2, 3), 0);
        assert_eq!(up.step(0, 3), 2);
        assert_eq!(Nav::default().step(9, 3), 2);
        assert_eq!(down.step(0, 0), 0);
    }

    /// A palette on a headless context, collecting activations.
    struct Rig {
        ui: TestUi,
        activated: Vec<String>,
        rows: usize,
    }

    impl Rig {
        fn new(rows: usize) -> Self {
            Self {
                ui: TestUi::new(),
                activated: Vec::new(),
                rows,
            }
        }

        fn build(&mut self) -> impl FnMut(&mut Ui) + '_ {
            |ui| {
                let filler = (0..self.rows)
                    .map(|i| CommandItem::new(format!("f{i}"), format!("Filler {i}")));
                let cmd = Command::new("test")
                    .width(300.0)
                    .empty_text("Nothing")
                    .group(CommandGroup::new("Suggestions").items([
                        CommandItem::new("calendar", "Calendar").icon(Icon::new(SEARCH)),
                        CommandItem::new("emoji", "Search Emoji").keywords(["smile"]),
                    ]))
                    .group(
                        CommandGroup::untitled()
                            .item(CommandItem::new("profile", "Profile").shortcut("Ctrl P")),
                    )
                    .group(CommandGroup::new("More").items(filler));
                self.activated.extend(cmd.show(ui));
            }
        }

        fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
            let ui = std::mem::take(&mut self.ui);
            let out = ui.frame_with(events, self.build());
            self.ui = ui;
            out
        }

        fn click(&mut self, pos: egui::Pos2) {
            let ui = std::mem::take(&mut self.ui);
            ui.click(pos, self.build());
            self.ui = ui;
        }

        fn key(&mut self, key: Key) {
            self.frame(vec![TestUi::key(key)]);
        }
    }

    #[test]
    fn arrows_and_enter_activate_the_highlight() {
        let mut rig = Rig::new(0);
        rig.frame(Vec::new());
        rig.frame(Vec::new());
        rig.key(Key::ArrowDown);
        rig.key(Key::Enter);
        // Two up from the second of three rows wraps to the last.
        rig.key(Key::ArrowUp);
        rig.key(Key::ArrowUp);
        rig.key(Key::Enter);
        assert_eq!(rig.activated, ["emoji", "profile"]);
    }

    #[test]
    fn typing_filters_by_label_and_keyword() {
        let mut rig = Rig::new(0);
        rig.frame(Vec::new());
        rig.frame(vec![Event::Text("smile".to_owned())]);
        let out = rig.frame(Vec::new());
        assert!(TestUi::text_x(&out, "Search Emoji") > 0.0);
        rig.key(Key::Enter);
        rig.frame(vec![Event::Text("zzz".to_owned())]);
        let out = rig.frame(Vec::new());
        assert!(TestUi::text_x(&out, "Nothing") > 0.0);
        assert_eq!(rig.activated, ["emoji"]);
    }

    #[test]
    fn shortcuts_and_clicks_activate() {
        let mut rig = Rig::new(12);
        let out = rig.frame(Vec::new());
        let ctrl_p = Event::Key {
            key: Key::P,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::CTRL,
        };
        rig.frame(vec![ctrl_p]);
        rig.click(TestUi::text_rect(&out, "Calendar").center());
        assert_eq!(rig.activated, ["profile", "calendar"]);
    }
}
