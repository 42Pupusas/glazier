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

use egui::{Event, Key, Modifiers, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::tokens::Tokens;

/// Cell edge length (`size-10` ≈ 40px, trimmed for egui density).
const CELL: f32 = 38.0;
/// Gap between adjacent cells within a group.
const CELL_GAP: f32 = 8.0;
/// Gap between groups of cells (where a separator sits).
const GROUP_GAP: f32 = 12.0;
/// Separator dash half-width.
const SEP_W: f32 = 6.0;
/// Cell glyph size (`text-sm`+).
const TEXT: f32 = 18.0;

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

/// Per-field caret state, persisted in egui memory.
#[derive(Clone, Copy, Default)]
struct OtpState {
    /// Caret position — the cell that will receive the next typed character
    /// (0..=len). At `len` the field is full and the caret rests on the last
    /// cell.
    caret: usize,
}

/// A segmented one-time-code input bound to a `&mut String`.
#[must_use = "inputs do nothing unless you show them"]
pub struct InputOtp<'a> {
    value: &'a mut String,
    len: usize,
    group: Option<usize>,
    mode: OtpMode,
    id_salt: egui::Id,
}

impl<'a> InputOtp<'a> {
    /// Create an OTP field of `len` cells bound to `value`.
    pub fn new(value: &'a mut String, len: usize) -> Self {
        Self {
            value,
            len,
            group: None,
            mode: OtpMode::Digits,
            id_salt: egui::Id::new("glazier-input-otp"),
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
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.id_salt = egui::Id::new(salt);
        self
    }

    /// Is `c` an accepted character for this field?
    const fn accepts(&self, c: char) -> bool {
        self.mode.accepts(c)
    }

    /// Render the field. Returns `true` the frame the code becomes complete
    /// (all `len` cells filled).
    pub fn show(self, ui: &mut Ui) -> bool {
        let tokens = Tokens::get(ui);
        let id = ui.make_persistent_id(self.id_salt);

        // Normalise the bound string: keep only accepted chars, clamp to len.
        let mut chars: Vec<char> = self
            .value
            .chars()
            .filter(|&c| self.accepts(c))
            .map(|c| c.to_ascii_uppercase())
            .take(self.len)
            .collect();

        // Geometry: cells, plus a gap wherever a group boundary falls.
        let group = self.group.filter(|&g| g > 0);
        let n_seps = group.map_or(0, |g| self.len.saturating_sub(1) / g);
        let sep_gap = 2.0_f32.mul_add(SEP_W, GROUP_GAP);
        // Inner gaps: a `sep_gap` at each group boundary, a `CELL_GAP` between
        // every other pair of adjacent cells.
        let n_cell_gaps = self.len.saturating_sub(1).saturating_sub(n_seps);
        #[allow(clippy::cast_precision_loss)]
        let total_w = (self.len as f32).mul_add(
            CELL,
            (n_seps as f32).mul_add(sep_gap, n_cell_gaps as f32 * CELL_GAP),
        );
        let (rect, mut response) = ui.allocate_exact_size(Vec2::new(total_w, CELL), Sense::click());
        let focused = response.has_focus();

        let mut state: OtpState = ui.data(|d| d.get_temp(id).unwrap_or_default());
        state.caret = state.caret.min(self.len);

        // Compute each cell's rect (left-to-right, inserting separator gaps).
        let mut cell_rects = Vec::with_capacity(self.len);
        let mut x = rect.left();
        for i in 0..self.len {
            if i > 0 {
                x += if group.is_some_and(|g| i % g == 0) {
                    sep_gap
                } else {
                    CELL_GAP
                };
            }
            cell_rects.push(egui::Rect::from_min_size(
                egui::pos2(x, rect.top()),
                Vec2::splat(CELL),
            ));
            x += CELL;
        }

        // Click a cell to focus and place the caret there (clamped to the first
        // empty cell so you can't edit past the filled run).
        for (i, cr) in cell_rects.iter().enumerate() {
            let r = ui.interact(*cr, id.with(("cell", i)), Sense::click());
            if r.clicked() {
                state.caret = i.min(chars.len());
                response.request_focus();
            }
        }
        if response.clicked() && !focused {
            state.caret = chars.len();
            response.request_focus();
        }

        let was_full = chars.len() == self.len;

        if response.has_focus() {
            // Keep the horizontal arrows for caret movement.
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    response.id,
                    egui::EventFilter {
                        tab: false,
                        horizontal_arrows: true,
                        vertical_arrows: false,
                        escape: false,
                    },
                );
            });
            self.handle_keys(ui, &mut chars, &mut state);
        }

        // Write the (possibly mutated) characters back into the bound string.
        let new_value: String = chars.iter().collect();
        if *self.value != new_value {
            *self.value = new_value;
        }

        if ui.is_rect_visible(rect) {
            self.paint(ui, tokens, &cell_rects, &chars, state, focused, sep_gap);
        }

        ui.data_mut(|d| d.insert_temp(id, state));

        let full = chars.len() == self.len;
        if full && (*self.value).chars().count() == self.len {
            response.mark_changed();
        }
        // "Just completed" edge.
        full && !was_full
    }

    /// Paint all cells and the group separators.
    #[allow(clippy::too_many_arguments)]
    fn paint(
        &self,
        ui: &Ui,
        tokens: Tokens,
        cell_rects: &[egui::Rect],
        chars: &[char],
        state: OtpState,
        focused: bool,
        sep_gap: f32,
    ) {
        let blink = ui.input(|i| i.time).fract() < 0.5;
        for (i, cr) in cell_rects.iter().enumerate() {
            paint_cell(
                ui,
                tokens,
                *cr,
                chars.get(i).copied(),
                CellState {
                    active: focused && i == state.caret.min(self.len - 1),
                    caret_on: focused && i == state.caret && state.caret < self.len && blink,
                },
            );
        }
        // Group separators (an en-dash between groups).
        if let Some(g) = self.group.filter(|&g| g > 0) {
            let n_seps = self.len.saturating_sub(1) / g;
            for sep in 1..=n_seps {
                let between = cell_rects[sep * g - 1].right();
                let cx = between + sep_gap / 2.0;
                let cy = cell_rects[0].center().y;
                ui.painter().hline(
                    (cx - SEP_W)..=(cx + SEP_W),
                    cy,
                    Stroke::new(2.0, tokens.muted_foreground),
                );
            }
        }
        // Repaint to animate the caret blink while focused.
        if focused {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(500));
        }
    }

    /// Process text / editing keys for one frame.
    fn handle_keys(&self, ui: &Ui, chars: &mut Vec<char>, state: &mut OtpState) {
        // Caret movement.
        let (mut left, mut right) = (false, false);
        ui.input_mut(|i| {
            left = i.consume_key(Modifiers::NONE, Key::ArrowLeft);
            right = i.consume_key(Modifiers::NONE, Key::ArrowRight);
        });
        if left {
            state.caret = state.caret.saturating_sub(1);
        }
        if right {
            state.caret = (state.caret + 1).min(chars.len().min(self.len));
        }

        let events = ui.input(|i| i.events.clone());
        for ev in events {
            match ev {
                // Typed characters and pasted text both fill cells from the
                // caret (paste skips rejected chars, so "123-456" works too).
                Event::Text(t) | Event::Paste(t) => {
                    for c in t.chars() {
                        self.feed_char(chars, state, c);
                    }
                }
                Event::Key {
                    key: Key::Backspace,
                    pressed: true,
                    ..
                } => {
                    if state.caret > 0 && state.caret == chars.len() {
                        // Caret at the end: pop the last char.
                        chars.pop();
                        state.caret -= 1;
                    } else if state.caret > 0 {
                        // Caret mid-run: step back and remove that cell.
                        state.caret -= 1;
                        if state.caret < chars.len() {
                            chars.remove(state.caret);
                        }
                    }
                }
                Event::Key {
                    key: Key::Delete,
                    pressed: true,
                    ..
                } if state.caret < chars.len() => {
                    chars.remove(state.caret);
                }
                _ => {}
            }
        }
        state.caret = state.caret.min(self.len);
    }

    /// Insert one character at the caret, if accepted and there's room.
    fn feed_char(&self, chars: &mut Vec<char>, state: &mut OtpState, c: char) {
        if !self.accepts(c) || state.caret >= self.len {
            return;
        }
        let c = c.to_ascii_uppercase();
        if state.caret < chars.len() {
            chars[state.caret] = c;
        } else {
            chars.push(c);
        }
        state.caret = (state.caret + 1).min(self.len);
    }
}

/// Transient per-cell paint flags.
#[derive(Clone, Copy)]
struct CellState {
    active: bool,
    caret_on: bool,
}

/// Paint one OTP cell: chrome, optional glyph, optional caret.
fn paint_cell(ui: &Ui, tokens: Tokens, rect: egui::Rect, ch: Option<char>, cell: CellState) {
    // Active cell takes the focus ring; others a hairline `input` border.
    let stroke = if cell.active {
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

    if let Some(c) = ch {
        let galley = ui.painter().layout_no_wrap(
            c.to_string(),
            egui::FontId::proportional(TEXT),
            tokens.foreground,
        );
        let pos = egui::pos2(
            rect.center().x - galley.size().x / 2.0,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(pos, galley, tokens.foreground);
    } else if cell.caret_on {
        // Blinking caret in an empty active cell.
        let cy = rect.center().y;
        let h = TEXT * 0.6;
        ui.painter().vline(
            rect.center().x,
            (cy - h / 2.0)..=(cy + h / 2.0),
            Stroke::new(2.0, tokens.foreground),
        );
    }
}
