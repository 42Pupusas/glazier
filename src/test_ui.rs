//! Headless rendering helpers shared by component tests.
//!
//! Rendering tests need a stable viewport and a moving clock. Keeping that
//! setup here prevents each component test from inventing a slightly different
//! approximation of an egui frame.

use egui::{Context, FullOutput, RawInput, Ui};

/// A headless egui context for layout and paint assertions.
pub struct TestUi {
    /// The context persists widget state across frames.
    ctx: Context,
    /// Simulated wall clock, advanced once per frame.
    time: std::cell::Cell<f64>,
}

impl TestUi {
    /// A fresh, focused test context.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ctx: Context::default(),
            time: std::cell::Cell::new(0.0),
        }
    }

    /// Render one 60 Hz frame and return its paint output.
    pub fn frame(&self, build: impl FnMut(&mut Ui)) -> FullOutput {
        self.time.set(self.time.get() + 1.0 / 60.0);
        let mut output = self.ctx.run_ui(
            RawInput {
                time: Some(self.time.get()),
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(800.0, 600.0),
                )),
                ..Default::default()
            },
            build,
        );

        // egui 0.36 makes ignoring texture uploads an explicit error. These
        // headless tests inspect layout, paint shapes, and repaint timing only;
        // none has a renderer to consume font-atlas uploads.
        output.textures_delta.clear();
        output
    }

    /// The shortest delay requested for the next viewport repaint.
    #[must_use]
    pub fn repaint_delay(output: &FullOutput) -> std::time::Duration {
        output
            .viewport_output
            .values()
            .map(|viewport| viewport.repaint_delay)
            .min()
            .expect("a frame always reports a viewport")
    }

    /// The x coordinate at which `needle` was actually painted.
    ///
    /// # Panics
    ///
    /// Panics when the expected text was not painted.
    #[must_use]
    pub fn text_x(output: &FullOutput, needle: &str) -> f32 {
        output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.job.text == needle => Some(text.pos.x),
                _ => None,
            })
            .unwrap_or_else(|| panic!("never painted the text {needle:?}"))
    }
}

impl Default for TestUi {
    fn default() -> Self {
        Self::new()
    }
}

/// Run one headless frame from an existing [`Context`].
pub trait RunHeadless {
    /// Render a frame and discard the output.
    fn run_headless(&self, build: impl FnMut(&mut Ui));
}

impl RunHeadless for Context {
    fn run_headless(&self, build: impl FnMut(&mut Ui)) {
        let mut output = self.run_ui(RawInput::default(), build);
        // See `TestUi::frame`: this helper intentionally has no renderer.
        output.textures_delta.clear();
    }
}
