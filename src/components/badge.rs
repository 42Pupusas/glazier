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

impl Variant {
    /// The variant's default paint.
    const fn style(self, tokens: Tokens) -> BadgeStyle {
        let (fill, stroke, text) = match self {
            Self::Default => (tokens.primary, Stroke::NONE, tokens.primary_foreground),
            Self::Secondary => (tokens.secondary, Stroke::NONE, tokens.secondary_foreground),
            Self::Destructive => (
                tokens.destructive,
                Stroke::NONE,
                tokens.destructive_foreground,
            ),
            Self::Outline => (
                Color32::TRANSPARENT,
                Stroke {
                    width: 1.0,
                    color: tokens.border,
                },
                tokens.foreground,
            ),
        };
        BadgeStyle { fill, stroke, text }
    }
}

impl Widget for Badge {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let mut style = self.variant.style(tokens);
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

        if !ui.is_rect_visible(rect) {
            return response;
        }
        // shadcn badge is `rounded-full`: a full pill, radius = half height.
        ui.painter()
            .rect(rect, rect.height() / 2.0, fill, stroke, StrokeKind::Inside);
        let cy = rect.center().y;
        let icon = |icon: Icon, x: f32| {
            let r =
                egui::Rect::from_min_size(egui::pos2(x, cy - m.icon / 2.0), Vec2::splat(m.icon));
            icon.size(m.icon)
                .color(text_color)
                .image(tokens)
                .paint_at(ui, r);
        };
        let mut x = rect.left() + padding.x;
        if let Some(dot) = self.dot {
            ui.painter()
                .circle_filled(egui::pos2(x + m.dot / 2.0, cy), m.dot / 2.0, dot);
            x += m.dot + m.dot_gap;
        }
        if let Some(i) = self.icon_start {
            icon(i, x);
            x += m.icon + m.icon_gap;
        }
        let text_pos = egui::pos2(x, cy - galley.size().y / 2.0);
        ui.painter().galley(text_pos, galley, text_color);
        if let Some(i) = self.icon_end {
            icon(i, rect.right() - padding.x - m.icon);
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;

    const DOT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="4" fill="currentColor"/></svg>"#;

    /// Each decoration widens the badge; the label sits after the leading ones.
    #[test]
    fn decorations_widen_the_badge() {
        let ui = TestUi::new();
        let mut widths = Vec::new();
        let out = ui.frame(|ui| {
            for badge in [
                Badge::new("New"),
                Badge::new("New").dot(Color32::RED),
                Badge::new("New")
                    .dot(Color32::RED)
                    .icon_start(Icon::new(DOT)),
                Badge::new("New")
                    .dot(Color32::RED)
                    .icon_start(Icon::new(DOT))
                    .icon_end(Icon::new(DOT)),
            ] {
                widths.push(ui.add(badge).rect.width());
            }
        });
        assert!(widths.windows(2).all(|w| w[1] > w[0]), "{widths:?}");
        assert!(TestUi::text_x(&out, "New") > 0.0);
    }

    /// The style hook overrides the variant's default paint.
    #[test]
    fn style_hook_overrides_the_variant() {
        let tokens = Tokens::from_visuals(&egui::Visuals::light());
        assert_eq!(Variant::Outline.style(tokens).fill, Color32::TRANSPARENT);
        assert_eq!(Variant::Destructive.style(tokens).fill, tokens.destructive);
        assert_eq!(
            Variant::Secondary.style(tokens).text,
            tokens.secondary_foreground
        );
        let out = TestUi::new().frame(|ui| {
            ui.add(
                Badge::new("Hot")
                    .variant(Variant::Secondary)
                    .style(|s: &mut BadgeStyle| s.fill = Color32::GOLD),
            );
        });
        let gold = out.shapes.iter().any(|c| match &c.shape {
            egui::Shape::Rect(r) => r.fill == Color32::GOLD,
            _ => false,
        });
        assert!(gold);
    }
}
