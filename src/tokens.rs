//! [`Tokens`] — glazier's semantic design tokens, mapped onto [`egui::Visuals`].
//!
//! shadcn components reference a small, fixed set of *semantic tokens*
//! (`background`, `primary`, `muted`, …) plus a radius scale derived from a
//! single `--radius`. glazier mirrors that vocabulary, but instead of keeping a
//! parallel color store we wire those tokens **straight into egui's
//! [`Visuals`]**. That means:
//!
//! * [`Tokens::apply`] writes the shadcn palette into a [`Visuals`], and
//!   [`shadcn_visuals`] builds a ready-to-install one. Call
//!   `ctx.set_visuals(glazier::shadcn_visuals(dark))` once at startup.
//! * [`Tokens::get`] is just a typed *view* over `ui.visuals()` — it reads the
//!   exact same fields back out, so components stay driven by the live
//!   [`Visuals`] and any user override (light/dark, custom theme) flows through.
//!
//! The default values are shadcn's "neutral" base, converted from OKLCH to
//! sRGB.

use egui::{Color32, CornerRadius, Stroke, Ui, Visuals};

/// shadcn semantic color + radius tokens.
///
/// Field names match shadcn's CSS variables. A `*_foreground` field is the
/// text/icon color that sits on the matching surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tokens {
    /// Default app background (`--background`).
    pub background: Color32,
    /// Default text/icon color (`--foreground`).
    pub foreground: Color32,
    /// Elevated surface — cards, panels (`--card`). Reserved for
    /// **persistent content containers** (`Card`, `Alert`, `Bubble`,
    /// `Sidebar`, `Attachment`). Not for interactive widgets or floating
    /// menus — see [`widget`](Self::widget), which several components used
    /// to (incorrectly) borrow this token for.
    pub card: Color32,
    /// Text on [`card`](Self::card).
    pub card_foreground: Color32,
    /// Opaque interactive-widget / floating-surface fill — buttons, form
    /// controls (switch, checkbox, input, select triggers, …) and floating
    /// overlays (popover, dropdown menu, dialog, tooltip). Distinct from
    /// both [`background`](Self::background) (the app canvas, which a theme
    /// may legitimately make translucent) and [`card`](Self::card)
    /// (reserved for static containers) — so a translucent app theme can't
    /// punch holes in buttons, and buttons don't visually double as cards.
    /// Mirrors shadcn's `--popover`, widened to cover form-control chrome
    /// glazier paints by hand instead of through egui's `Visuals` widget
    /// states. Round-trips through [`extreme_bg_color`](egui::Visuals::extreme_bg_color).
    pub widget: Color32,
    /// High-emphasis / brand surface — default button, selection (`--primary`).
    pub primary: Color32,
    /// Text on [`primary`](Self::primary).
    pub primary_foreground: Color32,
    /// Lower-emphasis filled surface — secondary button/badge (`--secondary`).
    pub secondary: Color32,
    /// Text on [`secondary`](Self::secondary).
    pub secondary_foreground: Color32,
    /// Subtle surface — skeletons, muted panels (`--muted`).
    pub muted: Color32,
    /// Lower-emphasis text — subtitles, helper text (`--muted-foreground`).
    pub muted_foreground: Color32,
    /// Interactive hover/focus surface — ghost hover, menu highlight (`--accent`).
    pub accent: Color32,
    /// Text on [`accent`](Self::accent).
    pub accent_foreground: Color32,
    /// Destructive / error emphasis (`--destructive`).
    pub destructive: Color32,
    /// Text on [`destructive`](Self::destructive).
    pub destructive_foreground: Color32,
    /// Default border / separator color (`--border`).
    pub border: Color32,
    /// Form-control border color (`--input`).
    pub input: Color32,
    /// Focus-ring color (`--ring`).
    pub ring: Color32,
    /// Base corner radius in points (`--radius`, default 10 ≈ 0.625rem).
    pub radius: f32,
}

impl Tokens {
    /// The shadcn **neutral light** defaults (their `:root`), converted to sRGB.
    #[must_use]
    pub const fn light() -> Self {
        Self {
            background: Color32::from_rgb(0xff, 0xff, 0xff),
            foreground: Color32::from_rgb(0x0a, 0x0a, 0x0a),
            card: Color32::from_rgb(0xff, 0xff, 0xff),
            card_foreground: Color32::from_rgb(0x0a, 0x0a, 0x0a),
            // shadcn's neutral `--popover` matches `--card` by default (both
            // white) — same value, independent token.
            widget: Color32::from_rgb(0xff, 0xff, 0xff),
            primary: Color32::from_rgb(0x17, 0x17, 0x17),
            primary_foreground: Color32::from_rgb(0xfa, 0xfa, 0xfa),
            secondary: Color32::from_rgb(0xf5, 0xf5, 0xf5),
            secondary_foreground: Color32::from_rgb(0x17, 0x17, 0x17),
            muted: Color32::from_rgb(0xf5, 0xf5, 0xf5),
            muted_foreground: Color32::from_rgb(0x73, 0x73, 0x73),
            accent: Color32::from_rgb(0xf5, 0xf5, 0xf5),
            accent_foreground: Color32::from_rgb(0x17, 0x17, 0x17),
            destructive: Color32::from_rgb(0xe7, 0x00, 0x0b),
            destructive_foreground: Color32::from_rgb(0xfa, 0xfa, 0xfa),
            border: Color32::from_rgb(0xe5, 0xe5, 0xe5),
            input: Color32::from_rgb(0xe5, 0xe5, 0xe5),
            ring: Color32::from_rgb(0xa1, 0xa1, 0xa1),
            radius: 10.0,
        }
    }

    /// The shadcn **neutral dark** defaults (their `.dark`), converted to sRGB.
    #[must_use]
    pub const fn dark() -> Self {
        Self {
            background: Color32::from_rgb(0x0a, 0x0a, 0x0a),
            foreground: Color32::from_rgb(0xfa, 0xfa, 0xfa),
            card: Color32::from_rgb(0x17, 0x17, 0x17),
            card_foreground: Color32::from_rgb(0xfa, 0xfa, 0xfa),
            // shadcn's neutral `--popover` matches `--card` by default (both
            // #171717) — same value, independent token.
            widget: Color32::from_rgb(0x17, 0x17, 0x17),
            primary: Color32::from_rgb(0xe5, 0xe5, 0xe5),
            primary_foreground: Color32::from_rgb(0x17, 0x17, 0x17),
            secondary: Color32::from_rgb(0x26, 0x26, 0x26),
            secondary_foreground: Color32::from_rgb(0xfa, 0xfa, 0xfa),
            muted: Color32::from_rgb(0x26, 0x26, 0x26),
            muted_foreground: Color32::from_rgb(0xa1, 0xa1, 0xa1),
            accent: Color32::from_rgb(0x26, 0x26, 0x26),
            accent_foreground: Color32::from_rgb(0xfa, 0xfa, 0xfa),
            destructive: Color32::from_rgb(0xff, 0x64, 0x67),
            destructive_foreground: Color32::from_rgb(0xfa, 0xfa, 0xfa),
            border: Color32::from_rgb(0x2b, 0x2b, 0x2b),
            input: Color32::from_rgb(0x33, 0x33, 0x33),
            ring: Color32::from_rgb(0x73, 0x73, 0x73),
            radius: 10.0,
        }
    }

    /// The shadcn default for the given mode (`true` = dark).
    #[must_use]
    pub const fn for_mode(dark: bool) -> Self {
        if dark {
            Self::dark()
        } else {
            Self::light()
        }
    }

    /// Linearly interpolate every colour token between `self` and `other` by
    /// `t` (0 = `self`, 1 = `other`). The non-colour `radius` follows `self`.
    ///
    /// Useful for animating a light↔dark theme switch: blend the two palettes
    /// each frame and [`apply`](Self::apply) the result.
    #[must_use]
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let f = |a: Color32, b: Color32| a.lerp_to_gamma(b, t);
        Self {
            background: f(self.background, other.background),
            foreground: f(self.foreground, other.foreground),
            card: f(self.card, other.card),
            card_foreground: f(self.card_foreground, other.card_foreground),
            widget: f(self.widget, other.widget),
            primary: f(self.primary, other.primary),
            primary_foreground: f(self.primary_foreground, other.primary_foreground),
            secondary: f(self.secondary, other.secondary),
            secondary_foreground: f(self.secondary_foreground, other.secondary_foreground),
            muted: f(self.muted, other.muted),
            muted_foreground: f(self.muted_foreground, other.muted_foreground),
            accent: f(self.accent, other.accent),
            accent_foreground: f(self.accent_foreground, other.accent_foreground),
            destructive: f(self.destructive, other.destructive),
            destructive_foreground: f(self.destructive_foreground, other.destructive_foreground),
            border: f(self.border, other.border),
            input: f(self.input, other.input),
            ring: f(self.ring, other.ring),
            radius: self.radius,
        }
    }

    /// Write these tokens into an [`egui::Visuals`], mapping each shadcn token
    /// onto the egui field components actually read.
    ///
    /// The mapping (so [`Tokens::get`] can read it back) is:
    ///
    /// | shadcn token         | egui `Visuals` field                         |
    /// |----------------------|----------------------------------------------|
    /// | `background`         | `panel_fill`, `window_fill`                  |
    /// | `foreground`         | `widgets.{noninteractive,inactive}.fg_stroke` |
    /// | `card`               | `widgets.noninteractive.bg_fill`             |
    /// | `widget`             | `extreme_bg_color` (native `TextEdit` fill)  |
    /// | `primary`            | `selection.bg_fill`                          |
    /// | `primary_foreground` | `selection.stroke`                           |
    /// | `secondary`          | `widgets.inactive.{bg_fill,weak_bg_fill}`    |
    /// | `accent`             | `widgets.{hovered,active}.{bg_fill,weak_bg_fill}` |
    /// | `muted`              | `faint_bg_color`, `code_bg_color`            |
    /// | `muted_foreground`   | `weak_text_color`                            |
    /// | `border`             | `widgets.noninteractive.bg_stroke`, `window_stroke` |
    /// | `input`              | `widgets.inactive.bg_stroke`                 |
    /// | `ring`               | `widgets.active.bg_stroke`                   |
    /// | `destructive`        | `error_fg_color`                             |
    /// | `radius`             | every `widgets.*.corner_radius`              |
    pub fn apply(&self, v: &mut Visuals) {
        // Surfaces.
        v.panel_fill = self.background;
        v.window_fill = self.background;
        // Opaque widget surface, not the (possibly translucent) app canvas —
        // native `TextEdit`s should stay solid under a translucent theme.
        v.extreme_bg_color = self.widget;
        v.faint_bg_color = self.muted;
        v.code_bg_color = self.muted;

        // Store shadcn's exact `muted-foreground` rather than letting egui
        // derive a washed-out `foreground × weak_text_alpha` blend.
        v.weak_text_color = Some(self.muted_foreground);

        // Borders.
        v.window_stroke = Stroke::new(1.0, self.border);

        let md = CornerRadius::same(self.radius_md());

        // Non-interactive (labels, separators, cards).
        let ni = &mut v.widgets.noninteractive;
        ni.bg_fill = self.card;
        ni.weak_bg_fill = self.card;
        ni.bg_stroke = Stroke::new(1.0, self.border);
        ni.fg_stroke = Stroke::new(1.0, self.foreground);
        ni.corner_radius = md;

        // Inactive (resting interactive widget = secondary surface).
        let ia = &mut v.widgets.inactive;
        ia.bg_fill = self.secondary;
        ia.weak_bg_fill = self.secondary;
        ia.bg_stroke = Stroke::new(1.0, self.input);
        ia.fg_stroke = Stroke::new(1.0, self.foreground);
        ia.corner_radius = md;

        // Hovered = accent surface.
        let hv = &mut v.widgets.hovered;
        hv.bg_fill = self.accent;
        hv.weak_bg_fill = self.accent;
        hv.bg_stroke = Stroke::new(1.0, self.border);
        hv.fg_stroke = Stroke::new(1.0, self.accent_foreground);
        hv.corner_radius = md;

        // Active = accent surface + focus ring.
        let ac = &mut v.widgets.active;
        ac.bg_fill = self.accent;
        ac.weak_bg_fill = self.accent;
        ac.bg_stroke = Stroke::new(2.0, self.ring);
        ac.fg_stroke = Stroke::new(1.0, self.accent_foreground);
        ac.corner_radius = md;

        // Open (dropdowns) — keep consistent with hovered.
        v.widgets.open.bg_fill = self.accent;
        v.widgets.open.weak_bg_fill = self.accent;
        v.widgets.open.fg_stroke = Stroke::new(1.0, self.accent_foreground);
        v.widgets.open.corner_radius = md;

        // Selection / primary accent.
        v.selection.bg_fill = self.primary;
        v.selection.stroke = Stroke::new(1.0, self.primary_foreground);

        // Semantic.
        v.hyperlink_color = self.foreground;
        v.error_fg_color = self.destructive;

        v.window_corner_radius = CornerRadius::same(self.radius_xl());
    }

    /// Read the tokens back out of a live [`Visuals`].
    ///
    /// The inverse of [`Tokens::apply`]. A handful of tokens have no dedicated
    /// egui field and are derived: `card_foreground`/`secondary_foreground`
    /// reuse `foreground`, `muted_foreground` uses [`Visuals::weak_text_color`],
    /// and `destructive_foreground` is a readable contrast color.
    #[must_use]
    pub fn from_visuals(v: &Visuals) -> Self {
        let foreground = v.widgets.noninteractive.fg_stroke.color;
        let destructive = v.error_fg_color;
        let radius = f32::from(v.widgets.inactive.corner_radius.nw) / 0.8;
        Self {
            background: v.panel_fill,
            foreground,
            card: v.widgets.noninteractive.bg_fill,
            card_foreground: foreground,
            widget: v.extreme_bg_color,
            primary: v.selection.bg_fill,
            primary_foreground: v.selection.stroke.color,
            secondary: v.widgets.inactive.bg_fill,
            secondary_foreground: foreground,
            muted: v.faint_bg_color,
            // Prefer the exact stored token; fall back to egui's derived blend.
            muted_foreground: v.weak_text_color.unwrap_or_else(|| v.weak_text_color()),
            accent: v.widgets.hovered.bg_fill,
            accent_foreground: v.widgets.hovered.fg_stroke.color,
            destructive,
            destructive_foreground: contrast_on(destructive),
            border: v.widgets.noninteractive.bg_stroke.color,
            input: v.widgets.inactive.bg_stroke.color,
            ring: v.widgets.active.bg_stroke.color,
            radius,
        }
    }

    /// Resolve the active tokens for this [`Ui`] — a typed view of
    /// `ui.visuals()`.
    #[must_use]
    pub fn get(ui: &Ui) -> Self {
        Self::from_visuals(ui.visuals())
    }

    /// Resolve the active tokens from a [`Context`](egui::Context) — for code
    /// (like modals) that runs outside a `Ui`.
    #[must_use]
    pub fn get_ctx(ctx: &egui::Context) -> Self {
        Self::from_visuals(&ctx.global_style().visuals)
    }

    // --- Radius scale (mirrors shadcn's --radius-sm/md/lg/xl) ---------------

    /// `radius-sm` = `radius × 0.6`.
    #[must_use]
    pub const fn radius_sm(self) -> u8 {
        to_u8(self.radius * 0.6)
    }

    /// `radius-md` = `radius × 0.8` (buttons, inputs, badges).
    #[must_use]
    pub const fn radius_md(self) -> u8 {
        to_u8(self.radius * 0.8)
    }

    /// `radius-lg` = `radius` (the base).
    #[must_use]
    pub const fn radius_lg(self) -> u8 {
        to_u8(self.radius)
    }

    /// `radius-xl` = `radius × 1.4` (cards).
    #[must_use]
    pub const fn radius_xl(self) -> u8 {
        to_u8(self.radius * 1.4)
    }

    /// `radius-2xl` = `radius × 1.8` (inner tiles).
    #[must_use]
    pub const fn radius_2xl(self) -> u8 {
        to_u8(self.radius * 1.8)
    }

    /// `radius-3xl` = `radius × 2.2`.
    #[must_use]
    pub const fn radius_3xl(self) -> u8 {
        to_u8(self.radius * 2.2)
    }

    /// `radius-4xl` = `min(radius × 2.6, 24)` (outer card corners, shadcn caps
    /// this at 24px).
    #[must_use]
    pub const fn radius_4xl(self) -> u8 {
        let r = to_u8(self.radius * 2.6);
        if r > 24 {
            24
        } else {
            r
        }
    }
}

impl Default for Tokens {
    fn default() -> Self {
        Self::light()
    }
}

/// Build an [`egui::Visuals`] preloaded with shadcn's neutral palette.
///
/// Install it once at startup:
///
/// ```no_run
/// # let ctx = egui::Context::default();
/// ctx.set_visuals(glazier::shadcn_visuals(false)); // light
/// ```
#[must_use]
pub fn shadcn_visuals(dark: bool) -> Visuals {
    let mut visuals = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    Tokens::for_mode(dark).apply(&mut visuals);
    visuals
}

/// Build a [`Visuals`] from an explicit token set (e.g. a [`Tokens::lerp`]
/// blend), choosing the dark/light egui base by which palette `t` is closer to.
#[must_use]
pub fn shadcn_visuals_from(tokens: Tokens, dark_base: bool) -> Visuals {
    let mut visuals = if dark_base {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    tokens.apply(&mut visuals);
    visuals
}

/// Apply glazier's opinionated interaction defaults to a [`Style`].
///
/// Right now this just makes **prose and headings non-selectable by default** —
/// egui ships with `selectable_labels = true` (a web-ish default), but in an
/// opinionated component system chrome text shouldn't be drag-selectable. Data
/// you *do* want copyable opts back in per-widget (e.g.
/// [`Label::selectable`](crate::Label::selectable)).
///
/// Call once at startup alongside [`shadcn_visuals`]:
///
/// ```no_run
/// # let ctx = egui::Context::default();
/// ctx.global_style_mut(glazier::apply_style);
/// ```
pub const fn apply_style(style: &mut egui::Style) {
    style.interaction.selectable_labels = false;
}

/// Pick black or white for legible text on `bg`.
fn contrast_on(bg: Color32) -> Color32 {
    // Rec. 601 luma on the un-premultiplied channels.
    let luma = 0.114f32.mul_add(
        f32::from(bg.b()),
        0.299f32.mul_add(f32::from(bg.r()), 0.587 * f32::from(bg.g())),
    );
    if luma > 140.0 {
        Color32::from_rgb(0x0a, 0x0a, 0x0a)
    } else {
        Color32::from_rgb(0xfa, 0xfa, 0xfa)
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
const fn to_u8(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}
