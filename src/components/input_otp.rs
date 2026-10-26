//! [`InputOtp`] — a segmented one-time-code field, mirroring shadcn's
//! `InputOTP`.
//!
//! A row of single-character cells that together edit one `&mut String` of the
//! entered code. Typing fills cells left to right (auto-advancing the caret);
//! Backspace clears/retreats; arrows and clicks move the caret; paste fills as
//! many cells as fit. The active cell shows a blinking caret and a focus ring,
//! mirroring shadcn's `InputOTPSlot`. Cells can be split into groups with a
//! separator (shadcn's `InputOTPSeparator`).
//!
//! By default only ASCII digits are accepted; pass an [`OtpMode`] to
//! [`mode`](InputOtp::mode) to allow letters or both (letters are upper-cased,
//! matching shadcn's `REGEXP_ONLY_*` presets).
//!
//! ```no_run
//! use glazier::input_otp::InputOtp;
//! # egui::__run_test_ui(|ui| {
//! let mut code = String::new();
//! // A 6-digit code split 3 + 3.
//! let done = InputOtp::new(&mut code, 6).group(3).show(ui);
//! if done {
//!     // all six cells filled
//! }
//! # });
//! ```

use egui::{Event, Id, Key, Modifiers, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry for [`InputOtp`] — reach in via [`InputOtp::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct InputOtpMetrics {
    /// Cell edge length (`size-10` ≈ 40px, trimmed for egui density).
    pub cell: f32,
    /// Gap between adjacent cells within a group.
    pub cell_gap: f32,
    /// Gap between groups of cells (where a separator sits).
    pub group_gap: f32,
    /// Separator dash half-width.
    pub sep_w: f32,
    /// Cell glyph size (`text-sm`+).
    pub text: f32,
}

impl Default for InputOtpMetrics {
    fn default() -> Self {
        Self {
            cell: 38.0,
            cell_gap: 8.0,
            group_gap: 12.0,
            sep_w: 6.0,
            text: 18.0,
        }
    }
}

impl InputOtpMetrics {
    /// The gap at a group boundary, wide enough for the separator dash.
    const fn sep_gap(self) -> f32 {
        2.0_f32.mul_add(self.sep_w, self.group_gap)
    }
}

/// Which characters an [`InputOtp`] accepts, mirroring shadcn's three
/// `input-otp` regex presets (`REGEXP_ONLY_DIGITS`, `…_ONLY_CHARS`,
/// `…_DIGITS_AND_CHARS`).
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum OtpMode {
    /// Digits only, `0-9` (the default).
    #[default]
    Digits,
    /// Letters only, `A-Z` (upper-cased).
    Letters,
    /// Digits and letters, `0-9A-Z` (letters upper-cased).
    Alphanumeric,
}

impl OtpMode {
    /// Does this mode accept `c`?
    const fn accepts(self, c: char) -> bool {
        match self {
            Self::Digits => c.is_ascii_digit(),
            Self::Letters => c.is_ascii_alphabetic(),
            Self::Alphanumeric => c.is_ascii_alphanumeric(),
        }
    }
}

/// The code being edited and its caret. Only the caret persists across frames
/// (in egui memory); the characters are re-read from the bound string.
struct Editor {
    chars: Vec<char>,
    /// The cell that will receive the next typed character (0..=len). At
    /// `len` the field is full and the caret rests on the last cell.
    caret: usize,
    len: usize,
    mode: OtpMode,
}

impl Editor {
    /// Normalise `value`: keep only accepted chars, upper-cased, clamped to
    /// `len`.
    fn new(value: &str, len: usize, mode: OtpMode, caret: usize) -> Self {
        let chars: Vec<char> = value
            .chars()
            .filter(|&c| mode.accepts(c))
            .map(|c| c.to_ascii_uppercase())
            .take(len)
            .collect();
        Self {
            chars,
            caret: caret.min(len),
            len,
            mode,
        }
    }

    const fn is_full(&self) -> bool {
        self.chars.len() == self.len
    }

    /// Place the caret on cell `i`, clamped to the first empty cell so the
    /// filled run stays contiguous.
    fn place(&mut self, i: usize) {
        self.caret = i.min(self.chars.len());
    }

    /// Insert one character at the caret, if accepted and there's room.
    fn feed(&mut self, c: char) {
        if !self.mode.accepts(c) || self.caret >= self.len {
            return;
        }
        let c = c.to_ascii_uppercase();
        match self.chars.get_mut(self.caret) {
            Some(cell) => *cell = c,
            None => self.chars.push(c),
        }
        self.caret += 1;
    }

    /// Step back and clear the cell before the caret.
    fn backspace(&mut self) {
        if self.caret == 0 {
            return;
        }
        self.caret -= 1;
        if self.caret < self.chars.len() {
            self.chars.remove(self.caret);
        }
    }

    /// Clear the cell under the caret.
    fn delete(&mut self) {
        if self.caret < self.chars.len() {
            self.chars.remove(self.caret);
        }
    }

    const fn left(&mut self) {
        self.caret = self.caret.saturating_sub(1);
    }

    fn right(&mut self) {
        self.caret = (self.caret + 1).min(self.chars.len());
    }

    /// Apply one input event.
    fn apply(&mut self, ev: &Event) {
        match ev {
            // Typed and pasted text both fill cells from the caret; rejected
            // chars are skipped, so pasting "123-456" works too.
            Event::Text(t) | Event::Paste(t) => t.chars().for_each(|c| self.feed(c)),
            Event::Key {
                key, pressed: true, ..
            } => self.key(*key),
            _ => {}
        }
    }

    fn key(&mut self, key: Key) {
        match key {
            Key::Backspace => self.backspace(),
            Key::Delete => self.delete(),
            _ => {}
        }
    }

    /// The cell holding the focus ring: the caret, or the last cell when full.
    fn active_cell(&self) -> usize {
        self.caret.min(self.len.saturating_sub(1))
    }

    fn text(&self) -> String {
        self.chars.iter().collect()
    }
}

/// Where the cells and separators sit this frame.
struct Layout {
    cells: Vec<Rect>,
    /// The x centre of each group separator.
    separators: Vec<f32>,
}

impl Layout {
    /// Every cell is `m.cell` square; a separator gap opens at each group
    /// boundary, a cell gap everywhere else.
    fn new(origin: egui::Pos2, len: usize, group: Option<usize>, m: InputOtpMetrics) -> Self {
        let mut cells = Vec::with_capacity(len);
        let mut separators = Vec::new();
        let mut x = origin.x;
        for i in 0..len {
            if i > 0 && group.is_some_and(|g| i % g == 0) {
                separators.push(x + m.sep_gap() / 2.0);
                x += m.sep_gap();
            } else if i > 0 {
                x += m.cell_gap;
            }
            cells.push(Rect::from_min_size(
                egui::pos2(x, origin.y),
                Vec2::splat(m.cell),
            ));
            x += m.cell;
        }
        Self { cells, separators }
    }

    fn width(&self) -> f32 {
        match (self.cells.first(), self.cells.last()) {
            (Some(first), Some(last)) => last.right() - first.left(),
            _ => 0.0,
        }
    }

    /// The cell clicked this frame, if any.
    fn clicked(&self, ui: &Ui, id: Id) -> Option<usize> {
        self.cells.iter().enumerate().fold(None, |hit, (i, r)| {
            let clicked = ui
                .interact(*r, id.with(("cell", i)), Sense::click())
                .clicked();
            if clicked {
                Some(i)
            } else {
                hit
            }
        })
    }
}

/// Paints one frame of the field.
struct Painter<'a> {
    ui: &'a Ui,
    tokens: Tokens,
    m: InputOtpMetrics,
}

impl Painter<'_> {
    /// Paint every cell and separator. `editor` is `Some` while focused.
    fn paint(&self, layout: &Layout, chars: &[char], editor: Option<&Editor>) {
        // The caret blinks at 1 Hz while focused.
        let blink = self.ui.input(|i| i.time).fract() < 0.5;
        for (i, rect) in layout.cells.iter().enumerate() {
            let active = editor.is_some_and(|e| e.active_cell() == i);
            let caret = blink && editor.is_some_and(|e| e.caret == i);
            self.cell(*rect, chars.get(i).copied(), active, caret);
        }
        let cy = layout.cells.first().map_or(0.0, |c| c.center().y);
        for cx in &layout.separators {
            self.ui.painter().hline(
                (cx - self.m.sep_w)..=(cx + self.m.sep_w),
                cy,
                Stroke::new(2.0, self.tokens.muted_foreground),
            );
        }
        if editor.is_some() {
            self.ui
                .ctx()
                .request_repaint_after(std::time::Duration::from_millis(500));
        }
    }

    /// One cell: chrome, then its glyph or, when empty and due, the caret.
    fn cell(&self, rect: Rect, ch: Option<char>, active: bool, caret: bool) {
        let tokens = self.tokens;
        // Active cell takes the focus ring; others a hairline `input` border.
        let stroke = if active {
            Stroke::new(2.0, tokens.ring)
        } else {
            Stroke::new(1.0, tokens.input)
        };
        // Opaque `widget` surface — `background` is the app-canvas token and
        // may be translucent under a user theme.
        self.ui.painter().rect(
            rect,
            tokens.radius_md(),
            tokens.widget,
            stroke,
            StrokeKind::Inside,
        );
        let c = rect.center();
        if let Some(ch) = ch {
            let galley = self.ui.painter().layout_no_wrap(
                ch.to_string(),
                egui::FontId::proportional(self.m.text),
                tokens.foreground,
            );
            let pos = c - galley.size() / 2.0;
            self.ui.painter().galley(pos, galley, tokens.foreground);
        } else if caret {
            let h = self.m.text * 0.3;
            self.ui.painter().vline(
                c.x,
                (c.y - h)..=(c.y + h),
                Stroke::new(2.0, tokens.foreground),
            );
        }
    }
}

/// A segmented one-time-code input bound to a `&mut String`.
#[must_use = "inputs do nothing unless you show them"]
pub struct InputOtp<'a> {
    value: &'a mut String,
    len: usize,
    group: Option<usize>,
    mode: OtpMode,
    id_salt: egui::Id,
    sizing_hook: SizingHook<InputOtpMetrics>,
}

impl Sizeable<InputOtpMetrics> for InputOtp<'_> {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<InputOtpMetrics> {
        &mut self.sizing_hook
    }
}

impl<'a> InputOtp<'a> {
    /// Create an OTP field of `len` cells bound to `value`.
    pub fn new(value: &'a mut String, len: usize) -> Self {
        Self {
            value,
            len,
            group: None,
            mode: OtpMode::Digits,
            id_salt: egui::Id::unique("glazier-input-otp"),
            sizing_hook: SizingHook::new(),
        }
    }

    /// Split the cells into groups of `size`, with a separator between groups
    /// (e.g. `group(3)` on a 6-cell field renders `•••–•••`).
    pub const fn group(mut self, size: usize) -> Self {
        self.group = Some(size);
        self
    }

    /// Choose which characters are accepted ([`OtpMode::Digits`] by default).
    pub const fn mode(mut self, mode: OtpMode) -> Self {
        self.mode = mode;
        self
    }

    /// Shorthand for [`mode(OtpMode::Alphanumeric)`](Self::mode) when `yes`,
    /// else [`OtpMode::Digits`].
    pub const fn alphanumeric(mut self, yes: bool) -> Self {
        self.mode = if yes {
            OtpMode::Alphanumeric
        } else {
            OtpMode::Digits
        };
        self
    }

    /// Set a stable id salt (needed if several OTP fields share a parent).
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = egui::Id::unique(salt);
        self
    }

    /// Render the field. Returns `true` the frame the code becomes complete
    /// (all `len` cells filled).
    pub fn show(mut self, ui: &mut Ui) -> bool {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let id = ui.make_persistent_id(self.id_salt);
        let caret = ui.data(|d| d.get_temp(id).unwrap_or_default());
        let mut editor = Editor::new(self.value, self.len, self.mode, caret);

        let group = self.group.filter(|&g| g > 0);
        let probe = Layout::new(egui::Pos2::ZERO, self.len, group, m);
        let (rect, mut response) =
            ui.allocate_exact_size(Vec2::new(probe.width(), m.cell), Sense::click());
        let layout = Layout::new(rect.min, self.len, group, m);

        let was_focused = response.has_focus();
        if let Some(i) = layout.clicked(ui, id) {
            editor.place(i);
            response.request_focus();
        }
        if response.clicked() && !was_focused {
            editor.place(self.len);
            response.request_focus();
        }

        let was_full = editor.is_full();
        let focused = response.has_focus();
        if focused {
            Self::edit(ui, response.id, &mut editor);
        }
        let text = editor.text();
        if *self.value != text {
            *self.value = text;
        }

        if ui.is_rect_visible(rect) {
            Painter { ui, tokens, m }.paint(&layout, &editor.chars, focused.then_some(&editor));
        }
        ui.data_mut(|d| d.insert_temp(id, editor.caret));

        let full = editor.is_full();
        if full {
            response.mark_changed();
        }
        // "Just completed" edge.
        full && !was_full
    }

    /// Process caret keys and typed text for one frame.
    fn edit(ui: &Ui, widget: Id, editor: &mut Editor) {
        // Keep the horizontal arrows for caret movement.
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                widget,
                egui::EventFilter {
                    tab: false,
                    horizontal_arrows: true,
                    vertical_arrows: false,
                    escape: false,
                },
            );
        });
        let [left, right] = ui.input_mut(|i| {
            [Key::ArrowLeft, Key::ArrowRight].map(|key| i.consume_key(Modifiers::NONE, key))
        });
        if left {
            editor.left();
        }
        if right {
            editor.right();
        }
        ui.input(|i| i.events.iter().for_each(|ev| editor.apply(ev)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;

    fn editor(value: &str, caret: usize) -> Editor {
        Editor::new(value, 6, OtpMode::Digits, caret)
    }

    #[test]
    fn normalises_the_bound_value() {
        let e = Editor::new("ab-12cd34ef", 4, OtpMode::Alphanumeric, 9);
        assert_eq!(e.text(), "AB12");
        assert_eq!(e.caret, 4);
        assert_eq!(Editor::new("a1b2", 6, OtpMode::Letters, 0).text(), "AB");
        assert_eq!(editor("12-34", 0).text(), "1234");
    }

    #[test]
    fn typing_fills_and_stops_when_full() {
        let mut e = editor("", 0);
        e.apply(&Event::Paste("123-4567".to_owned()));
        assert_eq!(e.text(), "123456");
        assert!(e.is_full());
        assert_eq!(e.active_cell(), 5);
    }

    #[test]
    fn typing_overwrites_mid_run() {
        let mut e = editor("1234", 0);
        e.place(1);
        e.feed('9');
        assert_eq!(e.text(), "1934");
        assert_eq!(e.caret, 2);
    }

    #[test]
    fn backspace_and_delete_clear_cells() {
        let mut e = editor("1234", 4);
        e.apply(&TestUi::key(Key::Backspace));
        assert_eq!((e.text().as_str(), e.caret), ("123", 3));
        e.left();
        e.left();
        e.apply(&TestUi::key(Key::Delete));
        assert_eq!((e.text().as_str(), e.caret), ("13", 1));
        e.apply(&TestUi::key(Key::Backspace));
        assert_eq!((e.text().as_str(), e.caret), ("3", 0));
        e.apply(&TestUi::key(Key::Backspace));
        assert_eq!(e.text(), "3");
    }

    #[test]
    fn caret_never_passes_the_filled_run() {
        let mut e = editor("12", 0);
        e.place(5);
        assert_eq!(e.caret, 2);
        e.right();
        assert_eq!(e.caret, 2);
    }

    #[test]
    fn layout_opens_a_gap_per_group() {
        let m = InputOtpMetrics::default();
        let grouped = Layout::new(egui::Pos2::ZERO, 6, Some(3), m);
        let plain = Layout::new(egui::Pos2::ZERO, 6, None, m);
        assert_eq!(grouped.separators.len(), 1);
        assert_eq!(plain.separators.len(), 0);
        assert!((grouped.width() - plain.width() - (m.sep_gap() - m.cell_gap)).abs() < 1e-3);
        assert!(Layout::new(egui::Pos2::ZERO, 0, None, m).width().abs() < f32::EPSILON);
    }

    /// Clicking the field, then typing, completes the code exactly once.
    #[test]
    fn typing_into_the_field_completes_it() {
        let ui = TestUi::new();
        let mut code = String::new();
        let mut completions = 0;
        {
            let mut build = |ui: &mut Ui| {
                completions += u8::from(InputOtp::new(&mut code, 4).group(2).show(ui));
            };
            // egui hit-tests against the previous frame's widgets.
            ui.frame(&mut build);
            // The field is the first widget, so its first cell sits at the
            // panel's top-left corner.
            ui.click(egui::pos2(16.0, 16.0), &mut build);
            ui.frame_with(vec![Event::Text("12".to_owned())], &mut build);
            ui.frame_with(vec![TestUi::key(Key::ArrowLeft)], &mut build);
            ui.frame_with(vec![TestUi::key(Key::ArrowRight)], &mut build);
            ui.frame_with(vec![Event::Paste("3-4".to_owned())], &mut build);
            ui.frame(&mut build);
        }
        assert_eq!(code, "1234");
        assert_eq!(completions, 1);
    }
}
