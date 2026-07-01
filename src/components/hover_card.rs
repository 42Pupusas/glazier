//! [`HoverCard`] — a rich hover popover, mirroring shadcn's `<HoverCard>`.
//!
//! Where a [tooltip](crate::tooltip::Tooltip) is a tiny, non-interactive label,
//! a hover card is a full popover-surface panel (`rounded-2xl`, `p-4`, a
//! hairline ring and a soft shadow) hosting arbitrary content — an avatar, a
//! bio, stats. It opens after a short hover on the trigger and, crucially,
//! **stays open while the pointer is over the card itself**, dismissing only a
//! beat after the pointer leaves both. That lets the user move onto the card to
//! read or interact with it (shadcn's classic "@username" preview).
//!
//! Drive it from any trigger [`Response`]:
//!
//! ```no_run
//! use glazier::hover_card::HoverCard;
//! use egui::Widget as _;
//! # egui::__run_test_ui(|ui| {
//! let trigger = egui::Button::new("@peduarte").ui(ui);
//! HoverCard::new().width(300.0).show(ui, &trigger, |ui| {
//!     ui.label("Pedro Duarte");
//!     ui.label("Created shadcn/ui. Building tools for developers.");
//! });
//! # });
//! ```

use egui::{Area, Frame, Margin, Order, Response, Stroke, Ui, Vec2};

use crate::customize::{Customize, StyleHook};
use crate::tokens::Tokens;

/// Hover delay before the card appears, in seconds (shadcn's default ~700ms,
/// trimmed a touch for snappier feedback).
const OPEN_DELAY: f32 = 0.5;
/// Grace period after the pointer leaves both trigger and card before it
/// dismisses — long enough to travel from the trigger onto the card.
const CLOSE_DELAY: f32 = 0.25;
/// Gap between the trigger and the card.
const GAP: f32 = 8.0;
/// Default card width (shadcn's `w-80` ≈ 320px, trimmed for egui density).
const DEFAULT_WIDTH: f32 = 300.0;

/// Per-trigger open/timing state, persisted in egui memory.
#[derive(Clone, Copy, Default)]
struct HoverState {
    /// Whether the card is currently shown.
    open: bool,
    /// When the *pending* transition began: a rest before opening (while
    /// hovering, closed) or before closing (while away, open). `None` once the
    /// transition has settled or been cancelled.
    pending_since: Option<f64>,
    /// Was the card hovered last frame? (Fed back in to keep it open.)
    card_hovered: bool,
}

/// A rich hover-triggered popover panel.
#[must_use = "hover cards do nothing unless you show them"]
pub struct HoverCard {
    width: Option<f32>,
    open_delay: f32,
    close_delay: f32,
    gap: f32,
    style_hook: StyleHook<Frame>,
}

impl Customize<Frame> for HoverCard {
    fn style_hook_mut(&mut self) -> &mut StyleHook<Frame> {
        &mut self.style_hook
    }
}

impl Default for HoverCard {
    fn default() -> Self {
        Self::new()
    }
}

impl HoverCard {
    /// Create a hover card with shadcn's default sizing and timing.
    pub const fn new() -> Self {
        Self {
            width: None,
            open_delay: OPEN_DELAY,
            close_delay: CLOSE_DELAY,
            gap: GAP,
            style_hook: StyleHook::new(),
        }
    }

    /// Set an explicit content width (shadcn's default is `w-80`).
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Override the hover delay before the card opens (seconds).
    pub const fn open_delay(mut self, secs: f32) -> Self {
        self.open_delay = secs;
        self
    }

    /// Override the grace period before the card closes (seconds).
    pub const fn close_delay(mut self, secs: f32) -> Self {
        self.close_delay = secs;
        self
    }

    /// Set the gap between the trigger and the card.
    pub const fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Track `response`'s hover and, once it rests, float `content` in a
    /// popover card below the trigger (flipping above when there's no room).
    /// The card stays open while the pointer is over it, dismissing a beat
    /// after the pointer leaves both.
    pub fn show<R>(
        mut self,
        ui: &Ui,
        response: &Response,
        content: impl FnOnce(&mut Ui) -> R,
    ) -> Option<R> {
        let ctx = ui.ctx();
        let id = response.id.with("glazier-hover-card");
        let now = ctx.input(|i| i.time);

        let mut state: HoverState = ctx.memory(|m| m.data.get_temp(id).unwrap_or_default());

        // "Active" = pointer over the trigger or over the card (last frame).
        let active = response.hovered() || state.card_hovered;
        let delay = f64::from(if state.open {
            self.close_delay
        } else {
            self.open_delay
        });

        // Drive the little open/close state machine. While `active` matches the
        // direction we'd transition toward, run a timer; when it elapses, flip.
        let wants_change = active != state.open;
        if wants_change {
            let start = state.pending_since.get_or_insert(now);
            if now - *start >= delay {
                state.open = active;
                state.pending_since = None;
            } else {
                // Wake exactly when the delay is due — a still pointer stops
                // generating events, which would otherwise starve the timer.
                let remaining = (delay - (now - *start)).max(0.0);
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(remaining));
            }
        } else {
            state.pending_since = None;
        }

        let t = ctx.animate_bool_with_time(id, state.open, 0.12);
        if t <= 0.0 {
            // Fully closed: clear hover memory and persist.
            state.card_hovered = false;
            ctx.memory_mut(|m| m.data.insert_temp(id, state));
            return None;
        }

        let tokens = Tokens::get(ui);
        let anchor = response.rect;
        let width = self.width.unwrap_or(DEFAULT_WIDTH);

        // Prefer opening below; flip above when the trigger sits near the
        // screen bottom and there's more room up top.
        let screen = ctx.content_rect();
        let below =
            screen.bottom() - anchor.bottom() > 160.0 || anchor.top() - screen.top() < 160.0;
        let (pivot, pivot_pos) = if below {
            (
                egui::Align2::LEFT_TOP,
                anchor.left_bottom() + Vec2::new(0.0, self.gap),
            )
        } else {
            (
                egui::Align2::LEFT_BOTTOM,
                anchor.left_top() - Vec2::new(0.0, self.gap),
            )
        };

        let mut frame = Frame::new()
            .fill(tokens.background)
            .stroke(Stroke::new(1.0, tokens.border))
            .corner_radius(tokens.radius_2xl())
            .inner_margin(Margin::same(16)) // p-4
            .shadow(egui::epaint::Shadow {
                offset: [0, 8],
                blur: 32,
                spread: 0,
                color: egui::Color32::from_black_alpha(45),
            });
        std::mem::take(&mut self.style_hook).apply(&mut frame);

        let area = Area::new(id)
            .order(Order::Foreground)
            .pivot(pivot)
            .fixed_pos(pivot_pos)
            .interactable(true);

        let inner = area.show(ctx, |ui| {
            ui.set_opacity(t);
            ui.set_width(width);
            frame.show(ui, content).inner
        });

        // Did the pointer land on the card this frame? Keep it open if so.
        state.card_hovered = ctx
            .input(|i| i.pointer.hover_pos())
            .is_some_and(|p| inner.response.rect.contains(p));
        ctx.memory_mut(|m| m.data.insert_temp(id, state));

        Some(inner.inner)
    }
}
