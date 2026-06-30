//! [`Card`] — a bordered surface with optional header/content/footer slots.
//!
//! Mirrors shadcn's `<Card>` family (`CardHeader`/`CardTitle`/`CardDescription`/
//! `CardContent`/`CardFooter`). The fill, hairline ring, and 24px corner radius
//! all resolve from the active [`Tokens`], so the look follows the user's theme
//! while matching shadcn's defaults (`rounded-4xl`, `bg-card`, `ring-1`).

use egui::{Frame, Response, RichText, Stroke, Ui, Widget};

use crate::customize::{Customize, StyleHook};
use crate::tokens::Tokens;

/// A bordered card surface.
///
/// Use the builder slots for the standard shadcn structure, then [`Card::show`]
/// to render arbitrary content beneath them:
///
/// ```no_run
/// use glazier::card::Card;
/// # egui::__run_test_ui(|ui| {
/// Card::new()
///     .title("Create project")
///     .description("Deploy your new project in one click.")
///     .show(ui, |ui| {
///         ui.label("…body…");
///     });
/// # });
/// ```
#[must_use = "cards do nothing unless shown"]
#[derive(Default)]
pub struct Card {
    title: Option<String>,
    description: Option<String>,
    footer: Option<String>,
    inner_margin: Option<egui::Margin>,
    style_hook: StyleHook<Frame>,
}

impl Card {
    /// Create an empty card.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the card title (medium weight, prominent).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the card description (muted subtitle).
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set a muted footer caption, drawn below the content.
    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    /// Override the card's inner padding (default: 20 px on all sides).
    pub fn inner_margin(mut self, margin: impl Into<egui::Margin>) -> Self {
        self.inner_margin = Some(margin.into());
        self
    }

    /// Render the card, with `content` drawn below the header slots.
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> Response {
        let tokens = Tokens::get(ui);
        let margin = self.inner_margin.unwrap_or_else(|| egui::Margin::same(20));
        let mut frame = Frame::new()
            .fill(tokens.card)
            .stroke(Stroke::new(1.0, tokens.border))
            .corner_radius(tokens.radius_4xl())
            .inner_margin(margin);
        self.style_hook.apply(&mut frame);

        frame
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                if let Some(title) = &self.title {
                    ui.label(
                        RichText::new(title)
                            .color(tokens.card_foreground)
                            .strong()
                            .size(15.0),
                    );
                }
                if let Some(description) = &self.description {
                    ui.label(
                        RichText::new(description)
                            .color(tokens.muted_foreground)
                            .size(13.0),
                    );
                }
                if self.title.is_some() || self.description.is_some() {
                    ui.add_space(6.0);
                }
                content(ui);
                if let Some(footer) = &self.footer {
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(footer)
                            .color(tokens.muted_foreground)
                            .size(12.0),
                    );
                }
            })
            .response
    }
}

impl Widget for Card {
    /// Renders just the header slots. Use [`Card::show`] for body content.
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui, |_| {})
    }
}

impl Customize<Frame> for Card {
    fn style_hook_mut(&mut self) -> &mut StyleHook<Frame> {
        &mut self.style_hook
    }
}
