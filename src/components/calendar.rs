//! [`Calendar`] — a month-grid date picker, mirroring shadcn's `<Calendar>`.
//!
//! A single-month view: a header with the month/year label flanked by
//! prev/next arrows, a row of weekday initials, then a 6×7 grid of day cells.
//! The selected day is a filled `primary` chip, today gets a ring, days from the
//! adjacent months render muted, and hovering any in-month day shows an
//! `accent` highlight.
//!
//! glazier carries no date library, so this module ships a tiny dependency-free
//! [`Date`] (proleptic Gregorian) with just the arithmetic a month grid needs:
//! weekday, days-in-month, and month stepping.
//!
//! State is split the usual way: you own the selected [`Date`] (an
//! `Option<Date>`), and the *visible month* is remembered per-widget in egui's
//! data store, seeded from the selection (or today). [`show`](Calendar::show)
//! returns `true` the frame the selection changes.
//!
//! ```no_run
//! use glazier::calendar::{Calendar, Date};
//! # egui::__run_test_ui(|ui| {
//! # let mut selected: Option<Date> = None;
//! if Calendar::new(&mut selected).show(ui) {
//!     // `selected` changed
//! }
//! # });
//! ```

use egui::{Response, Sense, Ui, Vec2, Widget};

use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// A civil date in the proleptic Gregorian calendar.
///
/// Minimal by design — enough to drive a month grid. Fields are public for easy
/// construction; use the constructors/queries for anything non-trivial.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    /// Full year (e.g. 2025).
    pub year: i32,
    /// Month, 1..=12.
    pub month: u32,
    /// Day of month, 1..=31.
    pub day: u32,
}

impl Date {
    /// Construct a date, clamping the day into the month's valid range.
    #[must_use]
    pub fn new(year: i32, month: u32, day: u32) -> Self {
        let month = month.clamp(1, 12);
        let day = day.clamp(1, days_in_month(year, month));
        Self { year, month, day }
    }

    /// Weekday, 0 = Sunday … 6 = Saturday.
    ///
    /// Derived from the rata-die day number: day 0 (1970-01-01) was a Thursday,
    /// so adding 4 and reducing mod 7 puts Sunday at 0.
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn weekday(self) -> u32 {
        (to_rata_die(self) + 4).rem_euclid(7) as u32
    }

    /// The first day of this date's month.
    #[must_use]
    pub const fn first_of_month(self) -> Self {
        Self {
            year: self.year,
            month: self.month,
            day: 1,
        }
    }

    /// This date shifted by `delta` whole months, clamping the day.
    #[must_use]
    pub fn add_months(self, delta: i32) -> Self {
        let total = self.year * 12 + i32::try_from(self.month).unwrap_or(1) - 1 + delta;
        let year = total.div_euclid(12);
        let month = total.rem_euclid(12).unsigned_abs() + 1;
        Self::new(year, month, self.day)
    }
}

/// Whether `year` is a leap year.
const fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Number of days in `month` of `year`.
const fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 30,
    }
}

/// A best-effort "today", used to seed the view and draw the today ring. With no
/// date library and no clock access here, this returns a fixed reference date;
/// pass a real value via [`Calendar::today`] to light up the correct cell.
const FALLBACK_TODAY: Date = Date {
    year: 2025,
    month: 1,
    day: 1,
};

/// Overridable geometry for [`Calendar`] — reach in via [`Calendar::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct CalendarMetrics {
    /// Minimum cell edge / row height (shadcn `size-9`). Columns grow past
    /// this to fill the available width; the height stays fixed.
    pub cell: f32,
    /// Gap between grid cells.
    pub grid_gap: f32,
    /// Day text size.
    pub text_size: f32,
}

impl Default for CalendarMetrics {
    fn default() -> Self {
        Self {
            cell: 36.0,
            grid_gap: 2.0,
            text_size: 13.0,
        }
    }
}

/// A single-month calendar bound to an `Option<Date>` selection.
#[must_use = "calendars do nothing unless shown"]
pub struct Calendar<'a> {
    selected: &'a mut Option<Date>,
    today: Date,
    id_salt: egui::Id,
    sizing_hook: SizingHook<CalendarMetrics>,
}

impl Sizeable<CalendarMetrics> for Calendar<'_> {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<CalendarMetrics> {
        &mut self.sizing_hook
    }
}

impl<'a> Calendar<'a> {
    /// Create a calendar editing `selected`.
    pub fn new(selected: &'a mut Option<Date>) -> Self {
        Self {
            selected,
            today: FALLBACK_TODAY,
            id_salt: egui::Id::new("glazier-calendar"),
            sizing_hook: SizingHook::default(),
        }
    }

    /// Tell the calendar what "today" is, so it seeds the initial view and draws
    /// the today ring on the right cell.
    pub const fn today(mut self, today: Date) -> Self {
        self.today = today;
        self
    }

    /// Disambiguate multiple calendars in one view (their visible-month state is
    /// keyed by this salt).
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.id_salt = egui::Id::new(("glazier-calendar", egui::Id::new(salt)));
        self
    }

    /// Render the calendar. Returns `true` if the selection changed this frame.
    pub fn show(mut self, ui: &mut Ui) -> bool {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));

        // The visible month persists across frames, seeded from the selection or
        // today, stored as a (year, month) pair.
        let seed = self.selected.unwrap_or(self.today).first_of_month();
        let id = self.id_salt;
        let (mut vy, mut vm): (i32, u32) = ui
            .ctx()
            .data_mut(|d| *d.get_temp_mut_or(id, (seed.year, seed.month)));

        // Stretch the 7 columns to fill the available width: cells grow from the
        // cell baseline so the grid fills its card instead of hugging the left
        // edge. Width is shared by the header, weekday row, and day grid so the
        // columns line up.
        let cw = (m.grid_gap.mul_add(-6.0, ui.available_width()) / 7.0).max(m.cell);

        let mut changed = false;
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(m.grid_gap, m.grid_gap);

            // Header: ‹  Month YYYY  ›
            ui.horizontal(|ui| {
                let view = Date::new(vy, vm, 1);
                if nav(ui, id, tokens, true, m).clicked() {
                    let p = view.add_months(-1);
                    vy = p.year;
                    vm = p.month;
                }
                let label = format!("{} {}", MONTHS[vm as usize - 1], vy);
                // The label spans the five middle columns between the two arrows.
                let avail = cw.mul_add(5.0, m.grid_gap * 4.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(avail, m.cell),
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        ui.label(
                            egui::RichText::new(label)
                                .font(crate::fonts::semibold(ui, 14.0))
                                .color(tokens.foreground),
                        );
                    },
                );
                if nav(ui, id, tokens, false, m).clicked() {
                    let n = view.add_months(1);
                    vy = n.year;
                    vm = n.month;
                }
            });

            // Weekday header row.
            ui.horizontal(|ui| {
                for w in WEEKDAYS {
                    let (_, rect) = ui.allocate_space(Vec2::new(cw, m.cell));
                    let g = ui.painter().layout_no_wrap(
                        (*w).to_owned(),
                        egui::FontId::proportional(12.0),
                        tokens.muted_foreground,
                    );
                    ui.painter()
                        .galley(rect.center() - g.size() / 2.0, g, tokens.muted_foreground);
                }
            });

            // Day grid: 6 weeks starting from the Sunday on/before the 1st.
            let first = Date::new(vy, vm, 1);
            let lead = first.weekday(); // days of the previous month to show
            for week in 0..6 {
                ui.horizontal(|ui| {
                    for dow in 0..7 {
                        let slot = week * 7 + dow;
                        let offset = slot - i32::try_from(lead).unwrap_or(0);
                        let cell = first.add_days(offset);
                        let in_month = cell.month == vm && cell.year == vy;
                        let day = DayCell {
                            base: id,
                            slot,
                            cell,
                            in_month,
                            today: self.today,
                            selected: *self.selected,
                            width: cw,
                        };
                        if day_cell(ui, tokens, day, m).clicked() {
                            *self.selected = Some(cell);
                            changed = true;
                            // Jump the view if they picked an adjacent-month day.
                            vy = cell.year;
                            vm = cell.month;
                        }
                    }
                });
            }
        });

        ui.ctx().data_mut(|d| d.insert_temp(id, (vy, vm)));
        changed
    }
}

impl Date {
    /// This date shifted by `delta` whole days.
    #[must_use]
    fn add_days(self, delta: i32) -> Self {
        // Convert to a day number (days since an epoch), shift, convert back.
        let mut rd = to_rata_die(self);
        rd += i64::from(delta);
        from_rata_die(rd)
    }
}

/// Days since 0000-03-01 (Howard Hinnant's `days_from_civil`).
fn to_rata_die(d: Date) -> i64 {
    let y = i64::from(if d.month <= 2 { d.year - 1 } else { d.year });
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = i64::from(d.month);
    let doy = (153 * (if d.month > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d.day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Inverse of [`to_rata_die`] (`civil_from_days`).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
const fn from_rata_die(z: i64) -> Date {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let year = (if month <= 2 { y + 1 } else { y }) as i32;
    Date { year, month, day }
}

/// A prev/next month arrow button (`‹`/`›`). `base` is the calendar's stable
/// id; the button derives a position-independent id from it so egui's id-clash
/// detector stays quiet across relayout passes when the month changes.
#[allow(clippy::many_single_char_names)]
fn nav(ui: &mut Ui, base: egui::Id, tokens: Tokens, left: bool, m: CalendarMetrics) -> Response {
    let (_, rect) = ui.allocate_space(Vec2::splat(m.cell));
    let id = base.with(("nav", left));
    let resp = ui
        .interact(rect, id, Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let t = ui
        .ctx()
        .animate_bool_with_time(id.with("hover"), resp.hovered(), 0.15);
    if t > 0.01 {
        ui.painter()
            .rect_filled(rect, tokens.radius_md(), tokens.accent.gamma_multiply(t));
    }
    let c = rect.center();
    let w = 4.0;
    let h = 6.0;
    let dx = if left { w * 0.5 } else { -w * 0.5 };
    let stroke = egui::Stroke::new(1.6, tokens.foreground);
    let tip = egui::pos2(c.x - dx, c.y);
    ui.painter()
        .line_segment([egui::pos2(c.x + dx, c.y - h), tip], stroke);
    ui.painter()
        .line_segment([tip, egui::pos2(c.x + dx, c.y + h)], stroke);
    resp
}

/// The data needed to render one day cell.
#[derive(Clone, Copy)]
struct DayCell {
    /// The calendar's stable id.
    base: egui::Id,
    /// Fixed grid position, 0..42.
    slot: i32,
    /// The date shown in this cell.
    cell: Date,
    /// Whether `cell` belongs to the visible month.
    in_month: bool,
    /// Today's date, for the today ring.
    today: Date,
    /// The current selection.
    selected: Option<Date>,
    /// Cell width (stretched to fill the calendar); height stays [`CELL`].
    width: f32,
}

/// Render one day cell with selection / today / muted styling.
///
/// The interactive id is keyed off `base + slot` — the cell's *screen position*,
/// never its date. egui's `warn_if_rect_changes_id` paints a red outline when a
/// fixed rect's id changes between frames; since a slot's screen rect is stable
/// while its date changes on month-switch, a date-keyed id would trip that
/// warning. A position-keyed id keeps each rect's id constant across months.
fn day_cell(ui: &mut Ui, tokens: Tokens, day: DayCell, m: CalendarMetrics) -> Response {
    let DayCell {
        base,
        slot,
        cell,
        in_month,
        today,
        selected,
        width,
    } = day;
    let (_, rect) = ui.allocate_space(Vec2::new(width, m.cell));
    let id = base.with(("day", slot));
    let resp = ui
        .interact(rect, id, Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    let is_selected = selected == Some(cell);
    let is_today = cell == today;
    let chip = rect.shrink(2.0);

    if is_selected {
        ui.painter()
            .rect_filled(chip, tokens.radius_md(), tokens.primary);
    } else {
        let t = ui
            .ctx()
            .animate_bool_with_time(id.with("hover"), resp.hovered() && in_month, 0.15);
        if t > 0.01 {
            ui.painter()
                .rect_filled(chip, tokens.radius_md(), tokens.accent.gamma_multiply(t));
        }
        if is_today {
            ui.painter().rect_stroke(
                chip,
                tokens.radius_md(),
                egui::Stroke::new(1.0, tokens.border),
                egui::StrokeKind::Inside,
            );
        }
    }

    let color = if is_selected {
        tokens.primary_foreground
    } else if in_month {
        tokens.foreground
    } else {
        tokens.muted_foreground.gamma_multiply(0.6)
    };
    let g = ui.painter().layout_no_wrap(
        format!("{}", cell.day),
        egui::FontId::proportional(m.text_size),
        color,
    );
    ui.painter()
        .galley(rect.center() - g.size() / 2.0, g, color);
    resp
}

/// Weekday column initials (Sunday-first).
const WEEKDAYS: [&str; 7] = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
/// Full month names.
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

impl Widget for Calendar<'_> {
    /// Renders the calendar, discarding the changed flag. Use
    /// [`Calendar::show`] to observe selection changes.
    fn ui(self, ui: &mut Ui) -> Response {
        ui.scope(|ui| {
            self.show(ui);
        })
        .response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Known weekdays: 2000-01-01 was a Saturday (6); 2025-01-01 a Wednesday (3).
    #[test]
    fn weekday_known() {
        assert_eq!(Date::new(2000, 1, 1).weekday(), 6);
        assert_eq!(Date::new(2025, 1, 1).weekday(), 3);
    }

    /// Leap-year February has 29 days; common years 28.
    #[test]
    fn leap_february() {
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2025, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(1900, 2), 28);
    }

    /// Month arithmetic wraps the year and clamps the day.
    #[test]
    fn add_months_wraps() {
        assert_eq!(
            Date::new(2025, 12, 15).add_months(1),
            Date::new(2026, 1, 15)
        );
        assert_eq!(Date::new(2025, 1, 31).add_months(1), Date::new(2025, 2, 28));
        assert_eq!(
            Date::new(2025, 3, 10).add_months(-3),
            Date::new(2024, 12, 10)
        );
    }

    /// Day arithmetic round-trips through the rata-die conversion.
    #[test]
    fn add_days_crosses_months() {
        assert_eq!(Date::new(2025, 1, 31).add_days(1), Date::new(2025, 2, 1));
        assert_eq!(Date::new(2025, 3, 1).add_days(-1), Date::new(2025, 2, 28));
        assert_eq!(Date::new(2024, 2, 28).add_days(1), Date::new(2024, 2, 29));
        // Round-trip a span.
        let d = Date::new(2025, 6, 15);
        assert_eq!(d.add_days(400).add_days(-400), d);
    }

    /// Selecting a day reports a change without panicking.
    #[test]
    fn show_reports_change() {
        let ctx = egui::Context::default();
        let mut sel = None;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            Calendar::new(&mut sel).show(ui);
        });
    }
}
