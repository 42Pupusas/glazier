//! [`SidebarMenu`] — a labelled navigation group, mirroring shadcn's
//! `SidebarGroup` + `SidebarMenu`.
//!
//! A muted `text-xs` group label sits above a vertical stack of
//! [items](SidebarMenuItem): each a `rounded-xl` row with a leading
//! [`Icon`](crate::icon::Icon) and a label, painted in the `sidebar-accent`
//! palette when **active** and on **hover**. Returns the index of a clicked
//! item so callers can drive selection.
//!
//! ```no_run
//! use glazier::sidebar_menu::SidebarMenu;
//! # const HOME: &str = "<svg/>";
//! # const BELL: &str = "<svg/>";
//! # egui::__run_test_ui(|ui| {
//! let mut active = 0;
//! SidebarMenu::new("Overview")
//!     .item(HOME, "Analytics")
//!     .item(BELL, "Notifications")
//!     .show(ui, &mut active);
//! # });
//! ```

use egui::{Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::customize::{Customize, StyleHook};
use crate::icon::Icon;
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry/timing for [`SidebarMenu`] — reach in via
/// [`SidebarMenu::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct SidebarMenuMetrics {
    /// Item inner padding (`px-3`).
    pub item_pad_x: f32,
    /// Row height (`h-8`).
    pub item_h: f32,
    /// Gap between rows (`gap-1`).
    pub item_gap: f32,
    /// Gap between icon and label (`gap-2`).
    pub icon_gap: f32,
    /// Item text size (`text-sm`).
    pub item_text: f32,
    /// Group-label / badge text size (`text-xs`).
    pub label_text: f32,
    /// Group padding (`p-2`).
    pub group_pad: f32,
    /// Group-label row height (`h-8`).
    pub label_h: f32,
    /// Leading icon edge length (`size-4`).
    pub icon_size: f32,
    /// Seconds for the hover accent transition.
    pub hover_time: f32,
}

impl Default for SidebarMenuMetrics {
    fn default() -> Self {
        Self {
            item_pad_x: 12.0,
            item_h: 32.0,
            item_gap: 4.0,
            icon_gap: 8.0,
            item_text: 13.0,
            label_text: 12.0,
            group_pad: 8.0,
            label_h: 32.0,
            icon_size: 16.0,
            hover_time: 0.15,
        }
    }
}

/// One row of a [`SidebarMenu`].
struct SidebarMenuItem {
    icon: &'static str,
    label: String,
    /// Optional trailing count badge (shadcn `SidebarMenuBadge`). Rendered as a
    /// pill at the row's trailing edge when expanded, or a small accent dot on
    /// the icon when collapsed to the icon rail.
    badge: Option<String>,
}

/// A labelled sidebar navigation group.
#[must_use = "menus do nothing unless you show them"]
pub struct SidebarMenu {
    label: String,
    items: Vec<SidebarMenuItem>,
    collapsed: bool,
    style_hook: StyleHook<egui::Frame>,
    sizing_hook: SizingHook<SidebarMenuMetrics>,
}

impl Customize<egui::Frame> for SidebarMenu {
    fn style_hook_mut(&mut self) -> &mut StyleHook<egui::Frame> {
        &mut self.style_hook
    }
}

impl Sizeable<SidebarMenuMetrics> for SidebarMenu {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<SidebarMenuMetrics> {
        &mut self.sizing_hook
    }
}

impl SidebarMenu {
    /// Create a menu under the muted group `label` (shadcn `SidebarGroupLabel`).
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            items: Vec::new(),
            collapsed: false,
            style_hook: StyleHook::new(),
            sizing_hook: SizingHook::new(),
        }
    }

    /// Append an item: a leading `icon` (raw SVG markup) and a text `label`.
    pub fn item(mut self, icon: &'static str, label: impl Into<String>) -> Self {
        self.items.push(SidebarMenuItem {
            icon,
            label: label.into(),
            badge: None,
        });
        self
    }

    /// Append an item carrying a trailing count `badge` (shadcn
    /// `SidebarMenuBadge`) — a pill when expanded, an accent dot on the icon
    /// when collapsed.
    pub fn item_with_badge(
        mut self,
        icon: &'static str,
        label: impl Into<String>,
        badge: impl Into<String>,
    ) -> Self {
        self.items.push(SidebarMenuItem {
            icon,
            label: label.into(),
            badge: Some(badge.into()),
        });
        self
    }

    /// Collapse the group to an icon-only rail (shadcn `collapsible="icon"`):
    /// the group label is hidden and each item shrinks to a centered icon
    /// button. Badges become a small accent dot on the icon.
    pub const fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Render the group, highlighting the item at `*active` and updating it when
    /// a row is clicked. Returns the index of the clicked item, if any.
    pub fn show(mut self, ui: &mut Ui, active: &mut usize) -> Option<usize> {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let mut clicked = None;
        let mut frame = egui::Frame::new().inner_margin(m.group_pad);
        std::mem::take(&mut self.style_hook).apply(&mut frame);
        frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = Vec2::ZERO;

            let rows = Rows { tokens, m };
            // Always reserve the label row height so items don't shift
            // when the sidebar collapses — only the text is hidden.
            rows.label(ui, &self.label, self.collapsed);
            for (i, item) in self.items.iter().enumerate() {
                if i > 0 {
                    ui.add_space(m.item_gap);
                }
                if rows.item(ui, item, i == *active, self.collapsed).clicked() {
                    *active = i;
                    clicked = Some(i);
                }
            }
        });
        clicked
    }
}

/// Paints a menu's rows.
struct Rows {
    tokens: Tokens,
    m: SidebarMenuMetrics,
}

impl Rows {
    /// The semibold face at `size`.
    fn semibold(ui: &Ui, size: f32) -> egui::FontId {
        egui::FontId::new(size, crate::fonts::semibold(ui, size).family)
    }

    /// A muted group label (`h-8 px-3 text-xs text-sidebar-foreground/70`).
    /// Space is always reserved; text is hidden when `hidden` (collapsed rail)
    /// so items don't jump when the sidebar opens or closes.
    fn label(&self, ui: &mut Ui, text: &str, hidden: bool) {
        let m = self.m;
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), m.label_h), Sense::hover());
        if hidden || !ui.is_rect_visible(rect) {
            return;
        }
        let color = self.tokens.muted_foreground;
        let galley =
            ui.painter()
                .layout_no_wrap(text.to_owned(), Self::semibold(ui, m.label_text), color);
        let pos = egui::pos2(
            rect.left() + m.item_pad_x,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(pos, galley, color);
    }

    /// A clickable nav row with an accent fill while active or hovered:
    /// icon and label, or on the collapsed rail a centred icon button.
    fn item(
        &self,
        ui: &mut Ui,
        item: &SidebarMenuItem,
        is_active: bool,
        collapsed: bool,
    ) -> Response {
        let (tokens, m) = (self.tokens, self.m);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), m.item_h), Sense::click());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        // `data-active:bg-sidebar-accent` / `hover:bg-sidebar-accent`: the
        // hover lift eases with `transition-colors`; the active state is solid.
        let hover_t = ui.ctx().animate_bool_with_time(
            response.id.with("hover"),
            response.hovered(),
            m.hover_time,
        );
        let highlight_t = if is_active { 1.0 } else { hover_t };
        // Collapsed, the fill is a square button centred in the row (`size-8`).
        let fill = if collapsed {
            Rect::from_center_size(rect.center(), Vec2::splat(m.item_h))
        } else {
            rect
        };
        if highlight_t > 0.01 {
            ui.painter().rect(
                fill,
                tokens.radius_xl(),
                tokens.accent.gamma_multiply(highlight_t),
                Stroke::NONE,
                StrokeKind::Inside,
            );
        }
        let fg = tokens
            .foreground
            .lerp_to_gamma(tokens.accent_foreground, highlight_t);
        let icon_rect = if collapsed {
            Rect::from_center_size(rect.center(), Vec2::splat(m.icon_size))
        } else {
            // `size-4`, vertically centred inside the `px-3` inset.
            Rect::from_min_size(
                egui::pos2(
                    rect.left() + m.item_pad_x,
                    rect.center().y - m.icon_size / 2.0,
                ),
                Vec2::splat(m.icon_size),
            )
        };
        Icon::new(item.icon)
            .size(m.icon_size)
            .color(fg)
            .image(tokens)
            .paint_at(ui, icon_rect);
        if collapsed {
            // A badge shrinks to an accent dot on the icon's top-right.
            if item.badge.is_some() {
                let dot = egui::pos2(icon_rect.right() + 1.0, icon_rect.top() - 1.0);
                ui.painter().circle_filled(dot, 3.0, tokens.primary);
            }
        } else {
            self.text(
                ui,
                item,
                rect,
                icon_rect.right() + m.icon_gap,
                is_active,
                highlight_t,
            );
        }
        response
    }

    /// The label from `text_x`, `font-medium` when active, then any badge.
    /// Truncated with `…` (shadcn's `truncate`) before the badge and the
    /// `px-3` right inset.
    fn text(
        &self,
        ui: &Ui,
        item: &SidebarMenuItem,
        rect: Rect,
        text_x: f32,
        is_active: bool,
        highlight_t: f32,
    ) {
        let (tokens, m) = (self.tokens, self.m);
        let fg = tokens
            .foreground
            .lerp_to_gamma(tokens.accent_foreground, highlight_t);
        let font = if is_active {
            Self::semibold(ui, m.item_text)
        } else {
            egui::FontId::proportional(m.item_text)
        };
        let badge_w = item
            .badge
            .as_deref()
            .map_or(0.0, |b| self.badge(ui, rect, b, highlight_t) + m.icon_gap);
        let avail = (rect.right() - m.item_pad_x - badge_w - text_x).max(0.0);
        let mut job = egui::text::LayoutJob::simple(item.label.clone(), font, fg, avail);
        job.wrap = egui::text::TextWrapping::truncate_at_width(avail);
        let galley = ui.painter().layout_job(job);
        let pos = egui::pos2(text_x, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(pos, galley, fg);
    }

    /// A trailing count pill (shadcn `SidebarMenuBadge`) at the row's right
    /// inset. Returns its width so the label can stop short of it.
    fn badge(&self, ui: &Ui, row: Rect, text: &str, highlight_t: f32) -> f32 {
        let (tokens, m) = (self.tokens, self.m);
        let fg = tokens
            .muted_foreground
            .lerp_to_gamma(tokens.accent_foreground, highlight_t);
        let galley =
            ui.painter()
                .layout_no_wrap(text.to_owned(), Self::semibold(ui, m.label_text), fg);
        let size = galley.size() + Vec2::new(6.0, 1.0) * 2.0;
        let center = egui::pos2(row.right() - m.item_pad_x - size.x / 2.0, row.center().y);
        let pill = Rect::from_center_size(center, size);
        ui.painter().rect(
            pill,
            pill.height() / 2.0,
            tokens.muted.gamma_multiply(0.6),
            Stroke::NONE,
            StrokeKind::Inside,
        );
        ui.painter()
            .galley(pill.center() - galley.size() / 2.0, galley, fg);
        size.x
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;

    const DOT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="4" fill="currentColor"/></svg>"#;

    fn menu(collapsed: bool) -> SidebarMenu {
        SidebarMenu::new("Platform")
            .item(DOT, "Home")
            .item_with_badge(DOT, "Inbox", "12")
            .collapsed(collapsed)
    }

    /// Clicking a row makes it active; expanded rows show labels and badges.
    #[test]
    fn clicking_a_row_activates_it() {
        let ui = TestUi::new();
        let mut active = 0;
        let mut clicked = None;
        let mut build = |ui: &mut Ui| clicked = menu(false).show(ui, &mut active).or(clicked);
        let out = ui.frame(&mut build);
        assert!(TestUi::text_x(&out, "12") > TestUi::text_x(&out, "Inbox"));
        ui.click(TestUi::text_rect(&out, "Inbox").center(), &mut build);
        drop(build);
        assert_eq!((active, clicked), (1, Some(1)));
    }

    /// The collapsed rail hides every label but keeps the rows clickable.
    #[test]
    fn the_collapsed_rail_hides_text() {
        let ui = TestUi::new();
        let mut active = 0;
        let rect = std::cell::Cell::new(Rect::NOTHING);
        let mut build = |ui: &mut Ui| {
            ui.set_width(48.0);
            let _ = menu(true).show(ui, &mut active);
            rect.set(ui.min_rect());
        };
        let out = ui.frame(&mut build);
        let texts: Vec<_> = out
            .shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::Shape::Text(t) => Some(t.galley.job.text.clone()),
                _ => None,
            })
            .collect();
        // Icons without an SVG loader paint egui's ⚠ placeholder; only the
        // menu's own strings matter here.
        for own in ["Platform", "Home", "Inbox", "12"] {
            assert!(!texts.iter().any(|t| t == own), "{texts:?}");
        }
        let r = rect.get();
        ui.click(egui::pos2(r.center().x, r.bottom() - 20.0), &mut build);
        drop(build);
        assert_eq!(active, 1);
    }
}
