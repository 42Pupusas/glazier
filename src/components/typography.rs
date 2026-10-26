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
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry for [`Typography`] — reach in via
/// [`Typography::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct TypographyMetrics {
    /// Blockquote left border thickness.
    pub blockquote_border_w: f32,
    /// Blockquote left padding, including the border (shadcn `pl-6`, trimmed).
    pub blockquote_pad_l: f32,
    /// List bullet column width (bullet + gap before the text).
    pub list_bullet_col: f32,
    /// List `h2`-style bottom-rule gap. Also reused as the `h2` post-heading
    /// spacer.
    pub h2_rule_gap: f32,
}

impl Default for TypographyMetrics {
    fn default() -> Self {
        Self {
            blockquote_border_w: 2.0,
            blockquote_pad_l: 16.0,
            list_bullet_col: 20.0,
            h2_rule_gap: 8.0,
        }
    }
}

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

    /// The face at `size`: bold for `h1`, semibold for the other headings and
    /// emphasised lines, regular otherwise.
    fn font(self, ui: &Ui, size: f32) -> egui::FontId {
        match self {
            Self::H1 => fonts::bold(ui, size),
            Self::H2 | Self::H3 | Self::H4 | Self::Large | Self::Small => fonts::semibold(ui, size),
            _ => egui::FontId::proportional(size),
        }
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
    sizing_hook: SizingHook<TypographyMetrics>,
}

impl Sizeable<TypographyMetrics> for Typography {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<TypographyMetrics> {
        &mut self.sizing_hook
    }
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
            sizing_hook: SizingHook::default(),
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
            sizing_hook: SizingHook::default(),
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
    fn ui(mut self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let variant = self.variant;
        let size = self.size.unwrap_or_else(|| variant.size());

        let color = self.color.unwrap_or_else(|| {
            if variant.is_muted() {
                tokens.muted_foreground
            } else {
                tokens.foreground
            }
        });

        let prose = Prose {
            size,
            color,
            tokens,
            m,
        };
        match variant {
            Variant::Blockquote => prose.blockquote(ui, &self.text),
            Variant::InlineCode => prose.inline_code(ui, self.text),
            Variant::List => prose.list(ui, &self.items),
            _ => prose.text(ui, variant, self.text),
        }
    }
}

/// The resolved size, colour, and tokens one [`Typography`] renders with.
struct Prose {
    size: f32,
    color: egui::Color32,
    tokens: Tokens,
    m: TypographyMetrics,
}

impl Prose {
    /// A heading or paragraph; `h2` carries a bottom hairline (shadcn
    /// `border-b pb-2`).
    fn text(&self, ui: &mut Ui, variant: Variant, text: String) -> Response {
        let mut rich = RichText::new(text)
            .color(self.color)
            .font(variant.font(ui, self.size));
        if variant == Variant::H1 {
            rich = rich.strong();
        }
        let resp = ui.add(egui::Label::new(rich).wrap());
        if variant == Variant::H2 {
            ui.add_space(self.m.h2_rule_gap);
            let y = ui.cursor().top();
            ui.painter().hline(
                ui.max_rect().x_range(),
                y,
                Stroke::new(1.0, self.tokens.border),
            );
        }
        resp
    }

    /// An italic quote inset behind a left border as tall as the wrapped text.
    fn blockquote(&self, ui: &mut Ui, text: &str) -> Response {
        let border_w = self.m.blockquote_border_w;
        let pad_l = self.m.blockquote_pad_l;
        ui.horizontal(|ui| {
            let job = egui::text::LayoutJob::simple(
                text.to_owned(),
                egui::FontId::proportional(self.size),
                self.color,
                (ui.available_width() - border_w - pad_l).max(0.0),
            );
            let height = ui.painter().layout_job(job).size().y;
            let (border, _) =
                ui.allocate_exact_size(Vec2::new(border_w, height), egui::Sense::hover());
            ui.painter().rect_filled(border, 0.0, self.tokens.border);
            ui.add_space(pad_l - border_w);
            let rich = RichText::new(text)
                .color(self.color)
                .italics()
                .size(self.size);
            ui.add(egui::Label::new(rich).wrap())
        })
        .inner
    }

    /// One row per item, hanging-indented under a `•` bullet (shadcn
    /// `list-disc`).
    fn list(&self, ui: &mut Ui, items: &[String]) -> Response {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0; // shadcn `[&>li]:mt-2`
            for item in items {
                ui.horizontal_top(|ui| {
                    ui.add_space(8.0); // `ml-6` inset
                    ui.label(RichText::new("\u{2022}").color(self.color).size(self.size));
                    let text_w = (ui.available_width() - self.m.list_bullet_col).max(0.0);
                    let rich = RichText::new(item).color(self.color).size(self.size);
                    ui.allocate_ui_with_layout(
                        Vec2::new(text_w, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| ui.add(egui::Label::new(rich).wrap()),
                    );
                });
            }
        })
        .response
    }

    /// A semibold mono run on a muted, hairline-bordered chip.
    fn inline_code(&self, ui: &mut Ui, text: String) -> Response {
        let tokens = self.tokens;
        let padding = Vec2::new(6.0, 2.0);
        let font = egui::FontId::new(13.0, fonts::semibold(ui, 13.0).family);
        let galley = ui.painter().layout_no_wrap(text, font, tokens.foreground);
        let size = galley.size() + padding * 2.0;
        let (rect, response) = ui.allocate_at_least(size, egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            ui.painter().rect(
                rect,
                tokens.radius_sm(),
                tokens.muted,
                Stroke::new(1.0, tokens.border),
                egui::StrokeKind::Inside,
            );
            let pos = rect.center() - galley.size() / 2.0;
            ui.painter().galley(pos, galley, tokens.foreground);
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;

    const ALL: [Variant; 12] = [
        Variant::H1,
        Variant::H2,
        Variant::H3,
        Variant::H4,
        Variant::P,
        Variant::Lead,
        Variant::Large,
        Variant::Small,
        Variant::Muted,
        Variant::Blockquote,
        Variant::InlineCode,
        Variant::List,
    ];

    /// Every variant renders its text; headings outsize body text.
    #[test]
    fn every_variant_renders() {
        let ui = TestUi::new();
        let mut heights = Vec::new();
        let out = ui.frame(|ui| {
            for v in ALL {
                let t = if v == Variant::List {
                    Typography::list([format!("{v:?}")])
                } else {
                    Typography::new(format!("{v:?}"), v)
                };
                heights.push(ui.add(t).rect.height());
            }
            ui.add(Typography::h3("tiny").size(9.0).color(egui::Color32::RED));
        });
        for v in ALL {
            assert!(TestUi::text_rect(&out, &format!("{v:?}")).height() > 0.0);
        }
        assert!(heights[0] > heights[4], "{heights:?}");
        assert!(TestUi::text_rect(&out, "tiny").height() < TestUi::text_rect(&out, "H3").height());
    }

    /// Sizes follow shadcn's text scale.
    #[test]
    fn sizes_follow_the_scale() {
        let sizes: Vec<f32> = ALL.iter().map(|v| v.size()).collect();
        assert_eq!(
            sizes,
            [36.0, 30.0, 24.0, 20.0, 16.0, 20.0, 18.0, 14.0, 14.0, 16.0, 14.0, 16.0]
        );
    }

    /// Shorthands pick their variants.
    #[test]
    fn shorthands_pick_variants() {
        let pairs = [
            (Typography::h1("").variant, Variant::H1),
            (Typography::h2("").variant, Variant::H2),
            (Typography::h4("").variant, Variant::H4),
            (
                Typography::p("").variant(Variant::Muted).variant,
                Variant::Muted,
            ),
        ];
        for (got, want) in pairs {
            assert_eq!(got, want);
        }
    }
}
