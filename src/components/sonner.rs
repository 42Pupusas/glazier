//! [`Toast`] + [`Toaster`] — Sonner-style stacked notifications.
//!
//! A faithful port of shadcn's Sonner: toasts pile up in a screen corner as a
//! *collapsed stack* — only the newest is fully visible, the ones behind it peek
//! out and shrink with depth, like a deck of cards. Hovering the stack
//! **expands** it: every toast animates to full size with real gaps between
//! them, and their auto-dismiss timers pause until the pointer leaves.
//!
//! The system has two halves that share state through egui's per-`Context` data
//! store, so they need no common object:
//!
//! - **[`Toast`]** describes one notification (title, optional description,
//!   [`variant`](Toast::variant), [`duration`](Toast::duration)). Call
//!   [`send`](Toast::send) to enqueue it from anywhere you have a
//!   [`Context`](egui::Context).
//! - **[`Toaster`]** drains and renders the queue. Call [`show`](Toaster::show)
//!   **once per frame**; it owns the stack layout, the collapse/expand
//!   animation, enter/exit tweens, timer pausing, and the close buttons.
//!
//! ```no_run
//! use glazier::sonner::{Toast, Toaster};
//! # let ctx = egui::Context::default();
//! // …in response to an action:
//! Toast::new("Event created").description("Sunday, December 03").send(&ctx);
//! Toast::new("Could not save").error().send(&ctx);
//!
//! // …once per frame, after your UI:
//! Toaster::new().show(&ctx);
//! ```

use std::time::Instant;

use egui::emath::TSTransform;
use egui::{Align2, Area, Color32, Frame, Margin, Order, Sense, Stroke, Ui, Vec2, Widget};

use crate::components::icon::Icon;
use crate::fonts;
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry/timing for [`Toaster`] (and [`Toast`]'s default
/// duration) — reach in via [`Toaster::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct SonnerMetrics {
    /// Default seconds a toast stays before auto-dismissing (Sonner uses 4s).
    pub default_duration: f32,
    /// Enter / exit tween duration.
    pub anim_secs: f32,
    /// Collapse ⇆ expand tween duration.
    pub expand_secs: f32,
    /// Toast card width (Sonner default 356px).
    pub width: f32,
    /// Gap between cards when the stack is expanded.
    pub gap: f32,
    /// How far each deeper card peeks out when the stack is collapsed.
    pub peek: f32,
    /// Scale shed per depth level when collapsed (so card 1 is 0.94×, etc.).
    pub scale_step: f32,
    /// Inset from the screen corner.
    pub screen_pad: f32,
    /// How many toasts render at full presence before the rest fade behind.
    pub max_visible: usize,
}

impl Default for SonnerMetrics {
    fn default() -> Self {
        Self {
            default_duration: 4.0,
            anim_secs: 0.35,
            expand_secs: 0.22,
            width: 356.0,
            gap: 14.0,
            peek: 16.0,
            scale_step: 0.05,
            screen_pad: 24.0,
            max_visible: 3,
        }
    }
}

/// Semantic style of a [`Toast`], mirroring Sonner's variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    /// Neutral surface — plain feedback, no leading icon.
    #[default]
    Default,
    /// Success (green check).
    Success,
    /// Error / destructive (red).
    Error,
    /// Warning (amber).
    Warning,
    /// Informational (accent).
    Info,
}

/// Which screen edge/corner the stack anchors to — the six Sonner positions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Corner {
    /// Top-left.
    TopLeft,
    /// Top-center.
    TopCenter,
    /// Top-right.
    TopRight,
    /// Bottom-left.
    BottomLeft,
    /// Bottom-center.
    BottomCenter,
    /// Bottom-right (Sonner's default).
    BottomRight,
}

/// Every [`Corner`], iterated each frame so all stacks render independently.
const ALL_CORNERS: [Corner; 6] = [
    Corner::TopLeft,
    Corner::TopCenter,
    Corner::TopRight,
    Corner::BottomLeft,
    Corner::BottomCenter,
    Corner::BottomRight,
];

impl Corner {
    /// Whether this position is along the top edge (stack grows downward).
    const fn is_top(self) -> bool {
        matches!(self, Self::TopLeft | Self::TopCenter | Self::TopRight)
    }

    /// Horizontal direction the card is pushed *inward* from its anchor edge:
    /// `+1` left-anchored, `-1` right-anchored, `0` centered.
    const fn h_dir(self) -> f32 {
        match self {
            Self::TopLeft | Self::BottomLeft => 1.0,
            Self::TopCenter | Self::BottomCenter => 0.0,
            Self::TopRight | Self::BottomRight => -1.0,
        }
    }

    /// The [`Align2`] used to anchor a card's area to this screen position.
    const fn align(self) -> Align2 {
        match self {
            Self::TopLeft => Align2::LEFT_TOP,
            Self::TopCenter => Align2::CENTER_TOP,
            Self::TopRight => Align2::RIGHT_TOP,
            Self::BottomLeft => Align2::LEFT_BOTTOM,
            Self::BottomCenter => Align2::CENTER_BOTTOM,
            Self::BottomRight => Align2::RIGHT_BOTTOM,
        }
    }
}

/// One queued notification. Build it, then [`send`](Self::send) it.
#[must_use = "build a toast then call .send(ctx) to enqueue it"]
pub struct Toast {
    title: String,
    description: Option<String>,
    variant: Variant,
    duration: f32,
    corner: Option<Corner>,
}

impl Toast {
    /// Create a toast with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            variant: Variant::Default,
            duration: SonnerMetrics::default().default_duration,
            corner: None,
        }
    }

    /// Add a second muted line beneath the title.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the [`Variant`].
    pub const fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Shorthand for [`Variant::Success`].
    pub const fn success(self) -> Self {
        self.variant(Variant::Success)
    }

    /// Shorthand for [`Variant::Error`].
    pub const fn error(self) -> Self {
        self.variant(Variant::Error)
    }

    /// Shorthand for [`Variant::Warning`].
    pub const fn warning(self) -> Self {
        self.variant(Variant::Warning)
    }

    /// Shorthand for [`Variant::Info`].
    pub const fn info(self) -> Self {
        self.variant(Variant::Info)
    }

    /// Set how long (seconds) the toast stays before auto-dismissing.
    pub const fn duration(mut self, seconds: f32) -> Self {
        self.duration = seconds;
        self
    }

    /// Pin this toast to a screen [`Corner`], overriding the [`Toaster`]'s
    /// default. The toast keeps this position for its whole life, even if the
    /// toaster's default changes later.
    pub const fn position(mut self, corner: Corner) -> Self {
        self.corner = Some(corner);
        self
    }

    /// Enqueue this toast into `ctx`'s shared store. It appears the next time a
    /// [`Toaster`] renders.
    pub fn send(self, ctx: &egui::Context) {
        // Resolve the corner now and freeze it onto the toast: explicit position
        // wins, else the last toaster's default, else bottom-right.
        let corner = self.corner.unwrap_or_else(|| {
            ctx.data(|d| {
                d.get_temp(default_corner_id())
                    .unwrap_or(Corner::BottomRight)
            })
        });
        let live = LiveToast {
            id: ctx.data_mut(|d| {
                let n: u64 = d.get_temp(seq_id()).unwrap_or(0);
                d.insert_temp(seq_id(), n + 1);
                n
            }),
            title: self.title,
            description: self.description,
            variant: self.variant,
            remaining: self.duration,
            born: Instant::now(),
            dismissed: None,
            corner,
        };
        ctx.data_mut(|d| {
            let queue: &mut Vec<LiveToast> = d.get_temp_mut_or_default(queue_id());
            queue.push(live);
        });
        ctx.request_repaint();
    }
}

/// A live toast in the queue: a [`Toast`]'s content plus runtime bookkeeping.
#[derive(Clone)]
struct LiveToast {
    id: u64,
    title: String,
    description: Option<String>,
    variant: Variant,
    /// Seconds left before auto-dismiss; frozen while the stack is hovered.
    remaining: f32,
    /// When the toast was created — drives the enter tween (never paused).
    born: Instant,
    /// Set the instant the toast began dismissing (timeout or close click).
    dismissed: Option<Instant>,
    /// The screen position this toast is anchored to (frozen at send time).
    corner: Corner,
}

/// The renderer for the toast queue. Construct, optionally pick a [`corner`],
/// and [`show`](Self::show) it once per frame.
#[must_use = "toasters do nothing unless shown"]
pub struct Toaster {
    corner: Corner,
    sizing_hook: SizingHook<SonnerMetrics>,
}

impl Default for Toaster {
    fn default() -> Self {
        Self {
            corner: Corner::BottomRight,
            sizing_hook: SizingHook::new(),
        }
    }
}

impl Sizeable<SonnerMetrics> for Toaster {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<SonnerMetrics> {
        &mut self.sizing_hook
    }
}

impl Toaster {
    /// Create a toaster anchored bottom-right (Sonner's default).
    pub fn new() -> Self {
        Self::default()
    }

    /// Anchor the stack to a screen [`Corner`].
    pub const fn corner(mut self, corner: Corner) -> Self {
        self.corner = corner;
        self
    }

    /// Render the queue once: collapse/expand, animate, auto-dismiss, draw close
    /// buttons. Call once per frame, after the rest of your UI.
    pub fn show(mut self, ctx: &egui::Context) {
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        // Publish this toaster's default corner so `Toast::send` (which has no
        // toaster) can freeze it onto new toasts.
        ctx.data_mut(|d| d.insert_temp(default_corner_id(), self.corner));

        let mut queue: Vec<LiveToast> =
            ctx.data_mut(|d| d.get_temp(queue_id()).unwrap_or_default());
        if queue.is_empty() {
            return;
        }

        let tokens = Tokens::from_visuals(&ctx.global_style().visuals);
        let now = Instant::now();

        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        let mut new_heights: std::collections::HashMap<u64, f32> =
            ctx.data_mut(|d| d.get_temp(heights_id()).unwrap_or_default());
        let mut finished: Vec<u64> = Vec::new();
        let mut closed_ids: Vec<u64> = Vec::new();

        // Each corner is an independent stack: its own hover, expand animation,
        // and layout. A toast lives in exactly the corner it was sent to, so
        // changing the toaster's default never moves existing toasts.
        for corner in ALL_CORNERS {
            // Indices into `queue` for this corner, oldest..newest (send order).
            let members: Vec<usize> = queue
                .iter()
                .enumerate()
                .filter(|(_, t)| t.corner == corner)
                .map(|(i, _)| i)
                .collect();
            if members.is_empty() {
                continue;
            }

            // Per-corner hover → expand factor (smooth, also gates the timers).
            let hovered = ctx.data_mut(|d| d.get_temp::<bool>(hover_id(corner)).unwrap_or(false));
            let expand = ctx.animate_bool_with_time(expand_anim_id(corner), hovered, m.expand_secs);

            // Advance auto-dismiss timers, paused while expanded (hovered).
            if expand < 0.5 {
                for &i in &members {
                    let t = &mut queue[i];
                    if t.dismissed.is_none() {
                        t.remaining -= dt;
                        if t.remaining <= 0.0 {
                            t.dismissed = Some(now);
                        }
                    }
                }
            }

            // Depth 0 = newest = front (nearest the corner) = last member.
            let count = members.len();
            let mut expanded_lift = vec![0.0_f32; count];
            let mut acc = 0.0_f32;
            for depth in 0..count {
                expanded_lift[depth] = acc;
                let id = queue[members[count - 1 - depth]].id;
                let h = new_heights.get(&id).copied().unwrap_or(64.0);
                acc += h + m.gap;
            }
            let stack_extent = acc;

            // Render back-to-front (deepest first) so the front card is on top.
            for depth in (0..count).rev() {
                let t = &queue[members[count - 1 - depth]];
                let geo = CardGeom {
                    corner,
                    depth,
                    expand,
                    expanded_lift: expanded_lift[depth],
                    now,
                };
                let out = render_card(ctx, tokens, m, t, geo);
                new_heights.insert(t.id, out.height);
                if out.closed {
                    closed_ids.push(t.id);
                }
                if out.finished {
                    finished.push(t.id);
                }
            }

            // Hover detection uses ONE fixed bounding box sized to the fully
            // *expanded* stack, not per-card hit-testing. Per-card areas leave
            // the inter-card gaps unhittable; when expanded the pointer falls
            // into a gap, the stack collapses, slides back under the pointer and
            // re-expands — a frame-rate jitter. A stable region sized to the
            // larger (expanded) extent makes hover monotonic, so it can't
            // oscillate.
            let any_hovered = ctx
                .pointer_hover_pos()
                .is_some_and(|p| stack_bounds(ctx, m, corner, stack_extent).contains(p));
            ctx.data_mut(|d| d.insert_temp(hover_id(corner), any_hovered));
        }

        // Begin the exit tween for any toast whose close button was clicked.
        for t in &mut queue {
            if t.dismissed.is_none() && closed_ids.contains(&t.id) {
                t.dismissed = Some(now);
            }
        }

        // Drop fully-faded toasts; persist the rest and the per-frame state.
        queue.retain(|t| !finished.contains(&t.id));
        ctx.data_mut(|d| {
            d.insert_temp(queue_id(), queue);
            d.insert_temp(heights_id(), new_heights);
        });
        ctx.request_repaint();
    }
}

/// Layout inputs for one card at a given stack depth.
#[derive(Clone, Copy)]
struct CardGeom {
    corner: Corner,
    /// 0 = front (newest), increasing toward the back.
    depth: usize,
    /// Collapse→expand factor, 0 (stacked) … 1 (fanned out).
    expand: f32,
    /// The card's offset from the corner when fully expanded.
    expanded_lift: f32,
    now: Instant,
}

/// What a rendered card reports back to the [`Toaster`].
struct CardOut {
    /// Measured card height (for next frame's expanded layout).
    height: f32,
    /// Whether the close (×) button was clicked this frame.
    closed: bool,
    /// Whether the exit tween has finished (safe to remove).
    finished: bool,
}

/// Render one toast card with its stack transform and enter/exit tween.
fn render_card(
    ctx: &egui::Context,
    tokens: Tokens,
    m: SonnerMetrics,
    t: &LiveToast,
    geo: CardGeom,
) -> CardOut {
    let CardGeom {
        corner,
        depth,
        expand,
        expanded_lift,
        now,
    } = geo;

    // Enter (0→1) is born-driven; exit (1→0) starts at `dismissed`.
    let enter = (now.duration_since(t.born).as_secs_f32() / m.anim_secs).clamp(0.0, 1.0);
    let (life, finished) = t.dismissed.map_or((enter, false), |d| {
        let exit = (now.duration_since(d).as_secs_f32() / m.anim_secs).clamp(0.0, 1.0);
        (1.0 - exit, exit >= 1.0)
    });
    let life_eased = ease_out_cubic(life);

    // Collapsed vs expanded geometry, blended by `expand`.
    #[allow(clippy::cast_precision_loss)]
    let d = depth as f32;
    let collapsed_lift = d * m.peek;
    let stack_lift = lerp(collapsed_lift, expanded_lift, expand);
    let collapsed_scale = m.scale_step.mul_add(-d, 1.0).max(0.5);
    let scale = lerp(collapsed_scale, 1.0, expand);

    // Cards beyond the visible count fade out behind the stack when collapsed.
    #[allow(clippy::cast_precision_loss)]
    let visible_edge = (m.max_visible - 1) as f32;
    let depth_alpha = if depth < m.max_visible {
        1.0
    } else {
        (d - visible_edge).mul_add(-0.5, 1.0).max(0.0)
    };
    let depth_alpha = lerp(depth_alpha, 1.0, expand);
    let alpha = life_eased * depth_alpha;

    // Anchor the card's corner inside the screen position, lifted by `stack_lift`.
    let sign = if corner.is_top() { 1.0 } else { -1.0 };
    let hx = corner.h_dir();
    let offset = Vec2::new(hx * m.screen_pad, sign * (m.screen_pad + stack_lift));

    // The card's anchored outer corner, in screen space — the pivot for scaling
    // (bottom-/top-center so cards shrink toward the stack's centre line).
    let corner_pt = corner.align().pos_in_rect(&ctx.content_rect()) + offset;
    let pivot_x = (hx * m.width).mul_add(0.5, corner_pt.x);
    let pivot = egui::pos2(pivot_x, corner_pt.y);

    // Enter slide: cards arrive from just outside the anchored edge.
    let slide_y = (1.0 - life_eased) * 16.0 * -sign;

    // Compose: scale about `pivot`, then apply the enter slide.
    let translation = pivot.to_vec2() * (1.0 - scale) + Vec2::new(0.0, slide_y);
    let transform = TSTransform::new(translation, scale);

    let area_id = egui::Id::new(("glazier-sonner", t.id));
    let mut closed = false;
    let response = Area::new(area_id)
        .order(Order::Foreground)
        .anchor(corner.align(), offset)
        .interactable(true)
        .show(ctx, |ui| {
            ui.set_opacity(alpha);
            ui.set_width(m.width);
            ui.with_visual_transform(transform, |ui| {
                closed = card_body(ui, tokens, m, t);
            });
        });

    CardOut {
        height: response.response.rect.height(),
        closed,
        finished,
    }
}

/// The fixed hover region for the whole stack, sized to the *expanded* extent.
///
/// Anchored to the screen position like the cards themselves, padded slightly so
/// the region is forgiving at the edges. Using the larger expanded height (never
/// the collapsed one) keeps the hover test independent of the animation state,
/// which is what stops the expand/collapse oscillation.
fn stack_bounds(ctx: &egui::Context, m: SonnerMetrics, corner: Corner, extent: f32) -> egui::Rect {
    let screen = ctx.content_rect();
    let height = extent.max(72.0) + m.screen_pad;
    let width = m.width + m.screen_pad;
    let hx = corner.h_dir();
    let left = if hx > 0.0 {
        screen.left()
    } else if hx < 0.0 {
        screen.right() - width
    } else {
        width.mul_add(-0.5, screen.center().x)
    };
    let (top, bottom) = if corner.is_top() {
        (screen.top(), screen.top() + height)
    } else {
        (screen.bottom() - height, screen.bottom())
    };
    egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(left + width, bottom))
}

/// Paint the toast surface and its contents; returns whether × was clicked.
///
/// Uses the opaque `widget` (popover) token — a toast is a floating overlay,
/// not a static container, and `background` may be translucent under a user
/// theme.
fn card_body(ui: &mut Ui, tokens: Tokens, m: SonnerMetrics, t: &LiveToast) -> bool {
    let frame = Frame::new()
        .fill(tokens.widget)
        .stroke(Stroke::new(1.0, tokens.border))
        .corner_radius(tokens.radius_lg())
        .inner_margin(Margin::symmetric(16, 15))
        .shadow(egui::epaint::Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: Color32::from_black_alpha(45),
        });

    let mut clicked = false;
    frame.show(ui, |ui| {
        ui.horizontal_top(|ui| {
            // Leading variant icon (default has none).
            if let Some((svg, color)) = variant_icon(tokens, t.variant) {
                ui.add_space(1.0);
                Icon::new(svg).size(18.0).color(color).ui(ui);
                ui.add_space(8.0);
            }

            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.set_width(m.width - 96.0);
                ui.label(
                    egui::RichText::new(&t.title)
                        .font(fonts::semibold(ui, 13.5))
                        .color(tokens.foreground),
                );
                if let Some(desc) = &t.description {
                    ui.label(
                        egui::RichText::new(desc)
                            .size(12.5)
                            .color(tokens.muted_foreground),
                    );
                }
            });

            // Close button hugs the right edge.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                clicked = close_button(ui, tokens);
            });
        });
    });
    clicked
}

/// A small × close button that brightens on hover.
fn close_button(ui: &mut Ui, tokens: Tokens) -> bool {
    let (id, rect) = ui.allocate_space(Vec2::splat(18.0));
    let resp = ui
        .interact(rect, id, Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let color = if resp.hovered() {
        tokens.foreground
    } else {
        tokens.muted_foreground
    };
    Icon::new(X).color(color).image(tokens).paint_at(
        ui,
        egui::Rect::from_center_size(rect.center(), Vec2::splat(13.0)),
    );
    resp.clicked()
}

/// The leading glyph + colour for a variant, or `None` for [`Variant::Default`].
const fn variant_icon(tokens: Tokens, variant: Variant) -> Option<(&'static str, Color32)> {
    match variant {
        Variant::Default => None,
        Variant::Success => Some((CHECK_CIRCLE, Color32::from_rgb(0x22, 0xc5, 0x5e))),
        Variant::Error => Some((X_CIRCLE, tokens.destructive)),
        Variant::Warning => Some((TRIANGLE_ALERT, Color32::from_rgb(0xf5, 0x9e, 0x0b))),
        Variant::Info => Some((INFO, tokens.primary)),
    }
}

/// Linear interpolation.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    (b - a).mul_add(t, a)
}

/// `ease-out` cubic.
fn ease_out_cubic(t: f32) -> f32 {
    let u = 1.0 - t;
    (u * u).mul_add(-u, 1.0)
}

/// The id of the shared toast queue in `ctx` data.
fn queue_id() -> egui::Id {
    egui::Id::new("glazier-sonner-queue")
}

/// The id of the monotonic toast sequence counter.
fn seq_id() -> egui::Id {
    egui::Id::new("glazier-sonner-seq")
}

/// The id of one corner's last-frame "stack is hovered" flag.
fn hover_id(corner: Corner) -> egui::Id {
    egui::Id::new(("glazier-sonner-hover", corner))
}

/// The id driving one corner's collapse⇆expand animation.
fn expand_anim_id(corner: Corner) -> egui::Id {
    egui::Id::new(("glazier-sonner-expand", corner))
}

/// The id of the toaster's published default corner (read by `Toast::send`).
fn default_corner_id() -> egui::Id {
    egui::Id::new("glazier-sonner-default-corner")
}

/// The id of the cached per-toast measured heights.
fn heights_id() -> egui::Id {
    egui::Id::new("glazier-sonner-heights")
}

/// The close (×) glyph — lucide `x`.
const X: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>"#;
/// lucide `circle-check`.
const CHECK_CIRCLE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/></svg>"#;
/// lucide `circle-x`.
const X_CIRCLE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/></svg>"#;
/// lucide `triangle-alert`.
const TRIANGLE_ALERT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><path d="M12 9v4"/><path d="M12 17h.01"/></svg>"#;
/// lucide `info`.
const INFO: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/></svg>"#;

#[cfg(test)]
mod tests {
    use super::*;

    /// Sending enqueues into the shared store.
    #[test]
    fn send_enqueues() {
        let ctx = egui::Context::default();
        Toast::new("Hi").send(&ctx);
        let len: usize = ctx.data(|d| {
            d.get_temp::<Vec<LiveToast>>(queue_id())
                .map_or(0, |q| q.len())
        });
        assert_eq!(len, 1);
    }

    /// An explicit `.position()` is frozen onto the toast and survives a later
    /// toaster default; toasts at different corners keep their own positions.
    #[test]
    fn position_is_frozen_per_toast() {
        let ctx = egui::Context::default();
        Toast::new("a").position(Corner::TopLeft).send(&ctx);
        Toast::new("b").position(Corner::BottomCenter).send(&ctx);
        let corners: Vec<Corner> = ctx.data(|d| {
            d.get_temp::<Vec<LiveToast>>(queue_id())
                .unwrap()
                .iter()
                .map(|t| t.corner)
                .collect()
        });
        assert_eq!(corners, vec![Corner::TopLeft, Corner::BottomCenter]);
    }

    /// Without an explicit position, a toast inherits the toaster's published
    /// default corner (not whatever a later toaster sets).
    #[test]
    fn position_inherits_published_default() {
        let ctx = egui::Context::default();
        ctx.data_mut(|d| d.insert_temp(default_corner_id(), Corner::TopRight));
        Toast::new("x").send(&ctx);
        let corner = ctx.data(|d| d.get_temp::<Vec<LiveToast>>(queue_id()).unwrap()[0].corner);
        assert_eq!(corner, Corner::TopRight);
    }

    /// An empty queue makes `show` a no-op (no panic, nothing added).
    #[test]
    fn empty_show_is_noop() {
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |_| {});
        Toaster::new().show(&ctx);
        let len: usize = ctx.data(|d| {
            d.get_temp::<Vec<LiveToast>>(queue_id())
                .map_or(0, |q| q.len())
        });
        assert_eq!(len, 0);
    }

    /// Each position reports its edge + inward direction correctly.
    #[test]
    fn corner_edges() {
        assert!(Corner::TopLeft.is_top());
        assert!(!Corner::BottomRight.is_top());
        assert!((Corner::TopLeft.h_dir() - 1.0).abs() < f32::EPSILON);
        assert!(Corner::TopCenter.h_dir().abs() < f32::EPSILON);
        assert!((Corner::BottomRight.h_dir() + 1.0).abs() < f32::EPSILON);
    }
}
