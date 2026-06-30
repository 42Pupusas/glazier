//! [`ContextMenu`] — a right-click menu, mirroring shadcn's `<ContextMenu>`.
//!
//! Shares the [`DropdownMenu`](crate::dropdown_menu::DropdownMenu) content
//! surface — labels, items, destructive items, separators — but instead of
//! hanging off a trigger's click it opens at the **pointer** on a secondary
//! (right) click anywhere on the target [`Response`], exactly like shadcn's
//! `ContextMenuTrigger`.
//!
//! ```no_run
//! use glazier::context_menu::ContextMenu;
//! use glazier::dropdown_menu::DropdownMenu;
//! use egui::Widget as _;
//! # egui::__run_test_ui(|ui| {
//! let target = ui.button("Right-click me");
//! let menu = DropdownMenu::new()
//!     .item("Back")
//!     .item("Forward")
//!     .separator()
//!     .destructive("Delete");
//! if let Some(i) = ContextMenu::new(menu).show(ui, &target) {
//!     // handle the clicked item index
//!     let _ = i;
//! }
//! # });
//! ```

use egui::{Response, Ui};

use crate::components::dropdown_menu::DropdownMenu;
use crate::tokens::Tokens;

/// A right-click (context) menu wrapping a [`DropdownMenu`] surface.
#[must_use = "context menus do nothing unless you show them"]
pub struct ContextMenu {
    menu: DropdownMenu,
}

impl ContextMenu {
    /// Create a context menu from a [`DropdownMenu`] content definition.
    pub const fn new(menu: DropdownMenu) -> Self {
        Self { menu }
    }

    /// Attach the menu to `target`: open it at the pointer on right-click, and
    /// while open render the menu body. Returns the clicked item index (in
    /// declaration order, counting only items), if any.
    ///
    /// Dismisses on click (inside or out) or Escape, like shadcn.
    #[must_use]
    pub fn show(self, ui: &Ui, target: &Response) -> Option<usize> {
        let tokens = Tokens::get(ui);
        let mut clicked = None;
        egui::Popup::context_menu(target)
            .frame(DropdownMenu::frame(tokens))
            .show(|ui| {
                clicked = self.menu.show_contents(ui, tokens);
            });
        clicked
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Widget as _;

    /// Without a right-click the menu stays closed and reports no selection.
    #[test]
    fn closed_by_default() {
        let ctx = egui::Context::default();
        let mut picked = Some(0);
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let target = egui::Button::new("target").ui(ui);
            let menu = DropdownMenu::new().item("One").item("Two");
            picked = ContextMenu::new(menu).show(ui, &target);
        });
        assert_eq!(picked, None);
    }
}
