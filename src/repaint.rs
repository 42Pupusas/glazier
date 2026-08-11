//! Repaint scheduling and attribution for applications embedding glazier.
//!
//! Egui records the immediate request site, but optimized native builds often
//! omit enough unwind data that a profiler cannot identify the widget that
//! asked for the next frame. Components schedule their intentional
//! continuous-animation frames through [`Repaint`], which labels each request;
//! an application can inspect and clear the labels at the end of each pass.
//!
//! # Why animation must name a cadence
//!
//! [`Context::request_repaint`] asks for the next frame *immediately*. For a
//! widget animating every frame that degenerates into a busy loop: egui redraws
//! as fast as the compositor accepts frames, pinning a core for what is usually
//! a decorative gradient. One always-on animated widget anywhere in the tree is
//! enough to keep the whole window at maximum refresh forever.
//!
//! So [`Repaint::request_after`] is the default for clock-driven animation, and
//! the cadence constants below say what "smooth enough" means for each kind of
//! motion. [`Repaint::request`] remains available for genuine one-shot
//! invalidation (a state change that must land on the very next frame).
//!
//! Note that passing a delay to egui is not by itself enough:
//! [`Context::request_repaint_after`] is a deadline ("no later than"), so a
//! widget re-arming every pass still free-runs at the display's rate.
//! [`Repaint::request_after`] tracks each cause's last frame and enforces the
//! interval itself — see its docs.

use std::collections::BTreeSet;
use std::time::Duration;

use egui::Context;

const CAUSES_ID: &str = "glazier.repaint-causes";
const ANIMATIONS_ENABLED_ID: &str = "glazier.animations-enabled";
const REQUEST_COUNT_ID: &str = "glazier.repaint-request-count";

/// Labelled repaint scheduling.
///
/// A zero-sized namespace: it owns the repaint policy (labelling, the
/// animation opt-out, and the cadence constants) so the rules live in one
/// place rather than being restated at each animating widget.
pub struct Repaint;

impl Repaint {
    /// Cadence for anything whose *position* changes — rotation, a sweeping
    /// band, a travelling highlight. 60fps: smooth, but bounded.
    ///
    /// Pick this whenever the animation has an edge the eye can follow. Motion
    /// is where halving the frame rate is most visible: a moving object at
    /// 30fps on a 60Hz display lands on every other refresh and reads as
    /// stepping rather than gliding.
    pub const MOTION: Duration = Duration::from_millis(16);

    /// Cadence for effects that change *colour in place* — a pulse, a fade, a
    /// tint drifting between two values. 30fps, half the cost.
    ///
    /// The distinction from [`Repaint::MOTION`] is movement, not the word
    /// "gradient": with nothing travelling across the screen there is no edge
    /// to track, and the eye genuinely cannot separate 30 from 60. A shimmer
    /// that *sweeps* belongs on `MOTION` however gradient-like it looks — that
    /// exact mix-up shipped once and was reported as a laggy animation.
    pub const GRADIENT: Duration = Duration::from_millis(33);

    /// Set whether clock-driven component animation may schedule another frame.
    ///
    /// Native hosts should update this from their window system's authoritative
    /// focus state once per pass. Egui's input snapshot can retain focus/hover
    /// state for a background Wayland surface, so components must not infer
    /// this policy independently from that snapshot.
    pub fn set_animations_enabled(ctx: &Context, enabled: bool) {
        ctx.data_mut(|data| data.insert_temp(egui::Id::new(ANIMATIONS_ENABLED_ID), enabled));
    }

    /// Whether clock-driven component animation may request another frame.
    ///
    /// A host that knows better can override this with
    /// [`Repaint::set_animations_enabled`]. Absent an override the answer is
    /// egui's own window focus.
    ///
    /// # Why focus is the default rather than `true`
    ///
    /// Decorative animation is worth a core only while someone is looking at
    /// it. Defaulting to `true` meant an unattended window kept every shimmer,
    /// spinner and skeleton scheduling frames forever, and because each widget
    /// re-arms on its own phase several of them interleave into a repaint
    /// roughly every frame — so the window sat at the compositor's maximum rate
    /// with nothing on screen changing meaningfully. Opting *in* also meant the
    /// safe behaviour only existed for hosts that remembered to ask for it, and
    /// in practice none did.
    #[must_use]
    pub fn animations_enabled(ctx: &Context) -> bool {
        if let Some(enabled) =
            ctx.data(|data| data.get_temp::<bool>(egui::Id::new(ANIMATIONS_ENABLED_ID)))
        {
            return enabled;
        }
        ctx.input(|i| i.focused)
    }

    /// Request an immediate frame and retain a component label.
    ///
    /// Same scheduling semantics as [`Context::request_repaint`]. Use this for
    /// one-shot invalidation; for per-frame animation prefer
    /// [`Repaint::request_after`], which cannot spin.
    pub fn request(ctx: &Context, cause: &'static str) {
        Self::label(ctx, cause);
        ctx.request_repaint();
    }

    /// Request the next frame of a `cause`'s animation, no sooner than `delay`
    /// after the previous one.
    ///
    /// The cadence to pass is normally [`Repaint::MOTION`] or
    /// [`Repaint::GRADIENT`] — see the module docs for why naming one matters.
    ///
    /// # Why this tracks the last frame itself
    ///
    /// [`Context::request_repaint_after`] is a *deadline*, not a throttle: it
    /// promises a frame no **later** than `delay`, and egui is free to coalesce
    /// it with a frame already scheduled. An animating widget re-arms on every
    /// pass, so each request lands inside the one in flight and the window
    /// free-runs at the display's refresh rate — measurably ~57fps for a widget
    /// politely asking for 30. The delay alone therefore rate-limits nothing.
    ///
    /// So the cadence is enforced here: remember when this cause last painted
    /// and, until the interval has genuinely elapsed, ask only for the frame at
    /// the *end* of it rather than re-arming immediately. Between those frames
    /// the cause stops asking altogether, which is what lets the window idle.
    pub fn request_after(ctx: &Context, cause: &'static str, delay: Duration) {
        Self::label(ctx, cause);

        let now = ctx.input(|i| i.time);
        let id = egui::Id::new((CAUSES_ID, "last-frame", cause));
        let last: Option<f64> = ctx.data(|data| data.get_temp(id));

        // Elapsed time since this cause last got a frame. A missing entry (or a
        // clock that went backwards) counts as "due now", so the first frame of
        // an animation is never delayed.
        let elapsed = last.map_or(f64::INFINITY, |last| now - last);
        let remaining = delay.as_secs_f64() - elapsed;

        // When the interval has elapsed this pass *is* the animation's frame:
        // mark it and start the next interval. Otherwise we woke early and owe
        // the rest of the current one.
        let remaining = if remaining <= 0.0 {
            ctx.data_mut(|data| data.insert_temp(id, now));
            delay.as_secs_f64()
        } else {
            remaining
        };

        // Ask for the remaining time *plus* the frame time egui is about to
        // subtract from it.
        //
        // This one line is the whole fix, and both bugs this had were two
        // faces of it. `request_repaint_after` internally does
        // `delay -= predicted_dt` ("make it less likely we over-shoot"), so:
        //
        //   - Asking for the bare remainder floors to ZERO once the remainder
        //     drops under one frame, and zero means *redraw now*. That bought
        //     one bonus immediate frame per interval — a 30fps cadence
        //     measured as the 60fps spin that started this investigation.
        //   - Staying silent instead (the obvious "fix") is worse: egui keeps
        //     only the minimum delay across requesters and resets it every
        //     pass, so a cause that says nothing has nothing scheduled at all.
        //     The window then parks until unrelated input arrives and the
        //     animation freezes solid — 3 frames per second, not 30.
        //
        // So the cause must speak on *every* pass, and must pre-compensate for
        // the subtraction. Adding `predicted_dt` back makes the delay egui
        // actually stores equal the time genuinely left in the interval, which
        // both keeps a real deadline alive and stops it collapsing to zero.
        let predicted = f64::from(ctx.input(|i| i.predicted_dt));
        ctx.request_repaint_after(Duration::from_secs_f64(remaining + predicted));
    }

    /// How many times components have asked for a frame through [`Repaint`]
    /// since this context was created.
    ///
    /// Monotonic, and counted per *call*, not per frame granted: a throttled
    /// [`Repaint::request_after`] still counts, because an active animation
    /// speaks on every pass whether or not this pass is its frame.
    ///
    /// # Why this exists
    ///
    /// A caller that reads this before and after a stretch of UI learns
    /// whether anything *inside that stretch* asked to animate — attribution
    /// egui itself cannot provide, since its context-wide "has requested
    /// repaint" flag never says *who* asked (one shimmer anywhere would
    /// falsely mark every region as animated). A paint-caching host uses the
    /// bracket as its authoritative "this content animates, never replay it"
    /// signal.
    #[must_use]
    pub fn request_count(ctx: &Context) -> u64 {
        ctx.data(|data| {
            data.get_temp(egui::Id::new(REQUEST_COUNT_ID))
                .unwrap_or_default()
        })
    }

    /// Take the labels of glazier components that asked for a frame since the
    /// previous call.
    #[must_use]
    pub fn take_causes(ctx: &Context) -> BTreeSet<&'static str> {
        ctx.data_mut(|data| std::mem::take(data.get_temp_mut_or_default(egui::Id::new(CAUSES_ID))))
    }

    /// Record a component label without scheduling a frame.
    ///
    /// The label is intentionally `'static` so it is allocation-free in the
    /// common path apart from the set's first insertion.
    fn label(ctx: &Context, cause: &'static str) {
        ctx.data_mut(|data| {
            let causes =
                data.get_temp_mut_or_default::<BTreeSet<&'static str>>(egui::Id::new(CAUSES_ID));
            causes.insert(cause);
            // Both request paths funnel through here, so this is the single
            // place where the animation-attribution counter stays honest.
            *data.get_temp_mut_or_default::<u64>(egui::Id::new(REQUEST_COUNT_ID)) += 1;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_clears_component_labels() {
        let ctx = Context::default();
        Repaint::request(&ctx, "test");
        assert!(Repaint::take_causes(&ctx).contains("test"));
        assert!(Repaint::take_causes(&ctx).is_empty());
    }

    /// A delayed request is labelled just like an immediate one, so the
    /// diagnostic stays complete once animation moves off `request`.
    #[test]
    fn delayed_requests_are_labelled_too() {
        let ctx = Context::default();
        Repaint::request_after(&ctx, "delayed", Repaint::GRADIENT);
        assert!(Repaint::take_causes(&ctx).contains("delayed"));
    }

    /// Every request path bumps the attribution counter — including throttled
    /// re-arms, because "asked" is the signal, not "was granted a frame".
    #[test]
    fn request_count_counts_every_call() {
        let ctx = Context::default();
        assert_eq!(Repaint::request_count(&ctx), 0);
        Repaint::request(&ctx, "one");
        Repaint::request_after(&ctx, "two", Repaint::GRADIENT);
        Repaint::request_after(&ctx, "two", Repaint::GRADIENT);
        assert_eq!(Repaint::request_count(&ctx), 3);
    }

    /// A cadence delivers frames at its stated rate — not zero, not every pass.
    ///
    /// Checks the two named cadences against a host-shaped loop, since the
    /// component-level tests can only speak for one widget each. The bounds are
    /// deliberately two-sided: this module has shipped a bug in *both*
    /// directions (a 30fps request spinning at 60, then the fix for it freezing
    /// the animation at 3), and only a floor catches the second kind.
    #[test]
    fn each_cadence_delivers_about_its_stated_rate() {
        for (label, cadence, expected) in [
            ("motion", Repaint::MOTION, 60.0),
            ("gradient", Repaint::GRADIENT, 30.0),
        ] {
            let ui = crate::test_ui::TestUi::new();
            let frames = ui.run_event_loop(1.0, |ui| {
                Repaint::request_after(ui.ctx(), label, cadence);
            });

            #[allow(clippy::cast_precision_loss)] // frame counts are small
            let rate = f64::from(frames);
            assert!(
                rate > expected * 0.6 && rate < expected * 1.5,
                "{label} cadence delivered {frames} frames in a simulated second, \
                 expected about {expected}"
            );
        }
    }

    #[test]
    fn a_cadence_limits_how_often_a_cause_paints() {
        use crate::test_ui::RunHeadless as _;

        let ctx = Context::default();
        let cadence = Duration::from_millis(100);
        // 60 passes over 1s of virtual time: a 60fps window hosting a widget
        // that wants 10fps.
        let passes = 60;
        let dt = 1.0 / f64::from(passes);
        let id = egui::Id::new((CAUSES_ID, "last-frame", "capped"));

        let mut painted = 0;
        for pass in 0..passes {
            let time = f64::from(pass) * dt;
            ctx.run_headless_with(
                egui::RawInput {
                    time: Some(time),
                    ..Default::default()
                },
                |_| {},
            );
            // A pass takes a frame when the interval has elapsed and the cause
            // restarts its clock; count those restarts.
            let before: Option<f64> = ctx.data(|d| d.get_temp(id));
            Repaint::request_after(&ctx, "capped", cadence);
            let after: Option<f64> = ctx.data(|d| d.get_temp(id));
            if before != after {
                painted += 1;
            }
        }

        // 1s at 100ms is ~10 frames. Allow the boundary frame either way, but
        // nothing close to the 60 passes we drove.
        assert!(
            (9..=11).contains(&painted),
            "expected ~10 frames at a 100ms cadence over 1s, got {painted}"
        );
    }

    /// Absent a host override, animation follows window focus — an unattended
    /// window must not keep decorative animation scheduling frames.
    #[test]
    fn animations_follow_focus_by_default() {
        use crate::test_ui::RunHeadless as _;
        for focused in [true, false] {
            let ctx = Context::default();
            ctx.run_headless_with(
                egui::RawInput {
                    focused,
                    ..Default::default()
                },
                |_| {},
            );
            assert_eq!(Repaint::animations_enabled(&ctx), focused);
        }
    }

    /// A host that knows better can still override focus in either direction,
    /// and the override wins.
    #[test]
    fn host_override_beats_focus() {
        use crate::test_ui::RunHeadless as _;
        let ctx = Context::default();
        ctx.run_headless(|_| {});
        // Unfocused by default, but the host insists on animating.
        Repaint::set_animations_enabled(&ctx, true);
        assert!(Repaint::animations_enabled(&ctx));
        Repaint::set_animations_enabled(&ctx, false);
        assert!(!Repaint::animations_enabled(&ctx));
    }
}
