//! [`Toggle`] — a two-state pressed/unpressed button, mirroring shadcn's
//! `<Toggle>`.
//!
//! Looks like a ghost/outline button that stays filled with the `accent` surface
//! while pressed (on). Borrows `&mut bool`. Carries an optional leading [`Icon`]
//! and label, sized with the same scale as [`Button`](crate::Button).

use std::sync::Arc;

use egui::{Color32, Galley, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget, WidgetText};

use crate::components::icon::Icon;
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry/timing for [`Toggle`] — reach in via
/// [`Toggle::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct ToggleMetrics {
    /// Seconds for the on/off + hover colour transition.
    pub toggle_time: f32,
    /// Gap between an inline icon and the label (shadcn `gap-1.5`).
    pub icon_gap: f32,
    /// Inline icon edge length (shadcn `size-4`).
    pub icon_size: f32,
}

impl Default for ToggleMetrics {
    fn default() -> Self {
        Self {
            toggle_time: 0.15,
            icon_gap: 6.0,
            icon_size: 16.0,
        }
    }
}

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

impl Variant {
    /// `(fill, border, text)` for the eased on (`on_t`) and hover (`hover_t`)
    /// values.
    fn paint(self, tokens: Tokens, on_t: f32, hover_t: f32) -> (Color32, Stroke, Color32) {
        // `widget`, not `background` — the app-canvas token can be
        // translucent under a user theme.
        let lift = |t: f32| {
            if t < 0.01 {
                Color32::TRANSPARENT
            } else {
                tokens.widget.lerp_to_gamma(tokens.accent, t)
            }
        };
        match self {
            // Ghost: transparent off, gliding to `accent` on hover or press.
            Self::Default => (
                lift(hover_t.max(on_t)),
                Stroke::NONE,
                tokens
                    .foreground
                    .lerp_to_gamma(tokens.accent_foreground, on_t),
            ),
            // Off is an outline button, on a `primary` chip. Both ends are
            // solid, so the off look lerps straight to `primary` with no
            // neutral midpoint to stall on; the border fades out as the chip
            // fades in.
            Self::Outline => (
                lift(hover_t).lerp_to_gamma(tokens.primary, on_t),
                Stroke::new(1.0, tokens.border.gamma_multiply(1.0 - on_t)),
                tokens
                    .foreground
                    .lerp_to_gamma(tokens.primary_foreground, on_t),
            ),
        }
    }
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
    sizing_hook: SizingHook<ToggleMetrics>,
}

impl Sizeable<ToggleMetrics> for Toggle<'_> {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<ToggleMetrics> {
        &mut self.sizing_hook
    }
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
            sizing_hook: SizingHook::default(),
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
    fn ui(mut self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let content = Content {
            galley: self.text.into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                egui::TextStyle::Button,
            ),
            icon: self.icon,
            m,
        };
        // At least square at the minimum height.
        let min = self.size.min_height();
        let desired = (content.size() + self.size.padding() * 2.0).max(Vec2::splat(min));
        let (rect, mut response) = ui.allocate_at_least(desired, Sense::click());

        if response.clicked() {
            *self.on = !*self.on;
            response.mark_changed();
        }

        // Eased on value, plus a hover value for the unpressed hover surface.
        let on_t = ui
            .ctx()
            .animate_bool_with_time(response.id.with("on"), *self.on, m.toggle_time);
        let hover_t = ui.ctx().animate_bool_with_time(
            response.id.with("hover"),
            response.hovered(),
            m.toggle_time,
        );

        if ui.is_rect_visible(rect) {
            let (fill, stroke, text) = self.variant.paint(tokens, on_t, hover_t);
            ui.painter().rect(
                rect,
                tokens.radius_md(),
                fill,
                stroke,
                egui::StrokeKind::Inside,
            );
            content.paint(ui, rect.center(), text, tokens);
        }

        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}

/// A toggle's optional leading icon and label, laid out as one run.
struct Content {
    galley: Arc<Galley>,
    icon: Option<Icon>,
    m: ToggleMetrics,
}

impl Content {
    /// The gap between icon and label, when there are both.
    fn gap(&self) -> f32 {
        if self.icon.is_some() && self.galley.size().x > 0.0 {
            self.m.icon_gap
        } else {
            0.0
        }
    }

    fn size(&self) -> Vec2 {
        let icon_w = self.icon.map_or(0.0, |_| self.m.icon_size);
        self.galley.size() + Vec2::X * (icon_w + self.gap())
    }

    /// Paint the run centred on `center`.
    fn paint(&self, ui: &Ui, center: Pos2, color: Color32, tokens: Tokens) {
        let size = self.m.icon_size;
        let mut x = center.x - self.size().x / 2.0;
        if let Some(icon) = self.icon {
            let r = Rect::from_min_size(Pos2::new(x, center.y - size / 2.0), Vec2::splat(size));
            icon.color(color).image(tokens).paint_at(ui, r);
            x += size + self.gap();
        }
        let pos = Pos2::new(x, center.y - self.galley.size().y / 2.0);
        ui.painter().galley(pos, self.galley.clone(), color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;

    const DOT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="4" fill="currentColor"/></svg>"#;

    /// Clicking flips the bound value both ways, for each variant.
    #[test]
    fn clicking_flips_the_value() {
        for variant in [Variant::Default, Variant::Outline] {
            let ui = TestUi::new();
            let mut on = false;
            let mut build = |ui: &mut Ui| {
                Toggle::new(&mut on, "Bold")
                    .icon(Icon::new(DOT))
                    .variant(variant)
                    .size(Size::Large)
                    .ui(ui);
            };
            let out = ui.frame(&mut build);
            let label = TestUi::text_rect(&out, "Bold").center();
            ui.click(label, &mut build);
            ui.click(label, &mut build);
            ui.click(label, &mut build);
            drop(build);
            assert!(on, "{variant:?}");
        }
    }

    /// An icon-only toggle is square at its minimum height.
    #[test]
    fn icon_only_toggle_is_square() {
        let ui = TestUi::new();
        let mut on = false;
        let mut size = Vec2::ZERO;
        ui.frame(|ui| {
            ui.horizontal(|ui| {
                size = Toggle::new(&mut on, "")
                    .icon(Icon::new(DOT))
                    .size(Size::Small)
                    .ui(ui)
                    .rect
                    .size();
            });
        });
        assert_eq!(size, Vec2::splat(Size::Small.min_height()));
    }

    /// The outline border fades out as the toggle turns on.
    #[test]
    fn outline_border_fades_when_on() {
        let tokens = Tokens::from_visuals(&egui::Visuals::light());
        let (_, off, _) = Variant::Outline.paint(tokens, 0.0, 0.0);
        let (_, on, _) = Variant::Outline.paint(tokens, 1.0, 1.0);
        assert_eq!(off.color, tokens.border);
        assert_eq!(on.color.a(), 0);
        let (fill, _, _) = Variant::Default.paint(tokens, 0.0, 0.0);
        assert_eq!(fill, Color32::TRANSPARENT);
    }
}
