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
    pub fn show(self, ui: &mut Ui, count: usize, mut cell: impl FnMut(usize, &mut Ui)) {
        let gap = self.gap;
        let col_w = self.column_width(ui.available_width());
        let indices: Vec<usize> = (0..count).collect();
        for (r, row) in indices.chunks(self.columns).enumerate() {
            if r > 0 {
                ui.add_space(gap);
            }
            // A row is a top-aligned horizontal band; egui tracks its height, so
            // the next row starts below the tallest cell.
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &idx in row {
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

    /// Equal tracks: the row minus the inter-column gaps, divided evenly.
    fn column_width(&self, total: f32) -> f32 {
        // Column counts are tiny; saturating at `u16::MAX` loses nothing.
        let cols = f32::from(u16::try_from(self.columns).unwrap_or(u16::MAX));
        (self.gap.mul_add(-(cols - 1.0), total) / cols).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;

    #[test]
    fn cells_fill_rows_in_order() {
        let mut rects = Vec::new();
        TestUi::new().frame(|ui| {
            ui.set_width(300.0);
            Grid::new(2).gap(20.0).show(ui, 3, |_, ui| {
                rects.push(ui.label("x").rect);
            });
            Grid::new(0).show(ui, 0, |_, _| unreachable!());
        });
        assert_eq!(rects.len(), 3);
        assert!((rects[0].top() - rects[1].top()).abs() < f32::EPSILON);
        assert!((rects[1].left() - rects[0].left() - 160.0).abs() < 1e-3);
        assert!(rects[2].top() > rects[0].bottom());
        assert!((Grid::new(3).gap(10.0).column_width(320.0) - 100.0).abs() < 1e-3);
    }
}
