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

## License

MIT
