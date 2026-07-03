//! [`Dialog`] — a centered modal window with arbitrary content, mirroring
//! shadcn's `Dialog`.
//!
//! Where [`AlertDialog`](crate::alert_dialog::AlertDialog) is a fixed
//! confirmation (title + description + two-button footer), `Dialog` is the
//! general case: a dimmed backdrop, a `rounded-4xl` card with a header (title +
//! muted description) and a close (×) button in the top-right corner, then
//! **whatever content you draw** inside it — a form, a list, controls. Compose
//! your own footer from [`Button`](crate::button::Button)s in the body.
//!
//! Clicking the backdrop, pressing Escape, or the × button dismisses it.
//!
//! Drive it from a `bool` you own: flip it `true` to open, and call
//! [`show`](Dialog::show) **every frame** — it animates in and out and paints
//! nothing while fully closed, clearing the `bool` when dismissed.
//!
//! ```no_run
//! use glazier::dialog::Dialog;
//! use glazier::button::Button;
//! use egui::Widget as _;
//! # let ctx = egui::Context::default();
//! # let mut open = true;
//! # let mut name = String::new();
//! Dialog::new("Edit profile")
//!     .description("Make changes to your profile here. Click save when you're done.")
//!     .show(&ctx, &mut open, |ui| {
//!         ui.text_edit_singleline(&mut name);
//!         if Button::new("Save changes").ui(ui).clicked() {
//!             // persist…
//!         }
//!     });
//! ```

use egui::emath::TSTransform;
use egui::{Align2, Frame, Margin, Modal, Sense, Stroke, Vec2};

use crate::components::icon::Icon;
use crate::customize::{Customize, StyleHook};
use crate::fonts;
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Duration of the open/close transition, in seconds.
const ANIM_SECS: f32 = 0.15;
/// Peak backdrop opacity — a heavier dim than egui's default.
const BACKDROP_ALPHA: u8 = 140;
/// The `Sheet` frame padding: roomier than the dialog's p-6, with an extra-tall
/// top inset so the header clears the edge.
const SHEET_MARGIN: Margin = Margin {
    left: 28,
    right: 28,
    top: 40,
    bottom: 28,
};
/// The close (×) glyph — lucide `x`, matching shadcn's `DialogClose`.
const X: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>"#;

/// Overridable geometry for [`Dialog`] — reach in via [`Dialog::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct DialogMetrics {
    /// shadcn `sm:max-w-lg`: the dialog's content width target.
    pub content_width: f32,
    /// Header title text size (`text-lg`).
    pub title_text: f32,
    /// Header description text size (`text-sm`).
    pub description_text: f32,
    /// Gap between the header column's title and description (`gap-1.5`).
    pub header_gap: f32,
    /// Gap between the header and the body content (`gap-4`).
    pub body_gap: f32,
}

impl Default for DialogMetrics {
    fn default() -> Self {
        Self {
            content_width: 512.0,
            title_text: 18.0,
            description_text: 14.0,
            header_gap: 6.0,
            body_gap: 16.0,
        }
    }
}

/// `ease-out` cubic: fast start, gentle settle — matches shadcn's enter curve.
fn ease_out_cubic(t: f32) -> f32 {
    let u = 1.0 - t;
    (u * u).mul_add(-u, 1.0)
}

/// Which screen edge a [`Sheet`](crate::sheet::Sheet) slides in from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    /// Slide down from the top edge.
    Top,
    /// Slide in from the right edge (shadcn's default).
    Right,
    /// Slide up from the bottom edge.
    Bottom,
    /// Slide in from the left edge.
    Left,
}

/// How the shared [`modal_shell`] enters and anchors: a centered card that
/// zooms in (`Dialog`/`AlertDialog`), or an edge-anchored panel that slides in
/// (`Sheet`).
#[derive(Clone, Copy)]
pub(crate) enum ModalStyle {
    /// Centered, `zoom-in-95` enter — the dialog look.
    Center,
    /// Edge-anchored, slide-in enter — the sheet look. The carried `Side` is
    /// the edge it hugs; the `width` argument is the panel's *cross-axis* size.
    Sheet(Side),
    /// Edge-anchored, slides in — the drawer look: like a sheet but with the
    /// inner-edge corners rounded and a drag-handle grip. The carried `Side` is
    /// the edge it hugs.
    Drawer(Side),
}

/// Per-style geometry resolved for the current animation progress: the area
/// anchor, the enter transform, the card frame, and the clamped cross-axis
/// width handed to `content`.
struct Geometry {
    anchor: Align2,
    transform: TSTransform,
    frame: Frame,
    content_width: f32,
}

/// Build the shared modal shadow (soft, large blur).
const fn modal_shadow() -> egui::epaint::Shadow {
    egui::epaint::Shadow {
        offset: [0, 0],
        blur: 48,
        spread: 0,
        color: egui::Color32::from_black_alpha(60),
    }
}

/// Resolve the [`Geometry`] for a given [`ModalStyle`] at progress `t`.
fn geometry(
    style: ModalStyle,
    t: f32,
    width: f32,
    viewport: egui::Rect,
    tokens: Tokens,
) -> Geometry {
    // Opaque `widget` (floating-surface) fill — `background` is the
    // app-canvas token and may be translucent under a user theme, which
    // would make every modal see-through against the dimmed backdrop; `card`
    // is reserved for static containers, not floating overlays.
    let base = Frame::new()
        .fill(tokens.widget)
        .stroke(Stroke::new(1.0, tokens.border))
        .inner_margin(Margin::same(24)) // p-6
        .shadow(modal_shadow());
    match style {
        ModalStyle::Center => {
            // `zoom-in-95`: scale 0.95 → 1.0, centered on the viewport.
            let scale = 0.05_f32.mul_add(t, 0.95);
            let center = viewport.center().to_vec2();
            Geometry {
                anchor: Align2::CENTER_CENTER,
                transform: TSTransform::new(center * (1.0 - scale), scale),
                frame: base.corner_radius(tokens.radius_4xl()),
                // Clamp so the card (content + 2×24px margin) keeps a 16px gutter.
                content_width: width.min((viewport.width() - 80.0).max(0.0)),
            }
        }
        ModalStyle::Sheet(side) => {
            let mx = f32::from(SHEET_MARGIN.left + SHEET_MARGIN.right);
            let cw = width.min((viewport.width() - mx).max(0.0));
            // The off-screen → rest translation along the entering axis.
            let dist = match side {
                Side::Left | Side::Right => cw + mx,
                Side::Top | Side::Bottom => viewport.height(),
            };
            let slid = (1.0 - t) * dist;
            let offset = match side {
                Side::Right => Vec2::new(slid, 0.0),
                Side::Left => Vec2::new(-slid, 0.0),
                Side::Bottom => Vec2::new(0.0, slid),
                Side::Top => Vec2::new(0.0, -slid),
            };
            let anchor = match side {
                Side::Right => Align2::RIGHT_CENTER,
                Side::Left => Align2::LEFT_CENTER,
                Side::Top => Align2::CENTER_TOP,
                Side::Bottom => Align2::CENTER_BOTTOM,
            };
            Geometry {
                anchor,
                transform: TSTransform::from_translation(offset),
                // Edge-hugging panels sit on the opaque `widget` surface
                // (shadcn's `bg-popover`); the darkest `background` wouldn't
                // contrast against the dimmed page in dark mode.
                frame: base.fill(tokens.widget).inner_margin(SHEET_MARGIN),
                content_width: cw,
            }
        }
        ModalStyle::Drawer(side) => {
            // Edge-anchored like a sheet, but with the inner-edge corners
            // rounded. The cross-axis (top/bottom: width; left/right: height)
            // spans the full viewport minus the 2×24px frame margins so the
            // content keeps its padding instead of overflowing the screen.
            let r = tokens.radius_4xl();
            let horizontal = matches!(side, Side::Left | Side::Right);
            let cw = if horizontal {
                width.min((viewport.width() - 48.0).max(0.0))
            } else {
                (viewport.width() - 48.0).max(0.0)
            };
            // Distance the panel travels from offscreen to rest (incl. margins).
            let dist = match side {
                Side::Left | Side::Right => cw + 48.0,
                Side::Top | Side::Bottom => viewport.height(),
            };
            let slid = (1.0 - t) * dist;
            let offset = match side {
                Side::Right => Vec2::new(slid, 0.0),
                Side::Left => Vec2::new(-slid, 0.0),
                Side::Bottom => Vec2::new(0.0, slid),
                Side::Top => Vec2::new(0.0, -slid),
            };
            let anchor = match side {
                Side::Right => Align2::RIGHT_CENTER,
                Side::Left => Align2::LEFT_CENTER,
                Side::Top => Align2::CENTER_TOP,
                Side::Bottom => Align2::CENTER_BOTTOM,
            };
            // Round only the inner edge, away from the hugged screen border.
            let [nw, ne, sw, se] = match side {
                Side::Bottom => [r, r, 0, 0],
                Side::Top => [0, 0, r, r],
                Side::Left => [0, r, 0, r],
                Side::Right => [r, 0, r, 0],
            };
            let corners = egui::CornerRadius { nw, ne, sw, se };
            Geometry {
                anchor,
                transform: TSTransform::from_translation(offset),
                // Opaque `widget` surface, as for the sheet, so it contrasts
                // against the dimmed page in dark mode.
                frame: base.fill(tokens.widget).corner_radius(corners),
                content_width: cw,
            }
        }
    }
}

/// Drives the shared modal chrome behind both [`Dialog`] and
/// [`AlertDialog`](crate::alert_dialog::AlertDialog): the open/close animation,
/// the dimmed backdrop, the `zoom-in-95` enter transform, the `rounded-4xl`
/// card frame, and viewport clamping.
///
/// Call **every frame**. Returns `None` while fully closed (nothing painted).
/// Otherwise runs `content` (handed the live [`Ui`](egui::Ui), the resolved
/// [`Tokens`], and the clamped content width) inside the card, and returns its
/// value paired with `backdrop_close` — `true` when the user clicked the
/// backdrop or pressed Escape this frame. Callers decide whether to honor it:
/// `Dialog` dismisses on outside-click, `AlertDialog` ignores it (button-only,
/// matching shadcn's `AlertDialog`).
pub(crate) fn modal_shell<R>(
    ctx: &egui::Context,
    id: egui::Id,
    open: bool,
    style: ModalStyle,
    width: f32,
    style_hook: StyleHook<Frame>,
    content: impl FnOnce(&mut egui::Ui, Tokens, f32) -> R,
) -> Option<(R, bool)> {
    // Drive a single 0→1 progress toward `open`. Run every frame so it sits at 0
    // while closed, eases up on open, and eases back *down* on close so the
    // modal animates *out* instead of vanishing.
    let linear = ctx.animate_bool_with_time(id, open, ANIM_SECS);
    if linear <= 0.0 && !open {
        return None; // fully closed: nothing to paint, no input to capture
    }
    let t = ease_out_cubic(linear);

    let tokens = Tokens::get_ctx(ctx);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let backdrop_alpha = (f32::from(BACKDROP_ALPHA) * t).round() as u8;
    let viewport = ctx.content_rect();

    let Geometry {
        anchor,
        transform,
        mut frame,
        content_width,
    } = geometry(style, t, width, viewport, tokens);
    style_hook.apply(&mut frame);

    let mut inner = None;
    let area = Modal::default_area(id).anchor(anchor, Vec2::ZERO);
    let modal = Modal::new(id)
        .area(area)
        .backdrop_color(egui::Color32::from_black_alpha(backdrop_alpha))
        .frame(frame)
        .show(ctx, |ui| {
            match style {
                ModalStyle::Center => ui.set_width(content_width),
                ModalStyle::Sheet(side) => {
                    ui.set_width(content_width);
                    // Span the main axis along the hugged edge, but inset by the
                    // frame margins on that axis so the padded card stays fully
                    // on-screen instead of overflowing past the edges.
                    if matches!(side, Side::Left | Side::Right) {
                        let my = f32::from(SHEET_MARGIN.top + SHEET_MARGIN.bottom);
                        ui.set_height((viewport.height() - my).max(0.0));
                    } else {
                        let mx = f32::from(SHEET_MARGIN.left + SHEET_MARGIN.right);
                        ui.set_width((viewport.width() - mx).max(0.0));
                        ui.set_height(0.0);
                    }
                }
                ModalStyle::Drawer(side) => {
                    ui.set_width(content_width);
                    // Span the main axis along the hugged edge, but inset by the
                    // 2×24px frame margins so the panel (and its rounded inner
                    // corners) stays fully on-screen instead of overflowing.
                    if matches!(side, Side::Left | Side::Right) {
                        ui.set_height((viewport.height() - 48.0).max(0.0));
                    }
                }
            }
            ui.set_opacity(t); // fade in / out
                               // Freeze interaction once we're playing the exit tween.
            if !open {
                ui.disable();
            }
            ui.with_visual_transform(transform, |ui| {
                inner = Some(content(ui, tokens, content_width));
            });
        });

    inner.map(|r| (r, modal.should_close()))
}

/// A centered modal dialog hosting arbitrary content.
#[must_use = "dialogs do nothing unless you show them"]
pub struct Dialog {
    title: String,
    description: Option<String>,
    width: Option<f32>,
    show_close: bool,
    style_hook: StyleHook<Frame>,
    sizing_hook: SizingHook<DialogMetrics>,
}

impl Customize<Frame> for Dialog {
    fn style_hook_mut(&mut self) -> &mut StyleHook<Frame> {
        &mut self.style_hook
    }
}

impl Sizeable<DialogMetrics> for Dialog {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<DialogMetrics> {
        &mut self.sizing_hook
    }
}

impl Dialog {
    /// Create a dialog with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            width: None,
            show_close: true,
            style_hook: StyleHook::new(),
            sizing_hook: SizingHook::new(),
        }
    }

    /// Set the muted description line below the title.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// Override the target content width (shadcn's default is `max-w-lg`, 512px).
    /// The card is still clamped to fit narrow viewports.
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Hide the top-right close (×) button. The backdrop and Escape still
    /// dismiss the dialog.
    pub const fn show_close(mut self, show: bool) -> Self {
        self.show_close = show;
        self
    }

    /// Show the dialog. Call this **every frame** — it animates in and out and
    /// paints nothing while fully closed. `content` draws the body; its return
    /// value is forwarded as `Some(R)` while the dialog is visible (`None` once
    /// fully closed). `open` is cleared to `false` when the dialog is dismissed
    /// (× button, backdrop click, or Escape).
    pub fn show<R>(
        mut self,
        ctx: &egui::Context,
        open: &mut bool,
        content: impl FnOnce(&mut egui::Ui) -> R,
    ) -> Option<R> {
        let id = egui::Id::new("glazier-dialog").with(&self.title);
        let style_hook = std::mem::take(&mut self.style_hook);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let width = self.width.unwrap_or(m.content_width);

        let mut closed = false;
        let out = modal_shell(
            ctx,
            id,
            *open,
            ModalStyle::Center,
            width,
            style_hook,
            |ui, tokens, _width| {
                ui.spacing_mut().item_spacing = Vec2::new(0.0, m.body_gap);
                self.header(ui, tokens, m, &mut closed);
                content(ui)
            },
        );

        let Some((inner, backdrop_close)) = out else {
            return None; // fully closed
        };

        // A `Dialog` dismisses on ×, backdrop click, or Escape.
        if *open && (closed || backdrop_close) {
            *open = false; // begins the exit animation on the next frame
        }
        Some(inner)
    }

    /// Paint the header: a title row with the top-right close (×) button, then
    /// the muted description beneath (shadcn's `gap-1.5` header column).
    fn header(&self, ui: &mut egui::Ui, tokens: Tokens, m: DialogMetrics, closed: &mut bool) {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = m.header_gap;

            // Title (left) + close button pinned to the top-right corner.
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(&self.title)
                        .font(fonts::semibold(ui, m.title_text))
                        .color(tokens.foreground),
                );
                if self.show_close {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if close_button(ui, tokens).clicked() {
                            *closed = true;
                        }
                    });
                }
            });

            if let Some(desc) = &self.description {
                ui.label(
                    egui::RichText::new(desc)
                        .size(m.description_text)
                        .color(tokens.muted_foreground),
                );
            }
        });
    }
}

/// A borderless `size-4` × button: muted by default, brightening to
/// `foreground` on hover, with a subtle rounded `accent` hover fill — shadcn's
/// `DialogClose` (`opacity-70 hover:opacity-100`, `ring-offset` omitted).
pub(crate) fn close_button(ui: &mut egui::Ui, tokens: Tokens) -> egui::Response {
    // A square hit target a touch larger than the glyph (shadcn's `size-4`
    // icon inside a small padded button).
    let size = Vec2::splat(24.0);
    let (rect, response) = ui.allocate_at_least(size, Sense::click());

    if ui.is_rect_visible(rect) {
        let hover_t = ui
            .ctx()
            .animate_bool_with_time(response.id, response.hovered(), 0.15);
        // Rounded hover surface, glides in toward `accent`.
        if hover_t > 0.0 {
            ui.painter().rect_filled(
                rect,
                tokens.radius_md(),
                tokens.accent.gamma_multiply(hover_t),
            );
        }
        let tint = tokens
            .muted_foreground
            .lerp_to_gamma(tokens.foreground, hover_t);
        let glyph = Vec2::splat(16.0);
        let icon_rect = egui::Rect::from_center_size(rect.center(), glyph);
        Icon::new(X)
            .color(tint)
            .image(tokens)
            .paint_at(ui, icon_rect);
    }

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Driving a closed dialog must paint nothing and forward `None`.
    #[test]
    fn closed_dialog_is_inert() {
        let ctx = egui::Context::default();
        let mut open = false;
        let mut ran = false;
        let mut out = Some(0);
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            out = Dialog::new("Edit profile").show(ui.ctx(), &mut open, |_| {
                ran = true;
                42
            });
        });
        assert_eq!(out, None);
        assert!(!ran, "content closure must not run while fully closed");
    }

    /// An open dialog runs its content and forwards the closure's value.
    #[test]
    fn open_dialog_forwards_value() {
        let ctx = egui::Context::default();
        let mut open = true;
        let mut out = None;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            out = Dialog::new("Edit profile")
                .description("Make changes.")
                .show(ui.ctx(), &mut open, |_| 7);
        });
        assert_eq!(out, Some(7));
        assert!(open, "dialog stays open until dismissed");
    }
}
