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
        self.frame_now(build)
    }

    /// Render one frame at the current clock reading, without advancing it.
    fn frame_now(&self, build: impl FnMut(&mut Ui)) -> FullOutput {
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

    /// Drive `build` the way a real event loop would for `seconds` of
    /// simulated time, returning how many frames actually got drawn.
    ///
    /// # Why this exists alongside [`Self::frame`]
    ///
    /// [`Self::frame`] draws unconditionally at a fixed 60Hz, so it measures
    /// what a widget *asks for* while saying nothing about what it *gets*. A
    /// component that has stopped animating altogether looks identical to one
    /// pacing itself perfectly: both simply never request an immediate frame.
    ///
    /// A real host (winit) sleeps until either input arrives or the deadline
    /// egui reported expires. This reproduces that: sleep out the requested
    /// delay, then draw. An unbounded delay means nobody asked for anything,
    /// so the loop stops — an idle window with no input never wakes again.
    /// The resulting frame count is the animation's true frame rate.
    pub fn run_event_loop(&self, seconds: f64, mut build: impl FnMut(&mut Ui)) -> u32 {
        let deadline = self.time.get() + seconds;
        let mut frames = 0_u32;
        loop {
            let output = self.frame_now(&mut build);
            frames += 1;
            let delay = Self::repaint_delay(&output).as_secs_f64();
            // Nothing scheduled the next frame: a real loop would block here
            // forever waiting on input that this test never sends.
            if !delay.is_finite() || delay > seconds {
                break;
            }
            if self.time.get() + delay > deadline {
                break;
            }
            self.time.set(self.time.get() + delay);
        }
        frames
    }

    /// The shortest delay requested for the next viewport repaint.
    ///
    /// # Panics
    ///
    /// Panics when the frame produced no viewport output at all, which no
    /// real egui frame does.
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

    /// Render one frame from an explicit input snapshot, dropping the output
    /// safely.
    ///
    /// Needed by tests over behaviour that reads the snapshot itself — focus
    /// and virtual time being the motivating cases, since animation policy
    /// keys off focus and cadences off `input.time`, and
    /// [`RunHeadless::run_headless`]'s default input has neither.
    fn run_headless_with(&self, input: RawInput, build: impl FnMut(&mut Ui));
}

impl RunHeadless for Context {
    fn run_headless(&self, build: impl FnMut(&mut Ui)) {
        self.run_headless_with(RawInput::default(), build);
    }

    fn run_headless_with(&self, input: RawInput, build: impl FnMut(&mut Ui)) {
        let mut output = self.run_ui(input, build);
        // See `TestUi::frame`: this helper intentionally has no renderer.
        output.textures_delta.clear();
    }
}
