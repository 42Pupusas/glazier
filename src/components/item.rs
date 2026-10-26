//! [`Item`] — a flexible inset content container, mirroring shadcn's `<Item>`.
//!
//! An `Item` is the "card-inside-a-card" surface: a `rounded-2xl` block that
//! groups rows of content. The [`Variant`] picks its chrome — `Muted` (a
//! `bg-muted/50` fill, no border), `Outline` (transparent fill, hairline
//! border), or `Default` (no fill or border, just padding).
//!
//! Mark an item [`interactive`](Item::interactive) to render shadcn's link-style
//! `<a>` row: it darkens to full `muted` on hover, shows a pointing-hand
//! cursor, and returns a clickable [`Response`].

use egui::{Frame, Response, Sense, Stroke, Ui, Widget};

use crate::customize::{Customize, StyleHook};
use crate::tokens::Tokens;

/// Visual style of an [`Item`], mirroring shadcn's `variant` prop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    /// No fill or border — just padding.
    #[default]
    Default,
    /// Transparent fill with a hairline border.
    Outline,
    /// A subtle `muted` fill, no border.
    Muted,
}

/// An inset content container.
///
/// ```no_run
/// use glazier::item::{Item, Variant};
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// Item::new().variant(Variant::Muted).show(ui, |ui| {
///     ui.label("…rows…");
/// });
/// # });
/// ```
#[must_use = "items do nothing unless shown"]
#[derive(Default)]
pub struct Item {
    variant: Variant,
    interactive: Option<egui::Id>,
    style_hook: StyleHook<Frame>,
}

impl Customize<Frame> for Item {
    fn style_hook_mut(&mut self) -> &mut StyleHook<Frame> {
        &mut self.style_hook
    }
}

impl Item {
    /// Create an item with the default variant.
    pub const fn new() -> Self {
        Self {
            variant: Variant::Default,
            interactive: None,
            style_hook: StyleHook::new(),
        }
    }

    /// Set the visual [`Variant`].
    pub const fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Make the item a clickable link-style row (shadcn's `<a>` item): it
    /// darkens to full `muted` on hover, shows a pointing-hand cursor, and the
    /// returned [`Response`] reports clicks. `id_salt` distinguishes sibling
    /// items so their interaction state doesn't collide.
    pub fn interactive(mut self, id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.interactive = Some(egui::Id::unique(id_salt));
        self
    }

    /// Render the item with `content` inside its padded surface.
    ///
    /// [`style`](Item::style) only affects the non-[`interactive`](Item::interactive)
    /// path.
    pub fn show<R>(mut self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> Response {
        let tokens = Tokens::get(ui);
        if let Some(id_salt) = self.interactive {
            return Self::show_interactive(ui, tokens, id_salt, content);
        }

        let (fill, stroke) = match self.variant {
            Variant::Default => (egui::Color32::TRANSPARENT, Stroke::NONE),
            Variant::Outline => (egui::Color32::TRANSPARENT, Stroke::new(1.0, tokens.border)),
            // `bg-muted/50` — half-strength muted over the card.
            Variant::Muted => (tokens.muted.gamma_multiply(0.5), Stroke::NONE),
        };

        let mut frame = Frame::new()
            .fill(fill)
            .stroke(stroke)
            .corner_radius(tokens.radius_2xl())
            .inner_margin(14.0);
        std::mem::take(&mut self.style_hook).apply(&mut frame);

        frame
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                content(ui);
            })
            .response
    }

    /// The link-style row.
    ///
    /// It must know its hover state before painting the fill, but the rect
    /// is only known once the content lays out. So it reserves a paint slot,
    /// draws the content, then back-fills the surface once hover is resolved.
    fn show_interactive<R>(
        ui: &mut Ui,
        tokens: Tokens,
        id_salt: egui::Id,
        content: impl FnOnce(&mut Ui) -> R,
    ) -> Response {
        let bg = ui.painter().add(egui::Shape::Noop);
        let inner = Frame::new().inner_margin(14.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            content(ui)
        });
        let rect = inner.response.rect;
        let response = ui.interact(rect, ui.scope_id().with(id_salt), Sense::click());
        // `bg-muted/50` at rest → full `bg-muted` on hover (shadcn
        // `[a]:hover:bg-muted`), eased with `transition-colors`.
        let hover_t =
            ui.ctx()
                .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.15);
        let fill = tokens
            .muted
            .gamma_multiply(0.5)
            .lerp_to_gamma(tokens.muted, hover_t);
        ui.painter().set(
            bg,
            egui::Shape::rect_filled(rect, tokens.radius_2xl(), fill),
        );
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}

impl Widget for Item {
    /// Renders an empty padded surface. Use [`Item::show`] for content.
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui, |_| {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;

    #[test]
    fn each_variant_wraps_its_content() {
        let mut rects = Vec::new();
        TestUi::new().frame(|ui| {
            for v in [Variant::Default, Variant::Outline, Variant::Muted] {
                rects.push(Item::new().variant(v).show(ui, |ui| ui.label("row")).rect);
            }
            rects.push(ui.add(Item::default()).rect);
        });
        assert!(rects[..3].iter().all(|r| r.height() > 28.0));
    }

    #[test]
    fn an_interactive_item_reports_clicks() {
        let ui = TestUi::new();
        let mut clicks = 0;
        let mut build = |ui: &mut Ui| {
            let r = Item::new()
                .interactive("link")
                .show(ui, |ui| ui.label("Open"));
            clicks += usize::from(r.clicked());
        };
        let out = ui.frame(&mut build);
        ui.click(TestUi::text_rect(&out, "Open").center(), &mut build);
        drop(build);
        assert_eq!(clicks, 1);
    }
}
