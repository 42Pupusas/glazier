//! [`Badge`] — a small status/label pill.
//!
//! Mirrors shadcn's `<Badge>`. Colors resolve from the semantic [`Tokens`]
//! view over the active visuals, so each variant maps to the same shadcn token
//! the website uses and follows the user's theme.

use egui::{Color32, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget, WidgetText};

use crate::components::icon::Icon;
use crate::tokens::Tokens;

/// A leading status dot diameter (shadcn `size-2`).
const DOT: f32 = 8.0;
/// Gap between the status dot and the label (`gap-1`).
const DOT_GAP: f32 = 4.0;
/// Inline icon edge length inside a badge (shadcn `size-3`).
const ICON: f32 = 12.0;
/// Gap between an inline icon and the label (`gap-1`).
const ICON_GAP: f32 = 4.0;

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

impl Widget for Badge {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let (fill, stroke, text_color) = match self.variant {
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

        // shadcn badge: text-xs (12px) font-medium, px-2 py-0.5.
        let padding = Vec2::new(8.0, 3.0);
        let lead = self.dot.map_or(0.0, |_| DOT + DOT_GAP)
            + self.icon_start.map_or(0.0, |_| ICON + ICON_GAP);
        let trail = self.icon_end.map_or(0.0, |_| ICON + ICON_GAP);
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
                let center = egui::pos2(cursor + DOT / 2.0, rect.center().y);
                ui.painter().circle_filled(center, DOT / 2.0, dot);
                cursor += DOT + DOT_GAP;
            }
            if let Some(icon) = self.icon_start {
                paint_badge_icon(ui, icon, text_color, cursor, rect.center().y, tokens);
                cursor += ICON + ICON_GAP;
            }
            let text_pos = egui::pos2(cursor, rect.center().y - galley.size().y / 2.0);
            ui.painter().galley(text_pos, galley, text_color);
            if let Some(icon) = self.icon_end {
                let icon_x = rect.right() - padding.x - ICON;
                paint_badge_icon(ui, icon, text_color, icon_x, rect.center().y, tokens);
            }
        }

        response
    }
}

/// Paint a `size-3` badge icon, tinted to the badge's text colour, with its
/// left edge at `x` and vertically centred on `cy`.
fn paint_badge_icon(ui: &Ui, icon: Icon, tint: Color32, x: f32, cy: f32, tokens: Tokens) {
    let rect = egui::Rect::from_min_size(egui::pos2(x, cy - ICON / 2.0), Vec2::splat(ICON));
    icon.size(ICON).color(tint).image(tokens).paint_at(ui, rect);
}
