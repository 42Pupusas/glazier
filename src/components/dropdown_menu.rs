//! [`DropdownMenu`] — a menu content surface, mirroring shadcn's
//! `DropdownMenuContent`.
//!
//! A menu is a list of [entries](DropdownMenuEntry): section **labels** (muted,
//! xs), clickable **items** (rounded-xl, accent on hover, an optional
//! `destructive` variant), and **separators**. It paints itself inside a
//! popover; the [`ButtonGroup`](crate::button_group::ButtonGroup) hangs one off
//! its chevron segment, but you can also drive it from any popup. shadcn shares
//! this one surface across `DropdownMenu`, `ContextMenu`, and `Menubar`.
//!
//! ```no_run
//! use glazier::dropdown_menu::DropdownMenu;
//! # use glazier::tokens::Tokens;
//! # egui::__run_test_ui(|ui| {
//! let menu = DropdownMenu::new()
//!     .label("Quick Actions")
//!     .item("Mute Conversation")
//!     .item("Mark as Read")
//!     .separator()
//!     .destructive("Delete Conversation");
//! // Render the body inside a popup `Ui`:
//! let picked = menu.show_contents(ui, Tokens::get(ui));
//! # });
//! ```

use egui::{Frame, Margin, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry/timing for [`DropdownMenu`] — reach in via
/// [`DropdownMenu::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct DropdownMenuMetrics {
    /// Padding inside the popover (shadcn `p-1`).
    pub pad: i8,
    /// Item horizontal padding (`px-2`).
    pub item_pad_x: f32,
    /// Item vertical padding (`py-1.5`).
    pub item_pad_y: f32,
    /// Minimum item height (`min-h-7`).
    pub item_min_h: f32,
    /// Item text size (`text-sm`).
    pub item_text: f32,
    /// Section-label text size (`text-xs`).
    pub label_text: f32,
    /// Seconds for the hover accent transition.
    pub hover_time: f32,
    /// Minimum menu width floor (`min-w-32`).
    pub min_width: f32,
}

impl Default for DropdownMenuMetrics {
    fn default() -> Self {
        Self {
            pad: 4,
            item_pad_x: 8.0,
            item_pad_y: 6.0,
            item_min_h: 28.0,
            item_text: 14.0,
            label_text: 12.0,
            hover_time: 0.15,
            min_width: 128.0,
        }
    }
}

/// One row of a [`DropdownMenu`].
enum DropdownMenuEntry {
    /// A non-interactive section heading.
    Label(String),
    /// A clickable command. `destructive` paints it in the destructive palette.
    Item { text: String, destructive: bool },
    /// A hairline divider.
    Separator,
}

impl DropdownMenuEntry {
    /// The entry's text and its size, or `None` for a separator.
    fn text(&self, m: DropdownMenuMetrics) -> Option<(&str, f32)> {
        match self {
            Self::Label(text) => Some((text, m.label_text)),
            Self::Item { text, .. } => Some((text, m.item_text)),
            Self::Separator => None,
        }
    }
}

/// A menu content surface (shadcn's `DropdownMenuContent`).
#[must_use = "menus do nothing unless you show them"]
pub struct DropdownMenu {
    entries: Vec<DropdownMenuEntry>,
    metrics: DropdownMenuMetrics,
    /// Unused at runtime — required only to satisfy [`Sizeable`]'s mandatory
    /// `sizing_hook_mut`. [`show_contents`](Self::show_contents) takes `&self`
    /// (menu entries are borrowed out of a parent `Menubar`/`ButtonGroup`
    /// rather than consumed), so there's no `self`-by-value call site left to
    /// resolve a deferred hook against — `sizing()` is overridden below to
    /// apply the closure immediately instead, while `self` is still owned.
    sizing_hook: SizingHook<DropdownMenuMetrics>,
}

impl Default for DropdownMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl Sizeable<DropdownMenuMetrics> for DropdownMenu {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<DropdownMenuMetrics> {
        &mut self.sizing_hook
    }

    fn sizing(mut self, f: impl FnOnce(&mut DropdownMenuMetrics) + 'static) -> Self {
        f(&mut self.metrics);
        self
    }
}

impl DropdownMenu {
    /// Create an empty menu.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            metrics: DropdownMenuMetrics::default(),
            sizing_hook: SizingHook::new(),
        }
    }

    /// Add a muted section label.
    pub fn label(mut self, text: impl Into<String>) -> Self {
        self.entries.push(DropdownMenuEntry::Label(text.into()));
        self
    }

    /// Add a clickable item.
    pub fn item(mut self, text: impl Into<String>) -> Self {
        self.entries.push(DropdownMenuEntry::Item {
            text: text.into(),
            destructive: false,
        });
        self
    }

    /// Add a clickable item in the destructive palette (e.g. "Delete").
    pub fn destructive(mut self, text: impl Into<String>) -> Self {
        self.entries.push(DropdownMenuEntry::Item {
            text: text.into(),
            destructive: true,
        });
        self
    }

    /// Add a separator between groups.
    pub fn separator(mut self) -> Self {
        self.entries.push(DropdownMenuEntry::Separator);
        self
    }

    /// The popover [`Frame`] for this menu: `rounded-2xl`, popover surface,
    /// `p-1`, a hairline ring and a soft shadow.
    pub fn frame(tokens: Tokens) -> Frame {
        // Opaque `widget` (popover) surface — `background` is the app-canvas
        // token and may be translucent under a user theme, which would make
        // this floating menu see-through; `card` is reserved for static
        // containers, not floating overlays.
        Frame::new()
            .fill(tokens.widget)
            .stroke(Stroke::new(1.0, tokens.border))
            .corner_radius(tokens.radius_2xl())
            .inner_margin(Margin::same(DropdownMenuMetrics::default().pad))
            .shadow(egui::epaint::Shadow {
                offset: [0, 6],
                blur: 24,
                spread: 0,
                color: egui::Color32::from_black_alpha(40),
            })
    }

    /// Paint the menu body inside a popup `Ui`, returning the index of the
    /// clicked item (counting only [items](Self::item)/[destructive](Self::destructive),
    /// in declaration order), if any.
    pub fn show_contents(&self, ui: &mut Ui, tokens: Tokens) -> Option<usize> {
        let m = self.metrics;
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        // Size to the widest row's intrinsic width (text + `px-2`), with a
        // `min-w-32` floor — never to the popup's available width.
        let rows = Rows {
            tokens,
            m,
            width: self.intrinsic_width(ui).max(m.min_width),
        };
        let mut clicked = None;
        let mut item_index = 0;
        for entry in &self.entries {
            match entry {
                DropdownMenuEntry::Label(text) => rows.label(ui, text),
                DropdownMenuEntry::Separator => rows.separator(ui),
                DropdownMenuEntry::Item { text, destructive } => {
                    if rows.item(ui, text, *destructive).clicked() {
                        clicked = Some(item_index);
                    }
                    item_index += 1;
                }
            }
        }
        clicked
    }

    /// Widest row width: each text run plus its horizontal padding (`px-2`).
    fn intrinsic_width(&self, ui: &Ui) -> f32 {
        let m = self.metrics;
        self.entries
            .iter()
            .filter_map(|e| e.text(m))
            .map(|(text, size)| {
                // Colour is irrelevant when measuring.
                let galley = ui.painter().layout_no_wrap(
                    text.to_owned(),
                    egui::FontId::proportional(size),
                    egui::Color32::WHITE,
                );
                m.item_pad_x.mul_add(2.0, galley.size().x)
            })
            .fold(0.0, f32::max)
    }
}

/// Paints a menu's rows at a shared width.
struct Rows {
    tokens: Tokens,
    m: DropdownMenuMetrics,
    width: f32,
}

impl Rows {
    /// A muted section label (`px-2 py-1 text-xs text-muted-foreground`).
    fn label(&self, ui: &mut Ui, text: &str) {
        let color = self.tokens.muted_foreground;
        let galley = ui.painter().layout_no_wrap(
            text.to_owned(),
            egui::FontId::proportional(self.m.label_text),
            color,
        );
        let height = galley.size().y + 8.0; // py-1
        let (rect, _) = ui.allocate_exact_size(Vec2::new(self.width, height), Sense::hover());
        self.text_at(ui, rect, galley, color);
    }

    /// A clickable command, accent on hover, or in the destructive palette.
    fn item(&self, ui: &mut Ui, text: &str, destructive: bool) -> Response {
        let (tokens, m) = (self.tokens, self.m);
        let font = egui::FontId::proportional(m.item_text);
        let measured =
            ui.painter()
                .layout_no_wrap(text.to_owned(), font.clone(), tokens.foreground);
        let height = m
            .item_pad_y
            .mul_add(2.0, measured.size().y)
            .max(m.item_min_h);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(self.width, height), Sense::click());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        // `focus:bg-accent` / `focus:bg-destructive/10`, eased like
        // `transition-colors`.
        let hover_t = ui.ctx().animate_bool_with_time(
            response.id.with("hover"),
            response.hovered(),
            m.hover_time,
        );
        let (fill, color) = if destructive {
            (
                tokens.destructive.gamma_multiply(0.10 * hover_t),
                tokens.destructive,
            )
        } else {
            (
                tokens.accent.gamma_multiply(hover_t),
                tokens
                    .foreground
                    .lerp_to_gamma(tokens.accent_foreground, hover_t),
            )
        };
        if hover_t > 0.01 {
            ui.painter().rect(
                rect,
                tokens.radius_xl(),
                fill,
                Stroke::NONE,
                StrokeKind::Inside,
            );
        }
        let galley = ui.painter().layout_no_wrap(text.to_owned(), font, color);
        self.text_at(ui, rect, galley, color);
        response
    }

    /// A hairline divider (`-mx-1 my-1 h-px bg-border/50`) bleeding into
    /// the popover padding.
    fn separator(&self, ui: &mut Ui) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(self.width, 9.0), Sense::hover());
        let pad = f32::from(self.m.pad);
        ui.painter().hline(
            (rect.left() - pad)..=(rect.right() + pad),
            rect.center().y.round(),
            Stroke::new(1.0, self.tokens.border.gamma_multiply(0.5)),
        );
    }

    /// Paint `galley` at the row's text inset, centred vertically.
    fn text_at(
        &self,
        ui: &Ui,
        rect: egui::Rect,
        galley: std::sync::Arc<egui::Galley>,
        color: egui::Color32,
    ) {
        let pos = egui::pos2(
            rect.left() + self.m.item_pad_x,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(pos, galley, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;

    fn menu() -> DropdownMenu {
        DropdownMenu::new()
            .label("Quick Actions")
            .item("Mute")
            .separator()
            .destructive("Delete Conversation")
            .sizing(|m| m.min_width = 40.0)
    }

    /// Items are numbered in order, skipping labels and separators.
    #[test]
    fn clicking_reports_the_item_index() {
        let ui = TestUi::new();
        let mut picked = None;
        let mut build = |ui: &mut Ui| {
            let tokens = Tokens::get(ui);
            picked = menu().show_contents(ui, tokens).or(picked);
        };
        let out = ui.frame(&mut build);
        ui.click(
            TestUi::text_rect(&out, "Delete Conversation").center(),
            &mut build,
        );
        drop(build);
        assert_eq!(picked, Some(1));
    }

    /// The menu is as wide as its widest row, never narrower than the floor.
    #[test]
    fn width_hugs_the_widest_row() {
        let ui = TestUi::new();
        let mut widths = (0.0, 0.0);
        ui.frame(|ui| {
            let m = menu();
            let floor = DropdownMenu::new().item("A");
            widths = (m.intrinsic_width(ui), floor.intrinsic_width(ui).max(128.0));
        });
        let delete = TestUi::new().frame(|ui| {
            ui.label(egui::RichText::new("Delete Conversation").size(14.0));
        });
        let text_w = TestUi::text_rect(&delete, "Delete Conversation").width();
        assert!(
            (widths.0 - text_w - 16.0).abs() < 1.0,
            "{widths:?} vs {text_w}"
        );
        assert!((widths.1 - 128.0).abs() < f32::EPSILON);
    }
}
