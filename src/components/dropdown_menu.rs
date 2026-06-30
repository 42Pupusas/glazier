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

use crate::tokens::Tokens;

/// Vertical padding inside the popover (shadcn `p-1`).
const PAD: i8 = 4;
/// Item inner padding: `px-2 py-1.5`.
const ITEM_PAD_X: f32 = 8.0;
const ITEM_PAD_Y: f32 = 6.0;
/// Minimum item height (`min-h-7`).
const ITEM_MIN_H: f32 = 28.0;
/// Item / label text size (`text-sm` / `text-xs`).
const ITEM_TEXT: f32 = 14.0;
const LABEL_TEXT: f32 = 12.0;

/// One row of a [`DropdownMenu`].
enum DropdownMenuEntry {
    /// A non-interactive section heading.
    Label(String),
    /// A clickable command. `destructive` paints it in the destructive palette.
    Item { text: String, destructive: bool },
    /// A hairline divider.
    Separator,
}

/// A menu content surface (shadcn's `DropdownMenuContent`).
#[must_use = "menus do nothing unless you show them"]
#[derive(Default)]
pub struct DropdownMenu {
    entries: Vec<DropdownMenuEntry>,
}

impl DropdownMenu {
    /// Create an empty menu.
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
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
        Frame::new()
            .fill(tokens.background)
            .stroke(Stroke::new(1.0, tokens.border))
            .corner_radius(tokens.radius_2xl())
            .inner_margin(Margin::same(PAD))
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
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        // Size to the widest row's intrinsic width (text + `px-2`), with a
        // `min-w-32` floor — never to the popup's available width.
        let width = self.intrinsic_width(ui).max(128.0);

        let mut clicked = None;
        let mut item_index = 0;
        for entry in &self.entries {
            match entry {
                DropdownMenuEntry::Label(text) => label_row(ui, tokens, width, text),
                DropdownMenuEntry::Separator => separator_row(ui, tokens, width),
                DropdownMenuEntry::Item { text, destructive } => {
                    if item_row(ui, tokens, width, text, *destructive).clicked() {
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
        let mut max = 0.0_f32;
        for entry in &self.entries {
            let (text, size) = match entry {
                DropdownMenuEntry::Label(text) => (text, LABEL_TEXT),
                DropdownMenuEntry::Item { text, .. } => (text, ITEM_TEXT),
                DropdownMenuEntry::Separator => continue,
            };
            // Colour is irrelevant when measuring.
            let galley = ui.painter().layout_no_wrap(
                text.clone(),
                egui::FontId::proportional(size),
                egui::Color32::WHITE,
            );
            max = max.max(2.0_f32.mul_add(ITEM_PAD_X, galley.size().x));
        }
        max
    }
}

/// A muted section label (`px-2 py-1 text-xs text-muted-foreground`).
fn label_row(ui: &mut Ui, tokens: Tokens, width: f32, text: &str) {
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        egui::FontId::proportional(LABEL_TEXT),
        tokens.muted_foreground,
    );
    let height = galley.size().y + 8.0; // py-1
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let pos = egui::pos2(
        rect.left() + ITEM_PAD_X,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(pos, galley, tokens.muted_foreground);
}

/// A clickable command row with accent-on-hover and a destructive variant.
fn item_row(ui: &mut Ui, tokens: Tokens, width: f32, text: &str, destructive: bool) -> Response {
    let base_text = if destructive {
        tokens.destructive
    } else {
        tokens.foreground
    };
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        egui::FontId::proportional(ITEM_TEXT),
        base_text,
    );
    let height = 2.0_f32.mul_add(ITEM_PAD_Y, galley.size().y).max(ITEM_MIN_H);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());

    if ui.is_rect_visible(rect) {
        // `focus:bg-accent` (default) / `focus:bg-destructive/10` (destructive),
        // eased so the highlight glides in/out with `transition-colors`.
        let hover_t =
            ui.ctx()
                .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.15);
        if hover_t > 0.01 {
            let fill = if destructive {
                tokens.destructive.gamma_multiply(0.10 * hover_t)
            } else {
                tokens.accent.gamma_multiply(hover_t)
            };
            ui.painter().rect(
                rect,
                tokens.radius_xl(),
                fill,
                Stroke::NONE,
                StrokeKind::Inside,
            );
        }
        let text_color = if destructive {
            tokens.destructive
        } else {
            tokens
                .foreground
                .lerp_to_gamma(tokens.accent_foreground, hover_t)
        };
        let galley = ui.painter().layout_no_wrap(
            text.to_owned(),
            egui::FontId::proportional(ITEM_TEXT),
            text_color,
        );
        let pos = egui::pos2(
            rect.left() + ITEM_PAD_X,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(pos, galley, text_color);
    }
    response
}

/// A hairline divider (`-mx-1 my-1 h-px bg-border/50`).
fn separator_row(ui: &mut Ui, tokens: Tokens, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 9.0), Sense::hover());
    let y = rect.center().y.round();
    // `-mx-1` bleeds into the popover padding.
    ui.painter().hline(
        (rect.left() - f32::from(PAD))..=(rect.right() + f32::from(PAD)),
        y,
        Stroke::new(1.0, tokens.border.gamma_multiply(0.5)),
    );
}
