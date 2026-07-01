//! [`Bubble`] — the framed conversational surface, mirroring shadcn's
//! standalone `<Bubble>` (split out from [`Message`](crate::Message)).
//!
//! Where [`Message`](crate::Message) owns the *row* — avatar, name, timestamp,
//! and message-level actions — `Bubble` is deliberately scoped to just the
//! bubble: a content surface in one of seven [`Variant`]s, aligned to the
//! [`start or end`](Align) of the conversation, optionally carrying a row of
//! [`reactions`](Bubble::reactions) that overlap its edge. Use it for chat
//! text, short structured output, quoted replies, and suggestion chips.
//!
//! Faithful to shadcn's feature list:
//! - **seven variants** — `default` (strong primary), `secondary`, `muted`,
//!   `tinted` (a soft primary wash), `outline`, `ghost` (unframed, full width),
//!   and `destructive`;
//! - **start / end alignment** — receiver bubbles hug the start, sender bubbles
//!   the end;
//! - bubbles **size to their content, up to 80%** of the row width; `ghost`
//!   drops the cap so assistant text can span the full width;
//! - **reactions** that anchor to an edge ([`side`](Bubble::reactions_side)
//!   top/bottom) and [`align`](Bubble::reactions_align) start/end, overlapping
//!   the bubble like shadcn's `BubbleReactions`;
//! - stack consecutive bubbles from one sender with [`BubbleGroup`].
//!
//! ```no_run
//! use glazier::bubble::{Bubble, Variant, Align};
//! # egui::__run_test_ui(|ui| {
//! Bubble::new(Variant::Secondary).show(ui, |ui| {
//!     ui.label("Hey there! what's up?");
//! });
//!
//! Bubble::new(Variant::Default)
//!     .align(Align::End)
//!     .reactions(["👍", "🔥"])
//!     .show(ui, |ui| {
//!         ui.label("Tests passed on the first try. All 142 of them.");
//!     });
//! # });
//! ```

use egui::{
    Color32, CornerRadius, FontId, Frame, Margin, Pos2, Rect, Response, Stroke, StrokeKind, Ui,
    Vec2,
};

use crate::customize::{Customize, StyleHook};
use crate::tokens::Tokens;

/// Fraction of the row width a framed bubble may occupy before its content
/// wraps (shadcn caps bubbles at `max-w-[80%]`).
const MAX_BUBBLE_FRAC: f32 = 0.8;
/// How far the reaction row overlaps the bubble edge.
const REACTION_OVERLAP: f32 = 11.0;

/// The visual treatment of a [`Bubble`].
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Variant {
    /// A strong primary bubble, usually for the current user.
    #[default]
    Default,
    /// The standard neutral bubble for conversation content.
    Secondary,
    /// A lower-emphasis bubble for quiet supporting content.
    Muted,
    /// A subtle primary-tinted bubble.
    Tinted,
    /// A bordered bubble for secondary or rich content.
    Outline,
    /// Unframed content for assistant text — full width, no surface.
    Ghost,
    /// A destructive bubble for an error or failed action.
    Destructive,
}

/// Inline alignment — which end of the conversation a bubble (or its reaction
/// row) hugs.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Align {
    /// Align to the start (receiver bubbles; default).
    #[default]
    Start,
    /// Align to the end (sender bubbles).
    End,
}

/// Which edge a reaction row anchors to.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Side {
    /// Anchor the reactions to the upper edge.
    Top,
    /// Anchor the reactions to the lower edge (default).
    #[default]
    Bottom,
}

/// [`Bubble`]'s resolved per-variant paint — fill, text colour, border
/// stroke, and framing. The real value [`Bubble`] paints with; reach in via
/// [`Bubble::style`].
#[derive(Clone, Copy, Debug)]
pub struct BubbleStyle {
    /// Background fill (`None` for the unframed `ghost` variant).
    pub fill: Option<Color32>,
    /// Content text colour.
    pub text: Color32,
    /// Border stroke, if any (used by [`Variant::Outline`]).
    pub stroke: Option<Stroke>,
    /// Whether the bubble draws a padded surface (false for `ghost`).
    pub framed: bool,
    /// Whether the bubble may span the full row (true for `ghost`).
    pub full_width: bool,
}

impl Variant {
    fn style(self, t: Tokens) -> BubbleStyle {
        let framed = BubbleStyle {
            fill: None,
            text: t.foreground,
            stroke: None,
            framed: true,
            full_width: false,
        };
        match self {
            Self::Default => BubbleStyle {
                fill: Some(t.primary),
                text: t.primary_foreground,
                ..framed
            },
            Self::Secondary => BubbleStyle {
                fill: Some(t.secondary),
                text: t.secondary_foreground,
                ..framed
            },
            Self::Muted => BubbleStyle {
                fill: Some(t.muted),
                text: t.muted_foreground,
                ..framed
            },
            Self::Tinted => BubbleStyle {
                fill: Some(tint(t.primary, t.card)),
                ..framed
            },
            Self::Outline => BubbleStyle {
                fill: Some(t.card),
                stroke: Some(Stroke::new(1.0, t.border)),
                ..framed
            },
            Self::Destructive => BubbleStyle {
                fill: Some(t.destructive),
                text: t.destructive_foreground,
                ..framed
            },
            Self::Ghost => BubbleStyle {
                framed: false,
                full_width: true,
                ..framed
            },
        }
    }
}

/// A framed conversational bubble.
#[must_use = "bubbles do nothing unless you show them"]
pub struct Bubble {
    variant: Variant,
    align: Align,
    reactions: Vec<String>,
    reactions_side: Side,
    reactions_align: Align,
    max_frac: f32,
    style_hook: StyleHook<BubbleStyle>,
}

impl Customize<BubbleStyle> for Bubble {
    fn style_hook_mut(&mut self) -> &mut StyleHook<BubbleStyle> {
        &mut self.style_hook
    }
}

impl Bubble {
    /// Start a bubble with the given [`Variant`].
    pub fn new(variant: Variant) -> Self {
        Self {
            variant,
            align: Align::Start,
            reactions: Vec::new(),
            reactions_side: Side::Bottom,
            reactions_align: Align::End,
            max_frac: MAX_BUBBLE_FRAC,
            style_hook: StyleHook::default(),
        }
    }

    /// Align the bubble to the [`start or end`](Align) of the conversation.
    pub const fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Attach a row of reaction glyphs (emoji or counters like `+8`) that
    /// overlap the bubble edge.
    pub fn reactions<S: Into<String>>(mut self, reactions: impl IntoIterator<Item = S>) -> Self {
        self.reactions = reactions.into_iter().map(Into::into).collect();
        self
    }

    /// Choose which edge the reaction row anchors to (default
    /// [`Bottom`](Side::Bottom)).
    pub const fn reactions_side(mut self, side: Side) -> Self {
        self.reactions_side = side;
        self
    }

    /// Choose the inline alignment of the reaction row (default
    /// [`End`](Align::End)).
    pub const fn reactions_align(mut self, align: Align) -> Self {
        self.reactions_align = align;
        self
    }

    /// Override the max bubble width as a fraction of the row (default `0.8`;
    /// ignored for `ghost`, which is always full width).
    pub const fn max_frac(mut self, frac: f32) -> Self {
        self.max_frac = frac;
        self
    }

    /// Render the bubble, drawing `content` inside the surface.
    pub fn show<R>(mut self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> Response {
        let tokens = Tokens::get(ui);
        let mut style = self.variant.style(tokens);
        std::mem::take(&mut self.style_hook).apply(&mut style);
        let end = self.align == Align::End;

        let full = ui.available_width();
        let max_w = if style.full_width {
            full
        } else {
            full * self.max_frac
        };

        // Lay the bubble in a full-width column so it can hug either edge.
        let layout = if end {
            egui::Layout::top_down(egui::Align::Max)
        } else {
            egui::Layout::top_down(egui::Align::Min)
        };
        let resp = ui
            .allocate_ui_with_layout(Vec2::new(full, 0.0), layout, |ui| {
                let mut frame = Frame::new().corner_radius(tokens.radius_lg());
                if let Some(fill) = style.fill {
                    frame = frame.fill(fill);
                }
                if let Some(stroke) = style.stroke {
                    frame = frame.stroke(stroke);
                }
                frame = frame.inner_margin(if style.framed {
                    Margin::symmetric(12, 8)
                } else {
                    Margin::ZERO
                });
                frame
                    .show(ui, |ui| {
                        ui.set_max_width(max_w.max(48.0));
                        ui.style_mut().visuals.override_text_color = Some(style.text);
                        ui.style_mut().interaction.selectable_labels = true;
                        content(ui);
                    })
                    .response
            })
            .inner;

        if !self.reactions.is_empty() {
            self.paint_reactions(ui, tokens, resp.rect);
            // Reserve the overlap so following rows clear the reaction chips.
            ui.add_space(REACTION_OVERLAP);
        }

        resp
    }

    /// Paint the reaction chips overlapping the chosen bubble edge.
    fn paint_reactions(&self, ui: &Ui, tokens: Tokens, bubble: Rect) {
        let painter = ui.painter();
        let font = FontId::proportional(12.5);
        let height = 22.0;
        let pad_x = 7.0;
        let gap = 4.0;

        // Lay out each chip's galley and width.
        let mut chips = Vec::with_capacity(self.reactions.len());
        let mut total = 0.0;
        for r in &self.reactions {
            let g = painter.layout_no_wrap(r.clone(), font.clone(), tokens.foreground);
            let w = g.size().x + pad_x * 2.0;
            chips.push((g, w));
            total += w;
        }
        #[allow(clippy::cast_precision_loss)]
        let span = self.reactions.len().saturating_sub(1) as f32;
        total += gap * span;

        let center_y = match self.reactions_side {
            Side::Top => bubble.top(),
            Side::Bottom => bubble.bottom(),
        };
        let top = center_y - height / 2.0;

        let inset = 8.0;
        let mut x = if self.reactions_align == Align::End {
            bubble.right() - inset - total
        } else {
            bubble.left() + inset
        };

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let chip_radius = (height / 2.0) as u8;
        for (g, w) in chips {
            let chip = Rect::from_min_size(Pos2::new(x, top), Vec2::new(w, height));
            painter.rect(
                chip,
                CornerRadius::same(chip_radius),
                tokens.card,
                Stroke::new(1.0, tokens.border),
                StrokeKind::Inside,
            );
            let gy = top + (height - g.size().y) / 2.0;
            painter.galley(Pos2::new(x + pad_x, gy), g, tokens.foreground);
            x += w + gap;
        }
    }
}

/// Blend `accent` a little way over `base` for the `tinted` variant.
fn tint(accent: Color32, base: Color32) -> Color32 {
    let mix = |a: u8, b: u8| {
        let t = 0.14_f32;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let v = (f32::from(a) - f32::from(b))
            .mul_add(t, f32::from(b))
            .round() as u8;
        v
    };
    Color32::from_rgb(
        mix(accent.r(), base.r()),
        mix(accent.g(), base.g()),
        mix(accent.b(), base.b()),
    )
}

/// Stack consecutive bubbles from the same sender, mirroring shadcn's
/// `<BubbleGroup>`. A thin vertical container with a tight gap; set
/// [`align`](Bubble::align) on each [`Bubble`], not the group.
///
/// ```no_run
/// use glazier::bubble::{Bubble, BubbleGroup, Variant, Align};
/// # egui::__run_test_ui(|ui| {
/// BubbleGroup::new().show(ui, |ui| {
///     Bubble::new(Variant::Secondary).align(Align::End)
///         .show(ui, |ui| { ui.label("It worked yesterday."); });
///     Bubble::new(Variant::Secondary).align(Align::End)
///         .show(ui, |ui| { ui.label("Find the bug and fix it."); });
/// });
/// # });
/// ```
#[must_use = "bubble groups do nothing unless shown"]
#[derive(Default)]
pub struct BubbleGroup {
    gap: Option<f32>,
}

impl BubbleGroup {
    /// Create a group with the default inter-bubble gap (`4.0`).
    pub const fn new() -> Self {
        Self { gap: None }
    }

    /// Override the vertical gap between stacked bubbles.
    pub const fn gap(mut self, gap: f32) -> Self {
        self.gap = Some(gap);
        self
    }

    /// Render the grouped bubbles.
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> R {
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = self.gap.unwrap_or(4.0);
            content(ui)
        })
        .inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ghost_is_unframed_and_full_width() {
        let t = Tokens::light();
        let g = Variant::Ghost.style(t);
        assert!(!g.framed);
        assert!(g.full_width);
        assert!(g.fill.is_none());

        let d = Variant::Default.style(t);
        assert!(d.framed);
        assert!(!d.full_width);
        assert_eq!(d.fill, Some(t.primary));
    }

    #[test]
    fn outline_has_stroke_no_fill_emphasis() {
        let t = Tokens::light();
        let o = Variant::Outline.style(t);
        assert!(o.stroke.is_some());
        assert_eq!(o.fill, Some(t.card));
    }

    #[test]
    fn tint_sits_between_accent_and_base() {
        // The tint is mostly the base with a hint of the accent.
        let t = tint(Color32::from_rgb(0, 0, 0), Color32::from_rgb(255, 255, 255));
        assert!(t.r() > 200 && t.r() < 255);
    }

    #[test]
    fn renders_aligns_and_reactions() {
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            Bubble::new(Variant::Secondary).show(ui, |ui| {
                ui.label("hi");
            });
            Bubble::new(Variant::Default)
                .align(Align::End)
                .reactions(["👍", "🔥", "+2"])
                .reactions_side(Side::Top)
                .show(ui, |ui| {
                    ui.label("reply");
                });
            BubbleGroup::new().show(ui, |ui| {
                Bubble::new(Variant::Muted).show(ui, |ui| {
                    ui.label("a");
                });
            });
        });
    }
}
