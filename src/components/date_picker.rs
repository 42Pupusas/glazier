//! [`DatePicker`] — a [`Calendar`] in a [`Popover`], mirroring shadcn's Date
//! Picker recipe.
//!
//! shadcn's date picker isn't a primitive — it's a `<Popover>` whose trigger is
//! an outline `<Button>` showing the chosen date (or a muted placeholder) with a
//! leading calendar glyph, and whose content is a `<Calendar>`. Picking a day
//! writes it back into the bound `Option<Date>` and closes the popover.
//!
//! shadcn also ships a *Date Picker with Input* variant: a typeable text field
//! plus a trailing calendar icon-button that opens the same popover. Reach it
//! with [`with_input`](DatePicker::with_input) — typing a recognised date
//! (`June 24, 2025`, `2025-06-24`, or `6/24/2025`) updates the selection, and
//! picking from the calendar writes the long form back into the field.
//!
//! Either way the widget owns no new state beyond what [`Calendar`] already
//! persists (the visible month) plus, in input mode, the caller's text buffer.
//!
//! ```no_run
//! use glazier::date_picker::DatePicker;
//! use glazier::calendar::Date;
//! # egui::__run_test_ui(|ui| {
//! let mut date: Option<Date> = None;
//! if DatePicker::new(&mut date).show(ui) {
//!     // `date` changed
//! }
//!
//! // …or the typeable variant:
//! let mut date2: Option<Date> = None;
//! let mut text = String::new();
//! DatePicker::new(&mut date2).with_input(&mut text).show(ui);
//! # });
//! ```

use egui::{PopupCloseBehavior, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget};

use crate::components::calendar::{Calendar, Date};
use crate::components::icon::Icon;
use crate::components::input::Input;
use crate::components::popover::Popover;
use crate::tokens::Tokens;

/// `calendar` (lucide) — the leading affordance.
const CALENDAR: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 2v4"/><path d="M16 2v4"/><rect width="18" height="18" x="3" y="4" rx="2"/><path d="M3 10h18"/></svg>"#;

/// Month names for the trigger label (e.g. "June 24, 2025") and parsing.
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Trigger horizontal / vertical padding (`px-3 py-2`, shadcn outline button).
const PAD_X: f32 = 12.0;
const PAD_Y: f32 = 8.0;
/// Leading icon edge (`size-4`) and its gap to the label.
const ICON_SIZE: f32 = 16.0;
const ICON_GAP: f32 = 8.0;
/// Trigger text size (`text-sm`).
const TEXT: f32 = 13.0;
/// Hover transition (shadcn `transition-colors`).
const HOVER_TIME: f32 = 0.15;
/// Square edge of the trailing icon-button in input mode (`size-9`).
const ICON_BUTTON: f32 = 36.0;
/// Gap between the input field and its trailing icon-button.
const INPUT_GAP: f32 = 8.0;

/// A date picker: an outline trigger that opens a [`Calendar`] popover.
#[must_use = "date pickers do nothing unless you show them"]
pub struct DatePicker<'a> {
    selected: &'a mut Option<Date>,
    buffer: Option<&'a mut String>,
    today: Option<Date>,
    placeholder: String,
    width: Option<f32>,
    id_salt: egui::Id,
}

impl<'a> DatePicker<'a> {
    /// Create a date picker editing `selected`.
    pub fn new(selected: &'a mut Option<Date>) -> Self {
        Self {
            selected,
            buffer: None,
            today: None,
            placeholder: "Pick a date".to_owned(),
            width: None,
            id_salt: egui::Id::new("glazier-date-picker"),
        }
    }

    /// Switch to the *with input* variant: a typeable field bound to `buffer`
    /// plus a trailing calendar icon-button. Typing a recognised date updates
    /// the selection; picking from the calendar writes the long form into
    /// `buffer`.
    pub const fn with_input(mut self, buffer: &'a mut String) -> Self {
        self.buffer = Some(buffer);
        self
    }

    /// Tell the embedded [`Calendar`] what "today" is (seeds the view + ring).
    pub const fn today(mut self, today: Date) -> Self {
        self.today = Some(today);
        self
    }

    /// Set the muted placeholder shown when no date is selected.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set an explicit trigger width. By default the trigger fills the available
    /// width (shadcn's recipe uses `w-[240px]`; pass that for a fixed size).
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Disambiguate multiple date pickers in one view.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.id_salt = egui::Id::new(("glazier-date-picker", egui::Id::new(salt)));
        self
    }

    /// Render the picker. Returns `true` if the selection changed this frame.
    pub fn show(self, ui: &mut Ui) -> bool {
        let tokens = Tokens::get(ui);
        let today = self.today;
        let id_salt = self.id_salt;
        let placeholder = self.placeholder;
        let width = self.width;
        let selected = self.selected;
        let mut buffer = self.buffer;

        let mut changed = false;

        // Two layouts share one popover: a button-style trigger (default) or a
        // typeable field + trailing icon-button (the "with input" variant).
        let trigger = if let Some(buf) = buffer.as_deref_mut() {
            input_trigger(ui, tokens, selected, buf, &placeholder, width, &mut changed)
        } else {
            button_trigger(ui, tokens, *selected, &placeholder, width)
        };

        // Calendar popover. The calendar is interactive (its prev/next arrows
        // click *inside* the panel), so close only on an outside click / Escape
        // / an explicit close — never on every inner click. We close it
        // ourselves the frame a day is picked.
        Popover::new()
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(ui, &trigger, |ui| {
                let mut cal = Calendar::new(selected).id_salt(id_salt);
                if let Some(today) = today {
                    cal = cal.today(today);
                }
                if cal.show(ui) {
                    changed = true;
                    // In input mode, reflect the pick back into the text buffer.
                    if let (Some(buf), Some(d)) = (buffer, *selected) {
                        *buf = format_date(d);
                    }
                    ui.close();
                }
            });

        changed
    }
}

/// Render the default button-style trigger (leading icon + date / placeholder).
/// Returns the click [`Response`] the popover anchors to.
fn button_trigger(
    ui: &mut Ui,
    tokens: Tokens,
    selected: Option<Date>,
    placeholder: &str,
    width: Option<f32>,
) -> Response {
    let has_date = selected.is_some();
    let label = selected.map_or_else(|| placeholder.to_owned(), format_date);

    let width = width.unwrap_or_else(|| ui.available_width());
    let height = 2.0_f32.mul_add(PAD_Y, TEXT.max(ICON_SIZE)).max(36.0);
    let (rect, trigger) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let trigger = trigger.on_hover_cursor(egui::CursorIcon::PointingHand);

    if ui.is_rect_visible(rect) {
        // Outline fill gliding to `accent` on hover (matches Button outline).
        let hover_t = ui.ctx().animate_bool_with_time(
            trigger.id.with("hover"),
            trigger.hovered(),
            HOVER_TIME,
        );
        let fill = tokens.background.lerp_to_gamma(tokens.accent, hover_t);
        ui.painter().rect(
            rect,
            tokens.radius_md(),
            fill,
            Stroke::new(1.0, tokens.border),
            StrokeKind::Inside,
        );

        // Leading calendar icon, muted like shadcn's `text-muted-foreground`.
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + PAD_X, rect.center().y - ICON_SIZE / 2.0),
            Vec2::splat(ICON_SIZE),
        );
        Icon::new(CALENDAR)
            .color(tokens.muted_foreground)
            .image(tokens)
            .paint_at(ui, icon_rect);

        // Label: foreground when a date is set, muted placeholder otherwise.
        let color = if has_date {
            tokens.foreground
        } else {
            tokens.muted_foreground
        };
        let galley = ui
            .painter()
            .layout_no_wrap(label, egui::FontId::proportional(TEXT), color);
        let pos = egui::pos2(
            icon_rect.right() + ICON_GAP,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(pos, galley, color);
    }

    trigger
}

/// Render the *with input* trigger: a typeable field plus a trailing calendar
/// icon-button. Parses the field on change (updating `selected`/`changed`) and
/// returns the icon-button's click [`Response`] for the popover to anchor to.
fn input_trigger(
    ui: &mut Ui,
    tokens: Tokens,
    selected: &mut Option<Date>,
    buffer: &mut String,
    placeholder: &str,
    width: Option<f32>,
    changed: &mut bool,
) -> Response {
    let total = width.unwrap_or_else(|| ui.available_width());
    let field_w = (total - ICON_BUTTON - INPUT_GAP).max(0.0);

    let mut trigger = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = INPUT_GAP;

        // The text field: typing a recognised date updates the selection.
        let resp = Input::new(buffer)
            .placeholder(placeholder)
            .width(field_w)
            .ui(ui);
        if resp.changed() {
            if let Some(d) = parse_date(buffer) {
                if *selected != Some(d) {
                    *selected = Some(d);
                    *changed = true;
                }
            } else if buffer.trim().is_empty() && selected.is_some() {
                *selected = None;
                *changed = true;
            }
        }

        trigger = Some(icon_button(ui, tokens));
    });

    trigger.expect("horizontal layout always runs its closure")
}

/// A square outline icon-button bearing the calendar glyph.
fn icon_button(ui: &mut Ui, tokens: Tokens) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(ICON_BUTTON), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

    if ui.is_rect_visible(rect) {
        let hover_t =
            ui.ctx()
                .animate_bool_with_time(resp.id.with("hover"), resp.hovered(), HOVER_TIME);
        let fill = tokens.background.lerp_to_gamma(tokens.accent, hover_t);
        ui.painter().rect(
            rect,
            tokens.radius_md(),
            fill,
            Stroke::new(1.0, tokens.border),
            StrokeKind::Inside,
        );
        let icon_rect = egui::Rect::from_center_size(rect.center(), Vec2::splat(ICON_SIZE));
        Icon::new(CALENDAR)
            .color(tokens.foreground)
            .image(tokens)
            .paint_at(ui, icon_rect);
    }

    resp
}

/// Format a [`Date`] as e.g. "June 24, 2025".
fn format_date(d: Date) -> String {
    format!("{} {}, {}", MONTHS[d.month as usize - 1], d.day, d.year)
}

/// Parse a date from the field. Accepts ISO (`2025-06-24`), US slash
/// (`6/24/2025`), and long form (`June 24, 2025` / `Jun 24 2025`). Returns
/// `None` if nothing recognisable is found.
fn parse_date(s: &str) -> Option<Date> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    parse_iso(s)
        .or_else(|| parse_slash(s))
        .or_else(|| parse_long(s))
}

/// `2025-06-24` → year-month-day.
fn parse_iso(s: &str) -> Option<Date> {
    let mut it = s.split('-');
    let year = it.next()?.trim().parse::<i32>().ok()?;
    let month = it.next()?.trim().parse::<u32>().ok()?;
    let day = it.next()?.trim().parse::<u32>().ok()?;
    if it.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(Date::new(year, month, day))
}

/// `6/24/2025` → month/day/year.
fn parse_slash(s: &str) -> Option<Date> {
    let mut it = s.split('/');
    let month = it.next()?.trim().parse::<u32>().ok()?;
    let day = it.next()?.trim().parse::<u32>().ok()?;
    let year = it.next()?.trim().parse::<i32>().ok()?;
    if it.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(Date::new(year, month, day))
}

/// `June 24, 2025` / `Jun 24 2025` → long form.
fn parse_long(s: &str) -> Option<Date> {
    let cleaned = s.replace(',', " ");
    let mut it = cleaned.split_whitespace();
    let name = it.next()?.to_ascii_lowercase();
    let idx = MONTHS.iter().position(|m| {
        let m = m.to_ascii_lowercase();
        m == name || (m.starts_with(&name) && name.len() >= 3)
    })?;
    let month = u32::try_from(idx).ok()? + 1;
    let day = it.next()?.parse::<u32>().ok()?;
    let year = it.next()?.parse::<i32>().ok()?;
    if it.next().is_some() || !(1..=31).contains(&day) {
        return None;
    }
    Some(Date::new(year, month, day))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The trigger label formats a selected date the long way.
    #[test]
    fn formats_selected_date() {
        assert_eq!(format_date(Date::new(2025, 6, 24)), "June 24, 2025");
        assert_eq!(format_date(Date::new(1999, 1, 1)), "January 1, 1999");
    }

    /// The parser accepts the three documented formats and round-trips.
    #[test]
    fn parses_known_formats() {
        let want = Date::new(2025, 6, 24);
        assert_eq!(parse_iso("2025-06-24"), Some(want));
        assert_eq!(parse_slash("6/24/2025"), Some(want));
        assert_eq!(parse_long("June 24, 2025"), Some(want));
        assert_eq!(parse_long("Jun 24 2025"), Some(want));
        assert_eq!(parse_date("  2025-06-24  "), Some(want));
        // The long form round-trips through `format_date`.
        assert_eq!(parse_date(&format_date(want)), Some(want));
    }

    /// Garbage and out-of-range input is rejected.
    #[test]
    fn rejects_nonsense() {
        assert_eq!(parse_date(""), None);
        assert_eq!(parse_date("not a date"), None);
        assert_eq!(parse_date("2025-13-01"), None);
        assert_eq!(parse_date("13/40/2025"), None);
    }

    /// Showing a closed picker is a no-op and never mutates the selection.
    #[test]
    fn show_without_interaction_keeps_selection() {
        let ctx = egui::Context::default();
        let mut date = Some(Date::new(2025, 6, 24));
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let changed = DatePicker::new(&mut date).show(ui);
            assert!(!changed);
        });
        assert_eq!(date, Some(Date::new(2025, 6, 24)));
    }
}
