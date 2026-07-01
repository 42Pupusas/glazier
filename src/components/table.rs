//! [`Table`] — a data table, mirroring shadcn's `<Table>`.
//!
//! A header row of muted, medium-weight column labels above a body of cells,
//! with a hairline border under the header and between rows (shadcn's
//! `border-b`), an optional row-hover highlight (`hover:bg-muted/50`), and
//! per-column [`Align`](Column::align) for numeric/trailing columns.
//!
//! Columns are declared with [`column`](Table::column); each carries a header
//! label, a width [`Sizing`], and an alignment. Rows are pushed as vectors of
//! cell strings via [`row`](Table::row). [`show`](Table::show) returns the index
//! of the body row clicked this frame.
//!
//! ```no_run
//! use glazier::table::{Table, Sizing};
//! use egui::{Align, Widget as _};
//! # egui::__run_test_ui(|ui| {
//! Table::new()
//!     .column("Invoice", Sizing::Auto, Align::LEFT)
//!     .column("Status", Sizing::Auto, Align::LEFT)
//!     .column("Amount", Sizing::Remainder, Align::RIGHT)
//!     .row(["INV001", "Paid", "$250.00"])
//!     .row(["INV002", "Pending", "$150.00"])
//!     .ui(ui);
//! # });
//! ```

use egui::{Align, Color32, Response, RichText, Sense, Ui, Vec2, Widget};

use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry for [`Table`] — reach in via [`Table::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct TableMetrics {
    /// Cell height (shadcn rows are `h-12` ≈ 48px; tuned to the card body
    /// scale).
    pub row_h: f32,
    /// Horizontal padding inside each cell (shadcn `px-2`, nudged up).
    pub cell_pad_x: f32,
    /// Cell text size (shadcn `text-sm`).
    pub text_size: f32,
    /// Gap between columns.
    pub col_gap: f32,
}

impl Default for TableMetrics {
    fn default() -> Self {
        Self {
            row_h: 40.0,
            cell_pad_x: 10.0,
            text_size: 13.0,
            col_gap: 8.0,
        }
    }
}

/// How a column claims horizontal space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sizing {
    /// Fixed width in points.
    Exact(f32),
    /// Sized to the column's content (header + widest cell), clamped sane.
    Auto,
    /// Shares the leftover width equally with other `Remainder` columns.
    Remainder,
}

/// A single column definition.
struct Column {
    header: String,
    sizing: Sizing,
    align: Align,
}

/// A data table.
#[must_use = "tables do nothing unless shown"]
#[derive(Default)]
pub struct Table {
    columns: Vec<Column>,
    rows: Vec<Vec<String>>,
    min_rows: usize,
    sizing_hook: SizingHook<TableMetrics>,
}

impl Sizeable<TableMetrics> for Table {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<TableMetrics> {
        &mut self.sizing_hook
    }
}

impl Table {
    /// Create an empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a column with a header label, [`Sizing`], and cell alignment.
    pub fn column(mut self, header: impl Into<String>, sizing: Sizing, align: Align) -> Self {
        self.columns.push(Column {
            header: header.into(),
            sizing,
            align,
        });
        self
    }

    /// Reserve space for at least `n` body rows, padding short pages with blank
    /// rows so the table keeps a constant height. Use this for paginated tables
    /// (set it to the page size) so the controls below don't jump when the last
    /// page has fewer rows.
    pub const fn min_rows(mut self, n: usize) -> Self {
        self.min_rows = n;
        self
    }

    /// Append a row of cell strings (one per column; missing cells render blank).
    pub fn row<I, S>(mut self, cells: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.rows.push(cells.into_iter().map(Into::into).collect());
        self
    }

    /// Render the table from its declared string rows. Returns the row index
    /// the user clicked this frame, or `None`.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        self.render(ui).1
    }

    /// Compute each column's resolved pixel width for the given total width.
    fn widths(&self, ui: &Ui, total: f32, m: TableMetrics) -> Vec<f32> {
        let n = self.columns.len();
        let mut widths = vec![0.0_f32; n];
        let mut remainder_idx = Vec::new();
        let mut used = 0.0;

        for (i, col) in self.columns.iter().enumerate() {
            match col.sizing {
                Sizing::Exact(w) => widths[i] = w,
                Sizing::Auto => widths[i] = self.auto_width(ui, i, m),
                Sizing::Remainder => remainder_idx.push(i),
            }
            if !matches!(col.sizing, Sizing::Remainder) {
                used += widths[i];
            }
        }

        #[allow(clippy::cast_precision_loss)]
        let gaps = m.col_gap * n.saturating_sub(1) as f32;
        let leftover = (total - used - gaps).max(0.0);
        if remainder_idx.is_empty() {
            // No remainder columns: hand any leftover to the last column so the
            // table fills its width (shadcn tables are `w-full`).
            if let Some(last) = widths.last_mut() {
                *last += leftover;
            }
        } else {
            #[allow(clippy::cast_precision_loss)]
            let share = leftover / remainder_idx.len() as f32;
            for i in remainder_idx {
                widths[i] = share;
            }
        }
        widths
    }

    /// Measure the natural width of column `i`: the wider of its header and its
    /// widest cell, plus padding, clamped to a sane range.
    fn auto_width(&self, ui: &Ui, i: usize, m: TableMetrics) -> f32 {
        let measure = |s: &str, size: f32| {
            ui.painter()
                .layout_no_wrap(
                    s.to_owned(),
                    egui::FontId::proportional(size),
                    Color32::PLACEHOLDER,
                )
                .size()
                .x
        };
        let mut w = measure(&self.columns[i].header, m.text_size);
        for row in &self.rows {
            if let Some(cell) = row.get(i) {
                w = w.max(measure(cell, m.text_size));
            }
        }
        m.cell_pad_x.mul_add(2.0, w).clamp(48.0, 360.0)
    }

    /// Core renderer shared by [`show`](Self::show) and the [`Widget`] impl.
    fn render(mut self, ui: &mut Ui) -> (Response, Option<usize>) {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let total = ui.available_width();
        let widths = self.widths(ui, total, m);
        let mut clicked = None;

        let resp = ui
            .vertical(|ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;

                // Header row.
                let (_, rects) = row_strip(ui, &widths, total, m);
                for (i, col) in self.columns.iter().enumerate() {
                    cell_text(
                        ui,
                        rects[i],
                        &col.header,
                        col.align,
                        tokens.muted_foreground,
                        true,
                        m,
                    );
                }
                hairline(ui, total, tokens.border);

                // Body rows.
                let aligns: Vec<Align> = self.columns.iter().map(|c| c.align).collect();
                for (r, row) in self.rows.iter().enumerate() {
                    let (resp, rects) = row_strip(ui, &widths, total, m);
                    // Hover highlight first, so cell text paints on top of it.
                    if resp.hovered() {
                        ui.painter()
                            .rect_filled(resp.rect, 0.0, tokens.muted.gamma_multiply(0.5));
                    }
                    for (i, align) in aligns.iter().enumerate() {
                        let text = row.get(i).map_or("", String::as_str);
                        cell_text(ui, rects[i], text, *align, tokens.foreground, false, m);
                    }
                    if resp.clicked() {
                        clicked = Some(r);
                    }
                    hairline(ui, total, tokens.border.gamma_multiply(0.6));
                }

                // Pad short pages with empty rows so the table keeps a constant
                // height (no layout jump when the last page is underfull).
                for _ in self.rows.len()..self.min_rows {
                    let _ = row_strip(ui, &widths, total, m);
                    hairline(ui, total, tokens.border.gamma_multiply(0.6));
                }
            })
            .response;
        (resp, clicked)
    }
}

/// Lay out one row: allocate a row-height-tall strip spanning `total`, split
/// it into per-column rects (respecting the column gap), and return the row's
/// click/hover [`Response`] alongside those rects.
fn row_strip(
    ui: &mut Ui,
    widths: &[f32],
    total: f32,
    m: TableMetrics,
) -> (Response, Vec<egui::Rect>) {
    let (id, rect) = ui.allocate_space(Vec2::new(total, m.row_h));
    let mut rects = Vec::with_capacity(widths.len());
    let mut x = rect.left();
    for &w in widths {
        let r = egui::Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::new(w, m.row_h));
        rects.push(r);
        x += w + m.col_gap;
    }
    (ui.interact(rect, id, Sense::click()), rects)
}

/// Paint one cell's text into `rect`, padded and aligned. Headers are
/// medium-weight; body cells are regular.
#[allow(clippy::too_many_arguments)]
fn cell_text(
    ui: &Ui,
    rect: egui::Rect,
    text: &str,
    align: Align,
    color: Color32,
    header: bool,
    m: TableMetrics,
) {
    if text.is_empty() {
        return;
    }
    let rt = if header {
        RichText::new(text)
            .font(crate::fonts::semibold(ui, m.text_size))
            .color(color)
    } else {
        RichText::new(text).size(m.text_size).color(color)
    };
    let galley = egui::WidgetText::from(rt).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        rect.width(),
        egui::TextStyle::Body,
    );
    let inner = rect.shrink2(Vec2::new(m.cell_pad_x, 0.0));
    let x = match align {
        Align::Min => inner.left(),
        Align::Center => inner.center().x - galley.size().x / 2.0,
        Align::Max => inner.right() - galley.size().x,
    };
    let y = rect.center().y - galley.size().y / 2.0;
    ui.painter().galley(egui::pos2(x, y), galley, color);
}

/// Draw a full-width 1px separator line.
fn hairline(ui: &mut Ui, total: f32, color: Color32) {
    let (_, rect) = ui.allocate_space(Vec2::new(total, 1.0));
    ui.painter().hline(
        rect.left()..=rect.right(),
        rect.center().y,
        egui::Stroke::new(1.0, color),
    );
}

impl Widget for Table {
    /// Renders the table, discarding the click index. Use [`Table::show`] to
    /// learn which row was clicked.
    fn ui(self, ui: &mut Ui) -> Response {
        self.render(ui).0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A populated table renders without panicking and reports no click.
    #[test]
    fn renders_rows() {
        let ctx = egui::Context::default();
        let mut clicked = Some(0);
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            clicked = Table::new()
                .column("A", Sizing::Auto, Align::LEFT)
                .column("B", Sizing::Remainder, Align::RIGHT)
                .row(["1", "2"])
                .row(["3", "4"])
                .show(ui);
        });
        assert_eq!(clicked, None);
    }

    /// Widths sum to the available total so the table fills its row.
    #[test]
    fn widths_fill_total() {
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let table = Table::new()
                .column("A", Sizing::Exact(100.0), Align::LEFT)
                .column("B", Sizing::Remainder, Align::LEFT);
            let m = TableMetrics::default();
            let widths = table.widths(ui, 400.0, m);
            let sum: f32 = widths.iter().sum::<f32>() + m.col_gap;
            assert!((sum - 400.0).abs() < 0.5, "sum {sum}");
        });
    }
}
