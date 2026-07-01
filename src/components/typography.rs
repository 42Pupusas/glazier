//! [`Typography`] — prose text styles, mirroring shadcn's Typography page.
//!
//! shadcn doesn't ship a `<Typography>` component; instead its docs codify a
//! fixed scale of heading/body/inline styles (`h1`…`h4`, `p`, `lead`, `large`,
//! `small`, `muted`, `blockquote`, `list`). glazier collapses that scale into a
//! single [`Variant`] enum so the same vocabulary is one widget call.
//!
//! Sizes follow shadcn's Tailwind classes (`text-4xl` → 36px, `text-sm` → 14px,
//! …); weights come from glazier's bundled faces via [`fonts`](crate::fonts),
//! and colours resolve from the live [`Tokens`] so prose follows the theme.
//!
//! ```no_run
//! use glazier::typography::{Typography, Variant};
//! use egui::Widget as _;
//! # egui::__run_test_ui(|ui| {
//! Typography::h1("The Joke Tax Chronicles").ui(ui);
//! Typography::new("A lead paragraph.", Variant::Lead).ui(ui);
//! Typography::p("Body copy that wraps to the available width.").ui(ui);
//! # });
//! ```

use egui::{Response, RichText, Stroke, Ui, Vec2, Widget};

use crate::fonts;
use crate::tokens::Tokens;

/// A prose style, mirroring the entries on shadcn's Typography page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    /// `h1` — page title: 36px, extra-bold, tight tracking.
    H1,
    /// `h2` — section heading: 30px, semibold, with a bottom hairline.
    H2,
    /// `h3` — subsection heading: 24px, semibold.
    H3,
    /// `h4` — minor heading: 20px, semibold.
    H4,
    /// `p` — body paragraph: 16px, regular, relaxed leading.
    #[default]
    P,
    /// `lead` — intro paragraph: 20px, muted.
    Lead,
    /// `large` — emphasised line: 18px, semibold.
    Large,
    /// `small` — fine print: 14px, medium weight.
    Small,
    /// `muted` — secondary text: 14px, muted colour.
    Muted,
    /// `blockquote` — italic quote with a left border.
    Blockquote,
    /// `inline code` — a semibold mono chip on a muted fill.
    InlineCode,
    /// `list` — a disc-bulleted list (one item per line). Build with
    /// [`Typography::list`].
    List,
}

impl Variant {
    /// Font size in points for this style (shadcn's Tailwind text-* scale).
    #[must_use]
    pub const fn size(self) -> f32 {
        match self {
            Self::H1 => 36.0,
            Self::H2 => 30.0,
            Self::H3 => 24.0,
            Self::H4 | Self::Lead => 20.0,
            Self::Large => 18.0,
            Self::P | Self::Blockquote | Self::List => 16.0,
            Self::Small | Self::Muted | Self::InlineCode => 14.0,
        }
    }

    /// Whether this style uses muted (secondary) text colour.
    const fn is_muted(self) -> bool {
        matches!(self, Self::Lead | Self::Muted)
    }

    /// Whether this style renders bold (vs semibold / regular).
    const fn is_bold(self) -> bool {
        matches!(self, Self::H1)
    }

    /// Whether this style renders with the semibold face.
    const fn is_semibold(self) -> bool {
        matches!(
            self,
            Self::H2 | Self::H3 | Self::H4 | Self::Large | Self::Small
        )
    }
}

/// A piece of prose rendered in one of shadcn's Typography styles.
#[must_use = "typography does nothing unless you add it to a Ui"]
pub struct Typography {
    text: String,
    /// List items, used only by [`Variant::List`].
    items: Vec<String>,
    variant: Variant,
    /// Optional explicit point size, overriding the variant's default.
    size: Option<f32>,
    /// Optional explicit colour, overriding the variant's default.
    color: Option<egui::Color32>,
}

impl Typography {
    /// Create prose with the given `text` and [`Variant`].
    pub fn new(text: impl Into<String>, variant: Variant) -> Self {
        Self {
            text: text.into(),
            items: Vec::new(),
            variant,
            size: None,
            color: None,
        }
    }

    /// Build a disc-bulleted [`Variant::List`] from `items`.
    pub fn list<I, S>(items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            text: String::new(),
            items: items.into_iter().map(Into::into).collect(),
            variant: Variant::List,
            size: None,
            color: None,
        }
    }

    /// Shorthand for a [`Variant::H1`] title.
    pub fn h1(text: impl Into<String>) -> Self {
        Self::new(text, Variant::H1)
    }

    /// Shorthand for a [`Variant::H2`] section heading.
    pub fn h2(text: impl Into<String>) -> Self {
        Self::new(text, Variant::H2)
    }

    /// Shorthand for a [`Variant::H3`] subsection heading.
    pub fn h3(text: impl Into<String>) -> Self {
        Self::new(text, Variant::H3)
    }

    /// Shorthand for a [`Variant::H4`] minor heading.
    pub fn h4(text: impl Into<String>) -> Self {
        Self::new(text, Variant::H4)
    }

    /// Shorthand for a [`Variant::P`] body paragraph.
    pub fn p(text: impl Into<String>) -> Self {
        Self::new(text, Variant::P)
    }

    /// Set (or change) the [`Variant`].
    pub const fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Override the point size (keeping the variant's weight & semantics).
    ///
    /// shadcn's Typography page is explicitly *“use utility classes to style
    /// your text”* — this is the egui equivalent of a `text-[13px]` override
    /// for a denser layout while still routing through one text vocabulary.
    pub const fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// Override the text colour (e.g. a destructive or accent tint).
    pub const fn color(mut self, color: egui::Color32) -> Self {
        self.color = Some(color);
        self
    }
}

impl Widget for Typography {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let variant = self.variant;
        let size = self.size.unwrap_or_else(|| variant.size());

        let color = self.color.unwrap_or_else(|| {
            if variant.is_muted() {
                tokens.muted_foreground
            } else {
                tokens.foreground
            }
        });

        match variant {
            Variant::Blockquote => blockquote(ui, &self.text, size, color, tokens),
            Variant::InlineCode => inline_code(ui, self.text, tokens),
            Variant::List => bulleted_list(ui, &self.items, size, color),
            _ => {
                let font = if variant.is_bold() {
                    fonts::bold(ui, size)
                } else if variant.is_semibold() {
                    fonts::semibold(ui, size)
                } else {
                    egui::FontId::proportional(size)
                };
                let mut text = RichText::new(self.text).color(color).font(font);
                if variant == Variant::H1 {
                    text = text.strong();
                }
                let resp = ui.add(egui::Label::new(text).wrap());
                // `h2` carries a bottom hairline (shadcn `border-b pb-2`).
                if variant == Variant::H2 {
                    ui.add_space(8.0);
                    let y = ui.cursor().top();
                    ui.painter()
                        .hline(ui.max_rect().x_range(), y, Stroke::new(1.0, tokens.border));
                }
                resp
            }
        }
    }
}

/// Render a `blockquote`: an italic line inset behind a 2px left border.
fn blockquote(
    ui: &mut Ui,
    text: &str,
    size: f32,
    color: egui::Color32,
    tokens: Tokens,
) -> Response {
    const BORDER_W: f32 = 2.0;
    const PAD_L: f32 = 16.0; // shadcn `pl-6` ≈ 24px, trimmed for egui density

    ui.horizontal(|ui| {
        let avail = ui.available_width();
        let job = egui::text::LayoutJob::simple(
            text.to_owned(),
            egui::FontId::proportional(size),
            color,
            (avail - BORDER_W - PAD_L).max(0.0),
        );
        let galley = ui.painter().layout_job(job);
        let height = galley.size().y;

        // Left border rule, full height of the wrapped quote.
        let (border_rect, _) =
            ui.allocate_exact_size(Vec2::new(BORDER_W, height), egui::Sense::hover());
        ui.painter().rect_filled(border_rect, 0.0, tokens.border);
        ui.add_space(PAD_L - BORDER_W);

        ui.add(egui::Label::new(RichText::new(text).color(color).italics().size(size)).wrap())
    })
    .inner
}

/// Render a disc-bulleted list: each item on its own row, hanging indent under
/// a `•` bullet (shadcn `list-disc`).
fn bulleted_list(ui: &mut Ui, items: &[String], size: f32, color: egui::Color32) -> Response {
    const BULLET_COL: f32 = 20.0; // bullet + gap before the text

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 6.0; // shadcn `[&>li]:mt-2`
        for item in items {
            ui.horizontal_top(|ui| {
                ui.add_space(8.0); // `ml-6` inset
                ui.label(RichText::new("\u{2022}").color(color).size(size));
                let text_w = (ui.available_width() - BULLET_COL).max(0.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(text_w, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.add(
                            egui::Label::new(RichText::new(item).color(color).size(size)).wrap(),
                        );
                    },
                );
            });
        }
    })
    .response
}

/// Render `inline code`: a semibold mono run on a muted, hairline-bordered chip.
fn inline_code(ui: &mut Ui, text: String, tokens: Tokens) -> Response {
    use egui::{Sense, StrokeKind};

    let padding = Vec2::new(6.0, 2.0);
    let font = egui::FontId::new(13.0, fonts::semibold(ui, 13.0).family);
    let galley = ui.painter().layout_no_wrap(text, font, tokens.foreground);
    let size = galley.size() + padding * 2.0;
    let (rect, response) = ui.allocate_at_least(size, Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect(
            rect,
            tokens.radius_sm(),
            tokens.muted,
            Stroke::new(1.0, tokens.border),
            StrokeKind::Inside,
        );
        let pos = rect.center() - galley.size() / 2.0;
        painter.galley(pos, galley, tokens.foreground);
    }

    response
}
