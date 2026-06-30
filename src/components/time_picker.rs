//! [`TimePicker`] — a segmented 12-hour time editor, mirroring shadcn's
//! `<Input type="time">`.
//!
//! shadcn relies on the browser's native time input, which edits each section in
//! place: click (or arrow) onto `HH`, `MM`, `SS`, or `AM/PM`, type digits to
//! fill the section (auto-advancing to the next), and press ↑/↓ to step the
//! focused section by ±1. egui has no native time control, so this reproduces
//! that interaction over a bound [`Time`]:
//!
//! * **Click** a section to focus it; **←/→** move between sections.
//! * **Digits** fill the focused numeric section and auto-advance (`7` in the
//!   minutes jumps straight to `07`, since no `7x` minute exists).
//! * **↑/↓** increment / decrement the focused section, wrapping.
//! * **A / P** set the meridiem.
//!
//! State is split as usual: you own the [`Time`]; the focused section lives in
//! egui's per-widget memory. [`show`](TimePicker::show) returns `true` the frame
//! the time changes. Pairs with [`DatePicker`](crate::DatePicker) for shadcn's
//! "Date and Time" field group.
//!
//! ```no_run
//! use glazier::time_picker::{TimePicker, Time};
//! # egui::__run_test_ui(|ui| {
//! let mut time = Time::new(10, 30, 0);
//! if TimePicker::new(&mut time).show(ui) {
//!     // `time` changed
//! }
//! # });
//! ```

use std::fmt;

use egui::{Event, Key, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::tokens::Tokens;

/// Field height (`h-9`).
const HEIGHT: f32 = 36.0;
/// Horizontal padding inside the field (`px-3`).
const PAD_X: f32 = 12.0;
/// Section / separator text size (`text-sm`).
const TEXT: f32 = 13.0;
/// Extra width padding around each focusable section's highlight chip.
const CHIP_PAD_X: f32 = 3.0;

/// The four editable sections, left to right.
const SEG_HOUR: u8 = 0;
const SEG_MIN: u8 = 1;
const SEG_SEC: u8 = 2;
const SEG_MERIDIEM: u8 = 3;

/// A 24-hour time of day, to second precision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time {
    /// Hour, 0..=23.
    pub hour: u32,
    /// Minute, 0..=59.
    pub minute: u32,
    /// Second, 0..=59.
    pub second: u32,
}

impl Time {
    /// Construct a time, clamping each component into range.
    #[must_use]
    pub fn new(hour: u32, minute: u32, second: u32) -> Self {
        Self {
            hour: hour.min(23),
            minute: minute.min(59),
            second: second.min(59),
        }
    }

    /// Parse `HH:MM` or `HH:MM:SS` (24-hour). Seconds default to 0. Returns
    /// `None` if the shape or ranges are wrong.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        let mut it = s.split(':');
        let hour = it.next()?.trim().parse::<u32>().ok()?;
        let minute = it.next()?.trim().parse::<u32>().ok()?;
        let second = match it.next() {
            Some(sec) => sec.trim().parse::<u32>().ok()?,
            None => 0,
        };
        if it.next().is_some() || hour > 23 || minute > 59 || second > 59 {
            return None;
        }
        Some(Self {
            hour,
            minute,
            second,
        })
    }

    /// Split into a 12-hour clock face: `(hour 1..=12, is_pm)`.
    #[must_use]
    const fn to_12(self) -> (u32, bool) {
        let is_pm = self.hour >= 12;
        let mut h = self.hour % 12;
        if h == 0 {
            h = 12;
        }
        (h, is_pm)
    }

    /// Rebuild the hour from a 12-hour face value (1..=12) and meridiem.
    const fn set_hour_12(&mut self, h12: u32, is_pm: bool) {
        let h = h12 % 12;
        self.hour = if is_pm { h + 12 } else { h };
    }
}

impl fmt::Display for Time {
    /// Zero-padded `HH:MM:SS` (24-hour).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
    }
}

/// Per-widget editing state held in egui memory: the focused section and the
/// digits typed into it so far (so a second keypress appends rather than
/// replaces).
#[derive(Clone, Default)]
struct EditState {
    seg: u8,
    buf: String,
}

/// A segmented time-of-day editor.
#[must_use = "time pickers do nothing unless you show them"]
pub struct TimePicker<'a> {
    time: &'a mut Time,
    width: Option<f32>,
    id_salt: egui::Id,
}

impl<'a> TimePicker<'a> {
    /// Create a time picker editing `time`.
    pub fn new(time: &'a mut Time) -> Self {
        Self {
            time,
            width: None,
            id_salt: egui::Id::new("glazier-time-picker"),
        }
    }

    /// Set an explicit width; defaults to the available width.
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Disambiguate multiple time pickers in one view.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.id_salt = egui::Id::new(("glazier-time-picker", egui::Id::new(salt)));
        self
    }

    /// Render the picker. Returns `true` if the time changed this frame.
    #[allow(clippy::too_many_lines)]
    pub fn show(self, ui: &mut Ui) -> bool {
        let tokens = Tokens::get(ui);
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let id = ui.make_persistent_id(self.id_salt);

        let (rect, mut response) = ui.allocate_exact_size(Vec2::new(width, HEIGHT), Sense::click());
        let focused = response.has_focus();

        // Claim the arrow keys so egui's focus navigation doesn't steal them:
        // by default a focused widget surrenders focus on arrow presses (it
        // moves to a neighbouring widget), which is why ↑/← used to defocus the
        // picker — the move only fired when a target existed in that direction,
        // hence the asymmetry. This filter keeps the arrows for us.
        if response.has_focus() {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    response.id,
                    egui::EventFilter {
                        tab: false,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: false,
                    },
                );
            });
        }

        let mut state: EditState = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
        let mut changed = false;

        // --- Section text + geometry --------------------------------------
        let (h12, is_pm) = self.time.to_12();
        let labels = [
            format!("{h12:02}"),
            format!("{:02}", self.time.minute),
            format!("{:02}", self.time.second),
            if is_pm {
                "PM".to_owned()
            } else {
                "AM".to_owned()
            },
        ];
        let font = egui::FontId::proportional(TEXT);
        let measure = |ui: &Ui, s: &str| {
            ui.painter()
                .layout_no_wrap(s.to_owned(), font.clone(), tokens.foreground)
                .size()
                .x
        };
        let colon_w = measure(ui, ":");
        let seg_w = [
            measure(ui, &labels[0]),
            measure(ui, &labels[1]),
            measure(ui, &labels[2]),
            measure(ui, "AM").max(measure(ui, "PM")),
        ];

        // Lay sections out left to right with ":" separators and a space before
        // the meridiem. Records each section's hit rect.
        let mut x = rect.left() + PAD_X;
        let cy = rect.center().y;
        let mut seg_rects = [Rect::NOTHING; 4];
        for i in 0..4 {
            let w = seg_w[i];
            seg_rects[i] = Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::new(w, HEIGHT));
            x += w;
            // Separators: ":" between numbers, a space before the meridiem.
            if i == SEG_HOUR as usize || i == SEG_MIN as usize {
                x += colon_w;
            } else if i == SEG_SEC as usize {
                x += colon_w; // a gap roughly a colon wide before AM/PM
            }
        }

        // --- Interaction: click a section, or focus the widget ------------
        for (i, sr) in seg_rects.iter().enumerate() {
            let r = ui.interact(*sr, id.with(("seg", i)), Sense::click());
            if r.clicked() {
                state.seg = u8::try_from(i).unwrap_or(0);
                state.buf.clear();
                response.request_focus();
            }
        }
        if response.clicked() && !focused {
            response.request_focus();
        }

        // --- Keyboard handling (only while focused) ----------------------
        if response.has_focus() {
            // Arrow keys must be *consumed*: otherwise egui's own keyboard focus
            // navigation and scroll-area handling act on the same press, which
            // steals focus (↑) and swallows section moves (←/→). Consuming them
            // here claims them for the picker before anyone else sees them.
            let (mut left, mut right, mut up, mut down) = (false, false, false, false);
            ui.input_mut(|i| {
                use egui::Modifiers;
                left = i.consume_key(Modifiers::NONE, Key::ArrowLeft);
                right = i.consume_key(Modifiers::NONE, Key::ArrowRight);
                up = i.consume_key(Modifiers::NONE, Key::ArrowUp);
                down = i.consume_key(Modifiers::NONE, Key::ArrowDown);
            });
            if left {
                state.seg = state.seg.saturating_sub(1);
                state.buf.clear();
            }
            if right {
                state.seg = (state.seg + 1).min(SEG_MERIDIEM);
                state.buf.clear();
            }
            if up {
                step(self.time, state.seg, 1);
                state.buf.clear();
                changed = true;
            }
            if down {
                step(self.time, state.seg, -1);
                state.buf.clear();
                changed = true;
            }

            // Digit / meridiem entry comes through as text events.
            let events = ui.input(|i| i.events.clone());
            for ev in events {
                if let Event::Text(t) = ev {
                    for c in t.chars() {
                        if feed_char(self.time, &mut state, c) {
                            changed = true;
                        }
                    }
                }
            }
        } else {
            // Dropping focus ends the current edit run.
            state.buf.clear();
        }

        // --- Paint --------------------------------------------------------
        if ui.is_rect_visible(rect) {
            // Field chrome: matches Input (bg, hairline border, radius-md), with
            // a focus ring when active (shadcn `focus-visible:ring`).
            let stroke = if response.has_focus() {
                Stroke::new(2.0, tokens.ring)
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

            let sel = ui.visuals().selection;
            let paint_seg = |ui: &Ui, i: usize| {
                let sr = seg_rects[i];
                let active = response.has_focus() && state.seg as usize == i;
                let color = if active {
                    let chip = Rect::from_min_max(
                        egui::pos2(sr.left() - CHIP_PAD_X, TEXT.mul_add(-0.75, cy)),
                        egui::pos2(sr.right() + CHIP_PAD_X, TEXT.mul_add(0.75, cy)),
                    );
                    ui.painter().rect_filled(chip, 4.0, sel.bg_fill);
                    sel.stroke.color
                } else {
                    tokens.foreground
                };
                let g = ui
                    .painter()
                    .layout_no_wrap(labels[i].clone(), font.clone(), color);
                ui.painter()
                    .galley(egui::pos2(sr.left(), cy - g.size().y / 2.0), g, color);
            };
            for i in 0..4 {
                paint_seg(ui, i);
            }

            // Separators in muted foreground.
            let colon1 = seg_rects[0].right();
            let colon2 = seg_rects[1].right();
            for cx in [colon1, colon2] {
                let g = ui.painter().layout_no_wrap(
                    ":".to_owned(),
                    font.clone(),
                    tokens.muted_foreground,
                );
                ui.painter().galley(
                    egui::pos2(cx, cy - g.size().y / 2.0),
                    g,
                    tokens.muted_foreground,
                );
            }
        }

        ui.data_mut(|d| d.insert_temp(id, state));
        if changed {
            response.mark_changed();
        }
        changed
    }
}

/// Step a section up (`up == true`) or down by one, wrapping. Meridiem toggles
/// via a 12-hour shift.
const fn bump(value: u32, modulus: u32, up: bool) -> u32 {
    if up {
        (value + 1) % modulus
    } else {
        (value + modulus - 1) % modulus
    }
}

/// Step a section by `delta` (±1), wrapping.
const fn step(time: &mut Time, seg: u8, delta: i32) {
    let up = delta > 0;
    match seg {
        SEG_HOUR => time.hour = bump(time.hour, 24, up),
        SEG_MIN => time.minute = bump(time.minute, 60, up),
        SEG_SEC => time.second = bump(time.second, 60, up),
        _ => time.hour = (time.hour + 12) % 24, // meridiem
    }
}

/// Feed one typed character into the focused section. Returns `true` if the time
/// changed. Numeric sections accumulate up to two digits and auto-advance once
/// no further digit could fit; the meridiem responds to `a`/`p`.
fn feed_char(time: &mut Time, state: &mut EditState, c: char) -> bool {
    if state.seg == SEG_MERIDIEM {
        let (h12, _) = time.to_12();
        match c.to_ascii_lowercase() {
            'a' => {
                time.set_hour_12(h12, false);
                true
            }
            'p' => {
                time.set_hour_12(h12, true);
                true
            }
            _ => false,
        }
    } else if let Some(d) = c.to_digit(10) {
        feed_digit(time, state, d);
        true
    } else {
        false
    }
}

/// Accumulate a digit into the focused numeric section, advancing when the
/// section is full or no second digit could be valid.
fn feed_digit(time: &mut Time, state: &mut EditState, d: u32) {
    let max = if state.seg == SEG_HOUR { 12 } else { 59 };
    state.buf.push(char::from_digit(d, 10).unwrap_or('0'));
    let n: u32 = state.buf.parse().unwrap_or(0);

    let (value, advance) = if state.buf.len() >= 2 {
        // Second digit: combine, clamp, and move on.
        (n.min(max), true)
    } else if n * 10 > max {
        // First digit can't be a tens digit (e.g. 7 in minutes) → commit now.
        (n, true)
    } else {
        (n, false)
    };

    match state.seg {
        SEG_HOUR => {
            let (_, is_pm) = time.to_12();
            let h = if value == 0 { 12 } else { value };
            time.set_hour_12(h, is_pm);
        }
        SEG_MIN => time.minute = value,
        SEG_SEC => time.second = value,
        _ => {}
    }

    if advance {
        state.seg = (state.seg + 1).min(SEG_MERIDIEM);
        state.buf.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Time::new` clamps each component into range.
    #[test]
    fn clamps_components() {
        assert_eq!(Time::new(30, 99, 99), Time::new(23, 59, 59));
        assert_eq!(
            Time::new(10, 30, 0),
            Time {
                hour: 10,
                minute: 30,
                second: 0
            }
        );
    }

    /// Parsing accepts HH:MM and HH:MM:SS, rejects garbage / out-of-range.
    #[test]
    fn parses_and_rejects() {
        assert_eq!(Time::parse("10:30"), Some(Time::new(10, 30, 0)));
        assert_eq!(Time::parse("10:30:45"), Some(Time::new(10, 30, 45)));
        assert_eq!(Time::parse("  9:5  "), Some(Time::new(9, 5, 0)));
        assert_eq!(Time::parse(""), None);
        assert_eq!(Time::parse("24:00"), None);
        assert_eq!(Time::parse("10:60"), None);
        assert_eq!(Time::parse("noon"), None);
        assert_eq!(Time::parse("1:2:3:4"), None);
    }

    /// Display is zero-padded HH:MM:SS and round-trips through `parse`.
    #[test]
    fn display_round_trips() {
        let t = Time::new(9, 5, 3);
        assert_eq!(t.to_string(), "09:05:03");
        assert_eq!(Time::parse(&t.to_string()), Some(t));
    }

    /// 12-hour conversion handles midnight / noon edges.
    #[test]
    fn twelve_hour_edges() {
        assert_eq!(Time::new(0, 0, 0).to_12(), (12, false)); // midnight
        assert_eq!(Time::new(12, 0, 0).to_12(), (12, true)); // noon
        assert_eq!(Time::new(13, 0, 0).to_12(), (1, true));
        assert_eq!(Time::new(23, 0, 0).to_12(), (11, true));
    }

    /// Stepping wraps each section and the meridiem flips by 12 hours.
    #[test]
    fn step_wraps() {
        let mut t = Time::new(23, 59, 59);
        step(&mut t, SEG_HOUR, 1);
        assert_eq!(t.hour, 0);
        step(&mut t, SEG_MIN, 1);
        assert_eq!(t.minute, 0);
        step(&mut t, SEG_SEC, 1);
        assert_eq!(t.second, 0);
        let mut pm = Time::new(9, 0, 0); // AM
        step(&mut pm, SEG_MERIDIEM, 1);
        assert_eq!(pm.hour, 21); // now PM
    }

    /// Typing digits fills sections and auto-advances.
    #[test]
    fn typing_fills_sections() {
        let mut t = Time::new(0, 0, 0);
        let mut st = EditState::default();
        // Hour: "1" waits, "1" → 11 and advances to minutes.
        feed_digit(&mut t, &mut st, 1);
        assert_eq!(st.seg, SEG_HOUR);
        feed_digit(&mut t, &mut st, 1);
        assert_eq!(st.seg, SEG_MIN);
        let (h, _) = t.to_12();
        assert_eq!(h, 11);
        // Minute: "7" can't start a 7x minute → commits 07, advances.
        feed_digit(&mut t, &mut st, 7);
        assert_eq!(t.minute, 7);
        assert_eq!(st.seg, SEG_SEC);
    }
}
