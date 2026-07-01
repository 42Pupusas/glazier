//! [`Breadcrumb`] — a navigation trail, mirroring shadcn's `<Breadcrumb>`.
//!
//! A horizontal list of crumbs separated by `chevron-right` glyphs. Crumbs come
//! in three kinds, matching shadcn's slots:
//! - [`link`](Breadcrumb::link) — a clickable ancestor in `muted_foreground`
//!   that brightens to `foreground` on hover;
//! - [`ellipsis`](Breadcrumb::ellipsis) — a collapsed `size-7` button (three
//!   horizontal dots) with a `muted` hover fill, for an overflow/options menu;
//! - [`page`](Breadcrumb::page) — the current page in `foreground`, not
//!   interactive (shadcn's `aria-current="page"`).
//!
//! [`show`](Breadcrumb::show) returns the index of the crumb the user clicked
//! this frame (links and ellipses only), or `None`.

use egui::{Response, Sense, Ui, Vec2, Widget};

use crate::tokens::Tokens;

/// Gap between crumbs and separators (shadcn `gap-1.5`).
const GAP: f32 = 6.0;
/// Separator chevron edge (shadcn `size-3.5`).
const SEP: f32 = 14.0;
/// Ellipsis button edge (shadcn `size-7`) and its dot icon box (`size-4`).
const ELLIPSIS_BTN: f32 = 28.0;
const ELLIPSIS_ICON: f32 = 16.0;
/// Shared row height for every crumb. Allocating each crumb at one uniform
/// height keeps them all on the same centerline — egui's horizontal layout does
/// not re-center earlier (shorter) items when a later, taller one grows the row.
const ROW_H: f32 = ELLIPSIS_BTN;
/// Crumb text size (shadcn `text-sm`, tuned to the card body scale).
const TEXT_SIZE: f32 = 13.0;

/// One crumb in the trail.
enum Crumb {
    /// A clickable ancestor link.
    Link(String),
    /// The current page (not interactive).
    Page(String),
    /// A collapsed overflow/options affordance.
    Ellipsis,
}

/// A breadcrumb navigation trail.
///
/// ```no_run
/// use glazier::breadcrumb::Breadcrumb;
/// # egui::__run_test_ui(|ui| {
/// if let Some(i) = Breadcrumb::new()
///     .link("Home")
///     .ellipsis()
///     .page("Payments")
///     .show(ui)
/// {
///     // crumb `i` was clicked — navigate there
///     let _ = i;
/// }
/// # });
/// ```
#[must_use = "breadcrumbs do nothing unless shown"]
#[derive(Default)]
pub struct Breadcrumb {
    crumbs: Vec<Crumb>,
    /// Optional extra seed mixed into every crumb's egui id, so two
    /// breadcrumb instances that share label+index pairs don't clash.
    id_salt: Option<egui::Id>,
}

impl Breadcrumb {
    /// Create an empty breadcrumb.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a stable salt mixed into every crumb's interaction id.
    ///
    /// Use this whenever more than one `Breadcrumb` might share the same
    /// label + index pairs in the same egui context (e.g. a file-tree
    /// breadcrumb and a CWD breadcrumb in the reasoning panel).
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.id_salt = Some(egui::Id::new(salt));
        self
    }

    /// Append a clickable ancestor link.
    pub fn link(mut self, label: impl Into<String>) -> Self {
        self.crumbs.push(Crumb::Link(label.into()));
        self
    }

    /// Append the current page (not interactive).
    pub fn page(mut self, label: impl Into<String>) -> Self {
        self.crumbs.push(Crumb::Page(label.into()));
        self
    }

    /// Append a collapsed ellipsis (overflow/options) crumb.
    pub fn ellipsis(mut self) -> Self {
        self.crumbs.push(Crumb::Ellipsis);
        self
    }

    /// Render the trail, returning the index of the crumb clicked this frame
    /// (links and ellipses only), or `None`.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        self.render(ui).1
    }

    /// Render the trail, returning both the row [`Response`] and the click
    /// index. Backs both [`show`](Self::show) and the [`Widget`] impl.
    fn render(self, ui: &mut Ui) -> (Response, Option<usize>) {
        let tokens = Tokens::get(ui);
        let salt = self.id_salt;
        let mut clicked = None;

        // Pre-measure natural width so we can allocate exactly the right
        // amount of space.  `with_layout(LTR)` inside an RTL parent would
        // claim all remaining space (from the RTL cursor to the left edge)
        // and place the widget next to the left-hand group instead of packing
        // it against the right-hand neighbours.  `allocate_ui_with_layout`
        // with an explicit size avoids that.
        let font = egui::FontId::proportional(TEXT_SIZE);
        let n = self.crumbs.len();
        let crumb_w: f32 = self.crumbs.iter().map(|c| match c {
            Crumb::Link(s) | Crumb::Page(s) =>
                ui.painter().layout_no_wrap(s.clone(), font.clone(), egui::Color32::PLACEHOLDER).size().x,
            Crumb::Ellipsis => ELLIPSIS_BTN,
        }).sum();
        // Each separator is its own allocated widget; with item_spacing.x=GAP
        // egui inserts GAP between every adjacent pair of widgets.
        // Total widgets = n crumbs + (n-1) separators → (2n-2) gaps.
        #[allow(clippy::cast_precision_loss)] // n is a crumb count, always tiny
        let n_f = n.saturating_sub(1) as f32;
        let natural_w = (2.0 * n_f).mul_add(GAP, n_f.mul_add(SEP, crumb_w));

        let resp = ui.allocate_ui_with_layout(
            egui::vec2(natural_w, ROW_H),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(GAP, 0.0);
                let n = self.crumbs.len();
                for (i, crumb) in self.crumbs.into_iter().enumerate() {
                    match crumb {
                        Crumb::Link(label) => {
                            if crumb_text(ui, tokens, &label, i, true, salt).clicked() {
                                clicked = Some(i);
                            }
                        }
                        Crumb::Page(label) => {
                            crumb_text(ui, tokens, &label, i, false, salt);
                        }
                        Crumb::Ellipsis => {
                            if ellipsis_button(ui, tokens, i, salt).clicked() {
                                clicked = Some(i);
                            }
                        }
                    }
                    if i + 1 < n {
                        separator(ui, tokens);
                    }
                }
            })
            .response;
        (resp, clicked)
    }
}

/// Render a text crumb. When `interactive`, it senses clicks, brightens from
/// `muted_foreground` to `foreground` on hover, and shows a pointing-hand
/// cursor; otherwise it paints flat in `foreground` (the current page).
///
/// The interaction id is derived from the label + index so it stays stable
/// across frames regardless of layout position — avoiding the auto-id drift
/// that can drop clicks inside reflowing containers.
fn crumb_text(ui: &mut Ui, tokens: Tokens, label: &str, idx: usize, interactive: bool, salt: Option<egui::Id>) -> Response {
    let font = egui::FontId::proportional(TEXT_SIZE);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, egui::Color32::PLACEHOLDER);
    // Allocate at the shared row height so this crumb shares the trail's
    // centerline; the click target spans the full row height too.
    let (_, rect) = ui.allocate_space(Vec2::new(galley.size().x, ROW_H));

    let (resp, color) = if interactive {
        let id = egui::Id::new(("glazier-breadcrumb", label, idx)).with(salt);
        let resp = ui
            .interact(rect, id, Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let t = ui
            .ctx()
            .animate_bool_with_time(id.with("hover"), resp.hovered(), 0.15);
        (
            resp,
            tokens.muted_foreground.lerp_to_gamma(tokens.foreground, t),
        )
    } else {
        (
            ui.interact(
                rect,
                egui::Id::new(("glazier-breadcrumb-page", label, idx)).with(salt),
                Sense::hover(),
            ),
            tokens.foreground,
        )
    };

    let pos = egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(pos, galley, color);
    resp
}

/// Render a `chevron-right` separator in faint `muted_foreground`.
fn separator(ui: &mut Ui, tokens: Tokens) {
    let (_, rect) = ui.allocate_space(Vec2::new(SEP, ROW_H));
    let c = rect.center();
    let h = SEP * 0.22;
    let w = SEP * 0.16;
    let color = tokens.muted_foreground.gamma_multiply(0.7);
    let stroke = egui::Stroke::new(1.5, color);
    let top = egui::pos2(c.x - w, c.y - h);
    let tip = egui::pos2(c.x + w, c.y);
    let bot = egui::pos2(c.x - w, c.y + h);
    ui.painter().line_segment([top, tip], stroke);
    ui.painter().line_segment([tip, bot], stroke);
}

/// Render the ellipsis crumb: a `size-7` rounded button with three horizontal
/// dots, gaining a `muted` fill on hover.
fn ellipsis_button(ui: &mut Ui, tokens: Tokens, idx: usize, salt: Option<egui::Id>) -> Response {
    let (_, rect) = ui.allocate_space(Vec2::splat(ELLIPSIS_BTN));
    let id = egui::Id::new(("glazier-breadcrumb-ellipsis", idx)).with(salt);
    let resp = ui
        .interact(rect, id, Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    let t = ui
        .ctx()
        .animate_bool_with_time(id.with("hover"), resp.hovered(), 0.15);
    if t > 0.01 {
        ui.painter()
            .rect_filled(rect, tokens.radius_2xl(), tokens.muted.gamma_multiply(t));
    }

    // Three horizontal dots centred in a size-4 box.
    let c = rect.center();
    let color = tokens.muted_foreground.lerp_to_gamma(tokens.foreground, t);
    let step = ELLIPSIS_ICON * 0.32;
    let r = ELLIPSIS_ICON * 0.09;
    for k in [-1.0_f32, 0.0, 1.0] {
        let cx = k.mul_add(step, c.x);
        ui.painter().circle_filled(egui::pos2(cx, c.y), r, color);
    }
    resp
}

impl Widget for Breadcrumb {
    /// Renders the trail, discarding the click index. Use [`Breadcrumb::show`]
    /// to learn which crumb was clicked.
    fn ui(self, ui: &mut Ui) -> Response {
        self.render(ui).0
    }
}
