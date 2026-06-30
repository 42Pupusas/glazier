//! [`Icon`] — a recolourable, crisply-scaled SVG icon.
//!
//! Feed it raw SVG markup — e.g. a [lucide](https://lucide.dev) icon, which is
//! what shadcn uses — and it renders through `egui_extras`' SVG loader, sized to
//! `size-4` (16px) by default and tinted to a token colour.
//!
//! ## currentColor
//!
//! Lucide icons paint with `stroke="currentColor"` / `fill="currentColor"`,
//! which resvg can't resolve on its own. [`Icon`] rewrites `currentColor` to
//! **white** in the source, then uses egui's image [`tint`](egui::Image::tint)
//! (a multiply) to colour it — so one rasterisation recolours to anything,
//! mirroring CSS `color`.
//!
//! ```no_run
//! use glazier::icon::Icon;
//! use egui::Widget as _;
//! # egui::__run_test_ui(|ui| {
//! // A lucide "search" glyph.
//! const SEARCH: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>"#;
//! Icon::new(SEARCH).size(16.0).ui(ui);
//! # });
//! ```
//!
//! Install the SVG loader once at startup (glazier does this for you in
//! [`install_fonts`](crate::install_fonts)'s sibling — call
//! [`install_image_loaders`](egui_extras::install_image_loaders) yourself, or
//! use [`Icon::install`]).

use std::hash::{Hash as _, Hasher as _};

use egui::{Color32, Context, Image, Response, Ui, Vec2, Widget};

use crate::tokens::Tokens;

/// Default icon edge in points — shadcn's `size-4`.
const DEFAULT_SIZE: f32 = 16.0;

/// A recolourable SVG icon.
#[must_use = "icons do nothing unless you add them to a Ui"]
#[derive(Clone, Copy)]
pub struct Icon {
    svg: &'static str,
    size: f32,
    color: Option<Color32>,
}

impl Icon {
    /// Create an icon from raw SVG markup (e.g. a lucide glyph).
    pub const fn new(svg: &'static str) -> Self {
        Self {
            svg,
            size: DEFAULT_SIZE,
            color: None,
        }
    }

    /// Set the square edge length in points (default 16 = `size-4`).
    pub const fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Tint the icon. Defaults to the active `foreground` token (CSS
    /// `currentColor` semantics).
    pub const fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    /// Install the SVG image loader on `ctx`. Call once at startup (idempotent).
    pub fn install(ctx: &Context) {
        egui_extras::install_image_loaders(ctx);
    }

    /// Build the underlying tinted [`egui::Image`] for this icon, resolving the
    /// colour against `tokens`. Exposed so other components (buttons, input
    /// addons) can embed an icon inline.
    pub fn image(&self, tokens: Tokens) -> Image<'static> {
        let color = self.color.unwrap_or(tokens.foreground);

        // resvg can't resolve `currentColor`; bake white so the tint multiply
        // reproduces `color` exactly. A stable per-source hash keeps the loader
        // cache keyed on the markup (not the colour — tint is applied later).
        let baked = self.svg.replace("currentColor", "#ffffff");
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.svg.hash(&mut hasher);
        let uri = format!("bytes://glazier-icon-{:016x}.svg", hasher.finish());

        Image::from_bytes(uri, baked.into_bytes())
            .fit_to_exact_size(Vec2::splat(self.size))
            .tint(color)
    }
}

impl Widget for Icon {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        // Ensure the SVG loader is present even if the app forgot to install it.
        Self::install(ui.ctx());
        ui.add(self.image(tokens))
    }
}
