//! [`Tooltip`] — a hover label, mirroring shadcn's `<Tooltip>`.
//!
//! shadcn's tooltip is a small `primary`-filled bubble with
//! `primary-foreground` text, `rounded-md`, `px-3 py-1.5 text-xs`, and a little
//! diamond arrow pointing at the trigger. It appears after a short hover and
//! fades in.
//!
//! Attach one to any [`Response`] with [`Tooltip::show`]:
//!
//! ```no_run
//! use glazier::tooltip::Tooltip;
//! use egui::Widget as _;
//! # egui::__run_test_ui(|ui| {
//! let resp = egui::Button::new("Hover me").ui(ui);
//! Tooltip::new("Add to library").show(ui, &resp);
//! # });
//! ```

use egui::{Align2, Area, Color32, Frame, Margin, Order, Pos2, Response, Stroke, Ui, Vec2};

use crate::customize::{Customize, StyleHook};
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry/timing for [`Tooltip`] — reach in via
/// [`Tooltip::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct TooltipMetrics {
    /// Tooltip text size (`text-xs`).
    pub text: f32,
    /// Gap between the trigger and the bubble (leaves room for the arrow).
    pub gap: f32,
    /// Arrow half-width at its base (a small downward triangle).
    pub arrow_half: f32,
    /// Arrow height from base to tip.
    pub arrow_h: f32,
    /// Hover delay before the tooltip appears, in seconds (shadcn's default
    /// ~700ms, trimmed for snappier feedback).
    pub delay: f32,
}

impl Default for TooltipMetrics {
    fn default() -> Self {
        Self {
            text: 12.0,
            gap: 8.0,
            arrow_half: 6.0,
            arrow_h: 5.0,
            delay: 0.4,
        }
    }
}

/// A hover tooltip bound to a trigger [`Response`].
#[must_use = "tooltips do nothing unless you show them"]
pub struct Tooltip {
    text: String,
    style_hook: StyleHook<Frame>,
    sizing_hook: SizingHook<TooltipMetrics>,
}

impl Customize<Frame> for Tooltip {
    fn style_hook_mut(&mut self) -> &mut StyleHook<Frame> {
        &mut self.style_hook
    }
}

impl Sizeable<TooltipMetrics> for Tooltip {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<TooltipMetrics> {
        &mut self.sizing_hook
    }
}

impl Tooltip {
    /// Create a tooltip with the given label.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style_hook: StyleHook::new(),
            sizing_hook: SizingHook::new(),
        }
    }

    /// Show the tooltip while `response` is hovered. It floats above the trigger
    /// (flipping below when there's no room) with a small arrow pointing at it,
    /// matching shadcn's dark bubble. Fades in after a short hover delay.
    pub fn show(mut self, ui: &Ui, response: &Response) {
        let ctx = ui.ctx();
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let id = response.id.with("glazier-tooltip");
        let now = ctx.input(|i| i.time);

        // Track when the current hover began (stored per-tooltip). Reset to
        // `None` whenever the pointer isn't over the trigger.
        let since = ctx.memory_mut(|m| {
            let slot = m.data.get_temp_mut_or::<Option<f64>>(id, None);
            if response.hovered() {
                if slot.is_none() {
                    *slot = Some(now);
                }
            } else {
                *slot = None;
            }
            *slot
        });

        // Visible once the hover has rested for `m.delay`. The animation eases
        // the bubble in and back out (when the hover ends `since` is `None`).
        let visible = since.is_some_and(|start| now - start >= f64::from(m.delay));

        // Schedule a frame for exactly when the delay elapses — otherwise, once
        // the pointer stops moving egui stops repainting and the tooltip is
        // starved before it's ever due. This MUST run before the early-out
        // below, or a still pointer never wakes the tooltip.
        if let Some(start) = since.filter(|_| !visible) {
            let remaining = (f64::from(m.delay) - (now - start)).max(0.0);
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(remaining));
        }

        let t = ctx.animate_bool_with_time(id, visible, 0.12);
        if t <= 0.0 {
            return;
        }

        let tokens = Tokens::get(ui);
        let anchor = response.rect;

        // Decide above vs. below by available room toward the screen top.
        let screen = ctx.content_rect();
        let below = anchor.top() - screen.top() < 64.0;

        let mut frame = Frame::new()
            .fill(tokens.primary)
            .stroke(Stroke::NONE)
            .corner_radius(tokens.radius_md())
            .inner_margin(Margin::symmetric(12, 6)) // px-3 py-1.5
            .shadow(egui::epaint::Shadow {
                offset: [0, 4],
                blur: 16,
                spread: 0,
                color: Color32::from_black_alpha(35),
            });
        std::mem::take(&mut self.style_hook).apply(&mut frame);

        // Anchor the bubble centered on the trigger, pushed off by the gap.
        let (pivot, pivot_pos) = if below {
            (
                Align2::CENTER_TOP,
                anchor.center_bottom() + Vec2::new(0.0, m.gap),
            )
        } else {
            (
                Align2::CENTER_BOTTOM,
                anchor.center_top() - Vec2::new(0.0, m.gap),
            )
        };

        let area = Area::new(id)
            .order(Order::Tooltip)
            .pivot(pivot)
            .fixed_pos(pivot_pos)
            .interactable(false);

        let inner = area
            .show(ctx, |ui| {
                ui.set_opacity(t);
                frame
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(&self.text)
                                .size(m.text)
                                .color(tokens.primary_foreground),
                        );
                    })
                    .response
                    .rect
            })
            .inner;

        // Paint the arrow as a small triangle bridging the bubble and trigger.
        paint_arrow(ctx, id, inner, anchor.center().x, below, tokens.primary, t, m);
    }
}

/// Paint the tooltip's diamond arrow: a small triangle whose base sits on the
/// bubble edge facing the trigger and whose tip points at it.
#[allow(clippy::too_many_arguments)]
fn paint_arrow(
    ctx: &egui::Context,
    id: egui::Id,
    bubble: egui::Rect,
    center_x: f32,
    below: bool,
    fill: Color32,
    opacity: f32,
    m: TooltipMetrics,
) {
    use egui::epaint::Shape;

    let x = center_x.clamp(
        bubble.left() + m.arrow_half + 4.0,
        bubble.right() - m.arrow_half - 4.0,
    );
    let (base_y, tip_y) = if below {
        (bubble.top(), bubble.top() - m.arrow_h)
    } else {
        (bubble.bottom(), bubble.bottom() + m.arrow_h)
    };
    let pts = vec![
        Pos2::new(x - m.arrow_half, base_y),
        Pos2::new(x + m.arrow_half, base_y),
        Pos2::new(x, tip_y),
    ];

    let painter = ctx.layer_painter(egui::LayerId::new(Order::Tooltip, id.with("arrow")));
    let color = fill.gamma_multiply(opacity);
    painter.add(Shape::convex_polygon(pts, color, Stroke::NONE));
}
