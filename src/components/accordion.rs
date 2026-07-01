//! [`Accordion`] — a stack of collapsible sections, mirroring shadcn's
//! `<Accordion>`.
//!
//! Each section is a trigger row (label + rotating chevron) over an animated
//! body, with a hairline divider between sections. The [`Mode`] controls how
//! many can be open at once: [`Single`](Mode::Single) closes the others when a
//! new one opens (like shadcn's `type="single"`), [`Multiple`](Mode::Multiple)
//! lets any subset stay open.
//!
//! Open state is persisted per-section via egui's [`CollapsingState`], keyed off
//! the accordion's id plus each section's index.

use egui::{collapsing_header::CollapsingState, Response, Sense, Ui, Vec2, Widget};

use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry for [`Accordion`] — reach in via [`Accordion::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct AccordionMetrics {
    /// Per-section trigger row height.
    pub trigger_h: f32,
    /// Chevron box edge length.
    pub chevron: f32,
}

impl Default for AccordionMetrics {
    fn default() -> Self {
        Self {
            trigger_h: 44.0,
            chevron: 16.0,
        }
    }
}

/// How many sections an [`Accordion`] may keep open at once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// At most one section open; opening one closes the rest.
    #[default]
    Single,
    /// Any number of sections open independently.
    Multiple,
}

/// One section: a trigger title and a body closure.
struct Section<'a> {
    title: String,
    body: Box<dyn FnOnce(&mut Ui) + 'a>,
}

/// A vertical stack of collapsible sections.
///
/// ```no_run
/// use glazier::accordion::Accordion;
/// # egui::__run_test_ui(|ui| {
/// Accordion::new("faq")
///     .section("Is it accessible?", |ui| {
///         ui.label("Yes. It follows the WAI-ARIA pattern.");
///     })
///     .section("Is it styled?", |ui| {
///         ui.label("Yes, with the active theme's tokens.");
///     })
///     .show(ui);
/// # });
/// ```
#[must_use = "accordions do nothing unless shown"]
pub struct Accordion<'a> {
    id_source: egui::Id,
    mode: Mode,
    sections: Vec<Section<'a>>,
    sizing_hook: SizingHook<AccordionMetrics>,
}

impl Sizeable<AccordionMetrics> for Accordion<'_> {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<AccordionMetrics> {
        &mut self.sizing_hook
    }
}

impl<'a> Accordion<'a> {
    /// Create an accordion identified by `id_source` (used to persist each
    /// section's open state across frames).
    pub fn new(id_source: impl std::hash::Hash) -> Self {
        Self {
            id_source: egui::Id::new(id_source),
            mode: Mode::default(),
            sections: Vec::new(),
            sizing_hook: SizingHook::default(),
        }
    }

    /// Set the open [`Mode`] (default [`Single`](Mode::Single)).
    pub const fn mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// Append a section with the given trigger `title` and `body`.
    pub fn section(mut self, title: impl Into<String>, body: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.sections.push(Section {
            title: title.into(),
            body: Box::new(body),
        });
        self
    }

    /// Render the accordion.
    pub fn show(mut self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let base = ui.make_persistent_id(self.id_source);
        let n = self.sections.len();

        ui.vertical(|ui| {
            for (i, section) in self.sections.into_iter().enumerate() {
                let id = base.with(i);
                let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, false);
                let openness = state.openness(ui.ctx());

                // Trigger row. Interact under the section's persistent `id`
                // (not an auto-id) so clicks survive layout shuffles — see the
                // note in `collapsible.rs`.
                let (_, rect) = ui.allocate_space(Vec2::new(ui.available_width(), m.trigger_h));
                let header = ui.interact(rect, id, Sense::click());
                if header.clicked() {
                    let opening = !state.is_open();
                    state.toggle(ui);
                    // Single mode: opening this one closes every other section.
                    if opening && self.mode == Mode::Single {
                        for j in 0..n {
                            if j != i {
                                let oid = base.with(j);
                                let mut other =
                                    CollapsingState::load_with_default_open(ui.ctx(), oid, false);
                                if other.is_open() {
                                    other.set_open(false);
                                    other.store(ui.ctx());
                                }
                            }
                        }
                        ui.ctx().request_repaint();
                    }
                }

                if ui.is_rect_visible(rect) {
                    let text_col = if header.hovered() {
                        tokens.foreground
                    } else {
                        tokens.foreground.gamma_multiply(0.92)
                    };
                    let painter = ui.painter();
                    let galley = painter.layout_no_wrap(
                        section.title.clone(),
                        crate::fonts::semibold(ui, 14.0),
                        text_col,
                    );
                    let ty = rect.center().y - galley.size().y / 2.0;
                    painter.galley(egui::pos2(rect.left(), ty), galley, text_col);

                    let center = egui::pos2(rect.right() - m.chevron / 2.0, rect.center().y);
                    paint_chevron(painter, center, openness, tokens.muted_foreground, m.chevron);
                }

                // Body.
                state.show_body_unindented(ui, |ui| {
                    ui.add_space(2.0);
                    ui.style_mut().visuals.override_text_color = Some(tokens.muted_foreground);
                    (section.body)(ui);
                    ui.add_space(12.0);
                });
                state.store(ui.ctx());

                // Divider between sections. shadcn's `last:border-b-0` drops
                // the rule under the final section, so we skip it too.
                if i + 1 < n {
                    let y = ui.cursor().top();
                    ui.painter().hline(
                        rect.left()..=rect.right(),
                        y,
                        egui::Stroke::new(1.0, tokens.border),
                    );
                }
            }
        })
        .response
    }
}

/// Paint a chevron of edge length `size`, centred on `center`, rotating from
/// down (`openness` 0) to up (`openness` 1) — shadcn's
/// `[&[data-state=open]>svg]:rotate-180`.
fn paint_chevron(
    painter: &egui::Painter,
    center: egui::Pos2,
    openness: f32,
    color: egui::Color32,
    size: f32,
) {
    let half = size * 0.28;
    let dy = 2.0f32.mul_add(-openness, 1.0) * half * 0.6;
    let tip = egui::pos2(center.x, center.y + dy);
    let left = egui::pos2(center.x - half, center.y - dy);
    let right = egui::pos2(center.x + half, center.y - dy);
    let stroke = egui::Stroke::new(2.0, color);
    painter.line_segment([left, tip], stroke);
    painter.line_segment([tip, right], stroke);
}

impl Widget for Accordion<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui)
    }
}
