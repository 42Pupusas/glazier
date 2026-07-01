//! [`MessageScroller`] — a chat transcript scroller, mirroring shadcn's
//! `<MessageScroller>` family.
//!
//! Building a streaming chat transcript well is mostly about *scroll intent*:
//! when to follow the live edge, when to hold the reader's place, and when to
//! let them decide. This widget bundles those behaviours so product code stays
//! focused on composing [`Message`](crate::Message)s and
//! [`Marker`](crate::Marker)s:
//!
//! * **Follow the live edge** — with [`auto_scroll`](MessageScroller::auto_scroll)
//!   the viewport pins to the bottom as content streams in, but *only while the
//!   reader is already there*. Scroll up and it backs off and preserves your
//!   place; return to the bottom and it re-engages. (This rides on egui's
//!   [`stick_to_bottom`](egui::ScrollArea::stick_to_bottom).)
//! * **Anchor new turns** — mark the row that starts a turn (usually the user's
//!   message) with [`anchor`](true) on [`item`](MessageScroller::item). When a
//!   new anchor appears it settles near the top of the viewport with a peek of
//!   the previous row above it ([`previous_item_peek`]), so a new turn can be
//!   read from the beginning without feeling detached.
//! * **Open where it matters** — [`default_position`] picks where a freshly
//!   shown transcript lands: [`Start`](Position::Start), [`End`](Position::End),
//!   or [`LastAnchor`](Position::LastAnchor) (the last meaningful turn).
//! * **Keep your place on prepend** — when older rows are loaded in above, the
//!   viewport offset is nudged to keep the currently-visible row steady.
//! * **Jump to latest** — an optional floating button appears when the reader
//!   has scrolled away from the live edge and returns them to it.
//!
//! [`previous_item_peek`]: MessageScroller::previous_item_peek
//! [`default_position`]: MessageScroller::default_position
//! [`true`]: MessageScroller::item

use std::hash::{Hash, Hasher};

use egui::{Align, Response, Sense, Ui, Vec2};

use crate::components::scroll_area::style_scrollbar;
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry for [`MessageScroller`] — reach in via
/// [`MessageScroller::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct MessageScrollerMetrics {
    /// Default peek of the previous row kept above a newly anchored turn (px).
    pub default_peek: f32,
    /// Jump-to-latest button diameter (shadcn `size-8`).
    pub button: f32,
    /// Inset of the floating button from the viewport's bottom-right corner.
    pub button_inset: f32,
    /// Slack (px) within which the viewport counts as "at the live edge".
    pub edge_slack: f32,
}

impl Default for MessageScrollerMetrics {
    fn default() -> Self {
        Self {
            default_peek: 64.0,
            button: 32.0,
            button_inset: 12.0,
            edge_slack: 4.0,
        }
    }
}

/// Where a freshly-shown transcript should open, mirroring shadcn's
/// `defaultScrollPosition`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    /// The first row, at the top.
    Start,
    /// The latest row, at the bottom.
    #[default]
    End,
    /// The last anchored turn near the top (falls back to [`End`](Self::End)
    /// when there is no anchor).
    LastAnchor,
}

/// Per-scroller persisted state, keyed off the scroller's id.
#[derive(Clone, Copy, Default)]
struct ScrollerState {
    /// Have we applied the initial [`Position`] yet?
    opened: bool,
    /// Hash of the last anchored row, to detect a new turn (0 = none).
    last_anchor: u64,
    /// Hash of the first row, to detect prepended history (0 = none).
    first_id: u64,
    /// Content height last frame, to size a prepend nudge.
    content_h: f32,
    /// Vertical scroll offset last frame, for relative external nudges.
    offset_y: f32,
    /// An explicit scroll request queued for the next frame.
    pending: Pending,
    /// A one-shot offset to apply next frame (prepend place-keeping).
    pending_offset: Option<f32>,
}

/// A queued, explicit scroll action.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Pending {
    #[default]
    None,
    End,
}

/// One transcript row.
struct Item<'a> {
    id: u64,
    anchor: bool,
    content: Box<dyn FnOnce(&mut Ui) + 'a>,
}

/// What the layout pass should do once the rows are measured.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    None,
    End,
    Start,
    /// Settle the last anchor near the top (used for new turns + last-anchor open).
    AnchorTop,
}

/// A chat transcript scroller.
///
/// ```no_run
/// use glazier::message_scroller::{MessageScroller, Position};
/// use glazier::message::{Message, Side};
/// # egui::__run_test_ui(|ui| {
/// MessageScroller::new("chat")
///     .auto_scroll(true)
///     .default_position(Position::LastAnchor)
///     .item("m1", false, |ui| {
///         Message::new().show(ui, |ui| { ui.label("Hi!"); });
///     })
///     .item("m2", true, |ui| {  // a user turn — anchors near the top
///         Message::new().align(Side::End).show(ui, |ui| { ui.label("Hello"); });
///     })
///     .show(ui);
/// # });
/// ```
#[must_use = "message scrollers do nothing unless shown"]
pub struct MessageScroller<'a> {
    id_source: egui::Id,
    auto_scroll: bool,
    peek: f32,
    default_position: Position,
    button: bool,
    max_height: Option<f32>,
    scroll_by: f32,
    content_right_pad: f32,
    items: Vec<Item<'a>>,
    sizing_hook: SizingHook<MessageScrollerMetrics>,
}

impl Sizeable<MessageScrollerMetrics> for MessageScroller<'_> {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<MessageScrollerMetrics> {
        &mut self.sizing_hook
    }
}

impl<'a> MessageScroller<'a> {
    /// Create a scroller identified by `id_source` (persists scroll state across
    /// frames).
    pub fn new(id_source: impl Hash) -> Self {
        Self {
            id_source: egui::Id::new(id_source),
            auto_scroll: false,
            peek: MessageScrollerMetrics::default().default_peek,
            default_position: Position::default(),
            button: true,
            max_height: None,
            scroll_by: 0.0,
            content_right_pad: 0.0,
            items: Vec::new(),
            sizing_hook: SizingHook::default(),
        }
    }

    /// Follow the live edge: pin to the bottom as content streams in, but only
    /// while the reader is already at the bottom (shadcn's `autoScroll`).
    pub const fn auto_scroll(mut self, auto_scroll: bool) -> Self {
        self.auto_scroll = auto_scroll;
        self
    }

    /// Keep this many points of the previous row visible above a newly anchored
    /// turn (shadcn's `scrollPreviousItemPeek`, default `64`).
    pub const fn previous_item_peek(mut self, peek: f32) -> Self {
        self.peek = peek;
        self
    }

    /// Where the transcript opens on its first frame (shadcn's
    /// `defaultScrollPosition`, default [`End`](Position::End)).
    pub const fn default_position(mut self, position: Position) -> Self {
        self.default_position = position;
        self
    }

    /// Show the floating "jump to latest" button when the reader has scrolled
    /// away from the live edge (default `true`).
    pub const fn button(mut self, button: bool) -> Self {
        self.button = button;
        self
    }

    /// Nudge the viewport by `delta` points (positive scrolls down) on the next
    /// frame, relative to the current offset.
    ///
    /// Useful for wiring external scroll intents — e.g. `PageUp`/`PageDown`
    /// while a sibling widget holds keyboard focus. Following the live edge
    /// ([`auto_scroll`](Self::auto_scroll)) backs off automatically once this
    /// moves the reader away from the bottom.
    pub const fn scroll_by(mut self, delta: f32) -> Self {
        self.scroll_by = delta;
        self
    }

    /// Add right-side padding inside the scroll content area (points).
    ///
    /// Useful when the card's right padding has been zeroed and you still want
    /// a gap between message bubbles/avatars and the scrollbar thumb.
    pub const fn content_right_pad(mut self, pad: f32) -> Self {
        self.content_right_pad = pad;
        self
    }

    /// Give the viewport a definite height.
    ///
    /// egui's [`ScrollArea`](egui::ScrollArea) treats its `max_height` as a
    /// *cap* on the available height, so without this the transcript collapses
    /// when it sits inside another scroll area or a content-sized column. Set
    /// this to reserve a fixed height; by default the scroller fills the height
    /// its parent makes available.
    pub const fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }

    /// Append a transcript row.
    ///
    /// `id` should be **stable** across frames so the scroller can keep your
    /// place when content changes. Set `anchor` on the row that starts a turn
    /// (usually the user's message, or a "joined the chat" marker) so new turns
    /// settle near the top.
    pub fn item(mut self, id: impl Hash, anchor: bool, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.items.push(Item {
            id: hash_id(&id),
            anchor,
            content: Box::new(content),
        });
        self
    }

    /// Render the transcript.
    pub fn show(mut self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let id = ui.make_persistent_id(self.id_source);
        let mut state: ScrollerState = ui.ctx().data_mut(|d| d.get_temp(id)).unwrap_or_default();

        // The current last-anchor / first-row ids drive new-turn and prepend
        // detection. Compute them before rendering.
        let cur_last_anchor = self.items.iter().rfind(|i| i.anchor).map_or(0, |i| i.id);
        let cur_first_id = self.items.first().map_or(0, |i| i.id);

        // Decide what scroll action this frame's layout should perform.
        // Explicit requests (button) win, then the initial open, then a new turn.
        let new_anchor =
            state.opened && cur_last_anchor != 0 && cur_last_anchor != state.last_anchor;
        let action = match state.pending {
            Pending::End => Action::End,
            Pending::None if !state.opened => match self.default_position {
                Position::Start => Action::Start,
                Position::End => Action::End,
                Position::LastAnchor => {
                    if cur_last_anchor == 0 {
                        Action::End
                    } else {
                        Action::AnchorTop
                    }
                }
            },
            Pending::None if new_anchor => Action::AnchorTop,
            Pending::None => Action::None,
        };
        state.pending = Pending::None;

        // An external relative nudge (e.g. PageUp/PageDown) queues a one-shot
        // offset from last frame's position. Skip it when an explicit action
        // already owns this frame's scroll.
        if self.scroll_by != 0.0 && action == Action::None && state.pending_offset.is_none() {
            state.pending_offset = Some((state.offset_y + self.scroll_by).max(0.0));
        }

        style_scrollbar(ui, tokens);
        // Thumb sits flush with the viewport edge; callers that zero the card's
        // right padding should use `content_right_pad` to keep bubbles clear.
        ui.style_mut().spacing.scroll.bar_outer_margin = 0.0;
        let show_button = self.button;
        let out = self.render_area(ui, id, action, state.pending_offset.take(), tokens);

        // --- Resolve scroll state for the button + prepend place-keeping. ---
        let viewport_h = out.inner_rect.height();
        let content_h = out.content_size.y;
        let offset_y = out.state.offset.y;
        let can_scroll = content_h > viewport_h + 1.0;
        let at_bottom = offset_y + viewport_h >= content_h - m.edge_slack;

        // Older rows were prepended: nudge the offset by the height that grew
        // above the fold so the visible row stays put.
        if state.opened
            && cur_first_id != 0
            && state.first_id != 0
            && cur_first_id != state.first_id
            && content_h > state.content_h
        {
            state.pending_offset = Some(offset_y + (content_h - state.content_h));
            ui.ctx().request_repaint();
        }

        // Floating jump-to-latest button: shown only when the reader has left
        // the live edge and there is somewhere below to go.
        if show_button && can_scroll && !at_bottom && jump_button(ui, id, out.inner_rect, tokens, m) {
            state.pending = Pending::End;
            ui.ctx().request_repaint();
        }

        // Stop scroll-chaining: egui's ScrollArea only consumes the wheel delta
        // when it actually moves, so at the top/bottom edge the leftover delta
        // bubbles up and scrolls the page behind us. While the pointer is over
        // the transcript, swallow the vertical wheel delta so the outer page
        // stays put.
        if can_scroll && ui.rect_contains_pointer(out.inner_rect) {
            ui.input_mut(|i| i.smooth_scroll_delta.y = 0.0);
        }

        // Persist detection state for next frame.
        state.opened = true;
        state.last_anchor = cur_last_anchor;
        state.first_id = cur_first_id;
        state.content_h = content_h;
        state.offset_y = offset_y;
        ui.ctx().data_mut(|d| d.insert_temp(id, state));

        ui.interact(out.inner_rect, out.id, Sense::hover())
    }

    /// Build and render the scroll area, giving it a definite height when one
    /// was requested (egui only treats `max_height` as a *cap*, so without this
    /// the transcript collapses inside another scroll area or content-sized
    /// column). Returns the area output for scroll-state resolution.
    fn render_area(
        self,
        ui: &mut Ui,
        id: egui::Id,
        action: Action,
        pending_offset: Option<f32>,
        _tokens: Tokens,
    ) -> egui::scroll_area::ScrollAreaOutput<()> {
        let fixed_h = self.max_height;
        let max_h = fixed_h.unwrap_or_else(|| ui.available_height());
        let peek = self.peek;
        let auto_scroll = self.auto_scroll;
        let items = self.items;
        let content_right_pad = self.content_right_pad;
        // Account for the scrollbar width so content doesn't slide under the
        // thumb when the bar is visible.
        let bar_w = ui.style().spacing.scroll.bar_width;

        let mut area = egui::ScrollArea::vertical()
            .id_salt(id)
            .auto_shrink([false, false])
            .max_height(max_h)
            .stick_to_bottom(auto_scroll && action == Action::None);
        if let Some(off) = pending_offset {
            area = area.vertical_scroll_offset(off);
        }

        let render = move |ui: &mut Ui| {
            area.show(ui, |ui| {
                // Reserve right space: explicit pad + scrollbar gutter so
                // bubbles never slide under the thumb.
                let right_reserve = content_right_pad + bar_w;
                ui.set_width(ui.available_width() - right_reserve);
                ui.spacing_mut().item_spacing.y = 12.0;

                let mut first_rect = None;
                let mut last_anchor_rect = None;
                for item in items {
                    let rect = ui.scope(item.content).response.rect;
                    if first_rect.is_none() {
                        first_rect = Some(rect);
                    }
                    if item.anchor {
                        last_anchor_rect = Some(rect);
                    }
                }

                apply_action(ui, action, first_rect, last_anchor_rect, peek);
            })
        };

        if let Some(h) = fixed_h {
            let w = ui.available_width();
            ui.allocate_ui_with_layout(Vec2::new(w, h), egui::Layout::top_down(Align::Min), |ui| {
                ui.set_min_height(h);
                render(ui)
            })
            .inner
        } else {
            render(ui)
        }
    }
}

/// Perform the chosen scroll [`Action`] once the rows have been measured.
fn apply_action(
    ui: &Ui,
    action: Action,
    first_rect: Option<egui::Rect>,
    last_anchor_rect: Option<egui::Rect>,
    peek: f32,
) {
    match action {
        Action::None => {}
        Action::End => ui.scroll_to_cursor(Some(Align::BOTTOM)),
        Action::Start => {
            if let Some(r) = first_rect {
                ui.scroll_to_rect(r, Some(Align::TOP));
            }
        }
        Action::AnchorTop => match last_anchor_rect {
            Some(r) => {
                // Place the anchor near the top, keeping `peek` of the previous
                // row visible above it.
                let target = egui::Rect::from_min_max(
                    egui::pos2(r.left(), r.top() - peek),
                    egui::pos2(r.right(), r.top() - peek + 1.0),
                );
                ui.scroll_to_rect(target, Some(Align::TOP));
            }
            None => ui.scroll_to_cursor(Some(Align::BOTTOM)),
        },
    }
}

/// Paint the floating jump-to-latest button in the viewport's bottom-right
/// corner. Returns `true` if it was clicked this frame.
fn jump_button(
    ui: &Ui,
    id: egui::Id,
    viewport: egui::Rect,
    tokens: Tokens,
    m: MessageScrollerMetrics,
) -> bool {
    let center = egui::pos2(
        viewport.right() - m.button_inset - m.button / 2.0,
        viewport.bottom() - m.button_inset - m.button / 2.0,
    );
    let rect = egui::Rect::from_center_size(center, Vec2::splat(m.button));
    let resp = ui.interact(rect, id.with("jump"), Sense::click());
    let fill = if resp.hovered() {
        tokens.primary.gamma_multiply(0.92)
    } else {
        tokens.primary
    };
    let painter = ui.painter();
    painter.circle_filled(center, m.button / 2.0, fill);
    painter.circle_stroke(center, m.button / 2.0, egui::Stroke::new(1.0, tokens.border));
    paint_chevron_down(painter, center, tokens.primary_foreground);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

/// Paint a downward chevron centred on `center` (the jump-to-latest glyph).
fn paint_chevron_down(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let half = 5.0;
    let dy = 2.5;
    let left = egui::pos2(center.x - half, center.y - dy);
    let tip = egui::pos2(center.x, center.y + dy);
    let right = egui::pos2(center.x + half, center.y - dy);
    let stroke = egui::Stroke::new(2.0, color);
    painter.line_segment([left, tip], stroke);
    painter.line_segment([tip, right], stroke);
}

/// Hash a row's id key into a stable `u64`.
fn hash_id(id: &impl Hash) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A transcript renders its rows and survives a second frame (where the
    /// open-position and detection state kick in) without panicking.
    #[test]
    fn renders_and_persists() {
        let ctx = egui::Context::default();
        let build = || {
            MessageScroller::new("t")
                .auto_scroll(true)
                .default_position(Position::LastAnchor)
                .item("a", false, |ui| {
                    ui.label("first");
                })
                .item("b", true, |ui| {
                    ui.label("turn");
                })
        };
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            build().show(ui);
        });
        // Second frame: `opened` is now true, exercising the anchor/edge logic.
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            build().show(ui);
        });
    }

    /// Stable ids hash deterministically; distinct ids differ.
    #[test]
    fn id_hashing_is_stable() {
        assert_eq!(hash_id(&"m1"), hash_id(&"m1"));
        assert_ne!(hash_id(&"m1"), hash_id(&"m2"));
    }
}
