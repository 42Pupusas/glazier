//! [`Grid`] — a fixed-column CSS-grid layout, mirroring Tailwind's `grid
//! grid-cols-N gap-G`.
//!
//! Unlike egui's built-in [`Grid`](egui::Grid) (which sizes columns to their
//! content), this fills the available width and divides it into `columns` equal
//! tracks separated by a `gap`, exactly like `grid-cols-N`. Cells flow in
//! row-major order; each row is as tall as its tallest cell, and the next row
//! starts beneath it — so a 2×2 of cards packs with no manual measurement.
//!
//! ```no_run
//! use glazier::grid::Grid;
//! # egui::__run_test_ui(|ui| {
//! Grid::new(2).gap(16.0).show(ui, 4, |idx, ui| {
//!     ui.label(format!("cell {idx}"));
//! });
//! # });
//! ```

use egui::{Ui, Vec2};

/// A fixed-column grid that fills the available width.
#[must_use = "grids do nothing unless you show them"]
pub struct Grid {
    columns: usize,
    gap: f32,
}

impl Grid {
    /// Create a grid with `columns` equal-width tracks (`grid-cols-N`).
    ///
    /// `columns` is clamped to at least 1.
    pub const fn new(columns: usize) -> Self {
        Self {
            columns: if columns == 0 { 1 } else { columns },
            gap: 16.0, // gap-4
        }
    }

    /// Set the gap between cells, in points (both axes — Tailwind's `gap-G`).
    pub const fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Lay out `count` cells in row-major order, invoking `cell` once per cell
    /// with its flat index and a fixed-width child [`Ui`].
    ///
    /// One `FnMut` is reused for every cell (rather than a list of closures),
    /// so cells can freely share mutable state across the whole grid.
    #[allow(clippy::cast_precision_loss)] // column counts are tiny
    pub fn show(self, ui: &mut Ui, count: usize, mut cell: impl FnMut(usize, &mut Ui)) {
        if count == 0 {
            return;
        }
        let cols = self.columns;
        let gap = self.gap;
        // Equal tracks: divide the row, minus the inter-column gaps.
        let total = ui.available_width();
        let col_w = (gap.mul_add(-(cols as f32 - 1.0), total) / cols as f32).max(0.0);
        let rows = count.div_ceil(cols);

        for r in 0..rows {
            if r > 0 {
                ui.add_space(gap);
            }
            // A row is a top-aligned horizontal band; egui tracks its height, so
            // the next row starts below the tallest cell.
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for c in 0..cols {
                    let idx = r * cols + c;
                    if idx >= count {
                        break;
                    }
                    ui.allocate_ui_with_layout(
                        Vec2::new(col_w, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(col_w);
                            cell(idx, ui);
                        },
                    );
                }
            });
        }
    }
}
