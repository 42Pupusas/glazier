//! Bundled font faces + registration, so glazier gets *real* weights.
//!
//! egui has no font-weight axis: a [`FontFamily`] maps to exactly one face, and
//! `.strong()` only recolours. shadcn buttons use `font-medium` (weight 500), so
//! to match them we bundle Open Sans (Regular / Semibold / Bold) and register
//! each as its own named family. [`install`] wires them in and points
//! [`TextStyle::Button`](egui::TextStyle::Button) at the semibold face — so every
//! [`Button`](crate::Button) renders with genuine weight, crisply, no faux-bold.

use egui::{Context, FontData, FontDefinitions, FontFamily, FontId, TextStyle, Ui};

/// Named family bound to the semibold face (`font-medium`/`font-semibold`).
pub const SEMIBOLD: &str = "glazier-semibold";
/// Named family bound to the bold face (`font-bold`).
pub const BOLD: &str = "glazier-bold";

// Faces bundled into the binary so apps need no system fonts.
const REGULAR: &[u8] = include_bytes!("../assets/fonts/OpenSans-Regular.ttf");
const SEMIBOLD_TTF: &[u8] = include_bytes!("../assets/fonts/OpenSans-Semibold.ttf");
const BOLD_TTF: &[u8] = include_bytes!("../assets/fonts/OpenSans-Bold.ttf");

/// A [`FontId`] at `size` in the semibold family — resolved from this `ui`'s
/// live [`TextStyle::Button`] family rather than the raw [`SEMIBOLD`] name.
///
/// [`set_fonts`](Context::set_fonts) only takes effect on the *next* frame, so
/// on the very first frame `FontFamily::Name(SEMIBOLD)` isn't bound yet and
/// laying text out with it panics. The `Ui`'s style snapshot lags in lockstep
/// with the bound fonts, so routing through it falls back to the default
/// proportional face for that one frame instead of crashing.
#[must_use]
pub fn semibold(ui: &Ui, size: f32) -> FontId {
    let family = ui
        .style()
        .text_styles
        .get(&TextStyle::Button)
        .map_or(FontFamily::Proportional, |f| f.family.clone());
    FontId::new(size, family)
}

/// A [`FontId`] at `size` in the bold family.
///
/// Guarded the same way as [`semibold`]: on the very first frame — before
/// [`install`] has taken effect — the [`BOLD`] family isn't bound yet, so probe
/// whether it resolves and fall back to the [`TextStyle::Button`] (semibold)
/// family for that one frame instead of panicking.
#[must_use]
pub fn bold(ui: &Ui, size: f32) -> FontId {
    let bound = ui.ctx().fonts(|f| {
        f.families()
            .iter()
            .any(|fam| matches!(fam, FontFamily::Name(n) if n.as_ref() == BOLD))
    });
    if bound {
        FontId::new(size, FontFamily::Name(BOLD.into()))
    } else {
        semibold(ui, size)
    }
}

/// Register glazier's bundled fonts on `ctx` and bind the button text style to
/// the semibold face.
///
/// Call once at startup, alongside [`shadcn_visuals`](crate::shadcn_visuals):
///
/// ```no_run
/// # let ctx = egui::Context::default();
/// glazier::install_fonts(&ctx);
/// ctx.set_visuals(glazier::shadcn_visuals(false));
/// ```
pub fn install(ctx: &Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "glazier-regular".to_owned(),
        FontData::from_static(REGULAR).into(),
    );
    fonts.font_data.insert(
        SEMIBOLD.to_owned(),
        FontData::from_static(SEMIBOLD_TTF).into(),
    );
    fonts
        .font_data
        .insert(BOLD.to_owned(), FontData::from_static(BOLD_TTF).into());

    // Open Sans Regular becomes the default proportional face (body text),
    // keeping egui's built-in fallbacks behind it for glyph coverage.
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "glazier-regular".to_owned());

    // Each weight is its own named family (egui has no weight axis).
    fonts
        .families
        .insert(FontFamily::Name(SEMIBOLD.into()), vec![SEMIBOLD.to_owned()]);
    fonts
        .families
        .insert(FontFamily::Name(BOLD.into()), vec![BOLD.to_owned()]);

    ctx.set_fonts(fonts);

    // Point the button text style at the semibold face so buttons get real
    // weight-500 without any per-widget plumbing.
    ctx.global_style_mut(|style| {
        if let Some(button) = style.text_styles.get_mut(&TextStyle::Button) {
            *button = FontId::new(button.size, FontFamily::Name(SEMIBOLD.into()));
        }
    });
}
