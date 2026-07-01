//! [`Button`] — a shadcn/ui-style button.
//!
//! Variants and sizes mirror shadcn's `<Button>`. Colors are resolved from the
//! semantic [`Tokens`] view over the active [`egui::Visuals`], so each variant
//! maps to the same shadcn token the website uses (Default→`primary`,
//! Secondary→`secondary`, Destructive→`destructive`, Ghost hover→`accent`) and a
//! user's style override flows straight through.
//!
//! This is the vocabulary-setter for glazier components: the [`Variant`] and
//! [`Size`] enums here are the template the rest of the library copies.

use egui::{Color32, Response, Sense, Stroke, Ui, Vec2, Widget, WidgetText};

use crate::components::icon::Icon;
use crate::customize::{Customize, StyleHook};
use crate::tokens::Tokens;

/// Seconds for the hover colour transition to complete (shadcn ~150ms).
const HOVER_TIME: f32 = 0.15;
/// Seconds for the press push/darken to complete (snappier than hover).
const PRESS_TIME: f32 = 0.07;

/// Visual style of a [`Button`], mirroring shadcn's variant prop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    /// Solid accent fill with contrasting text — the primary call to action.
    #[default]
    Default,
    /// Muted fill for secondary actions.
    Secondary,
    /// Solid error fill — for irreversible / dangerous actions.
    Destructive,
    /// Transparent with a hairline border.
    Outline,
    /// Transparent until hovered.
    Ghost,
    /// Looks like a hyperlink.
    Link,
}

/// Size of a [`Button`], mirroring shadcn's size prop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Size {
    /// Compact.
    Small,
    /// Default height.
    #[default]
    Medium,
    /// Tall / prominent.
    Large,
    /// Square, sized for a single glyph.
    Icon,
}

impl Size {
    /// Horizontal / vertical inner padding in points.
    const fn padding(self) -> Vec2 {
        match self {
            Self::Small => Vec2::new(10.0, 4.0),
            Self::Medium => Vec2::new(16.0, 8.0),
            Self::Large => Vec2::new(24.0, 10.0),
            Self::Icon => Vec2::new(6.0, 6.0),
        }
    }

    /// Minimum height in points.
    const fn min_height(self) -> f32 {
        match self {
            Self::Icon => 28.0, // shadcn `size-7`
            Self::Small => 32.0,
            Self::Medium => 36.0,
            Self::Large => 40.0,
        }
    }
}

/// [`Button`]'s resolved per-variant paint — background fill, border stroke,
/// text color, and whether the label is underlined ([`Variant::Link`]).
///
/// This is the *real* value [`Button`] paints with: [`Variant`] just picks
/// reasonable defaults for it from the active [`Tokens`]. Reach in and change
/// any field via [`Button::style`] — there's no separate, narrower override
/// API to keep in sync with this one.
#[derive(Clone, Copy, Debug)]
pub struct ButtonStyle {
    /// Background fill.
    pub fill: Color32,
    /// Border stroke (`Stroke::NONE` for filled/ghost variants).
    pub stroke: Stroke,
    /// Label + icon color.
    pub text: Color32,
    /// Whether the label is drawn with an underline ([`Variant::Link`]).
    pub underline: bool,
    /// Corner radius in points (defaults to [`Tokens::radius_md`]).
    pub radius: u8,
}

impl Variant {
    /// Resolve this variant's default [`ButtonStyle`] against the active
    /// shadcn [`Tokens`].
    fn paint(self, t: Tokens) -> ButtonStyle {
        match self {
            Self::Default => ButtonStyle {
                fill: t.primary,
                stroke: Stroke::NONE,
                text: t.primary_foreground,
                underline: false,
                radius: t.radius_md(),
            },
            Self::Secondary => ButtonStyle {
                fill: t.secondary,
                stroke: Stroke::NONE,
                text: t.secondary_foreground,
                underline: false,
                radius: t.radius_md(),
            },
            Self::Destructive => ButtonStyle {
                fill: t.destructive,
                stroke: Stroke::NONE,
                text: t.destructive_foreground,
                underline: false,
                radius: t.radius_md(),
            },
            Self::Outline => ButtonStyle {
                // Opaque `card` surface, not `background` — a translucent
                // app-canvas token would leak into every outline button.
                fill: t.card,
                stroke: Stroke::new(1.0, t.border),
                text: t.foreground,
                underline: false,
                radius: t.radius_md(),
            },
            Self::Ghost => ButtonStyle {
                fill: Color32::TRANSPARENT,
                stroke: Stroke::NONE,
                text: t.foreground,
                underline: false,
                radius: t.radius_md(),
            },
            Self::Link => ButtonStyle {
                fill: Color32::TRANSPARENT,
                stroke: Stroke::NONE,
                text: t.foreground,
                underline: true,
                radius: t.radius_md(),
            },
        }
    }
}

/// A shadcn/ui-style button.
///
/// ```no_run
/// use glazier::button::{Button, Variant, Size};
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// if Button::new("Save").ui(ui).clicked() {
///     // …
/// }
/// Button::new("Delete").variant(Variant::Destructive).ui(ui);
/// Button::new("+").variant(Variant::Outline).size(Size::Icon).ui(ui);
/// # });
/// ```
/// Gap between the label and an inline icon (shadcn `gap-1.5`).
const ICON_GAP: f32 = 6.0;
/// Inline icon edge length (shadcn `size-4`).
const ICON_SIZE: f32 = 16.0;

#[must_use = "buttons do nothing unless you add them to a Ui"]
pub struct Button {
    text: WidgetText,
    variant: Variant,
    size: Size,
    icon_start: Option<Icon>,
    icon_end: Option<Icon>,
    full_width: bool,
    style_hook: StyleHook<ButtonStyle>,
}

impl Button {
    /// Create a button with the given label.
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            variant: Variant::default(),
            size: Size::default(),
            icon_start: None,
            icon_end: None,
            full_width: false,
            style_hook: StyleHook::default(),
        }
    }

    /// Add a leading [`Icon`] (inline-start).
    pub const fn icon_start(mut self, icon: Icon) -> Self {
        self.icon_start = Some(icon);
        self
    }

    /// Add a trailing [`Icon`] (inline-end) — e.g. a lucide arrow.
    pub const fn icon_end(mut self, icon: Icon) -> Self {
        self.icon_end = Some(icon);
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

    /// Stretch the button to fill the available width (shadcn `w-full`).
    pub const fn full_width(mut self, full_width: bool) -> Self {
        self.full_width = full_width;
        self
    }
}

impl Customize<ButtonStyle> for Button {
    fn style_hook_mut(&mut self) -> &mut StyleHook<ButtonStyle> {
        &mut self.style_hook
    }
}

impl Widget for Button {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let mut style = self.variant.paint(tokens);
        self.style_hook.apply(&mut style);
        let ButtonStyle {
            fill,
            stroke,
            text,
            underline,
            radius,
        } = style;

        // A `Link` is inline text (shadcn `p-0 h-auto`): no box padding and no
        // min-height, so it aligns to the surrounding text baseline rather than
        // sitting in a tall 36px button shell.
        let is_link = self.variant == Variant::Link;
        let padding = if is_link {
            Vec2::ZERO
        } else {
            self.size.padding()
        };
        let min_height = if is_link { 0.0 } else { self.size.min_height() };

        // Lay the label out with the resolved text color.
        let rich = self.text.into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Button,
        );

        // Inline icons add their width; a `gap-1.5` only applies *between* an
        // icon and a label, so icon-only buttons stay square (no phantom gap).
        let has_label = rich.size().x > 0.0;
        let n_icons = u8::from(self.icon_start.is_some()) + u8::from(self.icon_end.is_some());
        let icons_w = f32::from(n_icons) * ICON_SIZE;
        let gaps_w = if has_label {
            f32::from(n_icons) * ICON_GAP
        } else {
            0.0
        };
        let content_w = rich.size().x + icons_w + gaps_w;

        let natural_w = padding.x.mul_add(2.0, content_w).max(min_height);
        let desired = Vec2::new(
            // `w-full`: stretch to the available width (but never narrower than
            // the content needs).
            if self.full_width {
                ui.available_width().max(natural_w)
            } else {
                natural_w
            },
            padding.y.mul_add(2.0, rich.size().y).max(min_height),
        );
        let (rect, response) = ui.allocate_at_least(desired, Sense::click());

        // Animate hover/press over time so colour + scale glide instead of
        // snapping (egui is immediate-mode; these drive an eased 0..1 value and
        // request repaints while in flight).
        let id = response.id;
        let hover_t =
            ui.ctx()
                .animate_bool_with_time(id.with("hover"), response.hovered(), HOVER_TIME);
        let press_t = ui.ctx().animate_bool_with_time(
            id.with("press"),
            response.is_pointer_button_down_on(),
            PRESS_TIME,
        );

        if ui.is_rect_visible(rect) {
            // shadcn hover model: transparent variants (ghost/outline) glide to
            // the `accent` surface; solid fills deepen for clear contrast, then
            // deepen further while held.
            let bg = match self.variant {
                Variant::Ghost | Variant::Outline => fill.lerp_to_gamma(tokens.accent, hover_t),
                Variant::Link => fill,
                // shadcn secondary: plain `bg-secondary`, hover mixes toward
                // `foreground` 5% — exactly `color-mix(in oklch, secondary,
                // foreground 5%)`. We replicate that mix in oklab (not sRGB
                // gamma space, which over-darkens light surfaces), so light and
                // dark both match the website. Holding nudges to 10%.
                Variant::Secondary => {
                    let t = 0.05_f32.mul_add(hover_t, 0.05 * press_t);
                    mix_oklch(fill, tokens.foreground, t)
                }
                _ => {
                    let darken = 0.18_f32.mul_add(hover_t, 0.12 * press_t);
                    fill.gamma_multiply(1.0 - darken)
                }
            };

            // Push animation: shrink the box slightly while pressed.
            let scale = 0.04_f32.mul_add(-press_t, 1.0);
            let draw_rect = egui::Rect::from_center_size(rect.center(), rect.size() * scale);

            let galley_size = rich.size();
            ui.painter()
                .rect(draw_rect, radius, bg, stroke, egui::StrokeKind::Inside);

            // Lay icon(s) + label as one centered horizontal run.
            let gap = if has_label { ICON_GAP } else { 0.0 };
            let mut cursor = draw_rect.center().x - content_w / 2.0;
            let cy = draw_rect.center().y;
            if let Some(icon) = self.icon_start {
                paint_icon(ui, icon, text, cursor, cy);
                cursor += ICON_SIZE + gap;
            }
            let text_pos = egui::pos2(cursor, cy - galley_size.y / 2.0);
            ui.painter().galley(text_pos, rich, text);
            cursor += galley_size.x;
            if underline {
                let y = text_pos.y + galley_size.y;
                ui.painter().hline(
                    text_pos.x..=(text_pos.x + galley_size.x),
                    y,
                    Stroke::new(1.0, text),
                );
            }
            if let Some(icon) = self.icon_end {
                cursor += gap;
                paint_icon(ui, icon, text, cursor, cy);
            }
        }

        // shadcn buttons are `cursor-pointer`.
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}

/// Mix two opaque colours by `t` in the **oklab** space, matching CSS
/// `color-mix(in oklch, a, b <t>)`. (Oklch and oklab share the same L/a/b axes;
/// a straight-line lerp between two colours is identical in both.) Working in
/// oklab keeps the perceptual lightness even, so a 5% nudge toward `foreground`
/// reads the same on light and dark fills — unlike an sRGB-gamma lerp, which
/// darkens light surfaces too hard.
fn mix_oklch(a: Color32, b: Color32, t: f32) -> Color32 {
    let (l1, a1, b1) = srgb_to_oklab(a);
    let (l2, a2, b2) = srgb_to_oklab(b);
    let l = t.mul_add(l2 - l1, l1);
    let aa = t.mul_add(a2 - a1, a1);
    let bb = t.mul_add(b2 - b1, b1);
    oklab_to_srgb(l, aa, bb)
}

/// Convert an sRGB [`Color32`] to oklab `(L, a, b)`.
fn srgb_to_oklab(c: Color32) -> (f32, f32, f32) {
    let lr = linearize(c.r());
    let lg = linearize(c.g());
    let lb = linearize(c.b());

    let long = 0.051_445_995f32.mul_add(lb, 0.412_456_4f32.mul_add(lr, 0.536_275_2 * lg));
    let med = 0.107_969_94f32.mul_add(lb, 0.211_039_06f32.mul_add(lr, 0.680_998_5 * lg));
    let short = 0.629_978_7f32.mul_add(lb, 0.088_302_46f32.mul_add(lr, 0.281_718_85 * lg));

    let lc = long.cbrt();
    let mc = med.cbrt();
    let sc = short.cbrt();

    (
        0.004_072_047f32.mul_add(-sc, 0.210_454_26f32.mul_add(lc, 0.793_617_8 * mc)),
        0.450_593_7f32.mul_add(sc, 1.977_998_5f32.mul_add(lc, -2.428_592_2 * mc)),
        0.808_675_77f32.mul_add(-sc, 0.025_904_037f32.mul_add(lc, 0.782_771_77 * mc)),
    )
}

/// Convert oklab `(L, a, b)` back to an opaque sRGB [`Color32`].
fn oklab_to_srgb(lightness: f32, green_red: f32, blue_yellow: f32) -> Color32 {
    let lc = 0.215_803_76f32.mul_add(blue_yellow, 0.396_337_78f32.mul_add(green_red, lightness));
    let mc = 0.063_854_17f32.mul_add(
        -blue_yellow,
        (-0.105_561_346f32).mul_add(green_red, lightness),
    );
    let sc = 1.291_485_5f32.mul_add(
        -blue_yellow,
        (-0.089_484_18f32).mul_add(green_red, lightness),
    );

    let l3 = lc * lc * lc;
    let m3 = mc * mc * mc;
    let s3 = sc * sc * sc;

    let r = 0.230_969_94f32.mul_add(s3, 4.076_741_7f32.mul_add(l3, -3.307_711_6 * m3));
    let g = 0.341_319_38f32.mul_add(-s3, (-1.268_438f32).mul_add(l3, 2.609_757_4 * m3));
    let b = 1.707_614_7f32.mul_add(s3, (-0.004_196_086_3f32).mul_add(l3, -0.703_418_6 * m3));

    Color32::from_rgb(delinearize(r), delinearize(g), delinearize(b))
}

/// sRGB 8-bit channel → linear `[0, 1]`.
fn linearize(u: u8) -> f32 {
    let c = f32::from(u) / 255.0;
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear `[0, 1]` → sRGB 8-bit channel (clamped).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn delinearize(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055f32.mul_add(c.powf(1.0 / 2.4), -0.055)
    };
    (s * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Paint a `size-4` icon, tinted to the button's text colour, with its left
/// edge at `x` and vertically centred on `cy`.
fn paint_icon(ui: &Ui, icon: Icon, tint: Color32, x: f32, cy: f32) {
    let tokens = Tokens::get(ui);
    let rect =
        egui::Rect::from_min_size(egui::pos2(x, cy - ICON_SIZE / 2.0), Vec2::splat(ICON_SIZE));
    icon.color(tint).image(tokens).paint_at(ui, rect);
}
