//! [`Marker`] — an inline conversation marker, mirroring shadcn's `<Marker>`.
//!
//! Markers are the small, non-message rows that punctuate a chat transcript:
//! status updates ("Thinking…"), system notes ("Conversation compacted"),
//! bordered row boundaries, and labelled dividers ("Today"). They pair with
//! [`Message`](crate::Message) inside a [`MessageScroller`](crate::MessageScroller).
//!
//! The [`Variant`] picks the chrome:
//! - [`Default`](Variant::Default) — an inline `muted_foreground` row with an
//!   optional leading [`Icon`];
//! - [`Border`](Variant::Border) — the same row with a hairline rule beneath it;
//! - [`Separator`](Variant::Separator) — a centred label flanked by divider
//!   lines (shadcn's `last:border-b-0` date/section break).
//!
//! Streaming markers ("Thinking…") can opt into [`shimmer`](Marker::shimmer):
//! a bright band sweeps across the text on wall-clock time, mirroring shadcn's
//! `shimmer` utility on `MarkerContent`.

use egui::{Color32, Response, Sense, Ui, Vec2, Widget};

use crate::components::icon::Icon;
use crate::customize::{Customize, StyleHook};
use crate::tokens::Tokens;

/// Marker text size (shadcn `text-sm`, tuned to the transcript scale).
const TEXT_SIZE: f32 = 13.0;
/// Leading icon edge (shadcn `size-4`).
const ICON: f32 = 16.0;
/// Gap between the icon and the label (shadcn `gap-2`).
const ICON_GAP: f32 = 8.0;
/// Gap between a separator's label and its divider lines (shadcn `gap-2`).
const SEP_GAP: f32 = 8.0;
/// Row height, so consecutive markers share a centreline.
const ROW_H: f32 = 24.0;
/// Shimmer sweep period in seconds.
const SHIMMER_PERIOD: f64 = 1.6;

/// Visual style of a [`Marker`], mirroring shadcn's `variant` prop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    /// An inline marker for status, notes, and actions.
    #[default]
    Default,
    /// A default marker with a hairline rule beneath the row.
    Border,
    /// A centred label with divider lines on each side.
    Separator,
}

/// An inline conversation marker.
///
/// ```no_run
/// use glazier::marker::{Marker, Variant};
/// use glazier::icon::Icon;
/// use egui::Widget as _;
/// # const CHECK: &str = "<svg/>";
/// # egui::__run_test_ui(|ui| {
/// Marker::new("Explored 4 files").icon(Icon::new(CHECK)).ui(ui);
/// Marker::new("Today").variant(Variant::Separator).ui(ui);
/// Marker::new("Thinking…").shimmer(true).ui(ui);
/// # });
/// ```
#[must_use = "markers do nothing unless you add them to a Ui"]
pub struct Marker {
    text: String,
    variant: Variant,
    icon: Option<Icon>,
    shimmer: bool,
    style_hook: StyleHook<MarkerStyle>,
}

/// [`Marker`]'s resolved paint — the text/icon colour. The real value
/// [`Marker`] paints with; reach in via [`Marker::style`].
#[derive(Clone, Copy, Debug)]
pub struct MarkerStyle {
    /// Text + icon colour (defaults to `muted_foreground`).
    pub color: Color32,
}

impl Marker {
    /// Create a marker with the given label.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            variant: Variant::default(),
            icon: None,
            shimmer: false,
            style_hook: StyleHook::default(),
        }
    }

    /// Set the visual [`Variant`].
    pub const fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Add a leading [`Icon`] (`size-4`), tinted to the marker's text colour.
    /// Ignored by the [`Separator`](Variant::Separator) variant.
    pub const fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Animate the label with shadcn's streaming `shimmer` sweep. Use for
    /// in-progress status text ("Thinking…", "Running tests").
    pub const fn shimmer(mut self, shimmer: bool) -> Self {
        self.shimmer = shimmer;
        self
    }
}

impl Customize<MarkerStyle> for Marker {
    fn style_hook_mut(&mut self) -> &mut StyleHook<MarkerStyle> {
        &mut self.style_hook
    }
}

impl Widget for Marker {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let mut style = MarkerStyle {
            color: tokens.muted_foreground,
        };
        self.style_hook.apply(&mut style);
        let MarkerStyle { color } = style;

        if self.variant == Variant::Separator {
            return separator(ui, &self.text, color, tokens);
        }

        let width = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, ROW_H), Sense::hover());

        if ui.is_rect_visible(rect) {
            let mut cursor = rect.left();
            if let Some(icon) = self.icon {
                let ir = egui::Rect::from_min_size(
                    egui::pos2(cursor, rect.center().y - ICON / 2.0),
                    Vec2::splat(ICON),
                );
                icon.size(ICON).color(color).image(tokens).paint_at(ui, ir);
                cursor += ICON + ICON_GAP;
            }

            let font = egui::FontId::proportional(TEXT_SIZE);
            let galley = ui
                .painter()
                .layout_no_wrap(self.text.clone(), font.clone(), color);
            let ty = rect.center().y - galley.size().y / 2.0;
            let pos = egui::pos2(cursor, ty);
            ui.painter().galley(pos, galley.clone(), color);

            if self.shimmer {
                paint_shimmer(ui, &self.text, font, pos, galley.size(), tokens);
            }

            // The border variant rules off the bottom of the row.
            if self.variant == Variant::Border {
                ui.painter().hline(
                    rect.left()..=rect.right(),
                    rect.bottom(),
                    egui::Stroke::new(1.0, tokens.border),
                );
            }
        }

        response
    }
}

/// Paint the [`Separator`](Variant::Separator) variant: a centred label with a
/// hairline rule extending to each edge.
fn separator(ui: &mut Ui, text: &str, color: Color32, tokens: Tokens) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, ROW_H), Sense::hover());

    if ui.is_rect_visible(rect) {
        let galley = ui.painter().layout_no_wrap(
            text.to_owned(),
            egui::FontId::proportional(TEXT_SIZE),
            color,
        );
        let tw = galley.size().x;
        let cx = rect.center().x;
        let half = tw / 2.0;
        let cy = rect.center().y;
        let stroke = egui::Stroke::new(1.0, tokens.border);
        // Left rule.
        ui.painter()
            .hline(rect.left()..=(cx - half - SEP_GAP), cy, stroke);
        // Right rule.
        ui.painter()
            .hline((cx + half + SEP_GAP)..=rect.right(), cy, stroke);
        let pos = egui::pos2(cx - half, cy - galley.size().y / 2.0);
        ui.painter().galley(pos, galley, color);
    }

    response
}

/// Overlay a sweeping bright band on the just-painted label, brightening it
/// toward `foreground` as the band passes — shadcn's `shimmer` utility.
fn paint_shimmer(
    ui: &Ui,
    text: &str,
    font: egui::FontId,
    pos: egui::Pos2,
    size: Vec2,
    tokens: Tokens,
) {
    let time = ui.input(|i| i.time);
    #[allow(clippy::cast_possible_truncation)]
    let t = (time / SHIMMER_PERIOD).rem_euclid(1.0) as f32;

    let band = size.x.mul_add(0.35, 24.0);
    let start = pos.x - band;
    let end = pos.x + size.x + band;
    let bx = egui::lerp(start..=end, t);
    let clip = egui::Rect::from_min_max(
        egui::pos2(bx - band / 2.0, pos.y - 2.0),
        egui::pos2(bx + band / 2.0, pos.y + size.y + 2.0),
    );

    let bright = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font, tokens.foreground);
    ui.painter()
        .with_clip_rect(clip)
        .galley(pos, bright, tokens.foreground);
    ui.ctx().request_repaint();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant renders and spans the available width without panicking.
    #[test]
    fn variants_render() {
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            for variant in [Variant::Default, Variant::Border, Variant::Separator] {
                let r = Marker::new("status").variant(variant).ui(ui);
                assert!(r.rect.height() > 0.0);
            }
            // Shimmer + icon paths are exercised too.
            Marker::new("Thinking\u{2026}").shimmer(true).ui(ui);
        });
    }
}
