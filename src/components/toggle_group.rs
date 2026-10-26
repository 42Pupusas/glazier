//! [`ToggleGroup`] — a segmented set of toggles, mirroring shadcn's
//! `<ToggleGroup>`.
//!
//! A joined row of [`Toggle`](crate::Toggle)-style segments sharing one border,
//! with rounded outer corners and hairline dividers between items. Supports two
//! selection models: [`single`](ToggleGroup::single) (one index, like a radio)
//! and [`multiple`](ToggleGroup::multiple) (a set of pressed indices).

use std::collections::BTreeSet;
use std::sync::Arc;

use egui::{Galley, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};

use super::segments::Segments;
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry/timing for [`ToggleGroup`] — reach in via
/// [`ToggleGroup::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct ToggleGroupMetrics {
    /// Seconds for the per-segment on/hover transition.
    pub toggle_time: f32,
    /// Per-segment horizontal padding — matches `ButtonGroup` (`px-3`).
    pub pad_x: f32,
    /// Row height — matches `ButtonGroup` (`h-8`).
    pub height: f32,
}

impl Default for ToggleGroupMetrics {
    fn default() -> Self {
        Self {
            toggle_time: 0.15,
            pad_x: 12.0,
            height: 32.0,
        }
    }
}

/// Selection model for a [`ToggleGroup`].
enum Selection<'a> {
    /// Exactly one (or none, when the active item is re-clicked) — `usize::MAX`
    /// encodes "nothing selected".
    Single(&'a mut usize),
    /// Any subset of indices.
    Multiple(&'a mut BTreeSet<usize>),
}

impl Selection<'_> {
    fn is_on(&self, i: usize) -> bool {
        match self {
            Self::Single(sel) => **sel == i,
            Self::Multiple(set) => set.contains(&i),
        }
    }

    /// Flip segment `i`. A single selection clears when its own item is
    /// clicked again.
    fn toggle(&mut self, i: usize) {
        match self {
            Self::Single(sel) => **sel = if **sel == i { usize::MAX } else { i },
            Self::Multiple(set) => {
                if !set.remove(&i) {
                    set.insert(i);
                }
            }
        }
    }
}

/// A segmented group of mutually-joined toggles.
///
/// ```no_run
/// use glazier::toggle_group::ToggleGroup;
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// let mut align = 0usize;
/// ToggleGroup::single(&mut align, ["Left", "Center", "Right"]).ui(ui);
/// # });
/// ```
#[must_use = "toggle groups do nothing unless you add them to a Ui"]
pub struct ToggleGroup<'a> {
    selection: Selection<'a>,
    options: Vec<String>,
    sizing_hook: SizingHook<ToggleGroupMetrics>,
}

impl Sizeable<ToggleGroupMetrics> for ToggleGroup<'_> {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<ToggleGroupMetrics> {
        &mut self.sizing_hook
    }
}

impl<'a> ToggleGroup<'a> {
    /// Single-selection group bound to `selected` (the chosen index; set to
    /// `usize::MAX` for "none"). Re-clicking the active item clears it.
    pub fn single<I, S>(selected: &'a mut usize, options: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            selection: Selection::Single(selected),
            options: options.into_iter().map(Into::into).collect(),
            sizing_hook: SizingHook::default(),
        }
    }

    /// Multi-selection group bound to a set of pressed indices.
    pub fn multiple<I, S>(selected: &'a mut BTreeSet<usize>, options: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            selection: Selection::Multiple(selected),
            options: options.into_iter().map(Into::into).collect(),
            sizing_hook: SizingHook::default(),
        }
    }
}

impl Widget for ToggleGroup<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let row = Row::new(ui, &self.options, m);
        let (rect, response) = ui.allocate_at_least(row.size(), Sense::hover());
        // The layout may stretch `rect` wider than the segments need (e.g. in a
        // column); pin geometry to the intrinsic size so the group hugs its
        // content and stays left-aligned instead of covering the row.
        let bounds = Rect::from_min_size(rect.left_top(), row.size());
        let clicked = ui
            .is_rect_visible(bounds)
            .then(|| row.show(ui, bounds, &response, &self.selection))
            .flatten();
        if let Some(i) = clicked {
            self.selection.toggle(i);
        }
        response
    }
}

/// One frame of the group: its laid-out labels and their widths.
struct Row {
    tokens: Tokens,
    m: ToggleGroupMetrics,
    galleys: Vec<Arc<Galley>>,
    widths: Vec<f32>,
}

impl Row {
    fn new(ui: &Ui, options: &[String], m: ToggleGroupMetrics) -> Self {
        // Laid out with `PLACEHOLDER` so the per-segment text colour passed to
        // `painter.galley` applies; a galley laid out with an explicit colour
        // ignores the paint-time override (which left selected labels black on
        // the black `primary` chip in light mode). Semibold 14 px matches
        // `ButtonGroup`'s `font-medium`.
        let font = crate::fonts::semibold(ui, 14.0);
        let galleys: Vec<_> = options
            .iter()
            .map(|label| {
                ui.painter()
                    .layout_no_wrap(label.clone(), font.clone(), egui::Color32::PLACEHOLDER)
            })
            .collect();
        let widths = galleys
            .iter()
            .map(|g| m.pad_x.mul_add(2.0, g.size().x))
            .collect();
        Self {
            tokens: Tokens::get(ui),
            m,
            galleys,
            widths,
        }
    }

    fn size(&self) -> Vec2 {
        Vec2::new(self.widths.iter().sum(), self.m.height)
    }

    /// Paint the border and every segment. Returns the clicked segment.
    fn show(
        &self,
        ui: &Ui,
        bounds: Rect,
        response: &Response,
        selection: &Selection<'_>,
    ) -> Option<usize> {
        // `ButtonGroup`'s radius, so both families read as the same pill.
        let radius = self.tokens.radius_2xl();
        ui.painter().rect_stroke(
            bounds,
            radius,
            Stroke::new(1.0, self.tokens.border),
            egui::StrokeKind::Inside,
        );
        let mut x = bounds.left();
        let mut clicked = None;
        for (i, w) in self.widths.iter().enumerate() {
            let seg =
                Rect::from_min_size(egui::pos2(x, bounds.top()), Vec2::new(*w, self.m.height));
            if self.segment(ui, i, seg, response, selection.is_on(i)) {
                clicked = Some(i);
            }
            x += w;
        }
        clicked
    }

    /// Hit-test and paint segment `i`. Returns `true` when clicked.
    fn segment(&self, ui: &Ui, i: usize, seg: Rect, response: &Response, selected: bool) -> bool {
        let (tokens, time) = (self.tokens, self.m.toggle_time);
        let seg_resp = ui.interact(seg, response.id.with(i), Sense::click());
        let ctx = ui.ctx();
        let hover_t =
            ctx.animate_bool_with_time(response.id.with(("hover", i)), seg_resp.hovered(), time);
        // Selection fades from the segment's own unselected look (the opaque
        // `widget` surface plus any hover `accent`) straight to `primary`: one
        // solid-to-solid lerp, so there is no neutral midpoint to stall on.
        let on_t = ctx.animate_bool_with_time(response.id.with(("on", i)), selected, time);
        if on_t > 0.01 || hover_t > 0.01 {
            let fill = tokens
                .widget
                .lerp_to_gamma(tokens.accent, hover_t)
                .lerp_to_gamma(tokens.primary, on_t);
            // Inset 1px with outer corners rounded so the fill never pokes past
            // the container's rounded border.
            let row = Segments {
                count: self.widths.len(),
            };
            let cr = row.radius(i, tokens.radius_2xl().saturating_sub(1));
            ui.painter().rect_filled(seg.shrink(1.0), cr, fill);
        }
        if i > 0 {
            ui.painter().vline(
                seg.left(),
                seg.top()..=seg.bottom(),
                Stroke::new(1.0, tokens.border),
            );
        }
        // The label fades to `primary_foreground` in step with the chip.
        let text_col = tokens
            .foreground
            .lerp_to_gamma(tokens.primary_foreground, on_t);
        let g = &self.galleys[i];
        ui.painter()
            .galley(seg.center() - g.size() / 2.0, g.clone(), text_col);
        seg_resp.clicked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;

    const OPTIONS: [&str; 3] = ["Left", "Center", "Right"];

    /// Click the label `text` once the group has been drawn.
    fn click(ui: &TestUi, text: &str, mut build: impl FnMut(&mut Ui)) {
        let out = ui.frame(&mut build);
        ui.click(TestUi::text_rect(&out, text).center(), &mut build);
    }

    #[test]
    fn single_selects_and_reclick_clears() {
        let ui = TestUi::new();
        let mut selected = 0;
        click(&ui, "Right", |ui| {
            ToggleGroup::single(&mut selected, OPTIONS).ui(ui);
        });
        assert_eq!(selected, 2);
        click(&ui, "Right", |ui| {
            ToggleGroup::single(&mut selected, OPTIONS).ui(ui);
        });
        assert_eq!(selected, usize::MAX);
    }

    #[test]
    fn multiple_toggles_membership() {
        let ui = TestUi::new();
        let mut set = BTreeSet::from([0]);
        click(&ui, "Center", |ui| {
            ToggleGroup::multiple(&mut set, OPTIONS).ui(ui);
        });
        assert_eq!(set, BTreeSet::from([0, 1]));
        click(&ui, "Left", |ui| {
            ToggleGroup::multiple(&mut set, OPTIONS).ui(ui);
        });
        assert_eq!(set, BTreeSet::from([1]));
    }
}
