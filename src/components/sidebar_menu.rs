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
use crate::tokens::Tokens;

/// Item inner padding: `px-3 py-2`.
const ITEM_PAD_X: f32 = 12.0;
/// Row height (`h-8`).
const ITEM_H: f32 = 32.0;
/// Gap between rows (`gap-1`).
const ITEM_GAP: f32 = 4.0;
/// Gap between icon and label (`gap-2`).
const ICON_GAP: f32 = 8.0;
/// Item / label text size (`text-sm` / `text-xs`).
const ITEM_TEXT: f32 = 13.0;
const LABEL_TEXT: f32 = 12.0;
/// Group padding (`p-2`).
const GROUP_PAD: f32 = 8.0;
/// Group-label row height (`h-8`).
const LABEL_H: f32 = 32.0;

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
}

impl Customize<egui::Frame> for SidebarMenu {
    fn style_hook_mut(&mut self) -> &mut StyleHook<egui::Frame> {
        &mut self.style_hook
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
        let mut clicked = None;
        let mut frame = egui::Frame::new().inner_margin(GROUP_PAD);
        std::mem::take(&mut self.style_hook).apply(&mut frame);
        frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = Vec2::ZERO;

            // Always reserve the label row height so items don't shift
            // when the sidebar collapses — only the text is hidden.
            label_row(ui, tokens, &self.label, self.collapsed);
            for (i, item) in self.items.iter().enumerate() {
                if i > 0 {
                    ui.add_space(ITEM_GAP);
                }
                let resp = if self.collapsed {
                    icon_row(ui, tokens, item, i == *active)
                } else {
                    item_row(ui, tokens, item, i == *active)
                };
                if resp.clicked() {
                    *active = i;
                    clicked = Some(i);
                }
            }
        });
        clicked
    }
}

/// A muted group label (`h-8 px-3 text-xs text-sidebar-foreground/70`).
/// Space is always reserved; text is hidden when `hidden` (collapsed rail) to
/// prevent items from jumping when the sidebar opens/closes.
fn label_row(ui: &mut Ui, tokens: Tokens, text: &str, hidden: bool) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), LABEL_H), Sense::hover());
    if hidden || !ui.is_rect_visible(rect) { return; }
    let color = tokens.muted_foreground;
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        egui::FontId::new(LABEL_TEXT, crate::fonts::semibold(ui, LABEL_TEXT).family),
        color,
    );
    let pos = egui::pos2(
        rect.left() + ITEM_PAD_X,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(pos, galley, color);
}

/// A clickable nav row: leading icon + label, accent fill when active/hovered.
fn item_row(ui: &mut Ui, tokens: Tokens, item: &SidebarMenuItem, is_active: bool) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), ITEM_H), Sense::click());

    if ui.is_rect_visible(rect) {
        // `data-active:bg-sidebar-accent` / `hover:bg-sidebar-accent`. The hover
        // lift eases with `transition-colors`; the active state stays solid.
        let hover_t =
            ui.ctx()
                .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.15);
        let highlight_t = if is_active { 1.0 } else { hover_t };
        if highlight_t > 0.01 {
            ui.painter().rect(
                rect,
                tokens.radius_xl(),
                tokens.accent.gamma_multiply(highlight_t),
                Stroke::NONE,
                StrokeKind::Inside,
            );
        }
        let fg = tokens
            .foreground
            .lerp_to_gamma(tokens.accent_foreground, highlight_t);

        // Leading icon (`size-4`), vertically centered with `px-3` inset.
        let icon_size = 16.0;
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + ITEM_PAD_X, rect.center().y - icon_size / 2.0),
            Vec2::splat(icon_size),
        );
        Icon::new(item.icon)
            .size(icon_size)
            .color(fg)
            .image(tokens)
            .paint_at(ui, icon_rect);

        // Label, `font-medium` when active. Truncated with `…` (shadcn's
        // `truncate`) so a long label never spills past the `px-3` right inset
        // when the card is narrow.
        let font = if is_active {
            egui::FontId::new(ITEM_TEXT, crate::fonts::semibold(ui, ITEM_TEXT).family)
        } else {
            egui::FontId::proportional(ITEM_TEXT)
        };
        let text_x = icon_rect.right() + ICON_GAP;
        // A trailing badge pill (shadcn `SidebarMenuBadge`) reserves room on the
        // right so the label truncates before it rather than under it.
        let badge_w = item
            .badge
            .as_deref()
            .map_or(0.0, |b| badge_pill(ui, tokens, rect, b, highlight_t) + ICON_GAP);
        let avail = (rect.right() - ITEM_PAD_X - badge_w - text_x).max(0.0);
        let mut job = egui::text::LayoutJob::simple(item.label.clone(), font, fg, avail);
        job.wrap = egui::text::TextWrapping::truncate_at_width(avail);
        let galley = ui.painter().layout_job(job);
        let pos = egui::pos2(text_x, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(pos, galley, fg);
    }
    response
}

/// A collapsed (icon-rail) nav row: a centered icon button, no label, with the
/// badge rendered as a small accent dot on the icon's top-right corner.
fn icon_row(ui: &mut Ui, tokens: Tokens, item: &SidebarMenuItem, is_active: bool) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), ITEM_H), Sense::click());

    if ui.is_rect_visible(rect) {
        let hover_t =
            ui.ctx()
                .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.15);
        let highlight_t = if is_active { 1.0 } else { hover_t };
        // The accent fill is a square button centered in the row (`size-8`).
        let btn = egui::Rect::from_center_size(rect.center(), Vec2::splat(ITEM_H));
        if highlight_t > 0.01 {
            ui.painter().rect(
                btn,
                tokens.radius_xl(),
                tokens.accent.gamma_multiply(highlight_t),
                Stroke::NONE,
                StrokeKind::Inside,
            );
        }
        let fg = tokens
            .foreground
            .lerp_to_gamma(tokens.accent_foreground, highlight_t);

        let icon_size = 16.0;
        let icon_rect = egui::Rect::from_center_size(rect.center(), Vec2::splat(icon_size));
        Icon::new(item.icon)
            .size(icon_size)
            .color(fg)
            .image(tokens)
            .paint_at(ui, icon_rect);

        // Badge → a small accent dot on the icon's top-right.
        if item.badge.is_some() {
            let dot = egui::pos2(icon_rect.right() + 1.0, icon_rect.top() - 1.0);
            ui.painter().circle_filled(dot, 3.0, tokens.primary);
        }
    }
    response
}

/// Paint a trailing count pill (shadcn `SidebarMenuBadge`) at the row's right
/// inset; returns the pill width so the label can reserve room for it.
fn badge_pill(ui: &Ui, tokens: Tokens, row: Rect, text: &str, highlight_t: f32) -> f32 {
    let font = egui::FontId::new(LABEL_TEXT, crate::fonts::semibold(ui, LABEL_TEXT).family);
    let fg = tokens
        .muted_foreground
        .lerp_to_gamma(tokens.accent_foreground, highlight_t);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, fg);
    let pad = Vec2::new(6.0, 1.0);
    let size = galley.size() + pad * 2.0;
    let center = egui::pos2(
        row.right() - ITEM_PAD_X - size.x / 2.0,
        row.center().y,
    );
    let pill = egui::Rect::from_center_size(center, size);
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
