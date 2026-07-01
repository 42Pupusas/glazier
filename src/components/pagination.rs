//! [`Pagination`] — page navigation, mirroring shadcn's `<Pagination>`.
//!
//! A centered row of page controls: a *Previous* affordance, a run of numbered
//! page buttons with `…` ellipses collapsing the middle when there are many
//! pages, and a *Next* affordance. The current page is rendered as an
//! `outline` button; the rest are `ghost`, matching shadcn's
//! `isActive ? "outline" : "ghost"`. Prev/Next dim to non-interactive at the
//! ends.
//!
//! [`show`](Pagination::show) takes the current page by `&mut usize` (0-based),
//! updates it in place when the user navigates, and returns `true` on change.
//!
//! ```no_run
//! use glazier::pagination::Pagination;
//! # egui::__run_test_ui(|ui| {
//! # let mut page = 0usize;
//! if Pagination::new(10).show(ui, &mut page) {
//!     // page changed — refetch rows for `page`
//! }
//! # });
//! ```

use egui::{Response, Sense, Ui, Vec2, Widget};

use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry for [`Pagination`] — reach in via
/// [`Pagination::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct PaginationMetrics {
    /// Edge of each page button (shadcn `size-9`).
    pub btn: f32,
    /// Width of a prev/next button (icon + label).
    pub nav_w: f32,
    /// Gap between controls.
    pub gap: f32,
    /// Label / number text size.
    pub text_size: f32,
    /// How many pages to show before collapsing with ellipses.
    pub window: usize,
}

impl Default for PaginationMetrics {
    fn default() -> Self {
        Self {
            btn: 36.0,
            nav_w: 92.0,
            gap: 4.0,
            text_size: 13.0,
            window: 1,
        }
    }
}

/// One slot in the rendered control row.
enum Slot {
    /// A numbered page (0-based index).
    Page(usize),
    /// A non-interactive ellipsis gap.
    Ellipsis,
}

/// A pagination control over `total` pages.
#[must_use = "pagination does nothing unless shown"]
pub struct Pagination {
    total: usize,
    sizing_hook: SizingHook<PaginationMetrics>,
}

impl Sizeable<PaginationMetrics> for Pagination {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<PaginationMetrics> {
        &mut self.sizing_hook
    }
}

impl Pagination {
    /// Create a control spanning `total` pages.
    pub const fn new(total: usize) -> Self {
        Self {
            total,
            sizing_hook: SizingHook::new(),
        }
    }

    /// Render the control. `page` is the current 0-based page; it is clamped to
    /// range, updated in place on navigation, and `true` is returned if it
    /// changed this frame.
    pub fn show(mut self, ui: &mut Ui, page: &mut usize) -> bool {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let total = self.total.max(1);
        let current = (*page).min(total - 1);
        let mut next = current;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(m.gap, 0.0);

            // Previous.
            if nav_button(ui, tokens, "Previous", true, current > 0, m).clicked() {
                next = current.saturating_sub(1);
            }

            // Numbered slots with ellipses.
            for slot in slots(total, current, m.window) {
                match slot {
                    Slot::Page(p) => {
                        if page_button(ui, tokens, p, p == current, m).clicked() {
                            next = p;
                        }
                    }
                    Slot::Ellipsis => ellipsis(ui, tokens, m),
                }
            }

            // Next.
            if nav_button(ui, tokens, "Next", false, current + 1 < total, m).clicked() {
                next = (current + 1).min(total - 1);
            }
        });

        *page = next;
        next != current
    }
}

/// Compute the visible slot sequence: first and last page always show; a
/// `window` of pages around the current page shows; gaps collapse to
/// ellipses.
fn slots(total: usize, current: usize, window: usize) -> Vec<Slot> {
    // Few enough pages to show them all.
    if total <= window * 2 + 5 {
        return (0..total).map(Slot::Page).collect();
    }

    let mut out = Vec::new();
    let last = total - 1;
    let lo = current.saturating_sub(window);
    let hi = (current + window).min(last);

    out.push(Slot::Page(0));
    if lo > 1 {
        out.push(Slot::Ellipsis);
    }
    for p in lo.max(1)..=hi.min(last - 1) {
        out.push(Slot::Page(p));
    }
    if hi < last - 1 {
        out.push(Slot::Ellipsis);
    }
    out.push(Slot::Page(last));
    out
}

/// A numbered page button: `outline` chrome when active, `ghost` otherwise.
fn page_button(
    ui: &mut Ui,
    tokens: Tokens,
    page: usize,
    active: bool,
    m: PaginationMetrics,
) -> Response {
    let (id, rect) = ui.allocate_space(Vec2::splat(m.btn));
    let resp = ui
        .interact(rect, id.with(("page", page)), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    let hover_t = ui
        .ctx()
        .animate_bool_with_time(id.with(("hover", page)), resp.hovered(), 0.15);

    if active {
        // Opaque `card` surface — `background` is the (potentially
        // translucent) app canvas token, not a widget fill.
        ui.painter().rect(
            rect,
            tokens.radius_md(),
            tokens.card,
            egui::Stroke::new(1.0, tokens.border),
            egui::StrokeKind::Inside,
        );
    } else if hover_t > 0.01 {
        ui.painter().rect_filled(
            rect,
            tokens.radius_md(),
            tokens.accent.gamma_multiply(hover_t),
        );
    }

    let color = if active {
        tokens.foreground
    } else {
        tokens
            .muted_foreground
            .lerp_to_gamma(tokens.foreground, hover_t)
    };
    let galley = ui.painter().layout_no_wrap(
        format!("{}", page + 1),
        egui::FontId::proportional(m.text_size),
        color,
    );
    let pos = rect.center() - galley.size() / 2.0;
    ui.painter().galley(pos, galley, color);
    resp
}

/// A Previous/Next button: a chevron plus label. Dims and stops sensing when
/// `enabled` is false (at the respective end of the range).
fn nav_button(
    ui: &mut Ui,
    tokens: Tokens,
    label: &str,
    prev: bool,
    enabled: bool,
    m: PaginationMetrics,
) -> Response {
    let (id, rect) = ui.allocate_space(Vec2::new(m.nav_w, m.btn));
    let resp = if enabled {
        ui.interact(rect, id.with((label, "nav")), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        ui.interact(rect, id.with((label, "nav")), Sense::hover())
    };

    let hover_t = if enabled {
        ui.ctx()
            .animate_bool_with_time(id.with((label, "hover")), resp.hovered(), 0.15)
    } else {
        0.0
    };
    if hover_t > 0.01 {
        ui.painter().rect_filled(
            rect,
            tokens.radius_md(),
            tokens.accent.gamma_multiply(hover_t),
        );
    }

    let color = if enabled {
        tokens.foreground
    } else {
        tokens.muted_foreground.gamma_multiply(0.5)
    };

    // Lay out chevron + label centred as a group.
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(m.text_size),
        color,
    );
    let chevron_w = 10.0;
    let group_w = chevron_w + 6.0 + galley.size().x;
    let start_x = rect.center().x - group_w / 2.0;
    let cy = rect.center().y;

    let stroke = egui::Stroke::new(1.5, color);
    if prev {
        let cx = start_x + chevron_w / 2.0;
        chevron(ui, egui::pos2(cx, cy), chevron_w, true, stroke);
        let tx = start_x + chevron_w + 6.0;
        ui.painter()
            .galley(egui::pos2(tx, cy - galley.size().y / 2.0), galley, color);
    } else {
        ui.painter().galley(
            egui::pos2(start_x, cy - galley.size().y / 2.0),
            galley.clone(),
            color,
        );
        let cx = start_x + galley.size().x + 6.0 + chevron_w / 2.0;
        chevron(ui, egui::pos2(cx, cy), chevron_w, false, stroke);
    }
    resp
}

/// Draw a left/right chevron centred at `c` within a `w`-wide box.
fn chevron(ui: &Ui, c: egui::Pos2, w: f32, left: bool, stroke: egui::Stroke) {
    let h = w * 0.5;
    let dx = if left { w * 0.25 } else { -w * 0.25 };
    let tip = egui::pos2(c.x - dx, c.y);
    let top = egui::pos2(c.x + dx, c.y - h);
    let bot = egui::pos2(c.x + dx, c.y + h);
    ui.painter().line_segment([top, tip], stroke);
    ui.painter().line_segment([tip, bot], stroke);
}

/// Draw a non-interactive `…` gap at page-button width.
fn ellipsis(ui: &mut Ui, tokens: Tokens, m: PaginationMetrics) {
    let (_, rect) = ui.allocate_space(Vec2::splat(m.btn));
    let c = rect.center();
    let step = 5.0;
    for k in [-1.0_f32, 0.0, 1.0] {
        ui.painter().circle_filled(
            egui::pos2(k.mul_add(step, c.x), c.y),
            1.3,
            tokens.muted_foreground,
        );
    }
}

impl Widget for Pagination {
    /// Renders with an internal, frame-local page of 0 and discards changes.
    /// Prefer [`Pagination::show`] to bind real state.
    fn ui(self, ui: &mut Ui) -> Response {
        let mut page = 0;
        ui.scope(|ui| {
            self.show(ui, &mut page);
        })
        .response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// First page renders and reports no change without input.
    #[test]
    fn stable_without_input() {
        let ctx = egui::Context::default();
        let mut changed = true;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let mut page = 0;
            changed = Pagination::new(12).show(ui, &mut page);
        });
        assert!(!changed);
    }

    /// Small page counts show every page; large ones collapse with ellipses.
    #[test]
    fn slots_collapse() {
        let small = slots(5, 0, 1);
        assert_eq!(small.len(), 5);
        assert!(small.iter().all(|s| matches!(s, Slot::Page(_))));

        let big = slots(20, 10, 1);
        assert!(big.iter().any(|s| matches!(s, Slot::Ellipsis)));
        // First and last are always pages.
        assert!(matches!(big.first(), Some(Slot::Page(0))));
        assert!(matches!(big.last(), Some(Slot::Page(19))));
    }

    /// An out-of-range page is clamped rather than panicking.
    #[test]
    fn clamps_overflow() {
        let ctx = egui::Context::default();
        let mut page = 999;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            Pagination::new(5).show(ui, &mut page);
        });
        assert!(page < 5);
    }
}
