//! [`Tabs`] — a tab list over switchable panels, mirroring shadcn's `<Tabs>`.
//!
//! The trigger row is shadcn's default *pill* style: a `muted` rounded track
//! holding one trigger per tab, where the active trigger floats as a
//! `background`-filled chip with `foreground` text and a soft shadow, while
//! inactive triggers sit flat in `muted_foreground`. Selection is **controlled**
//! — the caller owns a `&mut usize` active index — so the active tab survives
//! across frames and the caller can drive it programmatically.
//!
//! Beyond the plain switcher, tabs can opt into editor-style behaviour:
//!
//! - [`closable`](Tabs::closable) draws a trailing `✕` on each trigger and
//!   reports the closed index via [`TabsResponse::closed`];
//! - [`reorderable`](Tabs::reorderable) lets the user drag a trigger past its
//!   neighbour, reported as a swap via [`TabsResponse::reordered`];
//! - [`scrollable`](Tabs::scrollable) wraps the row in a horizontal scroll area
//!   so an overflowing strip scrolls instead of clipping.
//!
//! These report *intents* — glazier stays stateless about your tab list, so the
//! caller applies the close/reorder to its own model (and any persistence) and
//! passes the labels back in their new order next frame.
//!
//! ```no_run
//! use glazier::tabs::Tabs;
//! # egui::__run_test_ui(|ui| {
//! let mut active = 0usize;
//! Tabs::new(&mut active, ["Account", "Password"]).show(ui, |ui, i| match i {
//!     0 => { ui.label("Account settings"); }
//!     _ => { ui.label("Change your password"); }
//! });
//! # });
//! ```

use egui::{Id, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};

use crate::tokens::Tokens;

/// Seconds for the active-chip slide / hover fade.
const ANIM_TIME: f32 = 0.15;
/// Track height and inner padding (shadcn `h-9`, `p-[3px]`).
const TRACK_H: f32 = 36.0;
const TRACK_PAD: f32 = 3.0;
/// Per-trigger horizontal padding.
const TRIGGER_PAD_X: f32 = 12.0;
/// Height of a trigger / the active chip (track height minus top+bottom pad).
const INNER_H: f32 = TRACK_H - 2.0 * TRACK_PAD;
/// Close-glyph box edge length and the gap between a label and its `✕`.
const CLOSE_SZ: f32 = 14.0;
const CLOSE_GAP: f32 = 6.0;

/// Outcome of rendering a [`Tabs`] trigger row.
///
/// The active index is written straight back into the caller's `&mut usize`;
/// the rest are one-shot intents for this frame.
pub struct TabsResponse {
    /// The row's overall response (covers the whole track).
    pub response: Response,
    /// Index of the trigger clicked this frame, if any (already applied to the
    /// bound active index).
    pub clicked: Option<usize>,
    /// Index whose trailing `✕` was clicked this frame (only when
    /// [`closable`](Tabs::closable)).
    pub closed: Option<usize>,
    /// A drag-reorder this frame as `(from, to)` display indices to swap (only
    /// when [`reorderable`](Tabs::reorderable)). Apply it to your own list.
    pub reordered: Option<(usize, usize)>,
}

/// A pill-style tab list bound to a caller-owned active index.
///
/// Render the list with [`Tabs::show`], passing a closure that draws the active
/// panel given its index:
///
/// ```no_run
/// use glazier::tabs::Tabs;
/// # egui::__run_test_ui(|ui| {
/// let mut active = 0usize;
/// Tabs::new(&mut active, ["Account", "Password"]).show(ui, |ui, i| match i {
///     0 => { ui.label("Account settings"); }
///     _ => { ui.label("Change your password"); }
/// });
/// # });
/// ```
#[must_use = "tabs do nothing unless shown"]
pub struct Tabs<'a> {
    active: &'a mut usize,
    labels: Vec<String>,
    closable: bool,
    reorderable: bool,
    scrollable: bool,
    id_salt: Option<Id>,
}

impl<'a> Tabs<'a> {
    /// Create a tab list bound to `active` (the selected index) with the given
    /// trigger `labels`.
    pub fn new<I, S>(active: &'a mut usize, labels: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            active,
            labels: labels.into_iter().map(Into::into).collect(),
            closable: false,
            reorderable: false,
            scrollable: false,
            id_salt: None,
        }
    }

    /// Draw a trailing `✕` on each trigger; the closed index is reported via
    /// [`TabsResponse::closed`].
    pub const fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Allow dragging a trigger past its neighbour to reorder; the swap is
    /// reported via [`TabsResponse::reordered`] for the caller to apply.
    pub const fn reorderable(mut self, reorderable: bool) -> Self {
        self.reorderable = reorderable;
        self
    }

    /// Wrap the trigger row in a horizontal scroll area so an overflowing strip
    /// scrolls rather than clipping. Pair with [`id_salt`](Self::id_salt) when
    /// more than one scrollable `Tabs` shares a parent.
    pub const fn scrollable(mut self, scrollable: bool) -> Self {
        self.scrollable = scrollable;
        self
    }

    /// Set a stable id seed, disambiguating the scroll area and per-trigger drag
    /// state when several tab rows live under one parent id.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.id_salt = Some(Id::new(salt));
        self
    }

    /// Render just the trigger row (the `TabsList`), returning its [`Response`].
    /// Use [`Tabs::show`] to also render the active panel beneath it, or
    /// [`Tabs::list_response`] to learn about close / reorder intents.
    pub fn list(self, ui: &mut Ui) -> Response {
        self.list_response(ui).response
    }

    /// Render the trigger row and report close / reorder intents.
    ///
    /// The bound active index is updated on click; `closed` / `reordered` are
    /// returned for the caller to apply to its own tab model.
    pub fn list_response(self, ui: &mut Ui) -> TabsResponse {
        // Pull the fields out so the closure can borrow them past `self`.
        let Self {
            active,
            labels,
            closable,
            reorderable,
            scrollable,
            id_salt,
        } = self;

        if scrollable {
            let salt = id_salt
                .unwrap_or_else(|| Id::new("glazier-tabs"))
                .with("scroll");
            egui::ScrollArea::horizontal()
                .id_salt(salt)
                .show(ui, |ui| {
                    list_inner(ui, active, &labels, closable, reorderable, id_salt)
                })
                .inner
        } else {
            list_inner(ui, active, &labels, closable, reorderable, id_salt)
        }
    }

    /// Render the trigger row and then the active `panel` beneath it.
    ///
    /// `panel` is called with the active tab index after the list draws.
    pub fn show<R>(self, ui: &mut Ui, panel: impl FnOnce(&mut Ui, usize) -> R) -> Response {
        ui.vertical(|ui| {
            // Read the active index before `self` is consumed by `list`.
            let active = *self.active;
            let resp = self.list(ui);
            ui.add_space(8.0);
            panel(ui, active);
            resp
        })
        .inner
    }
}

/// Paint the trigger row and run hit-testing. Factored out of [`Tabs`] so it can
/// run either bare or inside a horizontal [`egui::ScrollArea`].
#[allow(clippy::too_many_lines)]
fn list_inner(
    ui: &mut Ui,
    active: &mut usize,
    labels: &[String],
    closable: bool,
    reorderable: bool,
    id_salt: Option<Id>,
) -> TabsResponse {
    let tokens = Tokens::get(ui);
    let n = labels.len();
    if n == 0 {
        let response = ui.allocate_response(Vec2::ZERO, Sense::hover());
        return TabsResponse {
            response,
            clicked: None,
            closed: None,
            reordered: None,
        };
    }
    *active = (*active).min(n - 1);

    // Lay labels out with PLACEHOLDER so the per-trigger text colour applies at
    // paint time (see toggle_group.rs for the baked-colour trap).
    let font = crate::fonts::semibold(ui, 13.0);
    let galleys: Vec<_> = labels
        .iter()
        .map(|label| {
            ui.painter()
                .layout_no_wrap(label.clone(), font.clone(), egui::Color32::PLACEHOLDER)
        })
        .collect();
    // A closable trigger reserves room for the gap + glyph after its label.
    let close_extra = if closable { CLOSE_GAP + CLOSE_SZ } else { 0.0 };
    let widths: Vec<f32> = galleys
        .iter()
        .map(|g| TRIGGER_PAD_X.mul_add(2.0, g.size().x) + close_extra)
        .collect();
    let track_w = TRACK_PAD.mul_add(2.0, widths.iter().sum());

    let (rect, response) = ui.allocate_at_least(Vec2::new(track_w, TRACK_H), Sense::hover());
    // Pin geometry to the intrinsic width so the track hugs its triggers and
    // stays left-aligned even when the layout hands us a wider rect.
    let track = Rect::from_min_size(rect.left_top(), Vec2::new(track_w, TRACK_H));

    let base_id = id_salt.map_or(response.id, |s| response.id.with(s));

    // Per-trigger left edge in display order.
    let left_of = |i: usize| -> f32 { TRACK_PAD + widths.iter().take(i).sum::<f32>() };

    // Active trigger's live drag offset (only meaningful while reordering).
    let active_offset: f32 = if reorderable {
        ui.ctx()
            .data(|d| d.get_temp(base_id.with(("drag", *active))).unwrap_or(0.0))
    } else {
        0.0
    };

    // Animate the active chip's x-position so it slides between tabs; while the
    // active tab is being dragged, follow the pointer via its offset.
    let target_x = track.left() + left_of(*active) + active_offset;
    let chip_x = if active_offset.abs() > 0.5 {
        target_x // don't animate while actively dragging — track the pointer
    } else {
        ui.ctx()
            .animate_value_with_time(response.id.with("chip_x"), target_x, ANIM_TIME)
    };
    let chip_w = widths[*active];

    if ui.is_rect_visible(track) {
        let painter = ui.painter();
        // Muted track.
        painter.rect_filled(track, tokens.radius_lg(), tokens.muted);

        // Active chip (animated x), inset within the track padding.
        let chip = Rect::from_min_size(
            egui::pos2(chip_x, track.top() + TRACK_PAD),
            Vec2::new(chip_w, INNER_H),
        );
        // Soft shadow + opaque `widget` fill so the active trigger floats
        // above the (possibly translucent) app `background`.
        painter.rect_filled(
            chip.translate(Vec2::new(0.0, 1.0)),
            inner_radius(tokens),
            tokens.border.gamma_multiply(0.5),
        );
        painter.rect_filled(chip, inner_radius(tokens), tokens.widget);
    }

    let mut clicked: Option<usize> = None;
    let mut closed: Option<usize> = None;
    let mut reordered: Option<(usize, usize)> = None;

    // Hit-test + label each trigger.
    for (i, w) in widths.iter().enumerate() {
        // While a tab is dragged, paint it shifted by its stored offset.
        let drag_offset: f32 = if reorderable {
            ui.ctx()
                .data(|d| d.get_temp(base_id.with(("drag", i))).unwrap_or(0.0))
        } else {
            0.0
        };
        let seg_x = track.left() + left_of(i) + drag_offset;
        let seg = Rect::from_min_size(
            egui::pos2(seg_x, track.top() + TRACK_PAD),
            Vec2::new(*w, INNER_H),
        );

        // Trailing close box, carved from the right of the trigger.
        let close_rect = if closable {
            Rect::from_center_size(
                egui::pos2(seg.right() - TRIGGER_PAD_X - CLOSE_SZ / 2.0, seg.center().y),
                Vec2::splat(CLOSE_SZ),
            )
        } else {
            Rect::NOTHING
        };

        // The selectable / draggable region excludes the close box so a close
        // click never starts a drag or a select.
        let hit_rect = if closable {
            Rect::from_min_max(seg.min, egui::pos2(close_rect.left(), seg.bottom()))
        } else {
            seg
        };

        let sense = if reorderable {
            Sense::click_and_drag()
        } else {
            Sense::click()
        };
        let seg_resp = ui
            .interact(hit_rect, base_id.with(("seg", i)), sense)
            .on_hover_cursor(if reorderable {
                egui::CursorIcon::Grab
            } else {
                egui::CursorIcon::PointingHand
            });
        if seg_resp.clicked() {
            clicked = Some(i);
        }

        // Drag-to-reorder: accumulate an x offset, swap with a neighbour once
        // dragged past half its width, and report the swap.
        if reorderable {
            let off_id = base_id.with(("drag", i));
            let mut offset: f32 = ui.ctx().data(|d| d.get_temp(off_id).unwrap_or(0.0));
            if seg_resp.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                offset += seg_resp.drag_delta().x;
                if i + 1 < n && offset > widths[i + 1] / 2.0 {
                    reordered = Some((i, i + 1));
                    offset -= widths[i + 1];
                } else if i > 0 && offset < -widths[i - 1] / 2.0 {
                    reordered = Some((i, i - 1));
                    offset += widths[i - 1];
                }
            } else if seg_resp.drag_stopped() || !seg_resp.is_pointer_button_down_on() {
                offset = 0.0;
            }
            ui.ctx().data_mut(|d| d.insert_temp(off_id, offset));
        }

        // Close button.
        if closable && close_rect.is_positive() {
            let close_resp = ui.interact(close_rect, base_id.with(("close", i)), Sense::click());
            if close_resp.clicked() {
                closed = Some(i);
            }
            if ui.is_rect_visible(close_rect) {
                if close_resp.hovered() {
                    ui.painter().rect_filled(
                        close_rect,
                        tokens.radius_md().saturating_sub(2),
                        tokens.muted_foreground.gamma_multiply(0.25),
                    );
                }
                let c = close_rect.center();
                let h = CLOSE_SZ * 0.22;
                let stroke = Stroke::new(1.4, tokens.muted_foreground);
                ui.painter().line_segment(
                    [egui::pos2(c.x - h, c.y - h), egui::pos2(c.x + h, c.y + h)],
                    stroke,
                );
                ui.painter().line_segment(
                    [egui::pos2(c.x + h, c.y - h), egui::pos2(c.x - h, c.y + h)],
                    stroke,
                );
            }
        }

        if ui.is_rect_visible(seg) {
            let is_active = i == *active;
            // Inactive triggers lift slightly toward foreground on hover.
            let hover_t = ui.ctx().animate_bool_with_time(
                response.id.with(("hover", i)),
                seg_resp.hovered() && !is_active,
                ANIM_TIME,
            );
            let text_col = if is_active {
                tokens.foreground
            } else {
                tokens
                    .muted_foreground
                    .lerp_to_gamma(tokens.foreground, hover_t)
            };
            let g = &galleys[i];
            // Centre the label in the space left of the close box.
            let label_cx = if closable {
                f32::midpoint(seg.left() + TRIGGER_PAD_X, close_rect.left())
            } else {
                seg.center().x
            };
            let pos = egui::pos2(
                label_cx - g.size().x / 2.0,
                seg.center().y - g.size().y / 2.0,
            );
            ui.painter().galley(pos, g.clone(), text_col);
        }
    }

    if let Some(i) = clicked {
        *active = i;
    }

    TabsResponse {
        response,
        clicked,
        closed,
        reordered,
    }
}

/// Active-chip corner radius: one step tighter than the track so the chip nests
/// neatly inside the rounded track.
const fn inner_radius(tokens: Tokens) -> u8 {
    tokens.radius_lg().saturating_sub(2)
}

impl Widget for Tabs<'_> {
    /// Renders just the trigger row. Use [`Tabs::show`] for panels.
    fn ui(self, ui: &mut Ui) -> Response {
        self.list(ui)
    }
}
