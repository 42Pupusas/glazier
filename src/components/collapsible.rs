//! [`Collapsible`] — a single expand/collapse region, mirroring shadcn's
//! `<Collapsible>`.
//!
//! A clickable trigger row (a label and a chevron that rotates 180° as it
//! opens) sits above a body that reveals with a smooth height animation. Open
//! state is persisted per-id via egui's [`CollapsingState`], so it survives
//! across frames without the caller holding a `bool`.

use egui::{collapsing_header::CollapsingState, Response, Sense, Ui, Vec2, Widget};

use crate::icon::Icon as GlazierIcon;

use crate::tokens::Tokens;

/// Trigger row height (`h-9`-ish), horizontal padding, and the chevron glyph box.
const TRIGGER_H: f32 = 36.0;
const X_PAD: f32 = 8.0;
const CHEVRON: f32 = 16.0;

/// A single collapsible section.
///
/// ```no_run
/// use glazier::collapsible::Collapsible;
/// # egui::__run_test_ui(|ui| {
/// Collapsible::new("recovery-keys")
///     .title("Show recovery keys")
///     .show(ui, |ui| {
///         ui.label("9f3a-22b1-77de");
///     });
/// # });
/// ```
#[must_use = "collapsibles do nothing unless shown"]
pub struct Collapsible {
    id_source: egui::Id,
    title: String,
    icon: Option<GlazierIcon>,
    default_open: bool,
}

impl Collapsible {
    /// Create a collapsible identified by `id_source` (used to persist its
    /// open/closed state across frames).
    ///
    /// The id is stored as-is (`egui::Id::new(id_source)`) — **not** mixed
    /// with the parent Ui's id-stack — so it is stable and readable from
    /// outside the widget via [`Collapsible::openness`] and
    /// [`Collapsible::is_open`].
    pub fn new(id_source: impl std::hash::Hash) -> Self {
        Self {
            id_source: egui::Id::new(id_source),
            title: String::new(),
            icon: None,
            default_open: false,
        }
    }

    /// Return the animation progress (0.0 = fully closed, 1.0 = fully open)
    /// for the collapsible with the given `id_source`, without rendering it.
    ///
    /// Useful for sizing a parent container to match the collapsible's
    /// animated height before the collapsible itself is placed.
    pub fn openness(ctx: &egui::Context, id_source: impl std::hash::Hash) -> f32 {
        let id = egui::Id::new(id_source);
        CollapsingState::load_with_default_open(ctx, id, false).openness(ctx)
    }

    /// Return whether the collapsible with the given `id_source` is toggled
    /// open (independent of any in-progress animation).
    pub fn is_open(ctx: &egui::Context, id_source: impl std::hash::Hash) -> bool {
        let id = egui::Id::new(id_source);
        CollapsingState::load_with_default_open(ctx, id, false).is_open()
    }

    /// Set the trigger label.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Show a small icon to the left of the trigger label.
    pub const fn icon(mut self, icon: GlazierIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Start expanded (default: collapsed).
    pub const fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Render the trigger and, when open, the `body` beneath it.
    pub fn show<R>(self, ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> Response {
        // Icon geometry for the optional leading icon (used further below).
        const ICON_SIZE: f32 = 14.0;
        const ICON_GAP: f32 = 6.0;

        let tokens = Tokens::get(ui);
        // Use the raw id (not `ui.make_persistent_id`) so the key is stable
        // and readable from outside via `Collapsible::openness` / `is_open`.
        let id = self.id_source;
        let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, self.default_open);
        let openness = state.openness(ui.ctx());

        // --- Trigger row: full-width click target, label + rotating chevron ---
        // Interact under the *persistent* `id` (not an auto-id) so the
        // press→release pairing survives layout shuffles — matching egui's own
        // CollapsingState. An auto-id from `allocate_*` drifts when the widget
        // count before it changes (e.g. in the masonry `columns` layout), which
        // silently dropped clicks.
        let (_, rect) = ui.allocate_space(Vec2::new(ui.available_width(), TRIGGER_H));
        let mut header = ui.interact(rect, id, Sense::click());
        if header.clicked() {
            state.toggle(ui);
            header.mark_changed();
        }

        if ui.is_rect_visible(rect) {
            let hover_t =
                ui.ctx()
                    .animate_bool_with_time(header.id.with("hover"), header.hovered(), 0.15);
            let text_col = tokens
                .foreground
                .gamma_multiply(0.9)
                .lerp_to_gamma(tokens.foreground, hover_t);
            let painter = ui.painter();

            // Optional icon + label (semibold, left-aligned, inset by X_PAD).
            let mut text_x = rect.left() + X_PAD;
            if let Some(icon) = self.icon {
                let icon_rect = egui::Rect::from_min_size(
                    egui::pos2(text_x, rect.center().y - ICON_SIZE / 2.0),
                    egui::Vec2::splat(ICON_SIZE),
                );
                // Paint the icon directly into the allocated rect.
                icon.color(text_col).image(tokens).paint_at(ui, icon_rect);
                text_x += ICON_SIZE + ICON_GAP;
            }
            let galley =
                painter.layout_no_wrap(self.title, crate::fonts::semibold(ui, 14.0), text_col);
            let ty = rect.center().y - galley.size().y / 2.0;
            painter.galley(egui::pos2(text_x, ty), galley, text_col);

            // Chevron at the inline-end, inset by X_PAD, rotated by openness.
            let center = egui::pos2(rect.right() - CHEVRON / 2.0 - X_PAD, rect.center().y);
            paint_chevron(painter, center, openness, text_col);
        }

        // --- Body: animated height-clipped reveal (CollapsingState handles it).
        state.show_body_unindented(ui, |ui| {
            ui.add_space(4.0);
            body(ui);
        });
        state.store(ui.ctx());

        header.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}

/// Paint a chevron centred on `center`, interpolating from down (`openness` 0)
/// to up (`openness` 1) — a 180° flip, matching shadcn's
/// `[&[data-state=open]>svg]:rotate-180`.
fn paint_chevron(painter: &egui::Painter, center: egui::Pos2, openness: f32, color: egui::Color32) {
    // A downward chevron: two arms from a top-left/top-right down to the tip.
    let half = CHEVRON * 0.28;
    // Flip vertically as it opens: lerp the arm/tip y-offsets through zero.
    let dy = 2.0f32.mul_add(-openness, 1.0) * half * 0.6;
    let tip = egui::pos2(center.x, center.y + dy);
    let left = egui::pos2(center.x - half, center.y - dy);
    let right = egui::pos2(center.x + half, center.y - dy);
    let stroke = egui::Stroke::new(2.0, color);
    painter.line_segment([left, tip], stroke);
    painter.line_segment([tip, right], stroke);
}

impl Widget for Collapsible {
    /// Renders the trigger with an empty body. Use [`Collapsible::show`] for
    /// real content.
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui, |_| {})
    }
}
