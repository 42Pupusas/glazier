//! [`DataTable`] — a sortable, filterable, paginated data table.
//!
//! Where [`Table`](crate::table::Table) is the static presentation layer,
//! `DataTable` adds shadcn's data-table behaviours on top: a global text
//! **filter**, **click-to-sort** columns (cycling ascending → descending →
//! unsorted), and **pagination** via [`Pagination`](crate::pagination::Pagination).
//!
//! All interaction state (the active sort column + direction, the filter text,
//! and the current page) is persisted in egui memory keyed by the table's
//! `id_salt`, so the widget is constructed fresh each frame from your data and
//! needs no external state plumbing.
//!
//! ```no_run
//! use glazier::data_table::{DataTable, DataColumn};
//! use glazier::table::Sizing;
//! use egui::Align;
//! # egui::__run_test_ui(|ui| {
//! DataTable::new("invoices")
//!     .column(DataColumn::new("Invoice"))
//!     .column(DataColumn::new("Status"))
//!     .column(DataColumn::new("Amount").align(Align::RIGHT).numeric())
//!     .row(["INV001", "Paid", "250.00"])
//!     .row(["INV002", "Pending", "150.00"])
//!     .page_size(10)
//!     .show(ui);
//! # });
//! ```

use egui::{Align, Color32, Id, Sense, Ui, Vec2, Widget as _};

use crate::pagination::Pagination;
use crate::table::Sizing;
use crate::tokens::Tokens;

/// Row height (matches [`Table`](crate::table::Table)'s `ROW_H`).
const ROW_H: f32 = 40.0;
/// Horizontal padding inside each cell.
const CELL_PAD_X: f32 = 10.0;
/// Cell / header text size (`text-sm`).
const TEXT_SIZE: f32 = 13.0;
/// Gap between columns.
const COL_GAP: f32 = 8.0;
/// Gap below the filter field.
const FILTER_GAP: f32 = 10.0;

/// A column definition for a [`DataTable`].
pub struct DataColumn {
    header: String,
    align: Align,
    sizing: Sizing,
    sortable: bool,
    numeric: bool,
}

impl DataColumn {
    /// Create a left-aligned, auto-sized, sortable text column.
    pub fn new(header: impl Into<String>) -> Self {
        Self {
            header: header.into(),
            align: Align::LEFT,
            sizing: Sizing::Auto,
            sortable: true,
            numeric: false,
        }
    }

    /// Set the cell alignment (use [`Align::RIGHT`] for numeric columns).
    #[must_use]
    pub const fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Set the column's width [`Sizing`].
    #[must_use]
    pub const fn sizing(mut self, sizing: Sizing) -> Self {
        self.sizing = sizing;
        self
    }

    /// Mark the column non-sortable (no header affordance, no sort on click).
    #[must_use]
    pub const fn unsortable(mut self) -> Self {
        self.sortable = false;
        self
    }

    /// Sort this column numerically (parsing cells as numbers) rather than
    /// lexicographically. Non-numeric cells sort as the smallest value.
    #[must_use]
    pub const fn numeric(mut self) -> Self {
        self.numeric = true;
        self
    }
}

/// The persisted interaction state for one table.
#[derive(Clone, Default)]
struct DtState {
    /// The active sort column, if any.
    sort_col: Option<usize>,
    /// Whether the active sort is descending.
    desc: bool,
    /// The current filter text.
    filter: String,
    /// The current 0-based page.
    page: usize,
}

/// A sortable, filterable, paginated data table.
#[must_use = "data tables do nothing unless shown"]
pub struct DataTable {
    id_salt: String,
    columns: Vec<DataColumn>,
    rows: Vec<Vec<String>>,
    page_size: usize,
    searchable: bool,
    search_placeholder: String,
}

impl DataTable {
    /// Create an empty table identified by `id_salt` (used to persist sort,
    /// filter, and page state across frames).
    pub fn new(id_salt: impl Into<String>) -> Self {
        Self {
            id_salt: id_salt.into(),
            columns: Vec::new(),
            rows: Vec::new(),
            page_size: 10,
            searchable: true,
            search_placeholder: "Filter…".to_owned(),
        }
    }

    /// Append a column definition.
    pub fn column(mut self, column: DataColumn) -> Self {
        self.columns.push(column);
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

    /// Set how many rows show per page (default 10).
    pub const fn page_size(mut self, n: usize) -> Self {
        self.page_size = n;
        self
    }

    /// Hide the filter field (default shown).
    pub const fn searchable(mut self, on: bool) -> Self {
        self.searchable = on;
        self
    }

    /// Set the filter field's placeholder text.
    pub fn search_placeholder(mut self, hint: impl Into<String>) -> Self {
        self.search_placeholder = hint.into();
        self
    }

    /// Render the table. Returns the original (pre-sort, pre-filter) index of
    /// the data row clicked this frame, or `None`.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let tokens = Tokens::get(ui);
        let state_id = Id::new(("glazier-datatable", &self.id_salt));
        let mut state: DtState = ui.data_mut(|d| d.get_temp(state_id).unwrap_or_default());

        let mut clicked = None;
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;

            // Filter field.
            if self.searchable {
                let prev = state.filter.clone();
                crate::input::Input::new(&mut state.filter)
                    .placeholder(self.search_placeholder.clone())
                    .ui(ui);
                if state.filter != prev {
                    state.page = 0; // reset to first page when the filter changes
                }
                ui.add_space(FILTER_GAP);
            }

            // Resolve the visible (filtered + sorted) row order.
            let order = self.visible_order(&state);

            // Paginate.
            let total_pages = order.len().div_ceil(self.page_size).max(1);
            if state.page >= total_pages {
                state.page = total_pages - 1;
            }
            let start = state.page * self.page_size;
            let page_rows: Vec<usize> = order
                .iter()
                .skip(start)
                .take(self.page_size)
                .copied()
                .collect();

            // Render header + body.
            let total = ui.available_width();
            let widths = self.widths(ui, total);
            self.header(ui, tokens, &widths, total, &mut state);
            clicked = self.body(ui, tokens, &widths, total, &page_rows);

            // Pagination (only when more than one page).
            if total_pages > 1 {
                ui.add_space(FILTER_GAP);
                Pagination::new(total_pages).show(ui, &mut state.page);
            }
        });

        ui.data_mut(|d| d.insert_temp(state_id, state));
        clicked
    }

    /// Compute the filtered, sorted row indices into [`self.rows`](Self::rows).
    fn visible_order(&self, state: &DtState) -> Vec<usize> {
        let needle = state.filter.trim().to_lowercase();
        let mut order: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                needle.is_empty() || row.iter().any(|c| c.to_lowercase().contains(&needle))
            })
            .map(|(i, _)| i)
            .collect();

        if let Some(col) = state.sort_col {
            let numeric = self.columns.get(col).is_some_and(|c| c.numeric);
            order.sort_by(|&a, &b| {
                let sa = self.rows[a].get(col).map_or("", String::as_str);
                let sb = self.rows[b].get(col).map_or("", String::as_str);
                let ord = if numeric {
                    let na = parse_num(sa);
                    let nb = parse_num(sb);
                    na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal)
                } else {
                    sa.to_lowercase().cmp(&sb.to_lowercase())
                };
                if state.desc {
                    ord.reverse()
                } else {
                    ord
                }
            });
        }
        order
    }

    /// Resolve each column's pixel width for `total` (mirrors `Table::widths`).
    fn widths(&self, ui: &Ui, total: f32) -> Vec<f32> {
        let n = self.columns.len();
        let mut widths = vec![0.0_f32; n];
        let mut remainder = Vec::new();
        let mut used = 0.0;
        for (i, col) in self.columns.iter().enumerate() {
            match col.sizing {
                Sizing::Exact(w) => widths[i] = w,
                Sizing::Auto => widths[i] = self.auto_width(ui, i),
                Sizing::Remainder => remainder.push(i),
            }
            if !matches!(col.sizing, Sizing::Remainder) {
                used += widths[i];
            }
        }
        #[allow(clippy::cast_precision_loss)]
        let gaps = COL_GAP * n.saturating_sub(1) as f32;
        let leftover = (total - used - gaps).max(0.0);
        if remainder.is_empty() {
            if let Some(last) = widths.last_mut() {
                *last += leftover;
            }
        } else {
            #[allow(clippy::cast_precision_loss)]
            let share = leftover / remainder.len() as f32;
            for i in remainder {
                widths[i] = share;
            }
        }
        widths
    }

    /// Natural width of column `i`: wider of header (plus sort-arrow room) and
    /// its widest cell, padded and clamped.
    fn auto_width(&self, ui: &Ui, i: usize) -> f32 {
        let measure = |s: &str| {
            ui.painter()
                .layout_no_wrap(
                    s.to_owned(),
                    egui::FontId::proportional(TEXT_SIZE),
                    Color32::PLACEHOLDER,
                )
                .size()
                .x
        };
        let arrow = if self.columns[i].sortable { 14.0 } else { 0.0 };
        let mut w = measure(&self.columns[i].header) + arrow;
        for row in &self.rows {
            if let Some(cell) = row.get(i) {
                w = w.max(measure(cell));
            }
        }
        CELL_PAD_X.mul_add(2.0, w).clamp(48.0, 360.0)
    }

    /// Render the clickable header row, toggling sort state on click.
    fn header(&self, ui: &mut Ui, tokens: Tokens, widths: &[f32], total: f32, state: &mut DtState) {
        let (id, rect) = ui.allocate_space(Vec2::new(total, ROW_H));
        let mut x = rect.left();
        for (i, col) in self.columns.iter().enumerate() {
            let cell =
                egui::Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::new(widths[i], ROW_H));
            x += widths[i] + COL_GAP;

            let active = state.sort_col == Some(i);
            let color = if active {
                tokens.foreground
            } else {
                tokens.muted_foreground
            };
            let inner = cell.shrink2(Vec2::new(CELL_PAD_X, 0.0));

            // Header label.
            let galley = egui::WidgetText::from(
                egui::RichText::new(&col.header)
                    .font(crate::fonts::semibold(ui, TEXT_SIZE))
                    .color(color),
            )
            .into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                inner.width(),
                egui::TextStyle::Body,
            );
            let lx = match col.align {
                Align::Min => inner.left(),
                Align::Center => inner.center().x - galley.size().x / 2.0,
                Align::Max => inner.right() - galley.size().x - sort_gap(col.sortable),
            };
            let ly = cell.center().y - galley.size().y / 2.0;
            let gw = galley.size().x;
            ui.painter().galley(egui::pos2(lx, ly), galley, color);

            // Sort arrow.
            if col.sortable {
                let ax = if col.align == Align::Max {
                    inner.right() - 9.0
                } else {
                    lx + gw + 6.0
                };
                sort_arrow(
                    ui,
                    egui::pos2(ax, cell.center().y),
                    active,
                    state.desc,
                    color,
                );
            }

            // Interaction.
            if col.sortable {
                let resp = ui
                    .interact(cell, id.with(("dt-h", i)), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if resp.clicked() {
                    cycle_sort(state, i);
                }
            }
        }
        hairline(ui, total, tokens.border);
    }

    /// Render the body for the given page of original row indices. Returns the
    /// clicked original index, if any.
    fn body(
        &self,
        ui: &mut Ui,
        tokens: Tokens,
        widths: &[f32],
        total: f32,
        page_rows: &[usize],
    ) -> Option<usize> {
        let mut clicked = None;
        let aligns: Vec<Align> = self.columns.iter().map(|c| c.align).collect();
        for &orig in page_rows {
            let (id, rect) = ui.allocate_space(Vec2::new(total, ROW_H));
            let resp = ui.interact(rect, id.with(("dt-r", orig)), Sense::click());
            if resp.hovered() {
                ui.painter()
                    .rect_filled(rect, 0.0, tokens.muted.gamma_multiply(0.5));
            }
            let mut x = rect.left();
            for (i, align) in aligns.iter().enumerate() {
                let cell = egui::Rect::from_min_size(
                    egui::pos2(x, rect.top()),
                    Vec2::new(widths[i], ROW_H),
                );
                x += widths[i] + COL_GAP;
                let text = self.rows[orig].get(i).map_or("", String::as_str);
                cell_text(ui, cell, text, *align, tokens.foreground);
            }
            if resp.clicked() {
                clicked = Some(orig);
            }
            hairline(ui, total, tokens.border.gamma_multiply(0.6));
        }
        // Pad short pages so the table keeps a constant height.
        for _ in page_rows.len()..self.page_size {
            ui.allocate_space(Vec2::new(total, ROW_H));
            hairline(ui, total, tokens.border.gamma_multiply(0.6));
        }
        clicked
    }
}

/// Extra right padding reserved for the sort arrow on right-aligned columns.
const fn sort_gap(sortable: bool) -> f32 {
    if sortable {
        16.0
    } else {
        0.0
    }
}

/// Cycle a column's sort: unsorted → asc → desc → unsorted.
fn cycle_sort(state: &mut DtState, col: usize) {
    if state.sort_col == Some(col) {
        if state.desc {
            state.sort_col = None;
            state.desc = false;
        } else {
            state.desc = true;
        }
    } else {
        state.sort_col = Some(col);
        state.desc = false;
    }
    state.page = 0;
}

/// Parse a numeric cell, stripping common currency / grouping characters.
/// Non-numeric cells sort as negative infinity (smallest).
fn parse_num(s: &str) -> f64 {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    cleaned.parse().unwrap_or(f64::NEG_INFINITY)
}

/// Paint a small up/down sort arrow. Dimmed when the column isn't the active
/// sort; pointing down for descending, up for ascending.
fn sort_arrow(ui: &Ui, c: egui::Pos2, active: bool, desc: bool, color: Color32) {
    let col = if active {
        color
    } else {
        color.gamma_multiply(0.45)
    };
    let stroke = egui::Stroke::new(1.4, col);
    let w = 4.0;
    let h = 3.0;
    // Show the arrow for the direction that the *next* sorted state reflects:
    // active+desc → down, otherwise up.
    let down = active && desc;
    if down {
        ui.painter().line_segment(
            [egui::pos2(c.x - w, c.y - h), egui::pos2(c.x, c.y + h)],
            stroke,
        );
        ui.painter().line_segment(
            [egui::pos2(c.x + w, c.y - h), egui::pos2(c.x, c.y + h)],
            stroke,
        );
    } else {
        ui.painter().line_segment(
            [egui::pos2(c.x - w, c.y + h), egui::pos2(c.x, c.y - h)],
            stroke,
        );
        ui.painter().line_segment(
            [egui::pos2(c.x + w, c.y + h), egui::pos2(c.x, c.y - h)],
            stroke,
        );
    }
}

/// Paint one body cell's text, padded and aligned.
fn cell_text(ui: &Ui, rect: egui::Rect, text: &str, align: Align, color: Color32) {
    if text.is_empty() {
        return;
    }
    let galley = egui::WidgetText::from(egui::RichText::new(text).size(TEXT_SIZE).color(color))
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            rect.width(),
            egui::TextStyle::Body,
        );
    let inner = rect.shrink2(Vec2::new(CELL_PAD_X, 0.0));
    let x = match align {
        Align::Min => inner.left(),
        Align::Center => inner.center().x - galley.size().x / 2.0,
        Align::Max => inner.right() - galley.size().x,
    };
    let y = rect.center().y - galley.size().y / 2.0;
    ui.painter().galley(egui::pos2(x, y), galley, color);
}

/// Draw a full-width 1px separator.
fn hairline(ui: &mut Ui, total: f32, color: Color32) {
    let (_, rect) = ui.allocate_space(Vec2::new(total, 1.0));
    ui.painter().hline(
        rect.left()..=rect.right(),
        rect.center().y,
        egui::Stroke::new(1.0, color),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> DataTable {
        DataTable::new("t")
            .column(DataColumn::new("Name"))
            .column(DataColumn::new("Age").numeric())
            .row(["Charlie", "30"])
            .row(["alice", "25"])
            .row(["Bob", "40"])
    }

    /// Filtering keeps only rows containing the needle (case-insensitive).
    #[test]
    fn filter_matches_any_cell() {
        let dt = sample();
        let state = DtState {
            filter: "ALICE".to_owned(),
            ..Default::default()
        };
        let order = dt.visible_order(&state);
        assert_eq!(order, vec![1]);
    }

    /// Numeric ascending sort orders by value, not lexicographically.
    #[test]
    fn numeric_sort_ascending() {
        let dt = sample();
        let state = DtState {
            sort_col: Some(1),
            desc: false,
            ..Default::default()
        };
        let order = dt.visible_order(&state);
        // 25, 30, 40 → alice(1), Charlie(0), Bob(2)
        assert_eq!(order, vec![1, 0, 2]);
    }

    /// Text descending sort is case-insensitive and reversed.
    #[test]
    fn text_sort_descending() {
        let dt = sample();
        let state = DtState {
            sort_col: Some(0),
            desc: true,
            ..Default::default()
        };
        let order = dt.visible_order(&state);
        // desc: Charlie, Bob, alice → 0, 2, 1
        assert_eq!(order, vec![0, 2, 1]);
    }

    /// The sort cycle goes unsorted → asc → desc → unsorted.
    #[test]
    fn sort_cycles() {
        let mut s = DtState::default();
        cycle_sort(&mut s, 1);
        assert_eq!((s.sort_col, s.desc), (Some(1), false));
        cycle_sort(&mut s, 1);
        assert_eq!((s.sort_col, s.desc), (Some(1), true));
        cycle_sort(&mut s, 1);
        assert_eq!(s.sort_col, None);
    }

    /// Renders without panicking.
    #[test]
    fn renders() {
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let _ = sample().show(ui);
        });
    }
}
