# glazier

shadcn/ui-inspired component set for [egui](https://github.com/emilk/egui)/eframe.

Every component is a plain `egui::Widget` (no app-state coupling), styled through
glazier's `Decorate` chain so a user's `Style`/`Visuals` overrides flow through
everything automatically.

```no_run
use glazier::{ChatBubble, Decorate};
use egui::Widget as _;
# egui::__run_test_ui(|ui| {
ChatBubble::new("hello").carded().rounded().padded(8.0).ui(ui); // one Frame
# });
```

See `PLAN.md` for the full component roadmap.

## Known upstream issue: high CPU on Wayland (eframe 0.35+)

On Wayland, any continuously-repainting eframe 0.35/0.36 app (including the
`gallery` example while scrolling or animating) burns ~100% of a core in the
event loop, regardless of how cheap the UI code is. This is an upstream eframe
bug — [egui #8326](https://github.com/emilk/egui/issues/8326): after
`request_redraw()` the winit control flow is left at `ControlFlow::Poll`, and on
Wayland the redraw waits on the compositor's frame callback while the poll loop
spins. It is **not** a glazier issue: an A/B bench of identical plain-egui apps
measured ~12% of a core on eframe 0.34 vs ~100% on 0.36 (and ~15% for the same
0.36 binary forced onto X11).

Workaround until upstream ships a fix — run on XWayland:

```sh
WAYLAND_DISPLAY= cargo run --release --example gallery
```

## License

MIT
