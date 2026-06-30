//! The [`Decorate`] blanket trait and the single accumulating [`Styled`] wrapper.
//!
//! Calling a decorator on a plain [`Widget`] wraps it in a [`Styled`] that
//! carries a [`StyleSpec`]. Further decorators are *inherent* methods on
//! [`Styled`] — they shadow the trait and mutate the same spec, so the whole
//! chain collapses into a **single** [`egui::Frame`] draw (no border-in-border).
//!
//! Defaults are pulled from the active [`egui::Visuals`]/[`egui::Style`], so a
//! user's app-wide style override flows through every glazier component.

use egui::{CornerRadius, Frame, Response, Ui, Widget};

/// How a decorator wants its corner radius resolved.
#[derive(Clone, Copy)]
enum Corners {
    /// Inherit the radius from `visuals.widgets.noninteractive`.
    Inherit,
    /// Use an explicit radius.
    Fixed(u8),
}

/// Accumulated styling for a [`Styled`] widget. Decorators set these fields;
/// [`StyleSpec::apply`] resolves them against `ui.style()` into one [`Frame`].
#[derive(Clone, Copy, Default)]
struct StyleSpec {
    /// Apply the bordered "card" look (group fill + stroke + default margin).
    card: bool,
    /// Rounded corners, if requested.
    corners: Option<Corners>,
    /// Inner padding in points, if requested.
    padding: Option<f32>,
}

impl StyleSpec {
    /// Resolve this spec against the active style into a single [`Frame`].
    fn resolve(self, ui: &Ui) -> Frame {
        let noninteractive = ui.visuals().widgets.noninteractive;

        let mut frame = if self.card {
            Frame::group(ui.style()).fill(noninteractive.bg_fill)
        } else {
            Frame::new()
        };

        if let Some(corners) = self.corners {
            let radius = match corners {
                Corners::Inherit => noninteractive.corner_radius,
                Corners::Fixed(r) => CornerRadius::same(r),
            };
            frame = frame.corner_radius(radius);
            // A radius is only visible with a stroke or fill; give a plain
            // (non-card) frame the noninteractive stroke so it shows.
            if !self.card {
                frame = frame.stroke(noninteractive.bg_stroke);
            }
        }

        if let Some(padding) = self.padding {
            frame = frame.inner_margin(padding);
        }

        frame
    }
}

/// Chainable, style-driven decorators for any [`Widget`].
///
/// Implemented for *every* `Widget` via a blanket impl — bring the trait into
/// scope and the methods become available on all your components. Each method
/// starts a [`Styled`] accumulator; subsequent calls collapse into it.
pub trait Decorate: Widget + Sized {
    /// Wrap in a bordered "card" frame, matching the user's group styling.
    fn carded(self) -> Styled<Self> {
        Styled::new(self).carded()
    }

    /// Round the corners, inheriting the radius from the active [`egui::Visuals`].
    fn rounded(self) -> Styled<Self> {
        Styled::new(self).rounded()
    }

    /// Round the corners with an explicit radius.
    fn rounded_to(self, radius: u8) -> Styled<Self> {
        Styled::new(self).rounded_to(radius)
    }

    /// Add uniform inner padding (in points).
    fn padded(self, margin: f32) -> Styled<Self> {
        Styled::new(self).padded(margin)
    }
}

// The one line that wires the decorators onto everything.
impl<W: Widget> Decorate for W {}

/// A widget plus an accumulating [`StyleSpec`]. Decorator methods here are
/// *inherent*, so they shadow [`Decorate`] and keep mutating one spec instead
/// of nesting wrappers.
pub struct Styled<W> {
    inner: W,
    spec: StyleSpec,
}

impl<W: Widget> Styled<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            spec: StyleSpec::default(),
        }
    }

    /// Apply the bordered "card" look.
    #[must_use]
    pub const fn carded(mut self) -> Self {
        self.spec.card = true;
        self
    }

    /// Round the corners, inheriting the radius from the active [`egui::Visuals`].
    #[must_use]
    pub const fn rounded(mut self) -> Self {
        self.spec.corners = Some(Corners::Inherit);
        self
    }

    /// Round the corners with an explicit radius.
    #[must_use]
    pub const fn rounded_to(mut self, radius: u8) -> Self {
        self.spec.corners = Some(Corners::Fixed(radius));
        self
    }

    /// Add uniform inner padding (in points).
    #[must_use]
    pub const fn padded(mut self, margin: f32) -> Self {
        self.spec.padding = Some(margin);
        self
    }
}

impl<W: Widget> Widget for Styled<W> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.spec
            .resolve(ui)
            .show(ui, |ui| self.inner.ui(ui))
            .response
    }
}
