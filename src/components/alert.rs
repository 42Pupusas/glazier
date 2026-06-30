//! [`Alert`] — a static, inline callout, mirroring shadcn's `<Alert>`.
//!
//! A bordered, `rounded-lg` panel holding an optional leading [`Icon`], a
//! semibold title, and a muted description. The [`Variant`] tints the chrome:
//! [`Default`](Variant::Default) uses the neutral card palette, while
//! [`Destructive`](Variant::Destructive) paints the icon, title, and border in
//! the `destructive` colour.

use egui::{Frame, Response, RichText, Stroke, Ui, Widget};

use crate::components::icon::Icon;
use crate::tokens::Tokens;

/// Visual style of an [`Alert`], mirroring shadcn's `variant` prop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    /// Neutral card chrome (`bg-card`, `text-foreground`).
    #[default]
    Default,
    /// Error emphasis: `destructive` icon, title, and border.
    Destructive,
}

/// A static inline callout.
///
/// ```no_run
/// use glazier::alert::{Alert, Variant};
/// use glazier::icon::Icon;
/// use egui::Widget as _;
/// # const INFO: &str = "<svg/>";
/// # egui::__run_test_ui(|ui| {
/// Alert::new("Heads up!")
///     .description("You can add components to your app using the CLI.")
///     .icon(Icon::new(INFO))
///     .ui(ui);
///
/// Alert::new("Something went wrong")
///     .description("Your session has expired. Please log in again.")
///     .variant(Variant::Destructive)
///     .ui(ui);
/// # });
/// ```
#[must_use = "alerts do nothing unless you add them to a Ui"]
pub struct Alert {
    title: String,
    description: Option<String>,
    icon: Option<Icon>,
    variant: Variant,
}

impl Alert {
    /// Create an alert with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            icon: None,
            variant: Variant::default(),
        }
    }

    /// Set the muted description line(s) below the title.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a leading [`Icon`], tinted to the variant's accent colour.
    pub const fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set the visual [`Variant`].
    pub const fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }
}

/// Gap between the icon and the text column (shadcn `gap-3`).
const ICON_GAP: f32 = 12.0;

impl Widget for Alert {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);

        // The destructive variant tints the icon/title/border; the default uses
        // the neutral foreground over the card surface.
        let (accent, border) = match self.variant {
            Variant::Default => (tokens.foreground, tokens.border),
            Variant::Destructive => (tokens.destructive, tokens.destructive),
        };

        Frame::new()
            .fill(tokens.card)
            .stroke(Stroke::new(1.0, border))
            .corner_radius(tokens.radius_lg())
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_top(|ui| {
                    if let Some(icon) = self.icon {
                        // Nudge the icon down to align with the title's cap line.
                        ui.add_space(0.0);
                        icon.color(accent).size(16.0).ui(ui);
                        ui.add_space(ICON_GAP - ui.spacing().item_spacing.x);
                    }
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.label(
                            RichText::new(&self.title)
                                .font(crate::fonts::semibold(ui, 14.0))
                                .color(accent),
                        );
                        if let Some(description) = &self.description {
                            ui.label(
                                RichText::new(description)
                                    .color(tokens.muted_foreground)
                                    .size(13.0),
                            );
                        }
                    });
                });
            })
            .response
    }
}
