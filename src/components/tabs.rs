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

use std::sync::Arc;

use egui::{CursorIcon, Galley, Id, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};

use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry/timing for [`Tabs`] — reach in via [`Tabs::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct TabsMetrics {
    /// Seconds for the active-chip slide / hover fade.
    pub anim_time: f32,
    /// Track height (shadcn `h-9`).
    pub track_h: f32,
    /// Track inner padding (shadcn `p-[3px]`).
    pub track_pad: f32,
    /// Per-trigger horizontal padding.
    pub trigger_pad_x: f32,
    /// Close-glyph box edge length.
    pub close_sz: f32,
    /// Gap between a label and its trailing `✕`.
    pub close_gap: f32,
}

impl Default for TabsMetrics {
    fn default() -> Self {
        Self {
            anim_time: 0.15,
            track_h: 36.0,
            track_pad: 3.0,
            trigger_pad_x: 12.0,
            close_sz: 14.0,
            close_gap: 6.0,
        }
    }
}

impl TabsMetrics {
    /// Height of a trigger / the active chip (track height minus top+bottom pad).
    const fn inner_h(self) -> f32 {
        self.track_h - 2.0 * self.track_pad
    }
}

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
    /// Index whose trailing `✕` was clicked, or that was middle-clicked, this
    /// frame (only when [`closable`](Tabs::closable)).
    pub closed: Option<usize>,
    /// A drag-reorder this frame as `(from, to)` display indices to swap (only
    /// when [`reorderable`](Tabs::reorderable)). Apply it to your own list.
    pub reordered: Option<(usize, usize)>,
}

impl TabsResponse {
    /// A response that reports no intents.
    const fn idle(response: Response) -> Self {
        Self {
            response,
            clicked: None,
            closed: None,
            reordered: None,
        }
    }
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
    sizing_hook: SizingHook<TabsMetrics>,
}

impl Sizeable<TabsMetrics> for Tabs<'_> {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<TabsMetrics> {
        &mut self.sizing_hook
    }
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
            sizing_hook: SizingHook::new(),
        }
    }

    /// Draw a trailing `✕` on each trigger and close on middle-click, as
    /// editors and browsers do; the closed index is reported via
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
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(Id::unique(salt));
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
    pub fn list_response(mut self, ui: &mut Ui) -> TabsResponse {
        let options = StripOptions {
            closable: self.closable,
            reorderable: self.reorderable,
            id_salt: self.id_salt,
            m: crate::sizing::resolve(std::mem::take(&mut self.sizing_hook)),
        };
        let (active, labels) = (self.active, &self.labels);
        if self.scrollable {
            let salt = self
                .id_salt
                .unwrap_or_else(|| Id::unique("glazier-tabs"))
                .with("scroll");
            egui::ScrollArea::horizontal()
                .id_salt(salt)
                .show(ui, |ui| Strip::show(ui, options, active, labels))
                .inner
        } else {
            Strip::show(ui, options, active, labels)
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

impl Widget for Tabs<'_> {
    /// Renders just the trigger row. Use [`Tabs::show`] for panels.
    fn ui(self, ui: &mut Ui) -> Response {
        self.list(ui)
    }
}

/// The trigger row's settings, split from [`Tabs`] so the row can render
/// either bare or inside a horizontal [`egui::ScrollArea`].
#[derive(Clone, Copy)]
struct StripOptions {
    closable: bool,
    reorderable: bool,
    id_salt: Option<Id>,
    m: TabsMetrics,
}

/// One frame of the trigger row: its laid-out labels and where they sit.
struct Strip {
    opts: StripOptions,
    tokens: Tokens,
    galleys: Vec<Arc<Galley>>,
    widths: Vec<f32>,
    track: Rect,
    response: Response,
    base_id: Id,
}

impl Strip {
    /// Lay out, paint, and hit-test the row, applying a click to `active`.
    fn show(
        ui: &mut Ui,
        opts: StripOptions,
        active: &mut usize,
        labels: &[String],
    ) -> TabsResponse {
        let Some(last) = labels.len().checked_sub(1) else {
            return TabsResponse::idle(ui.allocate_response(Vec2::ZERO, Sense::hover()));
        };
        *active = (*active).min(last);
        let strip = Self::new(ui, opts, labels);
        strip.paint_track(ui, *active);
        let mut out = TabsResponse::idle(strip.response.clone());
        for i in 0..labels.len() {
            strip.trigger(ui, i, *active, &mut out);
        }
        if let Some(i) = out.clicked {
            *active = i;
        }
        out
    }

    fn new(ui: &mut Ui, opts: StripOptions, labels: &[String]) -> Self {
        let m = opts.m;
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
        let close_extra = if opts.closable {
            m.close_gap + m.close_sz
        } else {
            0.0
        };
        let widths: Vec<f32> = galleys
            .iter()
            .map(|g| m.trigger_pad_x.mul_add(2.0, g.size().x) + close_extra)
            .collect();
        let track_w = m.track_pad.mul_add(2.0, widths.iter().sum());
        let (rect, response) = ui.allocate_at_least(Vec2::new(track_w, m.track_h), Sense::hover());
        let base_id = opts.id_salt.map_or(response.id, |s| response.id.with(s));
        Self {
            opts,
            tokens: Tokens::get(ui),
            galleys,
            widths,
            // Pin geometry to the intrinsic width so the track hugs its triggers
            // and stays left-aligned even when the layout hands us a wider rect.
            track: Rect::from_min_size(rect.left_top(), Vec2::new(track_w, m.track_h)),
            response,
            base_id,
        }
    }

    /// Trigger `i`'s left edge, relative to the track.
    fn left_of(&self, i: usize) -> f32 {
        self.opts.m.track_pad + self.widths[..i].iter().sum::<f32>()
    }

    fn drag_id(&self, i: usize) -> Id {
        self.base_id.with(("drag", i))
    }

    /// How far trigger `i` is currently dragged from its slot.
    fn drag_offset(&self, ui: &Ui, i: usize) -> f32 {
        if !self.opts.reorderable {
            return 0.0;
        }
        ui.ctx()
            .data(|d| d.get_temp(self.drag_id(i)).unwrap_or(0.0))
    }

    /// The active chip's screen x.
    ///
    /// The animation runs on the offset within the track, not on screen x:
    /// screen x moves on every scroll tick, and animating it made the chip lag
    /// the strip. While the active tab is dragged the chip follows the pointer
    /// directly.
    fn chip_x(&self, ui: &Ui, active: usize) -> f32 {
        let dragged = self.drag_offset(ui, active);
        let target = self.left_of(active) + dragged;
        let offset = if dragged.abs() > 0.5 {
            target
        } else {
            ui.ctx().animate_value_with_time(
                self.response.id.with("chip_x"),
                target,
                self.opts.m.anim_time,
            )
        };
        self.track.left() + offset
    }

    /// Paint the muted track and the active chip floating on it.
    fn paint_track(&self, ui: &Ui, active: usize) {
        let chip_x = self.chip_x(ui, active);
        if !ui.is_rect_visible(self.track) {
            return;
        }
        let (m, tokens) = (self.opts.m, self.tokens);
        let painter = ui.painter();
        painter.rect_filled(self.track, tokens.radius_lg(), tokens.muted);
        let chip = Rect::from_min_size(
            egui::pos2(chip_x, self.track.top() + m.track_pad),
            Vec2::new(self.widths[active], m.inner_h()),
        );
        // One step tighter than the track so the chip nests inside it.
        let radius = tokens.radius_lg().saturating_sub(2);
        // Soft shadow + opaque `widget` fill so the active trigger floats
        // above the (possibly translucent) app `background`.
        painter.rect_filled(
            chip.translate(Vec2::new(0.0, 1.0)),
            radius,
            tokens.border.gamma_multiply(0.5),
        );
        painter.rect_filled(chip, radius, tokens.widget);
    }

    /// Trigger `i`'s rect, shifted by any live drag.
    fn segment(&self, ui: &Ui, i: usize) -> Rect {
        let m = self.opts.m;
        let x = self.track.left() + self.left_of(i) + self.drag_offset(ui, i);
        Rect::from_min_size(
            egui::pos2(x, self.track.top() + m.track_pad),
            Vec2::new(self.widths[i], m.inner_h()),
        )
    }

    /// The trailing close box carved from the right of `seg`, when closable.
    fn close_rect(&self, seg: Rect) -> Option<Rect> {
        let m = self.opts.m;
        self.opts.closable.then(|| {
            Rect::from_center_size(
                egui::pos2(
                    seg.right() - m.trigger_pad_x - m.close_sz / 2.0,
                    seg.center().y,
                ),
                Vec2::splat(m.close_sz),
            )
        })
    }

    /// Hit-test, drag, close, and label trigger `i`, recording intents in `out`.
    fn trigger(&self, ui: &Ui, i: usize, active: usize, out: &mut TabsResponse) {
        let seg = self.segment(ui, i);
        let close = self.close_rect(seg);
        // The close box is excluded so a close click never selects or drags.
        let hit = close.map_or(seg, |c| {
            Rect::from_min_max(seg.min, egui::pos2(c.left(), seg.bottom()))
        });
        let (sense, cursor) = if self.opts.reorderable {
            (Sense::click_and_drag(), CursorIcon::Grab)
        } else {
            (Sense::click(), CursorIcon::PointingHand)
        };
        let resp = ui
            .interact(hit, self.base_id.with(("seg", i)), sense)
            .on_hover_cursor(cursor);
        if resp.clicked() {
            out.clicked = Some(i);
        }
        if self.opts.reorderable {
            out.reordered = self.drag(ui, i, &resp).or(out.reordered);
        }
        let middle = close.is_some() && resp.clicked_by(egui::PointerButton::Middle);
        if middle || close.is_some_and(|c| self.close_button(ui, i, c)) {
            out.closed = Some(i);
        }
        if ui.is_rect_visible(seg) {
            self.paint_label(
                ui,
                i,
                seg,
                close,
                resp.hovered() && i != active,
                i == active,
            );
        }
    }

    /// Accumulate trigger `i`'s drag offset. Once it passes half a
    /// neighbour's width, report the swap and rebase the offset onto the new
    /// slot.
    fn drag(&self, ui: &Ui, i: usize, resp: &Response) -> Option<(usize, usize)> {
        let mut offset = self.drag_offset(ui, i);
        let mut swap = None;
        if resp.dragged() {
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            offset += resp.drag_delta().x;
            if let Some((to, shift)) = self.swap_target(i, offset) {
                swap = Some((i, to));
                offset -= shift;
            }
        } else if resp.drag_stopped() || !resp.is_pointer_button_down_on() {
            offset = 0.0;
        }
        ui.ctx()
            .data_mut(|d| d.insert_temp(self.drag_id(i), offset));
        swap
    }

    /// The neighbour trigger `i` has been dragged past, with the signed width
    /// it moved by.
    fn swap_target(&self, i: usize, offset: f32) -> Option<(usize, f32)> {
        let right = self
            .widths
            .get(i + 1)
            .filter(|w| offset > *w / 2.0)
            .map(|w| (i + 1, *w));
        right.or_else(|| {
            let left = i.checked_sub(1)?;
            let w = self.widths[left];
            (offset < -w / 2.0).then_some((left, -w))
        })
    }

    /// The `✕` box for trigger `i`. Returns `true` when clicked.
    fn close_button(&self, ui: &Ui, i: usize, rect: Rect) -> bool {
        let resp = ui.interact(rect, self.base_id.with(("close", i)), Sense::click());
        if ui.is_rect_visible(rect) {
            let tokens = self.tokens;
            if resp.hovered() {
                ui.painter().rect_filled(
                    rect,
                    tokens.radius_md().saturating_sub(2),
                    tokens.muted_foreground.gamma_multiply(0.25),
                );
            }
            let c = rect.center();
            let h = self.opts.m.close_sz * 0.22;
            let stroke = Stroke::new(1.4, tokens.muted_foreground);
            for dy in [-h, h] {
                ui.painter().line_segment(
                    [egui::pos2(c.x - h, c.y + dy), egui::pos2(c.x + h, c.y - dy)],
                    stroke,
                );
            }
        }
        resp.clicked()
    }

    /// Paint trigger `i`'s label, centred in the space left of its close box.
    fn paint_label(
        &self,
        ui: &Ui,
        i: usize,
        seg: Rect,
        close: Option<Rect>,
        hovered: bool,
        is_active: bool,
    ) {
        let tokens = self.tokens;
        // Inactive triggers lift slightly toward foreground on hover.
        let hover_t = ui.ctx().animate_bool_with_time(
            self.response.id.with(("hover", i)),
            hovered,
            self.opts.m.anim_time,
        );
        let color = if is_active {
            tokens.foreground
        } else {
            tokens
                .muted_foreground
                .lerp_to_gamma(tokens.foreground, hover_t)
        };
        let label_cx = close.map_or_else(
            || seg.center().x,
            |c| f32::midpoint(seg.left() + self.opts.m.trigger_pad_x, c.left()),
        );
        let g = &self.galleys[i];
        let pos = egui::pos2(
            label_cx - g.size().x / 2.0,
            seg.center().y - g.size().y / 2.0,
        );
        ui.painter().galley(pos, g.clone(), color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;
    use egui::FullOutput;

    const LABELS: [&str; 3] = ["Account", "Password", "Billing"];

    /// An editor-style tab row on a headless context, collecting intents.
    #[derive(Default)]
    struct Rig {
        ui: TestUi,
        active: usize,
        closed: Option<usize>,
        reordered: Option<(usize, usize)>,
    }

    impl Rig {
        fn build(&mut self) -> impl FnMut(&mut Ui) + '_ {
            |ui| {
                let out = Tabs::new(&mut self.active, LABELS)
                    .closable(true)
                    .reorderable(true)
                    .list_response(ui);
                self.closed = out.closed.or(self.closed);
                self.reordered = out.reordered.or(self.reordered);
            }
        }

        fn run(
            &mut self,
            drive: impl FnOnce(&TestUi, &mut dyn FnMut(&mut Ui)) -> FullOutput,
        ) -> FullOutput {
            let ui = std::mem::take(&mut self.ui);
            let out = drive(&ui, &mut self.build());
            self.ui = ui;
            out
        }

        fn label(&mut self, text: &str) -> Rect {
            let out = self.run(|ui, build| ui.frame(build));
            TestUi::text_rect(&out, text)
        }
    }

    #[test]
    fn clicking_a_trigger_selects_it() {
        let mut rig = Rig::default();
        let password = rig.label("Password").center();
        rig.run(|ui, build| ui.click(password, build));
        assert_eq!(rig.active, 1);
        assert_eq!(rig.closed, None);
    }

    #[test]
    fn clicking_the_cross_reports_a_close() {
        let mut rig = Rig::default();
        let billing = rig.label("Billing");
        let cross = egui::pos2(billing.right() + 10.0, billing.center().y);
        rig.run(|ui, build| ui.click(cross, build));
        assert_eq!(rig.closed, Some(2));
        assert_eq!(rig.active, 0);
    }

    #[test]
    fn middle_clicking_a_trigger_reports_a_close() {
        let mut rig = Rig::default();
        let password = rig.label("Password").center();
        rig.run(|ui, build| ui.click_with(egui::PointerButton::Middle, password, build));
        assert_eq!(rig.closed, Some(1));
        assert_eq!(rig.active, 0, "a middle-click must not select");
    }

    #[test]
    fn middle_clicking_a_fixed_tab_does_nothing() {
        let ui = TestUi::new();
        let mut active = 0;
        let mut closed = None;
        let mut build = |ui: &mut Ui| {
            closed = Tabs::new(&mut active, LABELS)
                .list_response(ui)
                .closed
                .or(closed);
        };
        let password = TestUi::text_rect(&ui.frame(&mut build), "Password").center();
        ui.click_with(egui::PointerButton::Middle, password, &mut build);
        assert_eq!(closed, None);
    }

    #[test]
    fn dragging_past_a_neighbour_reports_a_swap() {
        let mut rig = Rig::default();
        let account = rig.label("Account").center();
        let password = rig.label("Password").center();
        rig.run(|ui, build| ui.drag(account, password, build));
        assert_eq!(rig.reordered, Some((0, 1)));
    }

    #[test]
    fn active_index_is_clamped_to_the_labels() {
        let ui = TestUi::new();
        let mut active = 9;
        ui.frame(|ui| {
            Tabs::new(&mut active, LABELS)
                .scrollable(true)
                .show(ui, |_, _| ());
        });
        assert_eq!(active, 2);
    }

    #[test]
    fn empty_tabs_take_no_space() {
        let ui = TestUi::new();
        let mut active = 0;
        let mut size = Vec2::splat(1.0);
        ui.frame(|ui| {
            size = ui
                .add(Tabs::new(&mut active, Vec::<String>::new()))
                .rect
                .size();
        });
        assert_eq!(size, Vec2::ZERO);
    }
}
