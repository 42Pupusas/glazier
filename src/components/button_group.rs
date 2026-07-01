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

use egui::{CornerRadius, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget};

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

/// A row of joined buttons.
///
/// ```no_run
/// use glazier::button_group::ButtonGroup;
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// let picked = ButtonGroup::new(["Day", "Week", "Month"]).show(ui).clicked;
/// # });
/// ```
/// Outcome of showing a [`ButtonGroup`].
pub struct ButtonGroupResponse {
    /// The group's overall response (covers the whole row).
    pub response: Response,
    /// Index of the segment that was clicked, if any.
    pub clicked: Option<usize>,
    /// Index of the dropdown item picked from the last segment's menu, if any.
    pub menu_selected: Option<usize>,
}

#[must_use = "button groups do nothing unless you add them to a Ui"]
pub struct ButtonGroup {
    labels: Vec<String>,
    /// Trailing icon per segment.
    icons: Vec<Option<Icon>>,
    /// Leading icon per segment.
    icons_start: Vec<Option<Icon>>,
    /// Whether a segment is disabled.
    disabled: Vec<bool>,
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
        let labels: Vec<String> = labels.into_iter().map(Into::into).collect();
        let n = labels.len();
        Self {
            labels,
            icons: vec![None; n],
            icons_start: vec![None; n],
            disabled: vec![false; n],
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
        self.labels.push(label.into());
        self.icons.push(None);
        self.icons_start.push(None);
        self.disabled.push(false);
        self
    }

    /// Append an icon-only segment (e.g. a lucide chevron dropdown trigger).
    pub fn icon_segment(mut self, icon: Icon) -> Self {
        self.labels.push(String::new());
        self.icons.push(Some(icon));
        self.icons_start.push(None);
        self.disabled.push(false);
        self
    }

    /// Attach a trailing [`Icon`] to the last segment.
    pub fn icon_end(mut self, icon: Icon) -> Self {
        if let Some(slot) = self.icons.last_mut() {
            *slot = Some(icon);
        }
        self
    }

    /// Attach a leading [`Icon`] to the last segment.
    pub fn icon_start(mut self, icon: Icon) -> Self {
        if let Some(slot) = self.icons_start.last_mut() {
            *slot = Some(icon);
        }
        self
    }

    /// Mark the last segment as disabled (non-interactive, muted).
    pub fn disable_last(mut self) -> Self {
        if let Some(slot) = self.disabled.last_mut() {
            *slot = true;
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
    #[allow(clippy::too_many_lines)] // one linear layout-then-paint pass; splitting it obscures the single measure->draw flow
    pub fn show(mut self, ui: &mut Ui) -> ButtonGroupResponse {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let n = self.labels.len();
        let radius = tokens.radius_2xl();
        let height = m.height;
        let pad_x = m.pad_x;
        // Semibold face so segment labels match shadcn's `font-medium` buttons.
        let font = crate::fonts::semibold(ui, 14.0);

        // Lay out galleys and measure widths.
        let galleys: Vec<_> = self
            .labels
            .iter()
            .map(|label| {
                ui.painter()
                    .layout_no_wrap(label.clone(), font.clone(), tokens.foreground)
            })
            .collect();
        // Per-segment width: padding + optional leading icon + label +
        // optional trailing icon + padding.
        let seg_widths: Vec<f32> = galleys
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let has_label = g.size().x > 0.0;
                let lead_w = match &self.icons_start[i] {
                    Some(_) if has_label => m.icon_size + m.icon_gap,
                    Some(_) => m.icon_size,
                    None => 0.0,
                };
                let trail_w = match &self.icons[i] {
                    Some(_) if has_label => m.icon_gap + m.icon_size,
                    Some(_) => m.icon_size,
                    None => 0.0,
                };
                pad_x.mul_add(2.0, lead_w + g.size().x + trail_w)
            })
            .collect();
        let natural_w: f32 = seg_widths.iter().sum();
        let total_w = if self.full_width {
            ui.available_width().max(natural_w)
        } else {
            natural_w
        };
        // When stretching, scale each segment proportionally.
        let seg_widths: Vec<f32> = if self.full_width && natural_w > 0.0 {
            let scale = total_w / natural_w;
            seg_widths.iter().map(|w| w * scale).collect()
        } else {
            seg_widths
        };

        let (rect, response) = ui.allocate_at_least(Vec2::new(total_w, height), Sense::hover());

        let mut clicked = None;
        let mut menu_trigger = None;
        let last = n.saturating_sub(1);
        if ui.is_rect_visible(rect) {
            let painter = ui.painter().clone();
            let mut x = rect.left();
            for (i, galley) in galleys.into_iter().enumerate() {
                let seg_w = seg_widths[i];
                let icon_end = self.icons[i];
                let icon_start = self.icons_start[i];
                let is_disabled = self.disabled[i];
                let seg =
                    egui::Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::new(seg_w, height));

                // Per-segment corner rounding: outer corners only.
                let cr = segment_radius(i, n, radius);

                let seg_response = if is_disabled {
                    ui.interact(seg, response.id.with(i), Sense::hover())
                } else {
                    ui.interact(seg, response.id.with(i), Sense::click())
                };
                if seg_response.clicked() && !is_disabled {
                    clicked = Some(i);
                }
                if i == last && self.menu.is_some() {
                    menu_trigger = Some(seg_response.clone());
                }

                let (fill, text_color) = if is_disabled {
                    (tokens.background, tokens.muted_foreground)
                } else {
                    segment_paint(self.variant, tokens, seg_response.hovered())
                };

                painter.rect(
                    seg,
                    cr,
                    fill,
                    Stroke::new(1.0, tokens.border),
                    StrokeKind::Inside,
                );
                paint_segment_content(
                    ui, &painter, seg, galley,
                    icon_start, icon_end,
                    text_color, tokens, m,
                );

                x += seg_w;
            }
        }

        // Dropdown menu hung off the last (chevron) segment.
        let mut menu_selected = None;
        if let (Some(menu), Some(trigger)) = (&self.menu, &menu_trigger) {
            egui::Popup::menu(trigger)
                .id(response.id.with("menu"))
                .frame(DropdownMenu::frame(tokens))
                // shadcn opens this menu upward, right-aligned
                // (`data-side="top" data-align="end"`), with a small offset.
                .align(egui::RectAlign::TOP_END)
                .gap(4.0)
                .show(|ui| {
                    menu_selected = menu.show_contents(ui, tokens);
                });
        }

        ButtonGroupResponse {
            response,
            clicked,
            menu_selected,
        }
    }
}

/// Resolve a segment's `(fill, text)` colours for the given variant and hover.
fn segment_paint(
    variant: Variant,
    tokens: Tokens,
    hovered: bool,
) -> (egui::Color32, egui::Color32) {
    match variant {
        Variant::Outline if hovered => (tokens.muted, tokens.foreground),
        Variant::Outline => (tokens.background, tokens.foreground),
        Variant::Secondary if hovered => (
            tokens.secondary.gamma_multiply(0.9),
            tokens.secondary_foreground,
        ),
        Variant::Secondary => (tokens.secondary, tokens.secondary_foreground),
    }
}

/// Centre a segment's optional leading icon + label + optional trailing icon.
#[allow(clippy::too_many_arguments)] // private helper, one call site, each param is a distinct paint input
fn paint_segment_content(
    ui: &Ui,
    painter: &egui::Painter,
    seg: egui::Rect,
    galley: std::sync::Arc<egui::Galley>,
    icon_start: Option<Icon>,
    icon_end: Option<Icon>,
    text_color: egui::Color32,
    tokens: Tokens,
    m: ButtonGroupMetrics,
) {
    let galley_size = galley.size();
    let has_label = galley_size.x > 0.0;

    let lead_w = match icon_start {
        Some(_) if has_label => m.icon_size + m.icon_gap,
        Some(_) => m.icon_size,
        None => 0.0,
    };
    let trail_w = match icon_end {
        Some(_) if has_label => m.icon_gap + m.icon_size,
        Some(_) => m.icon_size,
        None => 0.0,
    };
    let run_w = lead_w + galley_size.x + trail_w;
    let mut cx = seg.center().x - run_w / 2.0;
    let cy = seg.center().y;

    if let Some(icon) = icon_start {
        let ir = egui::Rect::from_min_size(
            egui::pos2(cx, cy - m.icon_size / 2.0),
            Vec2::splat(m.icon_size),
        );
        icon.color(text_color).image(tokens).paint_at(ui, ir);
        cx += if has_label {
            m.icon_size + m.icon_gap
        } else {
            m.icon_size
        };
    }
    if has_label {
        painter.galley(egui::pos2(cx, cy - galley_size.y / 2.0), galley, text_color);
        cx += galley_size.x;
    }
    if let Some(icon) = icon_end {
        if has_label {
            cx += m.icon_gap;
        }
        let ir = egui::Rect::from_min_size(
            egui::pos2(cx, cy - m.icon_size / 2.0),
            Vec2::splat(m.icon_size),
        );
        icon.color(text_color).image(tokens).paint_at(ui, ir);
    }
}

/// Round only the outer corners: left-most segment rounds its left side,
/// right-most rounds its right side, interior segments stay square.
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

impl Widget for ButtonGroup {
    /// Renders the group and discards the click index. Use [`ButtonGroup::show`]
    /// to learn which segment was clicked.
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}
