//! [`Toggle`] — a two-state pressed/unpressed button, mirroring shadcn's
//! `<Toggle>`.
//!
//! Looks like a ghost/outline button that stays filled with the `accent` surface
//! while pressed (on). Borrows `&mut bool`. Carries an optional leading [`Icon`]
//! and label, sized with the same scale as [`Button`](crate::Button).

use egui::{Color32, Response, Sense, Stroke, Ui, Vec2, Widget, WidgetText};

use crate::components::icon::Icon;
use crate::tokens::Tokens;

/// Seconds for the on/off + hover colour transition.
const TOGGLE_TIME: f32 = 0.15;
/// Gap between an inline icon and the label (shadcn `gap-1.5`).
const ICON_GAP: f32 = 6.0;
/// Inline icon edge length (shadcn `size-4`).
const ICON_SIZE: f32 = 16.0;

/// Visual style of a [`Toggle`], mirroring shadcn's variant prop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    /// Ghost-like: transparent when off, gliding to the `accent` surface on
    /// hover and while pressed (on).
    #[default]
    Default,
    /// Reads as an **outline** button when off (background fill + hairline
    /// border, `accent` on hover) and a **secondary** button when on (secondary
    /// fill, no border).
    Outline,
}

/// Size of a [`Toggle`], mirroring shadcn's size prop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Size {
    /// Compact.
    Small,
    /// Default height.
    #[default]
    Medium,
    /// Tall / prominent.
    Large,
}

impl Size {
    /// Horizontal / vertical inner padding in points.
    const fn padding(self) -> Vec2 {
        match self {
            Self::Small => Vec2::new(6.0, 4.0),
            Self::Medium => Vec2::new(10.0, 6.0),
            Self::Large => Vec2::new(12.0, 8.0),
        }
    }

    /// Minimum height in points.
    const fn min_height(self) -> f32 {
        match self {
            Self::Small => 32.0,
            Self::Medium => 36.0,
            Self::Large => 40.0,
        }
    }
}

/// A two-state toggle button bound to a `&mut bool`.
///
/// ```no_run
/// use glazier::toggle::Toggle;
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// let mut bold = false;
/// Toggle::new(&mut bold, "B").ui(ui);
/// # });
/// ```
#[must_use = "toggles do nothing unless you add them to a Ui"]
pub struct Toggle<'a> {
    on: &'a mut bool,
    text: WidgetText,
    variant: Variant,
    size: Size,
    icon: Option<Icon>,
}

impl<'a> Toggle<'a> {
    /// Create a toggle bound to `on`, with the given label (may be empty for an
    /// icon-only toggle).
    pub fn new(on: &'a mut bool, text: impl Into<WidgetText>) -> Self {
        Self {
            on,
            text: text.into(),
            variant: Variant::default(),
            size: Size::default(),
            icon: None,
        }
    }

    /// Add a leading [`Icon`].
    pub const fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set the visual [`Variant`].
    pub const fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the [`Size`].
    pub const fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }
}

impl Widget for Toggle<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let padding = self.size.padding();

        let rich = self.text.into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Button,
        );
        let has_label = rich.size().x > 0.0;
        let has_icon = self.icon.is_some();
        let gap = if has_label && has_icon { ICON_GAP } else { 0.0 };
        let icon_w = if has_icon { ICON_SIZE } else { 0.0 };
        let content_w = rich.size().x + icon_w + gap;

        let desired = Vec2::new(
            padding
                .x
                .mul_add(2.0, content_w)
                .max(self.size.min_height()),
            padding
                .y
                .mul_add(2.0, rich.size().y)
                .max(self.size.min_height()),
        );
        let (rect, mut response) = ui.allocate_at_least(desired, Sense::click());

        if response.clicked() {
            *self.on = !*self.on;
            response.mark_changed();
        }

        // Eased on value, plus a hover value for the unpressed hover surface.
        let on_t = ui
            .ctx()
            .animate_bool_with_time(response.id.with("on"), *self.on, TOGGLE_TIME);
        let hover_t = ui.ctx().animate_bool_with_time(
            response.id.with("hover"),
            response.hovered(),
            TOGGLE_TIME,
        );

        if ui.is_rect_visible(rect) {
            let radius = tokens.radius_md();
            let (fill, stroke, text_col) = match self.variant {
                // Ghost toggle: transparent off, gliding to `accent` on
                // hover/press.
                Variant::Default => {
                    let lit = hover_t.max(on_t);
                    let fill = if lit < 0.01 {
                        Color32::TRANSPARENT
                    } else {
                        tokens.background.lerp_to_gamma(tokens.accent, lit)
                    };
                    let text = tokens
                        .foreground
                        .lerp_to_gamma(tokens.accent_foreground, on_t);
                    (fill, Stroke::NONE, text)
                }
                // Off → outline button (transparent fill + hairline border,
                // gliding to `accent` on hover); on → primary button (`primary`
                // fill, no border). Both ends are *solid*, so we lerp the off
                // look straight to `primary` by `on_t` — a single solid→solid
                // blend that always completes, with no neutral midpoint to get
                // stuck on. The border and the off-state accent lift fade out
                // as the chip fades in. `primary` keeps real contrast in both
                // palettes (white chip + dark text in dark mode; dark chip +
                // light text in light mode).
                Variant::Outline => {
                    let off = if hover_t < 0.01 {
                        Color32::TRANSPARENT
                    } else {
                        tokens.background.lerp_to_gamma(tokens.accent, hover_t)
                    };
                    let fill = off.lerp_to_gamma(tokens.primary, on_t);
                    let text = tokens
                        .foreground
                        .lerp_to_gamma(tokens.primary_foreground, on_t);
                    // Border belongs to the outline (off) look; fade it out as
                    // the primary chip takes over.
                    let stroke = Stroke::new(1.0, tokens.border.gamma_multiply(1.0 - on_t));
                    (fill, stroke, text)
                }
            };

            ui.painter()
                .rect(rect, radius, fill, stroke, egui::StrokeKind::Inside);

            let mut cursor = rect.center().x - content_w / 2.0;
            let cy = rect.center().y;
            if let Some(icon) = self.icon {
                let r = egui::Rect::from_min_size(
                    egui::pos2(cursor, cy - ICON_SIZE / 2.0),
                    Vec2::splat(ICON_SIZE),
                );
                icon.color(text_col).image(tokens).paint_at(ui, r);
                cursor += ICON_SIZE + gap;
            }
            let galley_size = rich.size();
            ui.painter()
                .galley(egui::pos2(cursor, cy - galley_size.y / 2.0), rich, text_col);
        }

        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}
