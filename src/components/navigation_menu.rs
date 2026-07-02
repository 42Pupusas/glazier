//! [`NavigationMenu`] — a horizontal navigation bar with rich flyout panels,
//! mirroring shadcn's `NavigationMenu`.
//!
//! Where a [`Menubar`](crate::menubar::Menubar) drops a simple list of menu
//! items, a navigation menu reveals a **content panel** beneath the bar — a
//! grid of link cards, descriptions, featured tiles. It's the site-header
//! pattern (Getting Started / Components / Docs…). Like the web component it's
//! **hover-driven**: moving onto a trigger opens its panel after a short rest,
//! switching is instant while any panel is open, and the panel stays up while
//! the pointer travels onto it, closing a beat after the pointer leaves both.
//!
//! Plain links (no panel) are supported too — they just return a click.
//!
//! ```no_run
//! use glazier::navigation_menu::{NavigationMenu, NavItem};
//! # egui::__run_test_ui(|ui| {
//! let nav = NavigationMenu::new("site-nav")
//!     .item(NavItem::menu("Getting Started").panel_width(400.0))
//!     .item(NavItem::menu("Components"))
//!     .item(NavItem::link("Docs"));
//! nav.show(ui, |idx, ui| {
//!     // Render the flyout body for trigger `idx`.
//!     ui.label(format!("panel {idx}"));
//! });
//! # });
//! ```

use egui::{Align2, Area, Id, Order, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::components::icon::Icon;
use crate::customize::{Customize, StyleHook};
use crate::tokens::Tokens;

/// `chevron-down` (lucide) — the trailing affordance on menu triggers.
const CHEVRON_DOWN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#;

/// Hover rest before a panel opens (seconds).
const OPEN_DELAY: f32 = 0.15;
/// Grace period after the pointer leaves both bar and panel before it closes.
const CLOSE_DELAY: f32 = 0.20;
/// Gap between the bar and the flyout panel.
const GAP: f32 = 8.0;
/// Default flyout panel width.
const DEFAULT_PANEL_W: f32 = 460.0;

/// Trigger horizontal / vertical padding (`px-4 py-2`).
const TRIGGER_PAD_X: f32 = 16.0;
const TRIGGER_PAD_Y: f32 = 8.0;
/// Minimum trigger height (`h-9`).
const TRIGGER_MIN_H: f32 = 36.0;
/// Trigger text size (`text-sm`).
const TRIGGER_TEXT: f32 = 14.0;
/// Gap between the label and the chevron.
const CHEVRON_GAP: f32 = 4.0;
const CHEVRON_SIZE: f32 = 14.0;

/// One top-level entry: either a `menu` (opens a flyout panel) or a plain
/// `link` (returns a click).
#[must_use = "nav items do nothing unless added to a NavigationMenu"]
pub struct NavItem {
    label: String,
    has_panel: bool,
    panel_width: f32,
}

impl NavItem {
    /// A trigger that reveals a flyout content panel.
    pub fn menu(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            has_panel: true,
            panel_width: DEFAULT_PANEL_W,
        }
    }

    /// A plain navigation link (no panel) — `show` returns its click.
    pub fn link(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            has_panel: false,
            panel_width: DEFAULT_PANEL_W,
        }
    }

    /// Override this menu's flyout panel width (default ~460px).
    pub const fn panel_width(mut self, width: f32) -> Self {
        self.panel_width = width;
        self
    }
}

/// Per-bar open/timing state, persisted in egui memory.
#[derive(Clone, Copy, Default)]
struct NavState {
    /// Index of the open menu, if any.
    open: Option<usize>,
    /// When the pending open/switch/close transition began. `None` once settled.
    pending_since: Option<f64>,
    /// Index the timer is moving *toward* (the hovered trigger), if any.
    pending_target: Option<usize>,
    /// Was the flyout panel hovered last frame? (Fed back to keep it open.)
    panel_hovered: bool,
}

/// What the bar resolves to this frame, returned by [`NavigationMenu::show`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NavResponse {
    /// A plain [`link`](NavItem::link) trigger was clicked — its index.
    pub link_clicked: Option<usize>,
}

/// Geometry + identity needed to float a flyout panel.
#[derive(Clone, Copy)]
struct PanelGeom {
    state_id: Id,
    anchor: egui::Rect,
    bar_rect: egui::Rect,
    width: f32,
    open: usize,
}

/// A horizontal navigation bar with rich flyout panels (shadcn's
/// `NavigationMenu`).
#[must_use = "navigation menus do nothing unless you show them"]
pub struct NavigationMenu {
    id_salt: Id,
    items: Vec<NavItem>,
    style_hook: StyleHook<egui::Frame>,
}

impl Customize<egui::Frame> for NavigationMenu {
    fn style_hook_mut(&mut self) -> &mut StyleHook<egui::Frame> {
        &mut self.style_hook
    }
}

impl NavigationMenu {
    /// Create an empty navigation menu with a stable `id_salt` (remembers which
    /// panel is open across frames).
    pub fn new(id_salt: impl std::hash::Hash) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            items: Vec::new(),
            style_hook: StyleHook::new(),
        }
    }

    /// Add a top-level item ([`NavItem::menu`] or [`NavItem::link`]).
    pub fn item(mut self, item: NavItem) -> Self {
        self.items.push(item);
        self
    }

    /// Render the bar and (when open) the flyout panel. `panel` renders the
    /// body for the open menu, given its trigger index. Returns a
    /// [`NavResponse`] describing link clicks this frame.
    pub fn show(mut self, ui: &mut Ui, panel: impl FnOnce(usize, &mut Ui)) -> NavResponse {
        let tokens = Tokens::get(ui);
        let ctx = ui.ctx().clone();
        let state_id = self.id_salt.with("nav-state");
        let now = ctx.input(|i| i.time);

        let mut state: NavState = ctx.memory(|m| m.data.get_temp(state_id).unwrap_or_default());

        let mut link_clicked = None;
        let mut bar_rect = egui::Rect::NOTHING;
        // The trigger rect of the currently-open menu (anchor for the panel).
        let mut open_anchor = None;
        let mut hovered_trigger: Option<usize> = None;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (i, item) in self.items.iter().enumerate() {
                let is_open = state.open == Some(i);
                let resp = trigger_row(ui, tokens, &item.label, item.has_panel, is_open);

                if item.has_panel {
                    if resp.hovered() {
                        hovered_trigger = Some(i);
                    }
                    if resp.clicked() {
                        // Click toggles immediately (and cancels any timer).
                        state.open = if is_open { None } else { Some(i) };
                        state.pending_since = None;
                        state.pending_target = None;
                    }
                    if is_open {
                        open_anchor = Some((resp.rect, item.panel_width));
                    }
                } else if resp.clicked() {
                    link_clicked = Some(i);
                    // Clicking a plain link closes any open panel.
                    state.open = None;
                    state.pending_since = None;
                }
            }
            bar_rect = ui.min_rect();
        });

        // Drive the hover state machine: the target is the hovered trigger (to
        // open/switch) or `None` once the pointer rests off both bar and panel
        // (to close). Switching between already-open menus is instant.
        Self::drive(&ctx, &mut state, now, hovered_trigger);

        // Render the flyout panel for the open menu.
        if let (Some(open), Some((anchor, width))) = (state.open, open_anchor) {
            let geom = PanelGeom {
                state_id,
                anchor,
                bar_rect,
                width,
                open,
            };
            let style_hook = std::mem::take(&mut self.style_hook);
            state.panel_hovered = Self::show_panel(ui, tokens, geom, style_hook, panel);
        } else {
            state.panel_hovered = false;
        }

        ctx.memory_mut(|m| m.data.insert_temp(state_id, state));
        NavResponse { link_clicked }
    }

    /// Advance the open/switch/close timer for one frame.
    fn drive(ctx: &egui::Context, state: &mut NavState, now: f64, hovered: Option<usize>) {
        // "Active target" = hovered trigger, or keep-open if the panel is hovered.
        let target = hovered.or(if state.panel_hovered {
            state.open
        } else {
            None
        });

        if target == state.open {
            // Already where we want to be; cancel any pending transition.
            state.pending_since = None;
            state.pending_target = None;
            return;
        }

        // An open→other-open switch is instant (no rest), matching the web's
        // "motion-safe" immediate swap once the menu is already showing.
        if state.open.is_some() && target.is_some() {
            state.open = target;
            state.pending_since = None;
            state.pending_target = None;
            return;
        }

        // Opening from closed, or closing to none: run the appropriate timer.
        let delay = f64::from(if target.is_some() {
            OPEN_DELAY
        } else {
            CLOSE_DELAY
        });
        if state.pending_target != target {
            state.pending_target = target;
            state.pending_since = Some(now);
        }
        let start = state.pending_since.get_or_insert(now);
        if now - *start >= delay {
            state.open = target;
            state.pending_since = None;
            state.pending_target = None;
        } else {
            let remaining = (delay - (now - *start)).max(0.0);
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(remaining));
        }
    }

    /// Float the flyout panel below the bar; returns whether it's hovered.
    fn show_panel(
        ui: &Ui,
        tokens: Tokens,
        geom: PanelGeom,
        style_hook: StyleHook<egui::Frame>,
        panel: impl FnOnce(usize, &mut Ui),
    ) -> bool {
        let PanelGeom {
            state_id,
            anchor,
            bar_rect,
            width,
            open,
        } = geom;
        let ctx = ui.ctx();
        let t = ctx.animate_bool_with_time(state_id.with("anim"), true, 0.12);

        // Anchor the panel's left edge to the trigger, but keep it on-screen.
        let screen = ctx.content_rect();
        let mut left = anchor.left();
        if left + width > screen.right() - 8.0 {
            left = (screen.right() - 8.0 - width).max(screen.left() + 8.0);
        }
        let top = bar_rect.bottom() + GAP;

        // Opaque `widget` (popover) surface — `background` is the app-canvas
        // token and may be translucent under a user theme; `card` is reserved
        // for static containers, not floating overlays.
        let mut frame = egui::Frame::new()
            .fill(tokens.widget)
            .stroke(Stroke::new(1.0, tokens.border))
            .corner_radius(tokens.radius_2xl())
            .inner_margin(egui::Margin::same(16))
            .shadow(egui::epaint::Shadow {
                offset: [0, 8],
                blur: 32,
                spread: 0,
                color: egui::Color32::from_black_alpha(45),
            });
        style_hook.apply(&mut frame);

        let inner = Area::new(state_id.with("panel"))
            .order(Order::Foreground)
            .pivot(Align2::LEFT_TOP)
            .fixed_pos(egui::pos2(left, top))
            .interactable(true)
            .show(ctx, |ui| {
                ui.set_opacity(t);
                ui.set_width(width);
                frame.show(ui, |ui| panel(open, ui));
            });

        ctx.input(|i| i.pointer.hover_pos())
            .is_some_and(|p| inner.response.rect.contains(p))
    }
}

/// A single nav trigger: ghost text (plus a chevron for menus) that takes the
/// `accent` surface on hover or while its panel is open.
fn trigger_row(ui: &mut Ui, tokens: Tokens, label: &str, has_panel: bool, open: bool) -> Response {
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(TRIGGER_TEXT),
        tokens.foreground,
    );
    let chevron_w = if has_panel {
        CHEVRON_GAP + CHEVRON_SIZE
    } else {
        0.0
    };
    let width = 2.0_f32.mul_add(TRIGGER_PAD_X, galley.size().x + chevron_w);
    let height = 2.0_f32
        .mul_add(TRIGGER_PAD_Y, galley.size().y)
        .max(TRIGGER_MIN_H);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

    if ui.is_rect_visible(rect) {
        let hover_t =
            ui.ctx()
                .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.15);
        let fill_t = if open { 1.0 } else { hover_t };
        if fill_t > 0.01 {
            ui.painter().rect(
                rect,
                tokens.radius_md(),
                tokens.accent.gamma_multiply(fill_t),
                Stroke::NONE,
                StrokeKind::Inside,
            );
        }
        let text_color = tokens
            .foreground
            .lerp_to_gamma(tokens.accent_foreground, fill_t);

        // Centre the label (+ chevron) as a unit.
        let content_w = galley.size().x + chevron_w;
        let mut x = rect.center().x - content_w / 2.0;
        let galley = ui.painter().layout_no_wrap(
            label.to_owned(),
            egui::FontId::proportional(TRIGGER_TEXT),
            text_color,
        );
        let gy = rect.center().y - galley.size().y / 2.0;
        ui.painter().galley(egui::pos2(x, gy), galley, text_color);
        x += content_w - chevron_w + CHEVRON_GAP;
        if has_panel {
            let icon_rect = egui::Rect::from_min_size(
                egui::pos2(x, rect.center().y - CHEVRON_SIZE / 2.0),
                Vec2::splat(CHEVRON_SIZE),
            );
            Icon::new(CHEVRON_DOWN)
                .color(text_color)
                .image(tokens)
                .paint_at(ui, icon_rect);
        }
    }
    response
}
