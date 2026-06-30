//! [`RadioGroup`] — a set of mutually-exclusive options, mirroring shadcn's
//! `<RadioGroup>` / `<RadioGroupItem>`.
//!
//! Each item is a circular `size-4` control: `bg-input` when unselected, a
//! `primary` fill with a small `primary-foreground` dot when selected. The group
//! borrows `&mut usize` (the selected index) and lays its items out in a row.

use egui::{Color32, Response, Sense, Ui, Vec2, Widget};

use crate::tokens::Tokens;

/// size-4 control, size-2 inner dot.
const DOT: f32 = 16.0;
const INNER: f32 = 8.0;
/// Seconds for the selection colour + dot transition.
const TOGGLE_TIME: f32 = 0.15;

/// A horizontal group of radio options.
///
/// ```no_run
/// use glazier::radio_group::RadioGroup;
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// let mut choice = 0;
/// RadioGroup::new(&mut choice, ["Apple", "Banana"]).ui(ui);
/// # });
/// ```
#[must_use = "radio groups do nothing unless you add them to a Ui"]
pub struct RadioGroup<'a> {
    selected: &'a mut usize,
    options: Vec<String>,
    spacing: f32,
}

impl<'a> RadioGroup<'a> {
    /// Create a radio group bound to `selected`, with the given option labels.
    /// An empty label renders a bare dot (no text).
    pub fn new<I, S>(selected: &'a mut usize, options: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            selected,
            options: options.into_iter().map(Into::into).collect(),
            spacing: 12.0,
        }
    }

    /// Set the gap between items (default 12).
    pub const fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
}

impl Widget for RadioGroup<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let RadioGroup {
            selected,
            options,
            spacing,
        } = self;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = spacing;
            for (i, label) in options.iter().enumerate() {
                if radio_item(ui, tokens, label, i == *selected).clicked() {
                    *selected = i;
                }
            }
        })
        .response
    }
}

/// One radio dot plus an optional trailing label.
fn radio_item(ui: &mut Ui, tokens: Tokens, label: &str, checked: bool) -> Response {
    let gap = 8.0;
    let galley = (!label.is_empty()).then(|| {
        ui.painter().layout_no_wrap(
            label.to_owned(),
            egui::TextStyle::Body.resolve(ui.style()),
            tokens.foreground,
        )
    });
    let label_w = galley.as_ref().map_or(0.0, |g| gap + g.size().x);
    let height = galley.as_ref().map_or(DOT, |g| g.size().y.max(DOT));

    let (rect, response) = ui.allocate_at_least(Vec2::new(DOT + label_w, height), Sense::click());

    // Eased selection value so the ring colour + inner dot glide in/out.
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id.with("on"), checked, TOGGLE_TIME);

    if ui.is_rect_visible(rect) {
        let center = egui::pos2(rect.left() + DOT / 2.0, rect.center().y);
        let painter = ui.painter();
        let fill = filled_input(tokens).lerp_to_gamma(tokens.primary, t);
        painter.circle_filled(center, DOT / 2.0, fill);
        if t > 0.01 {
            // Inner dot pops in (scale + fade) with the selection.
            let col = tokens.primary_foreground.gamma_multiply(t);
            painter.circle_filled(center, INNER / 2.0 * t, col);
        }
        if let Some(galley) = galley {
            let text_pos = egui::pos2(
                rect.left() + DOT + gap,
                rect.center().y - galley.size().y / 2.0,
            );
            painter.galley(text_pos, galley, tokens.foreground);
        }
    }

    response
}

/// shadcn's `bg-input/90`: the input color blended slightly toward the surface.
fn filled_input(tokens: Tokens) -> Color32 {
    tokens.input.lerp_to_gamma(tokens.background, 0.1)
}
