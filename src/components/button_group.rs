//! [`ButtonGroup`] — horizontally joined buttons, mirroring shadcn's
//! `<ButtonGroup>`.
//!
//! Segments sit flush against each other: only the outer corners are rounded,
//! inner borders collapse into a single shared hairline. Returns the index of
//! the clicked segment, if any.
//!
//! Unlike composing bare [`Button`](crate::button::Button)s, a group owns its
//! segments so it can suppress the inner corner radii — egui buttons always
//! round all four corners, so joining them requires drawing the row as a unit.

use std::sync::Arc;

use egui::{Color32, Galley, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget};

use super::segments::Segments;
use crate::components::dropdown_menu::DropdownMenu;
use crate::components::icon::Icon;
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry for [`ButtonGroup`] — reach in via
/// [`ButtonGroup::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct ButtonGroupMetrics {
    /// Inline icon edge length (shadcn `size-4`).
    pub icon_size: f32,
    /// Gap between a label and its icon (shadcn `gap-1.5`).
    pub icon_gap: f32,
    /// Segment row height (`h-8`).
    pub height: f32,
    /// Horizontal padding inside each segment.
    pub pad_x: f32,
}

impl Default for ButtonGroupMetrics {
    fn default() -> Self {
        Self {
            icon_size: 16.0,
            icon_gap: 6.0,
            height: 32.0,
            pad_x: 12.0,
        }
    }
}

/// Visual style of a [`ButtonGroup`]'s segments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    /// Hairline-bordered segments on the `background` (the shadcn default for
    /// grouped actions).
    #[default]
    Outline,
    /// Muted filled segments.
    Secondary,
}

impl Variant {
    /// A segment's `(fill, text)` colours.
    fn colors(self, tokens: Tokens, hovered: bool) -> (Color32, Color32) {
        match self {
            Self::Outline if hovered => (tokens.muted, tokens.foreground),
            // Opaque `widget`, not `background` — the app-canvas token can be
            // translucent under a user theme.
            Self::Outline => (tokens.widget, tokens.foreground),
            Self::Secondary if hovered => (
                tokens.secondary.gamma_multiply(0.9),
                tokens.secondary_foreground,
            ),
            Self::Secondary => (tokens.secondary, tokens.secondary_foreground),
        }
    }
}

/// Outcome of showing a [`ButtonGroup`].
pub struct ButtonGroupResponse {
    /// The group's overall response (covers the whole row).
    pub response: Response,
    /// Index of the segment that was clicked, if any.
    pub clicked: Option<usize>,
    /// Index of the dropdown item picked from the last segment's menu, if any.
    pub menu_selected: Option<usize>,
}

/// One segment's content.
#[derive(Clone, Default)]
struct Segment {
    label: String,
    icon_start: Option<Icon>,
    icon_end: Option<Icon>,
    disabled: bool,
}

/// A segment laid out for this frame.
struct Laid {
    galley: Arc<Galley>,
    icon_start: Option<Icon>,
    icon_end: Option<Icon>,
    disabled: bool,
}

impl Laid {
    fn has_label(&self) -> bool {
        self.galley.size().x > 0.0
    }

    /// Room an icon takes, including its gap to the label when there is one.
    fn icon_w(&self, icon: Option<Icon>, m: ButtonGroupMetrics) -> f32 {
        match icon {
            Some(_) if self.has_label() => m.icon_size + m.icon_gap,
            Some(_) => m.icon_size,
            None => 0.0,
        }
    }

    /// Width of the icon + label + icon run.
    fn content_w(&self, m: ButtonGroupMetrics) -> f32 {
        self.icon_w(self.icon_start, m) + self.galley.size().x + self.icon_w(self.icon_end, m)
    }

    /// Centre the run inside `seg`.
    fn paint(&self, ui: &Ui, seg: Rect, color: Color32, tokens: Tokens, m: ButtonGroupMetrics) {
        let cy = seg.center().y;
        let mut x = seg.center().x - self.content_w(m) / 2.0;
        let icon = |icon: Icon, x: f32| {
            let r = Rect::from_min_size(
                egui::pos2(x, cy - m.icon_size / 2.0),
                Vec2::splat(m.icon_size),
            );
            icon.color(color).image(tokens).paint_at(ui, r);
        };
        if let Some(i) = self.icon_start {
            icon(i, x);
            x += self.icon_w(self.icon_start, m);
        }
        if self.has_label() {
            let pos = egui::pos2(x, cy - self.galley.size().y / 2.0);
            ui.painter().galley(pos, self.galley.clone(), color);
            x += self.galley.size().x;
        }
        if let Some(i) = self.icon_end {
            icon(i, x + self.icon_w(self.icon_end, m) - m.icon_size);
        }
    }
}

/// A row of joined buttons.
///
/// ```no_run
/// use glazier::button_group::ButtonGroup;
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// let picked = ButtonGroup::new(["Day", "Week", "Month"]).show(ui).clicked;
/// # });
/// ```
#[must_use = "button groups do nothing unless you add them to a Ui"]
pub struct ButtonGroup {
    segments: Vec<Segment>,
    menu: Option<DropdownMenu>,
    variant: Variant,
    /// Stretch to fill the available width, distributing space evenly.
    full_width: bool,
    sizing_hook: SizingHook<ButtonGroupMetrics>,
}

impl Sizeable<ButtonGroupMetrics> for ButtonGroup {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<ButtonGroupMetrics> {
        &mut self.sizing_hook
    }
}

impl ButtonGroup {
    /// Create a group from the given segment labels.
    pub fn new<I, S>(labels: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            segments: labels
                .into_iter()
                .map(|label| Segment {
                    label: label.into(),
                    ..Segment::default()
                })
                .collect(),
            menu: None,
            variant: Variant::default(),
            full_width: false,
            sizing_hook: SizingHook::default(),
        }
    }

    /// Attach a [`DropdownMenu`] to the last segment (e.g. the chevron
    /// trigger). Clicking that segment opens the menu; the picked item index is
    /// reported via [`ButtonGroupResponse::menu_selected`].
    pub fn menu(mut self, menu: DropdownMenu) -> Self {
        self.menu = Some(menu);
        self
    }

    /// Append a label-only segment (no icon).
    pub fn segment(mut self, label: impl Into<String>) -> Self {
        self.segments.push(Segment {
            label: label.into(),
            ..Segment::default()
        });
        self
    }

    /// Append an icon-only segment (e.g. a lucide chevron dropdown trigger).
    pub fn icon_segment(mut self, icon: Icon) -> Self {
        self.segments.push(Segment {
            icon_end: Some(icon),
            ..Segment::default()
        });
        self
    }

    /// Attach a trailing [`Icon`] to the last segment.
    pub fn icon_end(mut self, icon: Icon) -> Self {
        if let Some(s) = self.segments.last_mut() {
            s.icon_end = Some(icon);
        }
        self
    }

    /// Attach a leading [`Icon`] to the last segment.
    pub fn icon_start(mut self, icon: Icon) -> Self {
        if let Some(s) = self.segments.last_mut() {
            s.icon_start = Some(icon);
        }
        self
    }

    /// Mark the last segment as disabled (non-interactive, muted).
    pub fn disable_last(mut self) -> Self {
        if let Some(s) = self.segments.last_mut() {
            s.disabled = true;
        }
        self
    }

    /// Set the segment [`Variant`].
    pub const fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Stretch the group to fill the available width, distributing space
    /// evenly across all segments.
    pub const fn full_width(mut self, full_width: bool) -> Self {
        self.full_width = full_width;
        self
    }

    /// Render the group, returning which segment was clicked and which dropdown
    /// item (if any) was picked.
    pub fn show(mut self, ui: &mut Ui) -> ButtonGroupResponse {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let laid = self.lay_out(ui);
        let widths = self.widths(ui, &laid, m);
        let size = Vec2::new(widths.iter().sum(), m.height);
        let (rect, response) = ui.allocate_at_least(size, Sense::hover());

        let row = Segments { count: laid.len() };
        let mut clicked = None;
        let mut x = rect.left();
        let mut last = None;
        for (i, (seg, w)) in laid.iter().zip(&widths).enumerate() {
            let r = Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::new(*w, m.height));
            let sense = if seg.disabled {
                Sense::hover()
            } else {
                Sense::click()
            };
            let resp = ui.interact(r, response.id.with(i), sense);
            if resp.clicked() {
                clicked = Some(i);
            }
            if ui.is_rect_visible(r) {
                let (fill, text) = if seg.disabled {
                    (tokens.widget, tokens.muted_foreground)
                } else {
                    self.variant.colors(tokens, resp.hovered())
                };
                let border = Stroke::new(1.0, tokens.border);
                let radius = row.radius(i, tokens.radius_2xl());
                ui.painter()
                    .rect(r, radius, fill, border, StrokeKind::Inside);
                seg.paint(ui, r, text, tokens, m);
            }
            x += w;
            last = Some(resp);
        }

        let menu_selected = self
            .menu
            .as_ref()
            .zip(last.as_ref())
            .and_then(|(menu, trigger)| Self::show_menu(menu, trigger, response.id, tokens));
        ButtonGroupResponse {
            response,
            clicked,
            menu_selected,
        }
    }

    /// Lay out each label in the semibold face shadcn's `font-medium`
    /// buttons use.
    fn lay_out(&self, ui: &Ui) -> Vec<Laid> {
        let font = crate::fonts::semibold(ui, 14.0);
        self.segments
            .iter()
            .map(|s| Laid {
                galley: ui.painter().layout_no_wrap(
                    s.label.clone(),
                    font.clone(),
                    Color32::PLACEHOLDER,
                ),
                icon_start: s.icon_start,
                icon_end: s.icon_end,
                disabled: s.disabled,
            })
            .collect()
    }

    /// Each segment's width: padded content, scaled up evenly when the group
    /// stretches to fill the row.
    fn widths(&self, ui: &Ui, laid: &[Laid], m: ButtonGroupMetrics) -> Vec<f32> {
        let natural: Vec<f32> = laid
            .iter()
            .map(|s| m.pad_x.mul_add(2.0, s.content_w(m)))
            .collect();
        let total: f32 = natural.iter().sum();
        let scale = if self.full_width && total > 0.0 {
            ui.available_width().max(total) / total
        } else {
            1.0
        };
        natural.into_iter().map(|w| w * scale).collect()
    }

    /// The dropdown hung off the last (chevron) segment. Returns the picked
    /// item.
    fn show_menu(
        menu: &DropdownMenu,
        trigger: &Response,
        id: egui::Id,
        tokens: Tokens,
    ) -> Option<usize> {
        // shadcn opens this menu upward, right-aligned (`data-side="top"
        // data-align="end"`), with a small offset.
        egui::Popup::menu(trigger)
            .id(id.with("menu"))
            .frame(DropdownMenu::frame(tokens))
            .align(egui::RectAlign::TOP_END)
            .gap(4.0)
            .show(|ui| menu.show_contents(ui, tokens))
            .and_then(|r| r.inner)
    }
}

impl Widget for ButtonGroup {
    /// Renders the group and discards the click index. Use [`ButtonGroup::show`]
    /// to learn which segment was clicked.
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;

    const DOT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="4" fill="currentColor"/></svg>"#;

    fn group(disabled: bool) -> ButtonGroup {
        let g = ButtonGroup::new(["Day", "Week"])
            .segment("Month")
            .icon_start(Icon::new(DOT))
            .icon_end(Icon::new(DOT))
            .icon_segment(Icon::new(DOT));
        if disabled {
            g.disable_last()
        } else {
            g
        }
    }

    /// Click the last segment's right edge; returns the reported click.
    fn click_last(disabled: bool) -> Option<usize> {
        let ui = TestUi::new();
        let right = std::cell::Cell::new(0.0);
        let mut clicked = None;
        let mut build = |ui: &mut Ui| {
            let out = group(disabled).show(ui);
            right.set(out.response.rect.right());
            clicked = out.clicked.or(clicked);
        };
        ui.frame(&mut build);
        ui.click(egui::pos2(right.get() - 4.0, 16.0), &mut build);
        drop(build);
        clicked
    }

    #[test]
    fn clicking_a_segment_reports_it() {
        let ui = TestUi::new();
        let mut clicked = None;
        let mut build = |ui: &mut Ui| {
            clicked = group(false)
                .variant(Variant::Secondary)
                .show(ui)
                .clicked
                .or(clicked);
        };
        let out = ui.frame(&mut build);
        ui.click(TestUi::text_rect(&out, "Week").center(), &mut build);
        drop(build);
        assert_eq!(clicked, Some(1));
    }

    #[test]
    fn disabled_segment_ignores_clicks() {
        assert_eq!(click_last(false), Some(3));
        assert_eq!(click_last(true), None);
    }

    #[test]
    fn full_width_stretches_to_the_row() {
        let ui = TestUi::new();
        let mut widths = (0.0, 0.0);
        ui.frame(|ui| {
            let natural = ButtonGroup::new(["A", "B"]).ui(ui).rect.width();
            let full = ButtonGroup::new(["A", "B"])
                .full_width(true)
                .ui(ui)
                .rect
                .width();
            widths = (natural, full);
        });
        assert!(widths.1 > widths.0);
    }
}
