//! [`Menubar`] — a desktop-style horizontal menu bar, mirroring shadcn's
//! `Menubar`.
//!
//! A menubar is a row of text triggers (File, Edit, View…), each hanging a
//! [`DropdownMenu`] beneath it. It behaves like a native app menu: click a
//! trigger to open its menu, and — while *any* menu is open — hovering a
//! sibling trigger switches to it without a second click. Clicking an item (or
//! anywhere outside) closes the bar. The open trigger is keyed in egui memory,
//! so at most one menu shows at a time.
//!
//! It reuses the same [`DropdownMenu`] content surface as `DropdownMenu` and
//! `ContextMenu`.
//!
//! ```no_run
//! use glazier::menubar::{Menubar, MenubarMenu};
//! use glazier::dropdown_menu::DropdownMenu;
//! # egui::__run_test_ui(|ui| {
//! let picked = Menubar::new("app-menubar")
//!     .menu(MenubarMenu::new("File", DropdownMenu::new()
//!         .item("New Tab")
//!         .item("New Window")
//!         .separator()
//!         .item("Print")))
//!     .menu(MenubarMenu::new("Edit", DropdownMenu::new()
//!         .item("Undo")
//!         .item("Redo")))
//!     .show(ui);
//! if let Some((menu, item)) = picked {
//!     // menu = trigger index, item = clicked item index within that menu
//! }
//! # });
//! ```

use egui::{Id, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::components::dropdown_menu::DropdownMenu;
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry/timing for [`Menubar`] — reach in via
/// [`Menubar::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct MenubarMetrics {
    /// Trigger horizontal padding (`px-3`).
    pub trigger_pad_x: f32,
    /// Trigger vertical padding (`py-1`).
    pub trigger_pad_y: f32,
    /// Minimum trigger height (`h-8` density-trimmed).
    pub trigger_min_h: f32,
    /// Trigger text size (`text-sm`).
    pub trigger_text: f32,
    /// Gap between adjacent triggers.
    pub trigger_gap: f32,
    /// Gap between the bar and an open menu's popup.
    pub popup_gap: f32,
    /// Seconds for the hover/open colour transition.
    pub hover_time: f32,
}

impl Default for MenubarMetrics {
    fn default() -> Self {
        Self {
            trigger_pad_x: 12.0,
            trigger_pad_y: 4.0,
            trigger_min_h: 28.0,
            trigger_text: 14.0,
            trigger_gap: 2.0,
            popup_gap: 4.0,
            hover_time: 0.15,
        }
    }
}

/// One top-level entry of a [`Menubar`]: a labelled trigger plus its menu.
#[must_use = "menubar menus do nothing unless added to a Menubar"]
pub struct MenubarMenu {
    label: String,
    menu: DropdownMenu,
}

impl MenubarMenu {
    /// Create a menu entry with the given trigger `label` and `menu` contents.
    pub fn new(label: impl Into<String>, menu: DropdownMenu) -> Self {
        Self {
            label: label.into(),
            menu,
        }
    }
}

/// A horizontal bar of dropdown menus (shadcn's `Menubar`).
#[must_use = "menubars do nothing unless you show them"]
pub struct Menubar {
    id_salt: Id,
    menus: Vec<MenubarMenu>,
    sizing_hook: SizingHook<MenubarMetrics>,
}

impl Sizeable<MenubarMetrics> for Menubar {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<MenubarMetrics> {
        &mut self.sizing_hook
    }
}

impl Menubar {
    /// Create an empty menubar with a stable `id_salt` (used to remember which
    /// menu is open across frames).
    pub fn new(id_salt: impl std::hash::Hash) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            menus: Vec::new(),
            sizing_hook: SizingHook::new(),
        }
    }

    /// Add a top-level menu.
    pub fn menu(mut self, menu: MenubarMenu) -> Self {
        self.menus.push(menu);
        self
    }

    /// Render the bar. Returns `Some((menu_index, item_index))` when an item is
    /// clicked this frame — `menu_index` is the trigger's position and
    /// `item_index` counts only [items](DropdownMenu::item) within that menu.
    pub fn show(mut self, ui: &mut Ui) -> Option<(usize, usize)> {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let active_id = self.id_salt.with("active");

        // Which menu (if any) is currently open, from last frame.
        let mut active: Option<usize> = ui.data(|d| d.get_temp(active_id)).flatten();

        let mut selection = None;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = m.trigger_gap;
            for (i, entry) in self.menus.iter().enumerate() {
                let is_open = active == Some(i);
                let trigger = trigger_row(ui, tokens, m, &entry.label, is_open);

                // Click toggles this menu; clicking the open one closes it.
                if trigger.clicked() {
                    active = if is_open { None } else { Some(i) };
                }
                // Hover-to-switch: once a menu is open, moving onto a sibling
                // trigger opens that one instead (native menubar behaviour).
                else if active.is_some() && active != Some(i) && trigger.hovered() {
                    active = Some(i);
                }

                // Drive the popup from a local bool so egui's outside-click /
                // Escape / item-click close paths flip it back for us.
                let mut open = active == Some(i);
                if open {
                    let was_open = open;
                    egui::Popup::from_response(&trigger)
                        .id(self.id_salt.with(("menu", i)))
                        .open_bool(&mut open)
                        .frame(DropdownMenu::frame(tokens))
                        .align(egui::RectAlign::BOTTOM_START)
                        .gap(m.popup_gap)
                        .show(|ui| {
                            if let Some(item) = entry.menu.show_contents(ui, tokens) {
                                selection = Some((i, item));
                            }
                        });
                    if was_open && !open {
                        active = None;
                    }
                }
            }
        });

        // An item click closes the bar.
        if selection.is_some() {
            active = None;
        }
        ui.data_mut(|d| d.insert_temp(active_id, active));

        selection
    }
}

/// A single menubar trigger: ghost text that takes the `accent` surface on
/// hover or while its menu is open.
fn trigger_row(
    ui: &mut Ui,
    tokens: Tokens,
    m: MenubarMetrics,
    label: &str,
    open: bool,
) -> Response {
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(m.trigger_text),
        tokens.foreground,
    );
    let width = 2.0_f32.mul_add(m.trigger_pad_x, galley.size().x);
    let height = 2.0_f32
        .mul_add(m.trigger_pad_y, galley.size().y)
        .max(m.trigger_min_h);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

    if ui.is_rect_visible(rect) {
        let hover_t = ui.ctx().animate_bool_with_time(
            response.id.with("hover"),
            response.hovered(),
            m.hover_time,
        );
        // `data-[state=open]:bg-accent` plus `focus:bg-accent`.
        let fill_t = if open { 1.0 } else { hover_t };
        if fill_t > 0.01 {
            ui.painter().rect(
                rect,
                tokens.radius_md(),
                tokens.accent.gamma_multiply(fill_t),
                Stroke::NONE,
                StrokeKind::Inside,
            );
        }
        let text_color = tokens
            .foreground
            .lerp_to_gamma(tokens.accent_foreground, fill_t);
        let galley = ui.painter().layout_no_wrap(
            label.to_owned(),
            egui::FontId::proportional(m.trigger_text),
            text_color,
        );
        let pos = egui::pos2(
            rect.center().x - galley.size().x / 2.0,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(pos, galley, text_color);
    }
    response
}
