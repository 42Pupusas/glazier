//! [`ScrollArea`] — a fixed-size scrollable viewport, mirroring shadcn's
//! `<ScrollArea>`.
//!
//! Wraps egui's [`egui::ScrollArea`] with shadcn's thin overlay scrollbar: a
//! rounded `border`-coloured handle that floats over the content and expands a
//! touch on hover. Optionally draws shadcn's `rounded-md border` chrome around a
//! fixed-size viewport (the canonical "Tags" demo), via [`bordered`] +
//! [`size`].
//!
//! [`bordered`]: ScrollArea::bordered
//! [`size`]: ScrollArea::size

use egui::{Frame, Response, Stroke, Ui, Vec2, Widget};

use crate::tokens::Tokens;

/// A scrollable viewport with a styled thin scrollbar.
///
/// ```no_run
/// use glazier::scroll_area::ScrollArea;
/// # egui::__run_test_ui(|ui| {
/// ScrollArea::new()
///     .size([192.0, 288.0]) // shadcn w-48 h-72
///     .bordered(true)
///     .show(ui, |ui| {
///         for i in 0..50 {
///             ui.label(format!("v1.2.0-beta.{i}"));
///         }
///     });
/// # });
/// ```
#[must_use = "scroll areas do nothing unless shown"]
#[derive(Default)]
pub struct ScrollArea {
    width: Option<f32>,
    height: Option<f32>,
    horizontal: bool,
    bordered: bool,
    inner_margin: Option<f32>,
    /// How many pixels to bleed the scrollbar past the right edge of the
    /// allocated rect — set this to the enclosing panel's inner padding so
    /// the bar sits flush with the panel border rather than floating inset.
    right_bleed: Option<f32>,
}

impl ScrollArea {
    /// Create a vertical scroll area that fills the available width.
    pub const fn new() -> Self {
        Self {
            width: None,
            height: None,
            horizontal: false,
            bordered: false,
            inner_margin: None,
            right_bleed: None,
        }
    }

    /// Fix the viewport to `[width, height]`; content beyond it scrolls. This is
    /// shadcn's `h-72 w-48` sizing on the demo card.
    pub fn size(mut self, size: impl Into<Vec2>) -> Self {
        let size = size.into();
        self.width = Some(size.x);
        self.height = Some(size.y);
        self
    }

    /// Clip the viewport to this height; taller content scrolls vertically.
    pub const fn max_height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Clip the viewport to this width.
    pub const fn max_width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Allow horizontal scrolling as well as vertical.
    pub const fn horizontal(mut self, horizontal: bool) -> Self {
        self.horizontal = horizontal;
        self
    }

    /// Draw shadcn's `rounded-md border` chrome around the viewport.
    pub const fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Pad the content inside the viewport by this many points (shadcn's `p-4`
    /// on the demo is `16.0`). Defaults to none.
    pub const fn inner_margin(mut self, margin: f32) -> Self {
        self.inner_margin = Some(margin);
        self
    }

    /// Extend the scrollbar `px` pixels past the right edge of the allocated
    /// area so it aligns with the enclosing panel's physical border.
    ///
    /// When rendered inside a panel that insets its content by `PAD` points,
    /// pass `PAD` here to reclaim that gap: the clip rect is widened by `px`
    /// and `bar_outer_margin` is set to `-px` so the bar slides flush to the
    /// panel wall instead of floating inset.
    ///
    /// ```no_run
    /// use glazier::scroll_area::ScrollArea;
    /// use glazier::sidebar::SIDEBAR_PAD;
    /// # egui::__run_test_ui(|ui| {
    /// ScrollArea::new()
    ///     .max_height(ui.available_height())
    ///     .right_bleed(SIDEBAR_PAD)
    ///     .show(ui, |ui| { /* … */ });
    /// # });
    /// ```
    pub const fn right_bleed(mut self, px: f32) -> Self {
        self.right_bleed = Some(px);
        self
    }

    /// Render `content` inside the clipped, scrollable viewport.
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> Response {
        let tokens = Tokens::get(ui);

        // The bordered chrome is a rounded `border` frame; without it we render
        // the bare viewport. Either way the inner scroll logic is identical.
        if self.bordered {
            let margin = self.inner_margin;
            Frame::new()
                .stroke(Stroke::new(1.0, tokens.border))
                .corner_radius(tokens.radius_md())
                .show(ui, |ui| {
                    Self {
                        bordered: false,
                        inner_margin: margin,
                        ..self
                    }
                    .viewport(ui, tokens, content);
                })
                .response
        } else {
            self.viewport(ui, tokens, content)
        }
    }

    /// The scrolling viewport proper: applies the thin-scrollbar style, fixes
    /// the size, and runs the content. Assumes any border frame is already set.
    fn viewport<R>(
        self,
        ui: &mut Ui,
        tokens: Tokens,
        content: impl FnOnce(&mut Ui) -> R,
    ) -> Response {
        style_scrollbar(ui, tokens);

        // If a right_bleed is requested, widen the clip rect and set a
        // negative bar_outer_margin so the scrollbar reaches the panel edge.
        if let Some(bleed) = self.right_bleed {
            let mut clip = ui.clip_rect();
            clip.max.x += bleed;
            ui.set_clip_rect(clip);
            ui.style_mut().spacing.scroll.bar_outer_margin = -bleed;
        }

        // With no explicit width, fill the available row (don't shrink to
        // content); with a fixed width, shrink to it.
        let mut area = egui::ScrollArea::new([self.horizontal, true])
            .auto_shrink([self.width.is_some(), true]);
        if let Some(h) = self.height {
            area = area.max_height(h);
        }
        if let Some(w) = self.width {
            area = area.max_width(w);
        }

        let margin = self.inner_margin;
        let width = self.width;
        let out = area.show(ui, |ui| {
            // Fill the fixed width so rows/separators span the viewport.
            if let Some(w) = width {
                ui.set_min_width(margin.map_or(w, |m| 2.0f32.mul_add(-m, w)));
            } else {
                ui.set_width(ui.available_width());
            }
            match margin {
                Some(m) => {
                    Frame::new().inner_margin(m).show(ui, content);
                }
                None => {
                    content(ui);
                }
            }
        });

        // The scroll output carries no Response; synthesize one over the
        // viewport rect so callers can chain `.on_hover_*` etc.
        ui.interact(out.inner_rect, out.id, egui::Sense::hover())
    }
}

/// Apply shadcn's thin overlay scrollbar styling to `ui`'s style in place.
///
/// The handle sits flush against the edge and stays visible at rest (even on
/// dark surfaces), darkening on hover. Useful for raw [`egui::ScrollArea`]s
/// outside this widget — e.g. a page-level scroll area — so their scrollbars
/// match.
pub fn style_scrollbar(ui: &mut Ui, tokens: Tokens) {
    let scroll = &mut ui.style_mut().spacing.scroll;
    *scroll = egui::style::ScrollStyle::thin();
    scroll.floating = true;
    scroll.bar_width = 8.0;
    scroll.handle_min_length = 24.0;
    // Paint the handle from the widgets' `fg_stroke` colour, NOT `bg_fill`:
    // `Tokens::from_visuals` derives `secondary` from `widgets.inactive.bg_fill`,
    // so tinting that slot would silently recolour every secondary button drawn
    // afterward. `fg_stroke` is unused by the token mapping, so it's safe.
    scroll.foreground_color = true;
    // Sit the bar flush against the edge (no inset gutter) so it doesn't
    // float awkwardly off the card border.
    scroll.bar_outer_margin = 0.0;
    scroll.bar_inner_margin = 2.0;
    // Keep the handle clearly visible at rest (the default thin preset
    // fades it almost to nothing, which disappears on dark surfaces).
    scroll.dormant_handle_opacity = 1.0;
    scroll.active_handle_opacity = 1.0;
    scroll.interact_handle_opacity = 1.0;
    scroll.dormant_background_opacity = 0.0;
    scroll.active_background_opacity = 0.0;
    scroll.interact_background_opacity = 0.0;

    // The handle paints from the widgets' `fg_stroke` colour. shadcn's thumb is
    // `bg-border`; on dark surfaces that's too subtle, so lift it toward
    // `muted_foreground` for a readable resting thumb that darkens on hover.
    let resting = tokens.border.lerp_to_gamma(tokens.muted_foreground, 0.55);
    let widgets = &mut ui.style_mut().visuals.widgets;
    widgets.inactive.fg_stroke.color = resting;
    widgets.hovered.fg_stroke.color = tokens.muted_foreground;
    widgets.active.fg_stroke.color = tokens.muted_foreground;
}

impl Widget for ScrollArea {
    /// Renders an empty viewport. Use [`ScrollArea::show`] for content.
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui, |_| {})
    }
}
