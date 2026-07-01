//! [`ToggleGroup`] — a segmented set of toggles, mirroring shadcn's
//! `<ToggleGroup>`.
//!
//! A joined row of [`Toggle`](crate::Toggle)-style segments sharing one border,
//! with rounded outer corners and hairline dividers between items. Supports two
//! selection models: [`single`](ToggleGroup::single) (one index, like a radio)
//! and [`multiple`](ToggleGroup::multiple) (a set of pressed indices).

use std::collections::BTreeSet;

use egui::{CornerRadius, Response, Sense, Stroke, Ui, Vec2, Widget};

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
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let ToggleGroup {
            selection, options, ..
        } = self;

        // Measure each segment so the row width is known up front. Lay the text
        // out with `PLACEHOLDER` as its colour so the per-segment `text_col`
        // passed to `painter.galley` below actually applies — a galley laid out
        // with an explicit colour ignores the paint-time override, which is what
        // left selected segments rendering `foreground` (black on the black
        // `primary` chip in light mode).
        //
        // Use semibold 14 px to match `ButtonGroup`'s `font-medium` weight —
        // `TextStyle::Button` resolves to the regular face and looks lighter.
        let font = crate::fonts::semibold(ui, 14.0);
        let galleys: Vec<_> = options
            .iter()
            .map(|label| {
                ui.painter()
                    .layout_no_wrap(label.clone(), font.clone(), egui::Color32::PLACEHOLDER)
            })
            .collect();
        let widths: Vec<f32> = galleys
            .iter()
            .map(|g| m.pad_x.mul_add(2.0, g.size().x))
            .collect();
        let total_w: f32 = widths.iter().sum();

        let (rect, response) = ui.allocate_at_least(Vec2::new(total_w, m.height), Sense::hover());
        // The layout may stretch `rect` wider than the segments need (e.g. in a
        // column); pin all geometry to the intrinsic `total_w` so the group
        // hugs its content and stays left-aligned instead of covering the row.
        let bounds = egui::Rect::from_min_size(rect.left_top(), Vec2::new(total_w, m.height));
        // Use the same large radius as `ButtonGroup` so both widget families
        // read as visually consistent pill-shaped controls.
        let radius = tokens.radius_2xl();

        let is_on = |i: usize| match &selection {
            Selection::Single(sel) => **sel == i,
            Selection::Multiple(set) => set.contains(&i),
        };

        // Hit-test + paint each segment.
        let mut clicked: Option<usize> = None;
        if ui.is_rect_visible(bounds) {
            let painter = ui.painter();
            // Outer container border.
            painter.rect_stroke(
                bounds,
                radius,
                Stroke::new(1.0, tokens.border),
                egui::StrokeKind::Inside,
            );

            let mut x = bounds.left();
            for (i, w) in widths.iter().enumerate() {
                let seg =
                    egui::Rect::from_min_size(egui::pos2(x, bounds.top()), Vec2::new(*w, m.height));
                let seg_resp = ui.interact(seg, response.id.with(i), Sense::click());
                if seg_resp.clicked() {
                    clicked = Some(i);
                }

                let hover_t = ui.ctx().animate_bool_with_time(
                    response.id.with(("hover", i)),
                    seg_resp.hovered(),
                    m.toggle_time,
                );
                // Selection eases in/out via its own time-based 0→1 (`on_t`)
                // so the `primary` chip fades between segments instead of
                // snapping. We blend *from the segment's own unselected look*
                // (the opaque `card` surface, plus any hover `accent` lift)
                // straight to `primary` — a single solid→solid lerp that
                // always completes, so there's no neutral midpoint to get
                // stuck on. `card`, not `background`, since the latter is the
                // app-canvas token and may be translucent under a user theme.
                let selected = is_on(i);
                let on_t = ui.ctx().animate_bool_with_time(
                    response.id.with(("on", i)),
                    selected,
                    m.toggle_time,
                );
                let base = tokens.card.lerp_to_gamma(tokens.accent, hover_t);
                let fill = base.lerp_to_gamma(tokens.primary, on_t);
                if on_t > 0.01 || hover_t > 0.01 {
                    // Round the outer corners so the fill never pokes past the
                    // container's rounded border; inset by 1px to sit inside it.
                    let fill_rect = seg.shrink(1.0);
                    let cr = segment_radius(i, widths.len(), radius.saturating_sub(1));
                    painter.rect_filled(fill_rect, cr, fill);
                }

                // Divider before every segment except the first.
                if i > 0 {
                    painter.vline(
                        x,
                        bounds.top()..=bounds.bottom(),
                        Stroke::new(1.0, tokens.border),
                    );
                }

                let g = &galleys[i];
                // Label fades from normal foreground to primary-foreground in
                // lockstep with the chip.
                let text_col = tokens
                    .foreground
                    .lerp_to_gamma(tokens.primary_foreground, on_t);
                let pos = egui::pos2(
                    seg.center().x - g.size().x / 2.0,
                    seg.center().y - g.size().y / 2.0,
                );
                painter.galley(pos, g.clone(), text_col);

                x += w;
            }
        }

        if let Some(i) = clicked {
            match selection {
                Selection::Single(sel) => {
                    *sel = if *sel == i { usize::MAX } else { i };
                }
                Selection::Multiple(set) => {
                    if !set.remove(&i) {
                        set.insert(i);
                    }
                }
            }
        }

        response
    }
}

/// Round only the outer corners of segment `i` of `n`: the first segment rounds
/// its left side, the last its right side, interior segments stay square — so a
/// selected end segment's fill follows the container's rounded border instead of
/// poking past it.
const fn segment_radius(i: usize, n: usize, r: u8) -> CornerRadius {
    let first = i == 0;
    let last = i + 1 == n;
    CornerRadius {
        nw: if first { r } else { 0 },
        sw: if first { r } else { 0 },
        ne: if last { r } else { 0 },
        se: if last { r } else { 0 },
    }
}
