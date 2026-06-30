//! Real font weights, bring-your-own bytes.
//!
//! egui has no font-weight axis: a [`FontFamily`] maps to exactly one face, and
//! `.strong()` only recolours. shadcn buttons use `font-medium` (weight 500),
//! so to match them glazier wants a real semibold/bold face — but a *library*
//! crate has no business bundling font binaries (publish-tarball weight,
//! licensing surface, and zero way for an app to swap in its own brand font).
//!
//! So glazier ships **no fonts**. [`install`] only registers the [`SEMIBOLD`]
//! / [`BOLD`] family *names* and points
//! [`TextStyle::Button`](egui::TextStyle::Button) at the semibold one — but
//! leaves them unbound, so they fall back to egui's default proportional face
//! (no faux-bold, just egui's normal weight) until you supply real faces:
//!
//! - [`install_with`] — hand it a [`FontSet`] (e.g. `include_bytes!` in *your*
//!   binary, or bytes loaded from disk/a system font at runtime) for genuine
//!   weight fidelity.
//! - [`FontSet::from_system`] *(feature `system-fonts`)* — shells out to
//!   `fc-match` to find a system sans face and its bold variant, so apps that
//!   don't care to bundle anything still get real weights for free.

use egui::{Context, FontData, FontDefinitions, FontFamily, FontId, TextStyle, Ui};

/// Named family bound to the semibold face (`font-medium`/`font-semibold`).
pub const SEMIBOLD: &str = "glazier-semibold";
/// Named family bound to the bold face (`font-bold`).
pub const BOLD: &str = "glazier-bold";

/// Font bytes for the three weights glazier's components reach for.
///
/// Build one yourself (e.g. `include_bytes!` your own TTFs/OTFs in your
/// binary) and pass it to [`install_with`], or — with the `system-fonts`
/// feature — let [`FontSet::from_system`] find one on disk.
pub struct FontSet {
    /// Regular weight; becomes the default proportional (body) face.
    pub regular: Vec<u8>,
    /// Semibold/medium weight; bound to [`SEMIBOLD`] and `TextStyle::Button`.
    pub semibold: Vec<u8>,
    /// Bold weight; bound to [`BOLD`].
    pub bold: Vec<u8>,
}

#[cfg(feature = "system-fonts")]
impl FontSet {
    /// Look up a system sans face and its bold variant via `fontconfig`
    /// (`fc-match`), reading the matched files off disk.
    ///
    /// Uses the same file for `regular`/`semibold` (fontconfig only resolves
    /// regular/bold, not the in-between weight egui has no axis for anyway) so
    /// callers still get *a* real face instead of nothing. Returns `None` if
    /// `fc-match` isn't installed or no match is found/readable.
    #[must_use]
    pub fn from_system() -> Option<Self> {
        let regular = Self::fc_match("sans-serif")?;
        let bold = Self::fc_match("sans-serif:bold").unwrap_or_else(|| regular.clone());
        Some(Self {
            regular: regular.clone(),
            semibold: regular,
            bold,
        })
    }

    fn fc_match(pattern: &str) -> Option<Vec<u8>> {
        let out = std::process::Command::new("fc-match")
            .args(["--format=%{file}", pattern])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let path = String::from_utf8(out.stdout).ok()?;
        std::fs::read(path.trim()).ok()
    }
}

/// A [`FontId`] at `size` in the semibold family — resolved from this `ui`'s
/// live [`TextStyle::Button`] family rather than the raw [`SEMIBOLD`] name.
///
/// [`set_fonts`](Context::set_fonts) only takes effect on the *next* frame, so
/// on the very first frame `FontFamily::Name(SEMIBOLD)` isn't bound yet and
/// laying text out with it panics. The `Ui`'s style snapshot lags in lockstep
/// with the bound fonts, so routing through it falls back to the default
/// proportional face for that one frame instead of crashing. The same fallback
/// also covers apps that never bind a real semibold face at all (see
/// [`install`]).
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
/// Guarded the same way as [`semibold`]: if the [`BOLD`] family isn't bound
/// (either it's the first frame, or no [`FontSet`] was ever installed), fall
/// back to the [`TextStyle::Button`] (semibold) family instead of panicking.
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

/// Register the [`SEMIBOLD`]/[`BOLD`] family *names* and point the button text
/// style at the semibold one, without binding any actual font data.
///
/// Until you call [`install_with`], [`semibold`]/[`bold`] (and therefore every
/// glazier component) quietly fall back to egui's default proportional face —
/// no faux-bold, no panic, just egui's normal weight.
///
/// ```no_run
/// # let ctx = egui::Context::default();
/// glazier::install_fonts(&ctx);
/// ctx.set_visuals(glazier::shadcn_visuals(false));
/// ```
pub fn install(ctx: &Context) {
    ctx.global_style_mut(|style| {
        if let Some(button) = style.text_styles.get_mut(&TextStyle::Button) {
            *button = FontId::new(button.size, FontFamily::Name(SEMIBOLD.into()));
        }
    });
}

/// Register `fonts` on `ctx` for real weight fidelity, and bind the button
/// text style to the semibold face.
///
/// ```no_run
/// # let ctx = egui::Context::default();
/// // Bytes come from your own binary (e.g. `include_bytes!` your bundled
/// // TTFs) or from disk/a system font at runtime.
/// let fonts = glazier::fonts::FontSet {
///     regular: std::fs::read("OpenSans-Regular.ttf").unwrap(),
///     semibold: std::fs::read("OpenSans-Semibold.ttf").unwrap(),
///     bold: std::fs::read("OpenSans-Bold.ttf").unwrap(),
/// };
/// glazier::fonts::install_with(&ctx, fonts);
/// ctx.set_visuals(glazier::shadcn_visuals(false));
/// ```
pub fn install_with(ctx: &Context, font_set: FontSet) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "glazier-regular".to_owned(),
        FontData::from_owned(font_set.regular).into(),
    );
    fonts.font_data.insert(
        SEMIBOLD.to_owned(),
        FontData::from_owned(font_set.semibold).into(),
    );
    fonts
        .font_data
        .insert(BOLD.to_owned(), FontData::from_owned(font_set.bold).into());

    // The regular face becomes the default proportional face (body text),
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
