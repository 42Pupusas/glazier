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

use egui::{Color32, Event, FontId, Id, Key, Modifiers, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry for [`TimePicker`] — reach in via
/// [`TimePicker::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct TimePickerMetrics {
    /// Field height (`h-9`).
    pub height: f32,
    /// Horizontal padding inside the field (`px-3`).
    pub pad_x: f32,
    /// Section / separator text size (`text-sm`).
    pub text: f32,
    /// Extra width padding around each focusable section's highlight chip.
    pub chip_pad_x: f32,
}

impl Default for TimePickerMetrics {
    fn default() -> Self {
        Self {
            height: 36.0,
            pad_x: 12.0,
            text: 13.0,
            chip_pad_x: 3.0,
        }
    }
}

/// The four editable sections, left to right.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum Segment {
    #[default]
    Hour,
    Minute,
    Second,
    Meridiem,
}

impl Segment {
    const ALL: [Self; 4] = [Self::Hour, Self::Minute, Self::Second, Self::Meridiem];

    const fn prev(self) -> Self {
        match self {
            Self::Hour | Self::Minute => Self::Hour,
            Self::Second => Self::Minute,
            Self::Meridiem => Self::Second,
        }
    }

    const fn next(self) -> Self {
        match self {
            Self::Hour => Self::Minute,
            Self::Minute => Self::Second,
            Self::Second | Self::Meridiem => Self::Meridiem,
        }
    }

    /// The largest value typing may enter into a numeric section.
    const fn max_typed(self) -> u32 {
        match self {
            Self::Hour => 12,
            _ => 59,
        }
    }
}

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

    /// Step `value` by one within `0..modulus`, wrapping.
    const fn bump(value: u32, modulus: u32, up: bool) -> u32 {
        if up {
            (value + 1) % modulus
        } else {
            (value + modulus - 1) % modulus
        }
    }

    /// Step a section up or down by one, wrapping. The meridiem flips by 12
    /// hours either way.
    const fn step(&mut self, seg: Segment, up: bool) {
        match seg {
            Segment::Hour => self.hour = Self::bump(self.hour, 24, up),
            Segment::Minute => self.minute = Self::bump(self.minute, 60, up),
            Segment::Second => self.second = Self::bump(self.second, 60, up),
            Segment::Meridiem => self.hour = (self.hour + 12) % 24,
        }
    }

    /// Write a typed value into a numeric section. A typed hour is on the
    /// 12-hour face, so `0` means 12 and the meridiem is kept.
    const fn set_typed(&mut self, seg: Segment, value: u32) {
        match seg {
            Segment::Hour => {
                let (_, is_pm) = self.to_12();
                let h = if value == 0 { 12 } else { value };
                self.set_hour_12(h, is_pm);
            }
            Segment::Minute => self.minute = value,
            Segment::Second => self.second = value,
            Segment::Meridiem => {}
        }
    }

    /// The text shown in a section.
    fn label(self, seg: Segment) -> String {
        let (h12, is_pm) = self.to_12();
        match seg {
            Segment::Hour => format!("{h12:02}"),
            Segment::Minute => format!("{:02}", self.minute),
            Segment::Second => format!("{:02}", self.second),
            Segment::Meridiem => if is_pm { "PM" } else { "AM" }.to_owned(),
        }
    }
}

impl fmt::Display for Time {
    /// Zero-padded `HH:MM:SS` (24-hour).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
    }
}

/// Per-widget editing state held in egui memory: the focused section and the
/// first digit typed into it, if any (so a second keypress appends rather than
/// replaces).
#[derive(Clone, Default)]
struct EditState {
    seg: Segment,
    pending: Option<u32>,
}

impl EditState {
    /// Move to `seg` and start a fresh edit run there.
    const fn focus(&mut self, seg: Segment) {
        self.seg = seg;
        self.pending = None;
    }

    /// Feed one typed character into the focused section. Returns `true` if
    /// the time changed.
    fn feed_char(&mut self, time: &mut Time, c: char) -> bool {
        if self.seg == Segment::Meridiem {
            return Self::feed_meridiem(time, c);
        }
        c.to_digit(10).is_some_and(|d| {
            self.feed_digit(time, d);
            true
        })
    }

    /// `a` sets AM, `p` sets PM; anything else is ignored.
    const fn feed_meridiem(time: &mut Time, c: char) -> bool {
        let c = c.to_ascii_lowercase();
        let is_pm = c == 'p';
        if !is_pm && c != 'a' {
            return false;
        }
        let (h12, _) = time.to_12();
        time.set_hour_12(h12, is_pm);
        true
    }

    /// Accumulate a digit into the focused numeric section, advancing when the
    /// section is full or no second digit could be valid (`7` in minutes).
    fn feed_digit(&mut self, time: &mut Time, d: u32) {
        let max = self.seg.max_typed();
        let full = self.pending.is_some();
        let n = self.pending.map_or(d, |tens| tens * 10 + d);
        time.set_typed(self.seg, if full { n.min(max) } else { n });
        if full || n * 10 > max {
            self.focus(self.seg.next());
        } else {
            self.pending = Some(n);
        }
    }
}

/// Where each section sits this frame, and what it reads.
struct Layout {
    labels: [String; 4],
    rects: [Rect; 4],
    font: FontId,
    cy: f32,
}

impl Layout {
    /// Lay the sections out left to right, a colon's width apart.
    fn new(ui: &Ui, time: Time, rect: Rect, m: &TimePickerMetrics, color: Color32) -> Self {
        let font = FontId::proportional(m.text);
        let measure = |s: &str| {
            ui.painter()
                .layout_no_wrap(s.to_owned(), font.clone(), color)
                .size()
                .x
        };
        let labels = Segment::ALL.map(|seg| time.label(seg));
        let colon_w = measure(":");
        let mut x = rect.left() + m.pad_x;
        let rects = std::array::from_fn(|i| {
            // The meridiem keeps one width so the field doesn't jitter on AM↔PM.
            let w = if Segment::ALL[i] == Segment::Meridiem {
                measure("AM").max(measure("PM"))
            } else {
                measure(&labels[i])
            };
            let r = Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::new(w, m.height));
            x += w + colon_w;
            r
        });
        Self {
            labels,
            rects,
            font,
            cy: rect.center().y,
        }
    }

    /// The section clicked this frame, if any. Every section is registered
    /// each frame, so the fold must not short-circuit.
    fn clicked(&self, ui: &Ui, id: Id) -> Option<Segment> {
        Segment::ALL
            .into_iter()
            .zip(self.rects)
            .fold(None, |hit, (seg, r)| {
                let clicked = ui.interact(r, id.with(seg), Sense::click()).clicked();
                if clicked {
                    Some(seg)
                } else {
                    hit
                }
            })
    }

    /// Paint the field chrome, the sections, and the colons. `active` is the
    /// focused section, or `None` when the picker isn't focused.
    fn paint(
        &self,
        ui: &Ui,
        rect: Rect,
        active: Option<Segment>,
        tokens: &Tokens,
        m: &TimePickerMetrics,
    ) {
        // Matches Input: opaque `widget` surface, hairline border, focus ring.
        let stroke = if active.is_some() {
            Stroke::new(2.0, tokens.ring)
        } else {
            Stroke::new(1.0, tokens.input)
        };
        ui.painter().rect(
            rect,
            tokens.radius_md(),
            tokens.widget,
            stroke,
            StrokeKind::Inside,
        );
        for (i, seg) in Segment::ALL.into_iter().enumerate() {
            self.paint_segment(ui, i, active == Some(seg), tokens, m);
        }
        for r in &self.rects[..2] {
            self.paint_text(ui, ":", r.right(), tokens.muted_foreground);
        }
    }

    fn paint_segment(
        &self,
        ui: &Ui,
        i: usize,
        active: bool,
        tokens: &Tokens,
        m: &TimePickerMetrics,
    ) {
        let sr = self.rects[i];
        let color = if active {
            let sel = ui.visuals().selection;
            let chip = Rect::from_min_max(
                egui::pos2(sr.left() - m.chip_pad_x, m.text.mul_add(-0.75, self.cy)),
                egui::pos2(sr.right() + m.chip_pad_x, m.text.mul_add(0.75, self.cy)),
            );
            ui.painter().rect_filled(chip, 4.0, sel.bg_fill);
            sel.stroke.color
        } else {
            tokens.foreground
        };
        self.paint_text(ui, &self.labels[i], sr.left(), color);
    }

    fn paint_text(&self, ui: &Ui, text: &str, x: f32, color: Color32) {
        let g = ui
            .painter()
            .layout_no_wrap(text.to_owned(), self.font.clone(), color);
        ui.painter()
            .galley(egui::pos2(x, self.cy - g.size().y / 2.0), g, color);
    }
}

/// A segmented time-of-day editor.
#[must_use = "time pickers do nothing unless you show them"]
pub struct TimePicker<'a> {
    time: &'a mut Time,
    width: Option<f32>,
    id_salt: egui::Id,
    sizing_hook: SizingHook<TimePickerMetrics>,
}

impl Sizeable<TimePickerMetrics> for TimePicker<'_> {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<TimePickerMetrics> {
        &mut self.sizing_hook
    }
}

impl<'a> TimePicker<'a> {
    /// Create a time picker editing `time`.
    pub fn new(time: &'a mut Time) -> Self {
        Self {
            time,
            width: None,
            id_salt: egui::Id::unique("glazier-time-picker"),
            sizing_hook: SizingHook::new(),
        }
    }

    /// Set an explicit width; defaults to the available width.
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Disambiguate multiple time pickers in one view.
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = egui::Id::unique(("glazier-time-picker", egui::Id::unique(salt)));
        self
    }

    /// Render the picker. Returns `true` if the time changed this frame.
    pub fn show(mut self, ui: &mut Ui) -> bool {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let id = ui.make_persistent_id(self.id_salt);

        let (rect, mut response) =
            ui.allocate_exact_size(Vec2::new(width, m.height), Sense::click());
        let was_focused = response.has_focus();
        let mut state: EditState = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
        let layout = Layout::new(ui, *self.time, rect, &m, tokens.foreground);

        if let Some(seg) = layout.clicked(ui, id) {
            state.focus(seg);
            response.request_focus();
        }
        if response.clicked() && !was_focused {
            response.request_focus();
        }

        let focused = response.has_focus();
        let changed = if focused {
            Self::claim_arrows(ui, response.id);
            self.handle_keys(ui, &mut state)
        } else {
            // Dropping focus ends the current edit run.
            state.pending = None;
            false
        };

        if ui.is_rect_visible(rect) {
            layout.paint(ui, rect, focused.then_some(state.seg), &tokens, &m);
        }

        ui.data_mut(|d| d.insert_temp(id, state));
        if changed {
            response.mark_changed();
        }
        changed
    }

    /// Keep the arrow keys away from egui's focus navigation, which would
    /// otherwise move focus to a neighbouring widget on ↑/← and lose the edit.
    fn claim_arrows(ui: &Ui, widget: Id) {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                widget,
                egui::EventFilter {
                    tab: false,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: false,
                },
            );
        });
    }

    /// Apply this frame's arrows and typed text. Returns `true` if the time
    /// changed.
    fn handle_keys(&mut self, ui: &Ui, state: &mut EditState) -> bool {
        // Consumed, so scroll areas and focus navigation never see them.
        let [left, right, up, down] = ui.input_mut(|i| {
            [
                Key::ArrowLeft,
                Key::ArrowRight,
                Key::ArrowUp,
                Key::ArrowDown,
            ]
            .map(|key| i.consume_key(Modifiers::NONE, key))
        });
        if left {
            state.focus(state.seg.prev());
        }
        if right {
            state.focus(state.seg.next());
        }
        for (pressed, step_up) in [(up, true), (down, false)] {
            if pressed {
                self.time.step(state.seg, step_up);
                state.pending = None;
            }
        }

        let typed: String = ui.input(|i| {
            i.events
                .iter()
                .filter_map(|ev| match ev {
                    Event::Text(t) => Some(t.as_str()),
                    _ => None,
                })
                .collect()
        });
        typed.chars().fold(up || down, |changed, c| {
            state.feed_char(self.time, c) | changed
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;

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
        t.step(Segment::Hour, true);
        assert_eq!(t.hour, 0);
        t.step(Segment::Minute, true);
        assert_eq!(t.minute, 0);
        t.step(Segment::Second, true);
        assert_eq!(t.second, 0);
        t.step(Segment::Second, false);
        assert_eq!(t.second, 59);
        let mut pm = Time::new(9, 0, 0); // AM
        pm.step(Segment::Meridiem, true);
        assert_eq!(pm.hour, 21); // now PM
    }

    /// Typing digits fills sections and auto-advances.
    #[test]
    fn typing_fills_sections() {
        let mut t = Time::new(0, 0, 0);
        let mut st = EditState::default();
        // Hour: "1" waits, "1" → 11 and advances to minutes.
        st.feed_digit(&mut t, 1);
        assert_eq!(st.seg, Segment::Hour);
        st.feed_digit(&mut t, 1);
        assert_eq!(st.seg, Segment::Minute);
        assert_eq!(t.to_12().0, 11);
        // Minute: "7" can't start a 7x minute → commits 07, advances.
        st.feed_digit(&mut t, 7);
        assert_eq!(t.minute, 7);
        assert_eq!(st.seg, Segment::Second);
    }

    /// The meridiem takes `a`/`p` and ignores other characters.
    #[test]
    fn meridiem_takes_a_and_p() {
        let mut t = Time::new(9, 0, 0);
        let mut st = EditState::default();
        st.focus(Segment::Meridiem);
        assert!(st.feed_char(&mut t, 'P'));
        assert_eq!(t.hour, 21);
        assert!(st.feed_char(&mut t, 'a'));
        assert_eq!(t.hour, 9);
        assert!(!st.feed_char(&mut t, '5'));
        st.focus(Segment::Minute);
        assert!(!st.feed_char(&mut t, 'x'));
    }

    /// A picker on a headless context, fed real pointer and key input.
    struct Rig {
        ui: TestUi,
        time: Time,
        changed: bool,
        origin: egui::Pos2,
    }

    impl Rig {
        fn new(time: Time) -> Self {
            let mut rig = Self {
                ui: TestUi::new(),
                time,
                changed: false,
                origin: egui::Pos2::ZERO,
            };
            rig.input(Vec::new());
            rig
        }

        fn build(&mut self) -> impl FnMut(&mut Ui) + '_ {
            |ui| {
                self.origin = ui.cursor().min;
                self.changed |= TimePicker::new(&mut self.time).width(240.0).show(ui);
            }
        }

        fn input(&mut self, events: Vec<Event>) {
            let ui = std::mem::take(&mut self.ui);
            ui.frame_with(events, self.build());
            self.ui = ui;
        }

        /// Click the hour section.
        fn click_hour(&mut self) {
            let pos = self.origin + egui::vec2(16.0, 18.0);
            let ui = std::mem::take(&mut self.ui);
            ui.click(pos, self.build());
            self.ui = ui;
        }
    }

    /// Clicking a section, then arrows and digits, edits the bound time.
    #[test]
    fn keyboard_edits_the_focused_section() {
        let mut rig = Rig::new(Time::new(10, 30, 0));
        rig.click_hour();
        rig.input(vec![TestUi::key(Key::ArrowUp)]);
        assert_eq!(rig.time.hour, 11);
        assert!(rig.changed);

        rig.input(vec![
            TestUi::key(Key::ArrowRight),
            TestUi::key(Key::ArrowRight),
            TestUi::key(Key::ArrowLeft),
        ]);
        rig.input(vec![Event::Text("45".to_owned())]);
        assert_eq!(rig.time.minute, 45);

        rig.input(vec![TestUi::key(Key::ArrowDown)]);
        assert_eq!(rig.time.second, 59);
    }

    /// Without focus the picker ignores typing.
    #[test]
    fn unfocused_picker_ignores_typing() {
        let mut rig = Rig::new(Time::new(10, 30, 0));
        rig.input(vec![Event::Text("7".to_owned()), TestUi::key(Key::ArrowUp)]);
        assert_eq!(rig.time, Time::new(10, 30, 0));
        assert!(!rig.changed);
    }
}
