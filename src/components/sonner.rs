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
use egui::{
    Align, Align2, Area, Color32, Frame, Margin, Order, Rect, Sense, Stroke, Ui, Vec2, Widget,
};

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

impl Corner {
    /// Every corner, iterated each frame so all stacks render independently.
    const ALL: [Self; 6] = [
        Self::TopLeft,
        Self::TopCenter,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomCenter,
        Self::BottomRight,
    ];

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
        let x = match self {
            Self::TopLeft | Self::BottomLeft => Align::Min,
            Self::TopCenter | Self::BottomCenter => Align::Center,
            Self::TopRight | Self::BottomRight => Align::Max,
        };
        let y = if self.is_top() {
            Align::Min
        } else {
            Align::Max
        };
        Align2([x, y])
    }

    /// The fixed hover region for this corner's stack, sized to the
    /// *expanded* extent and padded so it is forgiving at the edges.
    ///
    /// Per-card hit-testing leaves the gaps between expanded cards unhittable:
    /// the pointer falls into a gap, the stack collapses, slides back under
    /// the pointer and re-expands, jittering every frame. One region sized to
    /// the larger extent keeps hover independent of the animation.
    fn stack_bounds(self, screen: Rect, m: SonnerMetrics, extent: f32) -> Rect {
        let size = Vec2::new(m.width, extent.max(72.0)) + Vec2::splat(m.screen_pad);
        self.align().align_size_within_rect(size, screen)
    }

    /// The id of this corner's last-frame "stack is hovered" flag.
    fn hover_id(self) -> egui::Id {
        egui::Id::unique(("glazier-sonner-hover", self))
    }

    /// The id driving this corner's collapse⇆expand animation.
    fn expand_id(self) -> egui::Id {
        egui::Id::unique(("glazier-sonner-expand", self))
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
                d.get_temp(Ids::default_corner())
                    .unwrap_or(Corner::BottomRight)
            })
        });
        let live = LiveToast {
            id: ctx.data_mut(|d| {
                let n: u64 = d.get_temp(Ids::seq()).unwrap_or(0);
                d.insert_temp(Ids::seq(), n + 1);
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
            let queue: &mut Vec<LiveToast> = d.get_temp_mut_or_default(Ids::queue());
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

impl LiveToast {
    /// Count down the auto-dismiss timer; start the exit once it runs out.
    fn tick(&mut self, dt: f32, now: Instant) {
        if self.dismissed.is_none() {
            self.remaining -= dt;
            if self.remaining <= 0.0 {
                self.dismissed = Some(now);
            }
        }
    }

    /// How far `since` is into a tween of `m.anim_secs`, 0 to 1.
    fn progress(since: Instant, now: Instant, m: SonnerMetrics) -> f32 {
        (now.duration_since(since).as_secs_f32() / m.anim_secs).clamp(0.0, 1.0)
    }

    /// Presence, 0 to 1: entering from birth, then exiting once dismissed.
    fn life(&self, now: Instant, m: SonnerMetrics) -> f32 {
        self.dismissed.map_or_else(
            || Self::progress(self.born, now, m),
            |d| 1.0 - Self::progress(d, now, m),
        )
    }

    /// The exit tween has run its course; the toast can be dropped.
    fn finished(&self, now: Instant, m: SonnerMetrics) -> bool {
        self.dismissed
            .is_some_and(|d| Self::progress(d, now, m) >= 1.0)
    }

    /// The leading glyph and colour, or `None` for [`Variant::Default`].
    const fn icon(&self, tokens: Tokens) -> Option<(&'static str, Color32)> {
        match self.variant {
            Variant::Default => None,
            Variant::Success => Some((CHECK_CIRCLE, Color32::from_rgb(0x22, 0xc5, 0x5e))),
            Variant::Error => Some((X_CIRCLE, tokens.destructive)),
            Variant::Warning => Some((TRIANGLE_ALERT, Color32::from_rgb(0xf5, 0x9e, 0x0b))),
            Variant::Info => Some((INFO, tokens.primary)),
        }
    }

    /// Paint the toast surface and its contents; returns whether × was
    /// clicked.
    ///
    /// Uses the opaque `widget` (popover) token — a toast is a floating
    /// overlay, and `background` may be translucent under a user theme.
    fn body(&self, ui: &mut Ui, tokens: Tokens, m: SonnerMetrics) -> bool {
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
        frame
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    if let Some((svg, color)) = self.icon(tokens) {
                        ui.add_space(1.0);
                        Icon::new(svg).size(18.0).color(color).ui(ui);
                        ui.add_space(8.0);
                    }
                    ui.vertical(|ui| self.text(ui, tokens, m.width - 96.0));
                    // The close button hugs the right edge.
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        Self::close_button(ui, tokens)
                    })
                    .inner
                })
                .inner
            })
            .inner
    }

    /// The title, and the muted description beneath it.
    fn text(&self, ui: &mut Ui, tokens: Tokens, width: f32) {
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.set_width(width);
        ui.label(
            egui::RichText::new(&self.title)
                .font(fonts::semibold(ui, 13.5))
                .color(tokens.foreground),
        );
        if let Some(desc) = &self.description {
            ui.label(
                egui::RichText::new(desc)
                    .size(12.5)
                    .color(tokens.muted_foreground),
            );
        }
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
        Icon::new(X)
            .color(color)
            .image(tokens)
            .paint_at(ui, Rect::from_center_size(rect.center(), Vec2::splat(13.0)));
        resp.clicked()
    }
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
        ctx.data_mut(|d| d.insert_temp(Ids::default_corner(), self.corner));

        let mut queue: Vec<LiveToast> =
            ctx.data_mut(|d| d.get_temp(Ids::queue()).unwrap_or_default());
        if queue.is_empty() {
            return;
        }
        let mut pass = Pass {
            ctx,
            tokens: Tokens::from_visuals(&ctx.global_style().visuals),
            m,
            now: Instant::now(),
            heights: ctx.data_mut(|d| d.get_temp(Ids::heights()).unwrap_or_default()),
        };
        // Each corner is an independent stack: its own hover, expand animation,
        // and layout. A toast lives in exactly the corner it was sent to, so
        // changing the toaster's default never moves existing toasts.
        for corner in Corner::ALL {
            let mut stack: Vec<&mut LiveToast> =
                queue.iter_mut().filter(|t| t.corner == corner).collect();
            if !stack.is_empty() {
                pass.stack(corner, &mut stack);
            }
        }
        // Drop fully-faded toasts; persist the rest and the per-frame state.
        queue.retain(|t| !t.finished(pass.now, m));
        ctx.data_mut(|d| {
            d.insert_temp(Ids::queue(), queue);
            d.insert_temp(Ids::heights(), pass.heights);
        });
        ctx.request_repaint();
    }
}

/// One frame of the toaster's rendering.
struct Pass<'a> {
    ctx: &'a egui::Context,
    tokens: Tokens,
    m: SonnerMetrics,
    now: Instant,
    /// Each toast's measured height, read back next frame for the expanded
    /// layout.
    heights: std::collections::HashMap<u64, f32>,
}

impl Pass<'_> {
    /// Tick, lay out, and paint one corner's stack, oldest first.
    fn stack(&mut self, corner: Corner, stack: &mut [&mut LiveToast]) {
        let (ctx, m) = (self.ctx, self.m);
        let hovered = ctx.data(|d| d.get_temp::<bool>(corner.hover_id()).unwrap_or(false));
        let expand = ctx.animate_bool_with_time(corner.expand_id(), hovered, m.expand_secs);
        // Auto-dismiss timers pause while the stack is expanded (hovered).
        if expand < 0.5 {
            let dt = ctx.input(|i| i.stable_dt).min(0.1);
            for t in stack.iter_mut() {
                t.tick(dt, self.now);
            }
        }

        // Depth 0 = newest = front (nearest the corner) = last in send order.
        let lifts = self.expanded_lifts(stack);
        // Render back-to-front (deepest first) so the front card is on top.
        for (depth, t) in stack.iter_mut().rev().enumerate().rev() {
            let geo = CardGeom {
                corner,
                depth,
                expand,
                expanded_lift: lifts[depth],
                now: self.now,
            };
            let (height, closed) = self.card(t, geo);
            self.heights.insert(t.id, height);
            if closed && t.dismissed.is_none() {
                t.dismissed = Some(self.now);
            }
        }

        let extent = lifts.last().copied().unwrap_or(0.0);
        let bounds = corner.stack_bounds(ctx.content_rect(), m, extent);
        let over = ctx.pointer_hover_pos().is_some_and(|p| bounds.contains(p));
        ctx.data_mut(|d| d.insert_temp(corner.hover_id(), over));
    }

    /// Each card's offset from the corner when the stack is fully expanded,
    /// front first, plus a trailing entry for the whole stack's extent.
    fn expanded_lifts(&self, stack: &[&mut LiveToast]) -> Vec<f32> {
        let heights = stack
            .iter()
            .rev()
            .map(|t| self.heights.get(&t.id).copied().unwrap_or(64.0));
        std::iter::once(0.0)
            .chain(heights.scan(0.0, |acc, h| {
                *acc += h + self.m.gap;
                Some(*acc)
            }))
            .collect()
    }

    /// Render one toast card with its stack transform and enter/exit tween.
    /// Returns its height and whether × was clicked.
    fn card(&self, t: &LiveToast, geo: CardGeom) -> (f32, bool) {
        let m = self.m;
        let life = Tween(t.life(geo.now, m)).ease_out();
        let corner = geo.corner;
        // Anchor the card's corner inside the screen position, lifted up the
        // stack.
        let sign = if corner.is_top() { 1.0 } else { -1.0 };
        let hx = corner.h_dir();
        let offset = Vec2::new(hx * m.screen_pad, sign * (m.screen_pad + geo.lift(m)));

        // Scale about the card's anchored edge, centred horizontally, so
        // cards shrink toward the stack's centre line.
        let corner_pt = corner.align().pos_in_rect(&self.ctx.content_rect()) + offset;
        let pivot = egui::pos2((hx * m.width).mul_add(0.5, corner_pt.x), corner_pt.y);
        let scale = geo.scale(m);
        // Cards arrive from just outside the anchored edge.
        let slide_y = (1.0 - life) * 16.0 * -sign;
        let translation = pivot.to_vec2() * (1.0 - scale) + Vec2::new(0.0, slide_y);
        let transform = TSTransform::new(translation, scale);

        let mut closed = false;
        let response = Area::new(egui::Id::unique(("glazier-sonner", t.id)))
            .order(Order::Foreground)
            .anchor(corner.align(), offset)
            .interactable(true)
            .show(self.ctx, |ui| {
                ui.set_opacity(life * geo.alpha(m));
                ui.set_width(m.width);
                ui.with_visual_transform(transform, |ui| {
                    closed = t.body(ui, self.tokens, m);
                });
            });
        (response.response.rect.height(), closed)
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

impl CardGeom {
    #[allow(clippy::cast_precision_loss)]
    const fn depth(self) -> f32 {
        self.depth as f32
    }

    /// Distance from the corner: peeking when collapsed, fanned out when
    /// expanded.
    fn lift(self, m: SonnerMetrics) -> f32 {
        Tween(self.expand).lerp(self.depth() * m.peek, self.expanded_lift)
    }

    /// Deeper cards shrink when collapsed.
    fn scale(self, m: SonnerMetrics) -> f32 {
        let collapsed = m.scale_step.mul_add(-self.depth(), 1.0).max(0.5);
        Tween(self.expand).lerp(collapsed, 1.0)
    }

    /// Cards beyond `max_visible` fade out behind the collapsed stack.
    fn alpha(self, m: SonnerMetrics) -> f32 {
        let hidden = self.depth.saturating_sub(m.max_visible.saturating_sub(1));
        #[allow(clippy::cast_precision_loss)]
        let collapsed = (hidden as f32).mul_add(-0.5, 1.0).max(0.0);
        Tween(self.expand).lerp(collapsed, 1.0)
    }
}

/// Progress through an animation, 0 to 1.
#[derive(Clone, Copy)]
struct Tween(f32);

impl Tween {
    /// Linear interpolation from `a` to `b`.
    fn lerp(self, a: f32, b: f32) -> f32 {
        (b - a).mul_add(self.0, a)
    }

    /// `ease-out` cubic.
    fn ease_out(self) -> f32 {
        let u = 1.0 - self.0;
        (u * u).mul_add(-u, 1.0)
    }
}

/// Where the toast system keeps its shared state in `ctx` data.
struct Ids;

impl Ids {
    /// The shared toast queue.
    fn queue() -> egui::Id {
        egui::Id::unique("glazier-sonner-queue")
    }

    /// The monotonic toast sequence counter.
    fn seq() -> egui::Id {
        egui::Id::unique("glazier-sonner-seq")
    }

    /// The toaster's published default corner (read by `Toast::send`).
    fn default_corner() -> egui::Id {
        egui::Id::unique("glazier-sonner-default-corner")
    }

    /// The cached per-toast measured heights.
    fn heights() -> egui::Id {
        egui::Id::unique("glazier-sonner-heights")
    }
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
    use crate::test_ui::TestUi;

    /// Sending enqueues into the shared store.
    #[test]
    fn send_enqueues() {
        let ctx = egui::Context::default();
        Toast::new("Hi").send(&ctx);
        assert_eq!(queue(&ctx).len(), 1);
    }

    /// An explicit `.position()` is frozen onto the toast and survives a later
    /// toaster default; toasts at different corners keep their own positions.
    #[test]
    fn position_is_frozen_per_toast() {
        let ctx = egui::Context::default();
        Toast::new("a").position(Corner::TopLeft).send(&ctx);
        Toast::new("b").position(Corner::BottomCenter).send(&ctx);
        let corners: Vec<Corner> = queue(&ctx).iter().map(|t| t.corner).collect();
        assert_eq!(corners, vec![Corner::TopLeft, Corner::BottomCenter]);
    }

    /// Without an explicit position, a toast inherits the toaster's published
    /// default corner (not whatever a later toaster sets).
    #[test]
    fn position_inherits_published_default() {
        let ctx = egui::Context::default();
        ctx.data_mut(|d| d.insert_temp(Ids::default_corner(), Corner::TopRight));
        Toast::new("x").send(&ctx);
        assert_eq!(queue(&ctx)[0].corner, Corner::TopRight);
    }

    /// An empty queue makes `show` a no-op (no panic, nothing added).
    #[test]
    fn empty_show_is_noop() {
        let ctx = egui::Context::default();
        crate::test_ui::RunHeadless::run_headless(&ctx, |_| {});
        Toaster::new().show(&ctx);
        assert_eq!(queue(&ctx).len(), 0);
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

    /// The hover region hugs its corner of the screen.
    #[test]
    fn stack_bounds_hug_their_corner() {
        let screen = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));
        let m = SonnerMetrics::default();
        let b = Corner::TopLeft.stack_bounds(screen, m, 10.0);
        assert_eq!(b.min, egui::Pos2::ZERO);
        assert_eq!(
            b.size(),
            Vec2::new(m.width, 72.0) + Vec2::splat(m.screen_pad)
        );
        let b = Corner::BottomRight.stack_bounds(screen, m, 200.0);
        assert_eq!(b.max, screen.max);
        let b = Corner::BottomCenter.stack_bounds(screen, m, 200.0);
        assert!((b.center().x - 400.0).abs() < 1e-3);
        assert_eq!(Corner::TopCenter.align(), Align2::CENTER_TOP);
    }

    /// Deep cards peek, shrink, and fade when collapsed; expanded, all are
    /// whole.
    #[test]
    fn card_geometry_follows_depth() {
        let m = SonnerMetrics::default();
        let geo = |depth, expand| CardGeom {
            corner: Corner::BottomRight,
            depth,
            expand,
            expanded_lift: 100.0,
            now: Instant::now(),
        };
        assert!((geo(4, 0.0).alpha(m) - 0.0).abs() < 1e-6);
        assert!((geo(3, 0.0).alpha(m) - 0.5).abs() < 1e-6);
        assert!((geo(1, 0.0).alpha(m) - 1.0).abs() < 1e-6);
        assert!((geo(4, 1.0).alpha(m) - 1.0).abs() < 1e-6);
        assert!(m.peek.mul_add(-2.0, geo(2, 0.0).lift(m)).abs() < 1e-6);
        assert!((geo(2, 1.0).lift(m) - 100.0).abs() < 1e-6);
        assert!((geo(2, 0.0).scale(m) - 0.9).abs() < 1e-6);
        assert!((geo(2, 1.0).scale(m) - 1.0).abs() < 1e-6);
    }

    /// Timers count down and dismiss; the exit tween then retires the toast.
    #[test]
    fn toasts_tick_then_finish() {
        let m = SonnerMetrics::default();
        let ctx = egui::Context::default();
        Toast::new("x").duration(0.1).send(&ctx);
        let mut t = queue(&ctx).remove(0);
        let now = Instant::now();
        t.tick(0.05, now);
        assert!(t.dismissed.is_none());
        t.tick(0.05, now);
        assert_eq!(t.dismissed, Some(now));
        assert!((t.life(now, m) - 1.0).abs() < 1e-6);
        assert!(!t.finished(now, m));
        let later = now + std::time::Duration::from_secs(1);
        assert!(t.finished(later, m));
        assert!(t.life(later, m).abs() < 1e-6);
    }

    /// A toast whose exit has run its course is dropped from the queue.
    #[test]
    fn finished_toasts_leave_the_queue() {
        let ui = TestUi::new();
        let mut build = |ui: &mut Ui| Toaster::new().show(ui.ctx());
        ui.frame(&mut build);
        let ctx = ui.ctx();
        Toast::new("stays").send(ctx);
        Toast::new("goes").position(Corner::TopLeft).send(ctx);
        ctx.data_mut(|d| {
            let q: &mut Vec<LiveToast> = d.get_temp_mut_or_default(Ids::queue());
            q[1].dismissed = Instant::now().checked_sub(std::time::Duration::from_secs(1));
        });
        ui.frame(&mut build);
        let titles: Vec<String> = queue(ctx).into_iter().map(|t| t.title).collect();
        assert_eq!(titles, ["stays"]);
    }

    /// Clicking × starts the exit; hovering the stack is recorded.
    #[test]
    fn close_button_dismisses_and_hover_is_tracked() {
        let ui = TestUi::new();
        let m = SonnerMetrics::default();
        let mut build = |ui: &mut Ui| Toaster::new().corner(Corner::TopRight).show(ui.ctx());
        ui.frame(&mut build);
        Toast::new("Saved").send(ui.ctx());
        Toast::new("Behind")
            .error()
            .description("older")
            .send(ui.ctx());
        for variant in [Variant::Success, Variant::Warning, Variant::Info] {
            Toast::new("more").variant(variant).send(ui.ctx());
        }
        // Areas spend their first frame measuring before they take input.
        ui.frame(&mut build);
        ui.frame(&mut build);
        // The front card's ×: inset by the frame margin from the card's
        // top-right corner.
        let card_right = 800.0 - m.screen_pad;
        let close = egui::pos2(card_right - 16.0 - 9.0, m.screen_pad + 15.0 + 9.0);
        ui.click(close, &mut build);
        let front = queue(ui.ctx()).pop().unwrap();
        assert!(front.dismissed.is_some());
        let hovered = ui
            .ctx()
            .data(|d| d.get_temp::<bool>(Corner::TopRight.hover_id()));
        assert_eq!(hovered, Some(true));
    }

    fn queue(ctx: &egui::Context) -> Vec<LiveToast> {
        ctx.data(|d| d.get_temp(Ids::queue()).unwrap_or_default())
    }
}
