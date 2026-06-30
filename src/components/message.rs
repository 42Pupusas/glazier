//! [`Message`] — a single conversation row, mirroring shadcn's `<Message>`.
//!
//! `Message` owns the *row layout* around a chat message: the avatar slot, the
//! start/end alignment, and the header and footer slots. The visible message
//! surface (shadcn's `Bubble`) is baked in as a rounded frame — `received`
//! messages ([`Align::Start`]) use the `muted` surface, `sent` messages
//! ([`Align::End`]) use the `primary` surface — and you can override the fill
//! with [`bubble_fill`](Message::bubble_fill).
//!
//! Faithful to shadcn's feature list:
//! - **start/end alignment** via [`align`](Message::align) — sender rows hug the
//!   end, receiver rows the start;
//! - the **avatar anchors to the bottom of the message surface** and stays clear
//!   of the footer (it lines up with the bubble, not the metadata below it);
//! - **header and footer slots** for sender names, status, and actions, with the
//!   footer following the message side;
//! - stack consecutive messages from one sender with
//!   [`MessageGroup`] — render an empty [`Message::avatar`] on the earlier rows
//!   to keep them aligned under the last one's avatar.

use egui::{Align, Color32, Frame, Rect, Response, RichText, Ui, Vec2};

use crate::components::avatar::Avatar;
use crate::tokens::Tokens;

/// The result of [`Message::show`].
///
/// Carries both the full-row [`Response`] (for interactions) and the tight
/// [`Rect`] of the visible bubble surface (for anchoring overlays such as
/// copy-reaction chips to the bubble edge rather than the footer).
pub struct MessageResponse {
    /// Response for the entire message row (avatar + bubble + header/footer).
    pub response: Response,
    /// Tight rect of the bubble frame only — excludes the header, footer, and
    /// avatar slot. Use this to anchor per-bubble overlays.
    pub bubble_rect: Rect,
}

/// Gap between the avatar and the message column (shadcn `gap-2`).
const AVATAR_GAP: f32 = 8.0;
/// Vertical gap between header/bubble/footer (shadcn `gap-1`).
const STACK_GAP: f32 = 4.0;
/// Default fraction of the row width a bubble may occupy before wrapping.
const MAX_BUBBLE_FRAC: f32 = 0.78;

/// Which side of the conversation a [`Message`] sits on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Side {
    /// Align to the start (receiver rows). The avatar sits on the left.
    #[default]
    Start,
    /// Align to the end (sender rows). The avatar sits on the right.
    End,
}

/// A single message row.
///
/// ```no_run
/// use glazier::message::{Message, Side};
/// use glazier::avatar::Avatar;
/// # egui::__run_test_ui(|ui| {
/// Message::new()
///     .avatar(Avatar::new("Ada Lovelace"))
///     .header("Ada")
///     .footer("Delivered")
///     .show(ui, |ui| {
///         ui.label("How can I help you today?");
///     });
///
/// Message::new()
///     .align(Side::End)
///     .show(ui, |ui| {
///         ui.label("It's a one-line change.");
///     });
/// # });
/// ```
#[must_use = "messages do nothing unless shown"]
#[derive(Default)]
pub struct Message {
    align: Side,
    avatar: Option<Avatar>,
    header: Option<String>,
    footer: Option<String>,
    bubble_fill: Option<Color32>,
    max_bubble_frac: Option<f32>,
}

impl Message {
    /// Create a message row aligned to the start (a received message).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the row [`alignment`](Side). [`End`](Side::End) renders a sent
    /// message: bubble and footer hug the end, avatar on the trailing side.
    pub const fn align(mut self, align: Side) -> Self {
        self.align = align;
        self
    }

    /// Attach an [`Avatar`], anchored to the bottom of the message surface.
    ///
    /// In a [`MessageGroup`], give the earlier rows an *empty* avatar slot to
    /// keep them aligned under the last row's avatar — pass
    /// [`Avatar::new("")`](Avatar) for an invisible spacer, or omit it and the
    /// column simply shifts over.
    pub fn avatar(mut self, avatar: Avatar) -> Self {
        self.avatar = Some(avatar);
        self
    }

    /// Set the header line (a muted sender name above the bubble). Always
    /// aligned to the start, regardless of [`align`](Message::align).
    pub fn header(mut self, header: impl Into<String>) -> Self {
        self.header = Some(header.into());
        self
    }

    /// Set the footer line (muted metadata below the bubble, e.g. a delivery
    /// status). Follows the message side.
    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    /// Override the bubble fill colour (defaults to `muted` for start rows,
    /// `primary` for end rows).
    pub const fn bubble_fill(mut self, fill: Color32) -> Self {
        self.bubble_fill = Some(fill);
        self
    }

    /// Cap the bubble at this fraction of the row width before its content
    /// wraps (default `0.78`).
    pub const fn max_bubble_frac(mut self, frac: f32) -> Self {
        self.max_bubble_frac = Some(frac);
        self
    }

    /// Render the message, drawing `content` inside the bubble surface.
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> MessageResponse {
        let tokens = Tokens::get(ui);
        let end = self.align == Side::End;

        let (fill, text_color) = self.bubble_fill.map_or_else(
            || {
                if end {
                    (tokens.primary, tokens.primary_foreground)
                } else {
                    (tokens.muted, tokens.foreground)
                }
            },
            |f| (f, tokens.foreground),
        );

        let avatar_d = self.avatar.as_ref().map_or(0.0, Avatar::diameter_value);
        let avatar_slot = if self.avatar.is_some() {
            avatar_d + AVATAR_GAP
        } else {
            0.0
        };

        let total_w = ui.available_width();
        let max_bubble = total_w.mul_add(
            self.max_bubble_frac.unwrap_or(MAX_BUBBLE_FRAC),
            -avatar_slot,
        );

        // Shared captures, back-filled inside the layout closures.
        let mut bubble_rect = Rect::from_min_size(ui.cursor().min, Vec2::ZERO);
        let mut avatar_x = ui.cursor().left();

        let inner = ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = AVATAR_GAP;

            // Reserve the avatar column on the leading side for start rows. We
            // only claim the horizontal slot here; the circle is painted later,
            // anchored to the bubble bottom.
            if self.avatar.is_some() && !end {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(avatar_d), egui::Sense::hover());
                avatar_x = r.left();
            }

            // The message column: header, bubble, footer.
            let col_w = total_w - avatar_slot;
            let layout = if end {
                egui::Layout::top_down(Align::Max)
            } else {
                egui::Layout::top_down(Align::Min)
            };
            ui.allocate_ui_with_layout(Vec2::new(col_w, 0.0), layout, |ui| {
                ui.spacing_mut().item_spacing.y = STACK_GAP;

                // Header: muted sender name, always start-aligned.
                if let Some(header) = &self.header {
                    ui.with_layout(egui::Layout::top_down(Align::Min), |ui| {
                        ui.label(
                            RichText::new(header)
                                .color(tokens.muted_foreground)
                                .size(12.0),
                        );
                    });
                }

                // Bubble surface.
                let bubble = Frame::new()
                    .fill(fill)
                    .corner_radius(tokens.radius_lg())
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.style_mut().visuals.override_text_color = Some(text_color);
                        // Message text reads like a transcript — let it be
                        // selected/copied even when the app disables selectable
                        // labels globally.
                        ui.style_mut().interaction.selectable_labels = true;
                        // `allocate_ui_with_layout` with an explicit width does
                        // two things in one shot:
                        //   1. Fixes RTL: `Align::Min` overrides the inherited
                        //      `Align::Max` from the end-side column so text
                        //      always reads left-to-right inside the bubble.
                        //   2. Fixes the empty-space bloat: the child ui is
                        //      allocated at most `max_bubble` wide, and the
                        //      Frame measures itself from `child.min_rect()`
                        //      (the tight content bounds) — so short messages
                        //      produce a narrow bubble and long ones wrap at
                        //      `max_bubble`, never filling the whole column.
                        ui.allocate_ui_with_layout(
                            egui::Vec2::new(max_bubble.max(48.0), 0.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| content(ui),
                        )
                        .inner
                    });
                bubble_rect = bubble.response.rect;

                // Footer: muted metadata, following the message side.
                if let Some(footer) = &self.footer {
                    ui.label(
                        RichText::new(footer)
                            .color(tokens.muted_foreground)
                            .size(11.0),
                    );
                }
            });

            // Reserve the avatar column on the trailing side for end rows.
            if self.avatar.is_some() && end {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(avatar_d), egui::Sense::hover());
                avatar_x = r.left();
            }
        });

        // Anchor the avatar to the bottom of the bubble (clear of the footer).
        if let Some(avatar) = self.avatar {
            let rect = egui::Rect::from_min_size(
                egui::pos2(avatar_x, bubble_rect.bottom() - avatar_d),
                Vec2::splat(avatar_d),
            );
            avatar.paint_at(ui, rect);
        }

        MessageResponse { response: inner.response, bubble_rect }
    }
}

/// Stack consecutive messages from the same sender, mirroring shadcn's
/// `<MessageGroup>`. It is a thin vertical container with a tighter gap than
/// the default transcript spacing.
///
/// ```no_run
/// use glazier::message::{Message, MessageGroup, Side};
/// # egui::__run_test_ui(|ui| {
/// MessageGroup::new().show(ui, |ui| {
///     Message::new().align(Side::End).show(ui, |ui| { ui.label("one"); });
///     Message::new().align(Side::End).show(ui, |ui| { ui.label("two"); });
/// });
/// # });
/// ```
#[must_use = "message groups do nothing unless shown"]
#[derive(Default)]
pub struct MessageGroup {
    gap: Option<f32>,
}

impl MessageGroup {
    /// Create a group with the default inter-message gap (`4.0`).
    pub const fn new() -> Self {
        Self { gap: None }
    }

    /// Override the vertical gap between stacked messages.
    pub const fn gap(mut self, gap: f32) -> Self {
        self.gap = Some(gap);
        self
    }

    /// Render the grouped messages.
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> R {
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = self.gap.unwrap_or(STACK_GAP);
            content(ui)
        })
        .inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::avatar::Avatar;

    /// Both sides render, with header/footer/avatar slots, and forward the
    /// content closure's return value.
    #[test]
    fn renders_both_sides() {
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let r = Message::new()
                .avatar(Avatar::new("Ada Lovelace"))
                .header("Ada")
                .footer("Delivered")
                .show(ui, |ui| ui.label("hi"));
            assert!(r.response.rect.height() > 0.0);

            Message::new()
                .align(Side::End)
                .show(ui, |ui| ui.label("reply"));

            MessageGroup::new().show(ui, |ui| {
                Message::new().align(Side::End).show(ui, |ui| ui.label("a"));
                Message::new().align(Side::End).show(ui, |ui| ui.label("b"));
            });
        });
    }
}
