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
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// chevron-down (lucide): the trailing affordance on menu triggers.
const CHEVRON_DOWN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#;

/// Overridable geometry/timing for [`NavigationMenu`], reach in via
/// [`NavigationMenu::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct NavigationMenuMetrics {
    /// Hover rest before a panel opens (seconds).
    pub open_delay: f32,
    /// Grace period after the pointer leaves both bar and panel before it closes.
    pub close_delay: f32,
    /// Gap between the bar and the flyout panel.
    pub gap: f32,
    /// Default flyout panel width (overridable per-item via [`NavItem::panel_width`]).
    pub default_panel_w: f32,
    /// Trigger horizontal padding (px-4).
    pub trigger_pad_x: f32,
    /// Trigger vertical padding (py-2).
    pub trigger_pad_y: f32,
    /// Minimum trigger height (h-9).
    pub trigger_min_h: f32,
    /// Trigger text size (text-sm).
    pub trigger_text: f32,
    /// Gap between the label and the chevron.
    pub chevron_gap: f32,
    /// Chevron glyph edge length.
    pub chevron_size: f32,
}

impl Default for NavigationMenuMetrics {
    fn default() -> Self {
        Self {
            open_delay: 0.15,
            close_delay: 0.20,
            gap: 8.0,
            default_panel_w: 460.0,
            trigger_pad_x: 16.0,
            trigger_pad_y: 8.0,
            trigger_min_h: 36.0,
            trigger_text: 14.0,
            chevron_gap: 4.0,
            chevron_size: 14.0,
        }
    }
}

/// One top-level entry: either a `menu` (opens a flyout panel) or a plain
/// `link` (returns a click).
#[must_use = "nav items do nothing unless added to a NavigationMenu"]
pub struct NavItem {
    label: String,
    has_panel: bool,
    panel_width: Option<f32>,
}

impl NavItem {
    /// A trigger that reveals a flyout content panel.
    pub fn menu(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            has_panel: true,
            panel_width: None,
        }
    }

    /// A plain navigation link (no panel): show returns its click.
    pub fn link(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            has_panel: false,
            panel_width: None,
        }
    }

    /// Override this menu's flyout panel width (default from
    /// [`NavigationMenuMetrics::default_panel_w`], ~460px).
    pub const fn panel_width(mut self, width: f32) -> Self {
        self.panel_width = Some(width);
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

impl NavState {
    /// Settle on `open`, cancelling any pending transition.
    const fn settle(&mut self, open: Option<usize>) {
        self.open = open;
        self.pending_since = None;
        self.pending_target = None;
    }

    /// Advance the open/switch/close timer at `now`, given the hovered
    /// trigger. Returns the seconds still to wait, if a timer is running.
    fn tick(&mut self, hovered: Option<usize>, now: f64, m: &NavigationMenuMetrics) -> Option<f64> {
        // The target is the hovered trigger, or the open menu while the
        // pointer rests on its panel.
        let target = hovered.or_else(|| self.open.filter(|_| self.panel_hovered));
        if target == self.open {
            self.settle(self.open);
            return None;
        }
        // Switching between menus while one is open is instant, matching
        // the web's immediate swap once the menu is showing.
        if self.open.is_some() && target.is_some() {
            self.settle(target);
            return None;
        }
        // Opening from closed, or closing: rest for the matching delay.
        let delay = f64::from(if target.is_some() {
            m.open_delay
        } else {
            m.close_delay
        });
        if self.pending_target != target {
            self.pending_target = target;
            self.pending_since = Some(now);
        }
        let start = *self.pending_since.get_or_insert(now);
        let remaining = delay - (now - start);
        if remaining <= 0.0 {
            self.settle(target);
            return None;
        }
        Some(remaining)
    }
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
    sizing_hook: SizingHook<NavigationMenuMetrics>,
}

impl Customize<egui::Frame> for NavigationMenu {
    fn style_hook_mut(&mut self) -> &mut StyleHook<egui::Frame> {
        &mut self.style_hook
    }
}

impl Sizeable<NavigationMenuMetrics> for NavigationMenu {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<NavigationMenuMetrics> {
        &mut self.sizing_hook
    }
}

impl NavigationMenu {
    /// Create an empty navigation menu with a stable `id_salt` (remembers which
    /// panel is open across frames).
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id_salt: Id::unique(id_salt),
            items: Vec::new(),
            style_hook: StyleHook::new(),
            sizing_hook: SizingHook::new(),
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
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let ctx = ui.ctx().clone();
        let state_id = self.id_salt.with("nav-state");
        let now = ctx.input(|i| i.time);

        let mut state: NavState = ctx.memory(|m| m.data.get_temp(state_id).unwrap_or_default());

        let mut link_clicked = None;
        let mut bar_rect = egui::Rect::NOTHING;
        // The trigger rect of the currently-open menu (anchor for the panel).
        let mut open_anchor = None;
        let mut hovered_trigger: Option<usize> = None;

        let trigger = Trigger { tokens, m };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (i, item) in self.items.iter().enumerate() {
                let is_open = state.open == Some(i);
                let resp = trigger.show(ui, &item.label, item.has_panel, is_open);
                if !item.has_panel {
                    if resp.clicked() {
                        // Clicking a plain link closes any open panel.
                        link_clicked = Some(i);
                        state.settle(None);
                    }
                    continue;
                }
                if resp.hovered() {
                    hovered_trigger = Some(i);
                }
                if resp.clicked() {
                    // A click toggles immediately.
                    state.settle(if is_open { None } else { Some(i) });
                }
                if is_open {
                    let width = item.panel_width.unwrap_or(m.default_panel_w);
                    open_anchor = Some((resp.rect, width));
                }
            }
            bar_rect = ui.min_rect();
        });

        if let Some(remaining) = state.tick(hovered_trigger, now, &m) {
            // A still pointer sends no events; wake when the delay is due.
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(remaining));
        }

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
            state.panel_hovered = Self::show_panel(ui, tokens, geom, style_hook, m, panel);
        } else {
            state.panel_hovered = false;
        }

        ctx.memory_mut(|d| d.data.insert_temp(state_id, state));
        NavResponse { link_clicked }
    }

    /// Float the flyout panel below the bar; returns whether it's hovered.
    #[allow(clippy::too_many_arguments)]
    fn show_panel(
        ui: &Ui,
        tokens: Tokens,
        geom: PanelGeom,
        style_hook: StyleHook<egui::Frame>,
        m: NavigationMenuMetrics,
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
        let top = bar_rect.bottom() + m.gap;

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

/// Paints the bar's triggers.
struct Trigger {
    tokens: Tokens,
    m: NavigationMenuMetrics,
}

impl Trigger {
    /// Ghost text (plus a chevron for menus) that takes the `accent` surface
    /// on hover or while its panel is open.
    fn show(&self, ui: &mut Ui, label: &str, has_panel: bool, open: bool) -> Response {
        let (tokens, m) = (self.tokens, self.m);
        let font = egui::FontId::proportional(m.trigger_text);
        let galley =
            ui.painter()
                .layout_no_wrap(label.to_owned(), font, egui::Color32::PLACEHOLDER);
        let chevron_w = if has_panel {
            m.chevron_gap + m.chevron_size
        } else {
            0.0
        };
        let content_w = galley.size().x + chevron_w;
        let size = Vec2::new(
            m.trigger_pad_x.mul_add(2.0, content_w),
            m.trigger_pad_y
                .mul_add(2.0, galley.size().y)
                .max(m.trigger_min_h),
        );
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        if !ui.is_rect_visible(rect) {
            return response;
        }
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
        let color = tokens
            .foreground
            .lerp_to_gamma(tokens.accent_foreground, fill_t);
        // Centre the label and chevron as one run.
        let x = rect.center().x - content_w / 2.0;
        let text_w = galley.size().x;
        let gy = rect.center().y - galley.size().y / 2.0;
        ui.painter().galley(egui::pos2(x, gy), galley, color);
        if has_panel {
            let icon = egui::Rect::from_min_size(
                egui::pos2(
                    x + text_w + m.chevron_gap,
                    rect.center().y - m.chevron_size / 2.0,
                ),
                Vec2::splat(m.chevron_size),
            );
            Icon::new(CHEVRON_DOWN)
                .color(color)
                .image(tokens)
                .paint_at(ui, icon);
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;

    #[test]
    fn the_timer_opens_switches_and_closes() {
        let m = NavigationMenuMetrics::default();
        let mut s = NavState::default();
        // Opening waits out the open delay.
        assert!(s.tick(Some(0), 0.0, &m).is_some());
        assert_eq!(s.tick(Some(0), 1.0, &m), None);
        assert_eq!(s.open, Some(0));
        // Switching while open is instant.
        assert_eq!(s.tick(Some(1), 1.1, &m), None);
        assert_eq!(s.open, Some(1));
        // Resting on the panel keeps it open.
        s.panel_hovered = true;
        assert_eq!(s.tick(None, 1.2, &m), None);
        assert_eq!(s.open, Some(1));
        // Leaving both closes after the close delay.
        s.panel_hovered = false;
        assert!(s.tick(None, 1.3, &m).is_some());
        assert_eq!(s.tick(None, 2.0, &m), None);
        assert_eq!(s.open, None);
    }

    /// A click opens a menu's panel; a link reports itself and closes it.
    #[test]
    fn clicks_toggle_panels_and_report_links() {
        let ui = TestUi::new();
        let mut link = None;
        let mut shown = Vec::new();
        let mut build = |ui: &mut Ui| {
            let r = NavigationMenu::new("nav")
                .item(NavItem::menu("Components").panel_width(900.0))
                .item(NavItem::link("Docs"))
                .show(ui, |i, ui| {
                    shown.push(i);
                    ui.label("panel");
                });
            link = r.link_clicked.or(link);
        };
        let out = ui.frame(&mut build);
        let components = TestUi::text_rect(&out, "Components").center();
        let docs = TestUi::text_rect(&out, "Docs").center();
        ui.click(components, &mut build);
        ui.frame(&mut build);
        ui.click(docs, &mut build);
        drop(build);
        assert_eq!(link, Some(1));
        assert!(shown.contains(&0));
    }
}
