//! [`Sidebar`] — a collapsible application-shell navigation panel, mirroring
//! shadcn's `Sidebar` with `collapsible="icon"`.
//!
//! The panel animates its width between an **expanded** rail (labels + icons)
//! and a slim **icon rail** (icons only), with a pinned header and footer
//! bracketing a scrollable middle. A built-in **trigger** button and an
//! edge **rail** both toggle the collapse; the collapsed/expanded state
//! persists in egui memory keyed by the sidebar's `id_salt`, so an external
//! [`SidebarTrigger`] elsewhere in your layout can drive the same panel.
//!
//! The header, body, and footer closures each receive a `collapsed: bool` so
//! you can hide labels (and centre icons) when the rail is slim.
//!
//! ```no_run
//! use glazier::sidebar::Sidebar;
//! # egui::__run_test_ui(|ui| {
//! Sidebar::new("app")
//!     .height(320.0)
//!     .header(|ui, collapsed| {
//!         if !collapsed {
//!             ui.label("Acme Inc.");
//!         }
//!     })
//!     .footer(|ui, _collapsed| {
//!         ui.label("· profile");
//!     })
//!     .show(ui, |ui, _collapsed| {
//!         ui.label("Dashboard");
//!         ui.label("Projects");
//!     });
//! # });
//! ```

use egui::{Align, Id, Layout, Rect, Sense, Stroke, Ui, UiBuilder, Vec2};

use crate::icon::Icon;
use crate::tokens::Tokens;

/// Default expanded width (`w-64`).
const EXPANDED_W: f32 = 256.0;
/// Default collapsed icon-rail width (`w-[--sidebar-width-icon]` ≈ `w-12`+pad).
const COLLAPSED_W: f32 = 56.0;
/// Collapse animation duration (seconds).
const ANIM: f32 = 0.18;
/// Inner padding (`p-2`).
const PAD: f32 = 8.0;
/// Public re-export of the sidebar's inner padding so content widgets can
/// use [`ScrollArea::right_bleed`](crate::scroll_area::ScrollArea::right_bleed)
/// to push their scrollbar flush with the panel border.
pub const SIDEBAR_PAD: f32 = PAD;
/// Trigger / header-row height (`h-11`).
const TRIGGER_H: f32 = 44.0;
/// Edge-rail hover-zone width.
const RAIL_W: f32 = 14.0;

/// Lucide `panel-left` glyph used by the built-in trigger.
const PANEL_LEFT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M9 3v18"/></svg>"#;

/// The memory key for a sidebar's collapsed flag.
fn state_id(id_salt: &str) -> Id {
    Id::new(("glazier-sidebar", id_salt))
}

/// Read whether the sidebar with `id_salt` is currently collapsed.
#[must_use]
pub fn is_collapsed(ui: &Ui, id_salt: &str) -> bool {
    ui.data(|d| d.get_temp(state_id(id_salt)).unwrap_or(false))
}

/// Flip the collapsed state of the sidebar with `id_salt` (used by an external
/// [`SidebarTrigger`] or your own button).
pub fn toggle(ui: &Ui, id_salt: &str) {
    let id = state_id(id_salt);
    let now: bool = ui.data(|d| d.get_temp(id).unwrap_or(false));
    ui.data_mut(|d| d.insert_temp(id, !now));
}

/// What [`Sidebar::show`] reports back.
pub struct SidebarResponse {
    /// Whether the sidebar is collapsed to its icon rail.
    pub collapsed: bool,
    /// Whether the collapse was toggled this frame (trigger or rail click).
    pub toggled: bool,
}

type SectionFn<'a> = Box<dyn FnOnce(&mut Ui, bool) + 'a>;
type ExtraFn<'a>   = Box<dyn FnOnce(&mut Ui) + 'a>;

/// A collapsible application-shell sidebar.
#[must_use = "sidebars do nothing unless you show them"]
pub struct Sidebar<'a> {
    id_salt: String,
    expanded_w: f32,
    collapsed_w: f32,
    height: Option<f32>,
    trigger: bool,
    rail: bool,
    trigger_extra: Option<ExtraFn<'a>>,
    header: Option<SectionFn<'a>>,
    footer: Option<SectionFn<'a>>,
}

impl<'a> Sidebar<'a> {
    /// Create a sidebar whose collapse state persists under `id_salt`.
    pub fn new(id_salt: impl Into<String>) -> Self {
        Self {
            id_salt: id_salt.into(),
            expanded_w: EXPANDED_W,
            collapsed_w: COLLAPSED_W,
            height: None,
            trigger: true,
            rail: true,
            trigger_extra: None,
            header: None,
            footer: None,
        }
    }

    /// Set the expanded width (default 256).
    pub const fn expanded_width(mut self, w: f32) -> Self {
        self.expanded_w = w;
        self
    }

    /// Set the collapsed icon-rail width (default 56).
    pub const fn collapsed_width(mut self, w: f32) -> Self {
        self.collapsed_w = w;
        self
    }

    /// Pin the panel to an explicit height (defaults to the available height).
    pub const fn height(mut self, h: f32) -> Self {
        self.height = Some(h);
        self
    }

    /// Show (default) or hide the built-in trigger button at the top.
    pub const fn trigger(mut self, on: bool) -> Self {
        self.trigger = on;
        self
    }

    /// Show (default) or hide the click-to-toggle edge rail.
    pub const fn rail(mut self, on: bool) -> Self {
        self.rail = on;
        self
    }

    /// Add extra content rendered **right-aligned** in the trigger row,
    /// only when the sidebar is expanded (collapsed=false). Use this for
    /// small icon buttons that conceptually live beside the collapse toggle.
    pub fn trigger_extra(mut self, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.trigger_extra = Some(Box::new(add));
        self
    }

    /// Set the pinned header content (receives `collapsed`).
    pub fn header(mut self, add: impl FnOnce(&mut Ui, bool) + 'a) -> Self {
        self.header = Some(Box::new(add));
        self
    }

    /// Set the pinned footer content (receives `collapsed`).
    pub fn footer(mut self, add: impl FnOnce(&mut Ui, bool) + 'a) -> Self {
        self.footer = Some(Box::new(add));
        self
    }

    /// Render the sidebar; `body` fills the scrollable middle (receives
    /// `collapsed`).
    pub fn show(self, ui: &mut Ui, body: impl FnOnce(&mut Ui, bool)) -> SidebarResponse {
        let tokens = Tokens::get(ui);
        let id = state_id(&self.id_salt);
        let mut collapsed: bool = ui.data(|d| d.get_temp(id).unwrap_or(false));
        let mut toggled = false;

        // Animated width: 0 = expanded, 1 = collapsed.
        let t = ui
            .ctx()
            .animate_bool_with_time(id.with("anim"), collapsed, ANIM);
        let width = egui::lerp(self.expanded_w..=self.collapsed_w, t);
        let height = self.height.unwrap_or_else(|| ui.available_height());

        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
        ui.set_clip_rect(ui.clip_rect().intersect(rect));

        // Panel surface + right border (`border-r`).
        ui.painter().rect_filled(rect, 0.0, tokens.card);
        ui.painter().vline(
            rect.right(),
            rect.top()..=rect.bottom(),
            Stroke::new(1.0, tokens.border),
        );

        let inner = rect.shrink(PAD);
        let mut top = inner.top();
        let mut bottom = inner.bottom();

        // Trigger row (top).
        if self.trigger {
            let trow = Rect::from_min_max(
                egui::pos2(inner.left(), top),
                egui::pos2(inner.right(), top + TRIGGER_H),
            );
            if trigger_button(ui, tokens, trow).clicked() {
                collapsed = !collapsed;
                toggled = true;
            }
            // Optional extra content right-aligned in the trigger row,
            // only when the sidebar is expanded.
            if !collapsed {
                if let Some(extra) = self.trigger_extra {
                    // Reserve the rightmost portion of the trigger row for
                    // the extra widget. Give it the full height so the widget
                    // can centre itself vertically as it likes.
                    let extra_rect = Rect::from_min_max(
                        egui::pos2(inner.left() + TRIGGER_H, trow.top()),
                        egui::pos2(inner.right(), trow.bottom()),
                    );
                    let mut child = ui.new_child(
                        UiBuilder::new()
                            .max_rect(extra_rect)
                            .layout(Layout::right_to_left(Align::Center)),
                    );
                    extra(&mut child);
                }
            }
            top += TRIGGER_H;
        }

        // Header (pinned, auto height).
        if let Some(header) = self.header {
            let region = Rect::from_min_max(
                egui::pos2(inner.left(), top),
                egui::pos2(inner.right(), bottom),
            );
            let mut child = ui.new_child(
                UiBuilder::new()
                    .max_rect(region)
                    .layout(Layout::top_down(Align::Min)),
            );
            child.set_clip_rect(region);
            header(&mut child, collapsed);
            top += child.min_rect().height();
        }

        // Footer (pinned to bottom, auto height).
        if let Some(footer) = self.footer {
            let region = Rect::from_min_max(
                egui::pos2(inner.left(), top),
                egui::pos2(inner.right(), bottom),
            );
            let mut child = ui.new_child(
                UiBuilder::new()
                    .max_rect(region)
                    .layout(Layout::bottom_up(Align::Min)),
            );
            child.set_clip_rect(region);
            footer(&mut child, collapsed);
            // Separate the footer with a hairline.
            let fh = child.min_rect().height();
            bottom -= fh;
            if fh > 0.0 {
                ui.painter().hline(
                    inner.left()..=inner.right(),
                    bottom - PAD / 2.0,
                    Stroke::new(1.0, tokens.border.gamma_multiply(0.7)),
                );
            }
        }

        // Scrollable middle.
        let mid = Rect::from_min_max(
            egui::pos2(inner.left(), top + PAD / 2.0),
            egui::pos2(inner.right(), bottom - PAD / 2.0),
        );
        if mid.height() > 1.0 {
            let mut child = ui.new_child(
                UiBuilder::new()
                    .max_rect(mid)
                    .layout(Layout::top_down(Align::Min)),
            );
            child.set_clip_rect(mid);
            egui::ScrollArea::vertical()
                .id_salt(id.with("scroll"))
                .auto_shrink([false, false])
                .show(&mut child, |ui| {
                    ui.set_width(mid.width());
                    body(ui, collapsed);
                });
        }

        // Edge rail: a thin click-to-toggle strip on the panel's right edge.
        if self.rail && edge_rail(ui, tokens, rect, id) {
            collapsed = !collapsed;
            toggled = true;
        }

        ui.data_mut(|d| d.insert_temp(id, collapsed));
        SidebarResponse { collapsed, toggled }
    }
}

/// A ghost icon-button that toggles a sidebar; place it anywhere (e.g. in your
/// content header) and point it at the sidebar's `id_salt`.
///
/// ```no_run
/// use glazier::sidebar::SidebarTrigger;
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// SidebarTrigger::new("app").ui(ui);
/// # });
/// ```
#[must_use = "triggers do nothing unless added to a Ui"]
pub struct SidebarTrigger {
    id_salt: String,
}

impl SidebarTrigger {
    /// Create a trigger for the sidebar persisted under `id_salt`.
    pub fn new(id_salt: impl Into<String>) -> Self {
        Self {
            id_salt: id_salt.into(),
        }
    }
}

impl egui::Widget for SidebarTrigger {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let tokens = Tokens::get(ui);
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(TRIGGER_H * 0.8), Sense::click());
        paint_trigger(ui, tokens, rect, resp.hovered());
        if resp.clicked() {
            toggle(ui, &self.id_salt);
        }
        resp
    }
}

/// Render the built-in trigger inside the panel; returns its response.
fn trigger_button(ui: &Ui, tokens: Tokens, row: Rect) -> egui::Response {
    let btn = Rect::from_min_size(
        egui::pos2(row.left(), TRIGGER_H.mul_add(-0.4, row.center().y)),
        Vec2::splat(TRIGGER_H * 0.8),
    );
    let resp = ui
        .interact(btn, ui.id().with("sb-trigger"), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    paint_trigger(ui, tokens, btn, resp.hovered());
    resp
}

/// The click-to-toggle edge rail; returns whether it was clicked this frame.
fn edge_rail(ui: &Ui, tokens: Tokens, rect: Rect, id: Id) -> bool {
    let rail_rect = Rect::from_min_max(
        egui::pos2(rect.right() - RAIL_W / 2.0, rect.top()),
        egui::pos2(rect.right() + RAIL_W / 2.0, rect.bottom()),
    );
    let resp = ui
        .interact(rail_rect, id.with("rail"), Sense::click())
        .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
    let hot = ui
        .ctx()
        .animate_bool_with_time(id.with("rail-hot"), resp.hovered(), 0.12);
    if hot > 0.01 {
        ui.painter().vline(
            rect.right(),
            rect.top()..=rect.bottom(),
            Stroke::new(2.0, tokens.ring.gamma_multiply(hot)),
        );
    }
    resp.clicked()
}

/// Paint a ghost icon-button (accent fill on hover) with the panel-left glyph.
fn paint_trigger(ui: &Ui, tokens: Tokens, rect: Rect, hovered: bool) {
    let hot = ui
        .ctx()
        .animate_bool_with_time(rect.center().to_pos2_id(), hovered, 0.12);
    if hot > 0.01 {
        ui.painter()
            .rect_filled(rect, tokens.radius_md(), tokens.accent.gamma_multiply(hot));
    }
    let isz = 18.0;
    let irect = Rect::from_center_size(rect.center(), Vec2::splat(isz));
    Icon::new(PANEL_LEFT)
        .size(isz)
        .color(tokens.foreground)
        .image(tokens)
        .paint_at(ui, irect);
}

/// A tiny extension giving a stable [`Id`] from a position (for hover anim).
trait PosId {
    fn to_pos2_id(self) -> Id;
}

impl PosId for egui::Pos2 {
    fn to_pos2_id(self) -> Id {
        Id::new(("glazier-sb-trig", self.x.to_bits(), self.y.to_bits()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Collapsed state round-trips through memory via `toggle` / `is_collapsed`.
    #[test]
    fn toggle_round_trips() {
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            assert!(!is_collapsed(ui, "x"));
            toggle(ui, "x");
            assert!(is_collapsed(ui, "x"));
            toggle(ui, "x");
            assert!(!is_collapsed(ui, "x"));
        });
    }

    /// The shell renders (header + body + footer) without panicking.
    #[test]
    fn renders() {
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let r = Sidebar::new("app")
                .height(300.0)
                .header(|ui, c| {
                    if !c {
                        ui.label("Acme");
                    }
                })
                .footer(|ui, _| {
                    ui.label("me");
                })
                .show(ui, |ui, _| {
                    ui.label("Dashboard");
                    ui.label("Settings");
                });
            assert!(!r.collapsed);
        });
    }
}
