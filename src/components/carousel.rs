//! [`Carousel`] — a slide scroller, mirroring shadcn's `<Carousel>`.
//!
//! Shows one slide at a time inside a fixed-height viewport, with circular
//! *Previous*/*Next* arrow buttons flanking it (shadcn's `CarouselPrevious` /
//! `CarouselNext`, `size-8` outline circles) and a row of dot indicators
//! beneath. The active slide animates a horizontal slide-in when the index
//! changes, giving the snap-scroll feel without a real scroll area.
//!
//! [`show`](Carousel::show) takes the slide count, the current index by
//! `&mut usize`, and a closure that paints slide `i` into the viewport `Ui`.
//! It returns `true` when the index changed this frame.
//!
//! ```no_run
//! use glazier::carousel::Carousel;
//! use egui::RichText;
//! # egui::__run_test_ui(|ui| {
//! # let mut idx = 0usize;
//! Carousel::new(5).height(140.0).show(ui, &mut idx, |ui, i| {
//!     ui.centered_and_justified(|ui| {
//!         ui.label(RichText::new(format!("Slide {}", i + 1)).size(28.0));
//!     });
//! });
//! # });
//! ```

use egui::{Response, Sense, Ui, Vec2, Widget};

use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry/timing for [`Carousel`] — reach in via
/// [`Carousel::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct CarouselMetrics {
    /// Default viewport height.
    pub default_h: f32,
    /// Arrow button diameter (shadcn `size-8`).
    pub arrow: f32,
    /// Gap between arrows and the viewport.
    pub arrow_gap: f32,
    /// Dot indicator diameter.
    pub dot: f32,
    /// Gap between dot indicators.
    pub dot_gap: f32,
    /// Slide-transition duration, in seconds.
    pub slide_time: f32,
}

impl Default for CarouselMetrics {
    fn default() -> Self {
        Self {
            default_h: 160.0,
            arrow: 32.0,
            arrow_gap: 8.0,
            dot: 7.0,
            dot_gap: 8.0,
            slide_time: 0.25,
        }
    }
}

/// A slide carousel over a fixed number of items.
#[must_use = "carousels do nothing unless shown"]
pub struct Carousel {
    count: usize,
    height: f32,
    loop_ends: bool,
    sizing_hook: SizingHook<CarouselMetrics>,
}

impl Sizeable<CarouselMetrics> for Carousel {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<CarouselMetrics> {
        &mut self.sizing_hook
    }
}

impl Carousel {
    /// Create a carousel with `count` slides.
    pub fn new(count: usize) -> Self {
        Self {
            count,
            height: CarouselMetrics::default().default_h,
            loop_ends: false,
            sizing_hook: SizingHook::default(),
        }
    }

    /// Set the viewport height in points.
    pub const fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Wrap around at the ends instead of stopping (Next on the last slide goes
    /// to the first, and vice versa).
    pub const fn loop_ends(mut self, loop_ends: bool) -> Self {
        self.loop_ends = loop_ends;
        self
    }

    /// Render the carousel. `index` is the current 0-based slide; it is clamped,
    /// updated on navigation, and `true` is returned if it changed this frame.
    /// `slide` paints slide `i` into the clipped viewport `Ui`.
    pub fn show(mut self, ui: &mut Ui, index: &mut usize, slide: impl Fn(&mut Ui, usize)) -> bool {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let count = self.count.max(1);
        let current = (*index).min(count - 1);
        let mut next = current;

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(m.arrow_gap, 0.0);

                // Previous arrow.
                let can_prev = self.loop_ends || current > 0;
                if arrow(ui, tokens, true, can_prev, self.height, m).clicked() {
                    next = if current == 0 { count - 1 } else { current - 1 };
                }

                // Viewport: remaining width minus the trailing arrow + gap.
                let vp_w = (ui.available_width() - m.arrow - m.arrow_gap).max(0.0);
                let (id, rect) = ui.allocate_space(Vec2::new(vp_w, self.height));

                // Animate a horizontal offset whenever the index changes, so the
                // active slide slides in from the side it advanced toward.
                #[allow(clippy::cast_precision_loss)]
                let current_f = current as f32;
                let anim =
                    ui.ctx()
                        .animate_value_with_time(id.with("slide"), current_f, m.slide_time);
                let dx = (anim - current_f) * rect.width();

                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect.translate(Vec2::new(-dx, 0.0)))
                        .layout(*ui.layout()),
                );
                child.set_clip_rect(rect);
                slide(&mut child, current);

                // Next arrow.
                let can_next = self.loop_ends || current + 1 < count;
                if arrow(ui, tokens, false, can_next, self.height, m).clicked() {
                    next = if current + 1 >= count { 0 } else { current + 1 };
                }
            });

            // Dot indicators, centred under the viewport.
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                #[allow(clippy::cast_precision_loss)]
                let row_w =
                    (count as f32).mul_add(m.dot, count.saturating_sub(1) as f32 * m.dot_gap);
                let pad = (ui.available_width() - row_w) / 2.0;
                if pad > 0.0 {
                    ui.add_space(pad);
                }
                ui.spacing_mut().item_spacing = Vec2::new(m.dot_gap, 0.0);
                for d in 0..count {
                    if dot(ui, tokens, d == current, m).clicked() {
                        next = d;
                    }
                }
            });
        });

        *index = next;
        next != current
    }
}

/// A circular outline arrow button. Dims and stops sensing when `enabled` is
/// false. Occupies the full carousel `height` so both arrows centre vertically
/// against the viewport — allocating only `m.arrow` tall would top-align the
/// left arrow, which is laid out before the taller viewport sets the row
/// height.
fn arrow(
    ui: &mut Ui,
    tokens: Tokens,
    left: bool,
    enabled: bool,
    height: f32,
    m: CarouselMetrics,
) -> Response {
    let (id, row) = ui.allocate_space(Vec2::new(m.arrow, height.max(m.arrow)));
    let rect = egui::Rect::from_center_size(row.center(), Vec2::splat(m.arrow));
    let resp = if enabled {
        ui.interact(rect, id.with(left), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        ui.interact(rect, id.with(left), Sense::hover())
    };

    let hover_t = if enabled {
        ui.ctx()
            .animate_bool_with_time(id.with(("hover", left)), resp.hovered(), 0.15)
    } else {
        0.0
    };
    // Opaque `card` base, not `background` — the app-canvas token can be
    // translucent, which would make this arrow button see-through.
    let fill = tokens.card.lerp_to_gamma(tokens.accent, hover_t);
    let stroke_c = if enabled {
        tokens.border
    } else {
        tokens.border.gamma_multiply(0.5)
    };
    ui.painter().circle(
        rect.center(),
        m.arrow / 2.0,
        fill,
        egui::Stroke::new(1.0, stroke_c),
    );

    let color = if enabled {
        tokens.foreground
    } else {
        tokens.muted_foreground.gamma_multiply(0.5)
    };
    let stroke = egui::Stroke::new(1.6, color);
    let c = rect.center();
    let w = m.arrow * 0.26;
    let h = w * 0.85;
    let dx = if left { w * 0.4 } else { -w * 0.4 };
    let tip = egui::pos2(c.x - dx, c.y);
    let top = egui::pos2(c.x + dx, c.y - h);
    let bot = egui::pos2(c.x + dx, c.y + h);
    ui.painter().line_segment([top, tip], stroke);
    ui.painter().line_segment([tip, bot], stroke);
    resp
}

/// A clickable dot indicator; filled `primary` when active, muted otherwise.
fn dot(ui: &mut Ui, tokens: Tokens, active: bool, m: CarouselMetrics) -> Response {
    let (id, rect) = ui.allocate_space(Vec2::splat(m.dot));
    let resp = ui
        .interact(rect, id, Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let color = if active {
        tokens.primary
    } else if resp.hovered() {
        tokens.muted_foreground
    } else {
        tokens.border
    };
    ui.painter()
        .circle_filled(rect.center(), m.dot / 2.0, color);
    resp
}

impl Widget for Carousel {
    /// Renders an empty carousel shell at slide 0 (no slide painter). Prefer
    /// [`Carousel::show`] for real content and state.
    fn ui(self, ui: &mut Ui) -> Response {
        let mut idx = 0;
        ui.scope(|ui| {
            self.show(ui, &mut idx, |_, _| {});
        })
        .response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A carousel renders and reports no change without input.
    #[test]
    fn stable_without_input() {
        let ctx = egui::Context::default();
        let mut changed = true;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let mut idx = 0;
            changed = Carousel::new(4).show(ui, &mut idx, |ui, i| {
                ui.label(format!("{i}"));
            });
        });
        assert!(!changed);
    }

    /// An out-of-range index is clamped rather than panicking.
    #[test]
    fn clamps_overflow() {
        let ctx = egui::Context::default();
        let mut idx = 99;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            Carousel::new(3).show(ui, &mut idx, |_, _| {});
        });
        assert!(idx < 3);
    }
}
