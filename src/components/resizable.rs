//! [`Resizable`] — two panes split by a draggable handle, mirroring shadcn's
//! `<ResizablePanelGroup>` + `<ResizableHandle>`.
//!
//! Lays out two child panels side by side (or stacked) with a thin `border`
//! divider between them. Dragging the divider repartitions the space; the split
//! fraction persists per-id across frames, so the layout is stateless from the
//! caller's side. The handle shows a resize cursor on hover and an optional
//! grip dot-row (shadcn's `withHandle`).

use egui::{Color32, CursorIcon, Pos2, Rect, Response, Sense, Ui, Vec2, Widget};

use crate::customize::{Customize, StyleHook};
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

impl Direction {
    /// A vector with `along` on the split axis and `cross` across it.
    const fn vec(self, along: f32, cross: f32) -> Vec2 {
        match self {
            Self::Horizontal => Vec2::new(along, cross),
            Self::Vertical => Vec2::new(cross, along),
        }
    }

    /// The split-axis component of `v`.
    const fn along(self, v: Vec2) -> f32 {
        match self {
            Self::Horizontal => v.x,
            Self::Vertical => v.y,
        }
    }

    /// The cross-axis component of `v`.
    const fn cross(self, v: Vec2) -> f32 {
        match self {
            Self::Horizontal => v.y,
            Self::Vertical => v.x,
        }
    }

    const fn cursor(self) -> CursorIcon {
        match self {
            Self::Horizontal => CursorIcon::ResizeHorizontal,
            Self::Vertical => CursorIcon::ResizeVertical,
        }
    }

    /// Cut `rect` into the first pane, the divider, and the second pane.
    fn split(self, rect: Rect, first: f32, divider: f32) -> [Rect; 3] {
        let cross = self.cross(rect.size());
        let second = self.along(rect.size()) - first - divider;
        let at = |offset: f32| rect.min + self.vec(offset, 0.0);
        [
            Rect::from_min_size(at(0.0), self.vec(first, cross)),
            Rect::from_min_size(at(first), self.vec(divider, cross)),
            Rect::from_min_size(at(first + divider), self.vec(second, cross)),
        ]
    }

    /// Paint the drag grip: a translucent pill across the divider holding
    /// three dots in a row along it.
    fn paint_grip(self, ui: &Ui, center: Pos2, color: Color32) {
        let bar = Rect::from_center_size(center, self.vec(10.0, 24.0));
        ui.painter()
            .rect_filled(bar, 4.0, color.gamma_multiply(0.30));
        for s in [-5.0_f32, 0.0, 5.0] {
            ui.painter()
                .circle_filled(center + self.vec(0.0, s), 1.5, color);
        }
    }
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
            divider: 2.0,
            hit: 12.0,
        }
    }
}

/// [`Resizable`]'s resolved paint — the divider hairline and grip colours.
/// The real value [`Resizable`] paints with; reach in via
/// [`Resizable::style`].
#[derive(Clone, Copy, Debug)]
pub struct ResizableStyle {
    /// Divider hairline colour (defaults to `border`). Pass
    /// [`Color32::TRANSPARENT`] to hide it entirely (useful when adjacent
    /// card borders already provide visual separation).
    pub divider: Color32,
    /// Grip colour while hovered or dragged (defaults to `foreground`).
    pub grip_active: Color32,
    /// Grip colour at rest (defaults to `muted_foreground`).
    pub grip_inactive: Color32,
}

/// A two-pane resizable split.
///
/// ```no_run
/// use glazier::resizable::{Resizable, ResizableStyle};
/// use glazier::Customize as _;
/// # egui::__run_test_ui(|ui| {
/// Resizable::new("demo").show(
///     ui,
///     |ui| { ui.label("Left"); },
///     |ui| { ui.label("Right"); },
/// );
///
/// // Reach into the resolved divider/grip colours via `.style` -- e.g. to
/// // hide the hairline when adjacent card borders already separate the panes.
/// Resizable::new("demo-hidden-divider")
///     .style(|s: &mut ResizableStyle| s.divider = egui::Color32::TRANSPARENT)
///     .show(
///         ui,
///         |ui| { ui.label("Left"); },
///         |ui| { ui.label("Right"); },
///     );
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
    style_hook: StyleHook<ResizableStyle>,
    sizing_hook: SizingHook<ResizableMetrics>,
}

impl Sizeable<ResizableMetrics> for Resizable {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<ResizableMetrics> {
        &mut self.sizing_hook
    }
}

impl Customize<ResizableStyle> for Resizable {
    fn style_hook_mut(&mut self) -> &mut StyleHook<ResizableStyle> {
        &mut self.style_hook
    }
}

impl Resizable {
    /// Create a split identified by `id_source` (persists the divider position).
    pub fn new(id_source: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id_source: egui::Id::unique(id_source),
            direction: Direction::default(),
            default_fraction: 0.5,
            min_fraction: 0.1,
            handle_grip: false,
            size: None,
            style_hook: StyleHook::default(),
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

    /// Render the two panes and the divider between them.
    pub fn show(
        mut self,
        ui: &mut Ui,
        first: impl FnOnce(&mut Ui),
        second: impl FnOnce(&mut Ui),
    ) -> Response {
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let style = self.resolve_style(ui);
        let id = ui.make_persistent_id(self.id_source);
        let dir = self.direction;

        let (rect, _) = ui.allocate_exact_size(self.block_size(ui), Sense::hover());
        let span = dir.along(rect.size()) - m.divider;
        let fraction = ui
            .ctx()
            .data_mut(|d| *d.get_temp_mut_or(id, self.default_fraction));
        let first_len = span * self.clamp(fraction);
        let [first_rect, divider, second_rect] = dir.split(rect, first_len, m.divider);

        // Hit-test before the panes render so dragging feels immediate; the
        // handle is painted after them (below).
        let hit = Rect::from_center_size(divider.center(), dir.vec(m.hit, dir.cross(rect.size())));
        let drag = ui.interact(hit, id.with("handle"), Sense::drag());
        let active = drag.hovered() || drag.dragged();
        if active {
            ui.ctx().set_cursor_icon(dir.cursor());
        }
        if drag.dragged() {
            let fraction = self.clamp((first_len + dir.along(drag.drag_delta())) / span);
            ui.ctx().data_mut(|d| d.insert_temp(id, fraction));
        }

        Self::pane(ui, first_rect, first);
        Self::pane(ui, second_rect, second);

        // Painted after both panes so an edge-to-edge opaque pane fill (e.g. a
        // `Card`) never covers the handle.
        ui.painter().rect_filled(divider, 0.0, style.divider);
        if self.handle_grip {
            let color = if active {
                style.grip_active
            } else {
                style.grip_inactive
            };
            dir.paint_grip(ui, divider.center(), color);
        }
        drag
    }

    /// The whole block: the available span along the split axis, and the
    /// fixed or available size across it. A horizontal split is never shorter
    /// than 120pt by default.
    fn block_size(&self, ui: &Ui) -> Vec2 {
        let dir = self.direction;
        let avail = ui.available_size();
        let floor = if dir == Direction::Horizontal {
            120.0
        } else {
            0.0
        };
        let cross = self.size.unwrap_or_else(|| dir.cross(avail).max(floor));
        dir.vec(dir.along(avail), cross)
    }

    fn clamp(&self, fraction: f32) -> f32 {
        fraction.clamp(self.min_fraction, 1.0 - self.min_fraction)
    }

    /// Run `content` in a top-down child clipped to `rect`, whatever the
    /// parent's layout.
    fn pane(ui: &mut Ui, rect: Rect, content: impl FnOnce(&mut Ui)) {
        let layout = egui::Layout::top_down(egui::Align::Min);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(layout));
        child.set_clip_rect(rect);
        content(&mut child);
    }

    /// Resolve [`ResizableStyle`]'s defaults against the tokens, then apply any
    /// pending [`Resizable::style`] hook.
    fn resolve_style(&mut self, ui: &Ui) -> ResizableStyle {
        let tokens = Tokens::get(ui);
        let mut style = ResizableStyle {
            divider: tokens.border,
            grip_active: tokens.foreground,
            grip_inactive: tokens.muted_foreground,
        };
        std::mem::take(&mut self.style_hook).apply(&mut style);
        style
    }
}

impl Widget for Resizable {
    /// Renders two empty panes. Use [`Resizable::show`] for content.
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui, |_| {}, |_| {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;

    /// Drag the handle by `delta` along the split axis and return the first
    /// pane's width share before and after.
    fn drag_by(dir: Direction, delta: f32) -> (f32, f32) {
        let ui = TestUi::new();
        let first = std::cell::Cell::new(Rect::NOTHING);
        let handle = std::cell::Cell::new(Rect::NOTHING);
        let mut build = |ui: &mut Ui| {
            let r = Resizable::new("split")
                .direction(dir)
                .size(200.0)
                .handle_grip(true)
                .show(ui, |ui| first.set(ui.max_rect()), |_| {});
            handle.set(r.rect);
        };
        ui.frame(&mut build);
        let before = dir.along(first.get().size());
        let from = handle.get().center();
        ui.drag(from, from + dir.vec(delta, 0.0), &mut build);
        ui.frame(&mut build);
        (before, dir.along(first.get().size()))
    }

    #[test]
    fn dragging_the_handle_moves_the_split() {
        for dir in [Direction::Horizontal, Direction::Vertical] {
            let (before, after) = drag_by(dir, 60.0);
            assert!(
                (after - before - 60.0).abs() < 1.0,
                "{dir:?}: {before} -> {after}"
            );
        }
    }

    #[test]
    fn the_split_respects_the_minimum() {
        // The default split starts at half the span.
        let (before, after) = drag_by(Direction::Horizontal, -1000.0);
        let span = before * 2.0;
        assert!(span.mul_add(-0.1, after).abs() < 0.5, "{after} of {span}");
    }

    #[test]
    fn split_tiles_the_block() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 50.0));
        let [a, d, b] = Direction::Vertical.split(rect, 20.0, 2.0);
        assert_eq!(
            (a.height(), d.top(), b.top(), b.height()),
            (20.0, 20.0, 22.0, 28.0)
        );
        assert_eq!(a.width(), 100.0);
    }
}
