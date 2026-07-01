//! [`Badge`] — a small status/label pill.
//!
//! Mirrors shadcn's `<Badge>`. Colors resolve from the semantic [`Tokens`]
//! view over the active visuals, so each variant maps to the same shadcn token
//! the website uses and follows the user's theme.

use egui::{Color32, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget, WidgetText};

use crate::components::icon::Icon;
use crate::customize::{Customize, StyleHook};
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry for [`Badge`] — reach in via [`Badge::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct BadgeMetrics {
    /// Leading status dot diameter (shadcn `size-2`).
    pub dot: f32,
    /// Gap between the status dot and the label (`gap-1`).
    pub dot_gap: f32,
    /// Inline icon edge length inside a badge (shadcn `size-3`).
    pub icon: f32,
    /// Gap between an inline icon and the label (`gap-1`).
    pub icon_gap: f32,
}

impl Default for BadgeMetrics {
    fn default() -> Self {
        Self {
            dot: 8.0,
            dot_gap: 4.0,
            icon: 12.0,
            icon_gap: 4.0,
        }
    }
}

/// Visual style of a [`Badge`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    /// Solid accent fill.
    #[default]
    Default,
    /// Muted fill.
    Secondary,
    /// Error fill.
    Destructive,
    /// Transparent with a hairline border.
    Outline,
}

/// [`Badge`]'s resolved per-variant paint — background fill, border stroke,
/// and the label/icon colour. The real value [`Badge`] paints with; reach in
/// via [`Badge::style`].
#[derive(Clone, Copy, Debug)]
pub struct BadgeStyle {
    /// Background fill.
    pub fill: Color32,
    /// Border stroke (`Stroke::NONE` for the filled variants).
    pub stroke: Stroke,
    /// Label + icon colour.
    pub text: Color32,
}

/// A small rounded status label.
///
/// ```no_run
/// use glazier::badge::{Badge, Variant};
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// Badge::new("New").ui(ui);
/// Badge::new("Deprecated").variant(Variant::Destructive).ui(ui);
/// Badge::new("Pending").variant(Variant::Outline).dot(egui::Color32::from_rgb(0xf0, 0xb1, 0x00)).ui(ui);
/// # });
/// ```
#[must_use = "badges do nothing unless you add them to a Ui"]
pub struct Badge {
    text: WidgetText,
    variant: Variant,
    dot: Option<Color32>,
    icon_start: Option<Icon>,
    icon_end: Option<Icon>,
    style_hook: StyleHook<BadgeStyle>,
    sizing_hook: SizingHook<BadgeMetrics>,
}

impl Badge {
    /// Create a badge with the given label.
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            variant: Variant::default(),
            dot: None,
            icon_start: None,
            icon_end: None,
            style_hook: StyleHook::default(),
            sizing_hook: SizingHook::default(),
        }
    }

    /// Set the visual [`Variant`].
    pub const fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Add a leading status dot in `color` (shadcn's `size-2 rounded-full`).
    pub const fn dot(mut self, color: Color32) -> Self {
        self.dot = Some(color);
        self
    }

    /// Add a leading [`Icon`] (`size-3`), tinted to the badge's text colour.
    pub const fn icon_start(mut self, icon: Icon) -> Self {
        self.icon_start = Some(icon);
        self
    }

    /// Add a trailing [`Icon`] (`size-3`), tinted to the badge's text colour.
    pub const fn icon_end(mut self, icon: Icon) -> Self {
        self.icon_end = Some(icon);
        self
    }
}

impl Customize<BadgeStyle> for Badge {
    fn style_hook_mut(&mut self) -> &mut StyleHook<BadgeStyle> {
        &mut self.style_hook
    }
}

impl Sizeable<BadgeMetrics> for Badge {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<BadgeMetrics> {
        &mut self.sizing_hook
    }
}

impl Widget for Badge {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let (fill, stroke, text) = match self.variant {
            Variant::Default => (tokens.primary, Stroke::NONE, tokens.primary_foreground),
            Variant::Secondary => (tokens.secondary, Stroke::NONE, tokens.secondary_foreground),
            Variant::Destructive => (
                tokens.destructive,
                Stroke::NONE,
                tokens.destructive_foreground,
            ),
            Variant::Outline => (
                Color32::TRANSPARENT,
                Stroke::new(1.0, tokens.border),
                tokens.foreground,
            ),
        };
        let mut style = BadgeStyle { fill, stroke, text };
        self.style_hook.apply(&mut style);
        let BadgeStyle {
            fill,
            stroke,
            text: text_color,
        } = style;
        let m = crate::sizing::resolve(self.sizing_hook);

        // shadcn badge: text-xs (12px) font-medium, px-2 py-0.5.
        let padding = Vec2::new(8.0, 3.0);
        let lead = self.dot.map_or(0.0, |_| m.dot + m.dot_gap)
            + self.icon_start.map_or(0.0, |_| m.icon + m.icon_gap);
        let trail = self.icon_end.map_or(0.0, |_| m.icon + m.icon_gap);
        let galley = self.text.color(text_color).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::FontSelection::FontId(egui::FontId::proportional(12.0)),
        );
        let size = Vec2::new(lead + trail, 0.0) + galley.size() + padding * 2.0;
        let (rect, response) = ui.allocate_at_least(size, Sense::hover());

        if ui.is_rect_visible(rect) {
            // shadcn badge is `rounded-full`: a full pill, radius = half height.
            let radius = rect.height() / 2.0;
            ui.painter()
                .rect(rect, radius, fill, stroke, StrokeKind::Inside);
            let mut cursor = rect.left() + padding.x;
            if let Some(dot) = self.dot {
                let center = egui::pos2(cursor + m.dot / 2.0, rect.center().y);
                ui.painter().circle_filled(center, m.dot / 2.0, dot);
                cursor += m.dot + m.dot_gap;
            }
            if let Some(icon) = self.icon_start {
                paint_badge_icon(
                    ui,
                    icon,
                    text_color,
                    cursor,
                    rect.center().y,
                    tokens,
                    m.icon,
                );
                cursor += m.icon + m.icon_gap;
            }
            let text_pos = egui::pos2(cursor, rect.center().y - galley.size().y / 2.0);
            ui.painter().galley(text_pos, galley, text_color);
            if let Some(icon) = self.icon_end {
                let icon_x = rect.right() - padding.x - m.icon;
                paint_badge_icon(
                    ui,
                    icon,
                    text_color,
                    icon_x,
                    rect.center().y,
                    tokens,
                    m.icon,
                );
            }
        }

        response
    }
}

/// Paint a badge icon of edge length `size`, tinted to the badge's text
/// colour, with its left edge at `x` and vertically centred on `cy`.
fn paint_badge_icon(
    ui: &Ui,
    icon: Icon,
    tint: Color32,
    x: f32,
    cy: f32,
    tokens: Tokens,
    size: f32,
) {
    let rect = egui::Rect::from_min_size(egui::pos2(x, cy - size / 2.0), Vec2::splat(size));
    icon.size(size).color(tint).image(tokens).paint_at(ui, rect);
}
