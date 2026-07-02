//! [`Resizable`] — two panes split by a draggable handle, mirroring shadcn's
//! `<ResizablePanelGroup>` + `<ResizableHandle>`.
//!
//! Lays out two child panels side by side (or stacked) with a thin `border`
//! divider between them. Dragging the divider repartitions the space; the split
//! fraction persists per-id across frames, so the layout is stateless from the
//! caller's side. The handle shows a resize cursor on hover and an optional
//! grip dot-row (shadcn's `withHandle`).

use egui::{CursorIcon, Response, Sense, Ui, Vec2, Widget};

use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Split orientation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Direction {
    /// Panels side by side, divider vertical (shadcn `direction="horizontal"`).
    #[default]
    Horizontal,
    /// Panels stacked, divider horizontal (shadcn `direction="vertical"`).
    Vertical,
}

/// Overridable geometry for [`Resizable`] — reach in via
/// [`Resizable::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct ResizableMetrics {
    /// Divider thickness.
    pub divider: f32,
    /// Interactive hit-width straddling the divider.
    pub hit: f32,
}

impl Default for ResizableMetrics {
    fn default() -> Self {
        Self {
            divider: 1.0,
            hit: 11.0,
        }
    }
}

/// A two-pane resizable split.
///
/// ```no_run
/// use glazier::resizable::Resizable;
/// # egui::__run_test_ui(|ui| {
/// Resizable::new("demo").show(
///     ui,
///     |ui| { ui.label("Left"); },
///     |ui| { ui.label("Right"); },
/// );
/// # });
/// ```
#[must_use = "resizables do nothing unless shown"]
pub struct Resizable {
    id_source: egui::Id,
    direction: Direction,
    default_fraction: f32,
    min_fraction: f32,
    handle_grip: bool,
    size: Option<f32>,
    divider_color: Option<egui::Color32>,
    sizing_hook: SizingHook<ResizableMetrics>,
}

impl Sizeable<ResizableMetrics> for Resizable {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<ResizableMetrics> {
        &mut self.sizing_hook
    }
}

impl Resizable {
    /// Create a split identified by `id_source` (persists the divider position).
    pub fn new(id_source: impl std::hash::Hash) -> Self {
        Self {
            id_source: egui::Id::new(id_source),
            direction: Direction::default(),
            default_fraction: 0.5,
            min_fraction: 0.1,
            handle_grip: false,
            size: None,
            divider_color: None,
            sizing_hook: SizingHook::default(),
        }
    }

    /// Set the split [`Direction`].
    pub const fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// Initial fraction of space given to the first pane (default `0.5`).
    pub const fn default_fraction(mut self, fraction: f32) -> Self {
        self.default_fraction = fraction;
        self
    }

    /// Smallest fraction either pane can shrink to (default `0.1`).
    pub const fn min_fraction(mut self, fraction: f32) -> Self {
        self.min_fraction = fraction;
        self
    }

    /// Show a grip dot-row on the divider (shadcn's `withHandle`).
    pub const fn handle_grip(mut self, grip: bool) -> Self {
        self.handle_grip = grip;
        self
    }

    /// Fix the cross-axis size (height for horizontal splits, width for
    /// vertical). Defaults to the available space along that axis.
    pub const fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// Override the divider colour. Pass [`egui::Color32::TRANSPARENT`] to
    /// hide it entirely (useful when adjacent card borders already provide
    /// visual separation).
    pub const fn divider_color(mut self, color: egui::Color32) -> Self {
        self.divider_color = Some(color);
        self
    }

    /// Render the two panes and the divider between them.
    pub fn show(
        mut self,
        ui: &mut Ui,
        first: impl FnOnce(&mut Ui),
        second: impl FnOnce(&mut Ui),
    ) -> Response {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let id = ui.make_persistent_id(self.id_source);
        let horizontal = self.direction == Direction::Horizontal;

        // Total span along the split axis, and the fixed cross size.
        let avail = ui.available_size();
        let (along, cross) = if horizontal {
            (avail.x, self.size.unwrap_or_else(|| avail.y.max(120.0)))
        } else {
            (avail.y, self.size.unwrap_or(avail.x))
        };

        // Persisted split fraction.
        let mut fraction = ui
            .ctx()
            .data_mut(|d| *d.get_temp_mut_or(id, self.default_fraction));
        fraction = fraction.clamp(self.min_fraction, 1.0 - self.min_fraction);

        // Reserve the whole block.
        let size = if horizontal {
            Vec2::new(along, cross)
        } else {
            Vec2::new(cross, along)
        };
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());

        let first_len = (along - m.divider) * fraction;
        let second_len = (along - m.divider) - first_len;

        // Compute the three sub-rects.
        let (first_rect, divider_rect, second_rect) = if horizontal {
            let x0 = rect.left();
            let xd = x0 + first_len;
            (
                egui::Rect::from_min_size(rect.left_top(), Vec2::new(first_len, cross)),
                egui::Rect::from_min_size(egui::pos2(xd, rect.top()), Vec2::new(m.divider, cross)),
                egui::Rect::from_min_size(
                    egui::pos2(xd + m.divider, rect.top()),
                    Vec2::new(second_len, cross),
                ),
            )
        } else {
            let y0 = rect.top();
            let yd = y0 + first_len;
            (
                egui::Rect::from_min_size(rect.left_top(), Vec2::new(cross, first_len)),
                egui::Rect::from_min_size(egui::pos2(rect.left(), yd), Vec2::new(cross, m.divider)),
                egui::Rect::from_min_size(
                    egui::pos2(rect.left(), yd + m.divider),
                    Vec2::new(cross, second_len),
                ),
            )
        };

        // Interact over a wider hit-strip centred on the divider. Done
        // before the panes render so dragging feels immediate, but the
        // *painting* of the hairline/grip happens after (below) — a pane
        // with an edge-to-edge opaque fill (e.g. a `Card`) would otherwise
        // draw right over the handle, hiding it entirely.
        let hit = if horizontal {
            egui::Rect::from_center_size(divider_rect.center(), Vec2::new(m.hit, cross))
        } else {
            egui::Rect::from_center_size(divider_rect.center(), Vec2::new(cross, m.hit))
        };
        let drag = ui.interact(hit, id.with("handle"), Sense::drag());
        let cursor = if horizontal {
            CursorIcon::ResizeHorizontal
        } else {
            CursorIcon::ResizeVertical
        };
        if drag.hovered() || drag.dragged() {
            ui.ctx().set_cursor_icon(cursor);
        }
        if drag.dragged() {
            let delta = if horizontal {
                drag.drag_delta().x
            } else {
                drag.drag_delta().y
            };
            fraction = ((first_len + delta) / (along - m.divider))
                .clamp(self.min_fraction, 1.0 - self.min_fraction);
            ui.ctx().data_mut(|d| d.insert_temp(id, fraction));
        }

        // Render each pane clipped to its rect.
        // Always use top_down layout so pane content flows vertically
        // regardless of the parent layout (e.g. horizontal_top).
        let pane_layout = egui::Layout::top_down(egui::Align::Min);
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(first_rect)
                .layout(pane_layout),
        );
        child.set_clip_rect(first_rect);
        first(&mut child);

        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(second_rect)
                .layout(pane_layout),
        );
        child.set_clip_rect(second_rect);
        second(&mut child);

        // Paint the divider hairline and optional grip *after* both panes,
        // so an edge-to-edge opaque pane fill (e.g. a `Card`) never covers
        // the handle — draw calls issued later land on top within the same
        // layer.
        let divider_paint_color = self.divider_color.unwrap_or(tokens.border);
        ui.painter()
            .rect_filled(divider_rect, 0.0, divider_paint_color);
        if self.handle_grip {
            let active = drag.hovered() || drag.dragged();
            let color = if active {
                tokens.foreground
            } else {
                tokens.muted_foreground
            };
            paint_grip(ui, divider_rect.center(), horizontal, color);
        }

        drag
    }
}

/// Paint shadcn's `withHandle` grip: a tiny rounded bar straddling the divider
/// with a couple of dots, oriented across the split axis.
fn paint_grip(ui: &Ui, center: egui::Pos2, horizontal: bool, color: egui::Color32) {
    // The grip bar is a small rounded rect centred on the divider.
    let (w, h) = if horizontal {
        (10.0, 18.0)
    } else {
        (18.0, 10.0)
    };
    let bar = egui::Rect::from_center_size(center, Vec2::new(w, h));
    ui.painter()
        .rect_filled(bar, 3.0, color.gamma_multiply(0.18));
    // Two faint dots along the cross axis.
    for s in [-3.0_f32, 3.0] {
        let p = if horizontal {
            egui::pos2(center.x, center.y + s)
        } else {
            egui::pos2(center.x + s, center.y)
        };
        ui.painter().circle_filled(p, 1.0, color);
    }
}

impl Widget for Resizable {
    /// Renders two empty panes. Use [`Resizable::show`] for content.
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui, |_| {}, |_| {})
    }
}
