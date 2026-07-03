//! [`Chart`] — a compact bar / line / area chart, mirroring shadcn's `<Chart>`
//! container (which wraps Recharts) and its `ChartConfig` of named series.
//!
//! shadcn's chart is a *theming + tooltip + legend* shell around a Recharts
//! graph. glazier has no Recharts, so [`Chart`] hand-paints the common cases —
//! grouped **bars**, **lines**, and stacked-friendly **areas** — over the same
//! mental model: a set of x-axis [`labels`](Chart::labels) (categories) and one
//! or more named [`Series`], each a row of values and a colour drawn from a
//! shadcn-style `--chart-N` palette.
//!
//! Faithful to shadcn's feature list:
//! - **`ChartConfig`** → a [`Series`] per metric (label + colour + values);
//! - **`ChartTooltip`** → hovering a category floats a card listing each
//!   series' value at that point, with a colour swatch;
//! - **`ChartLegend`** → an optional swatch+label row under the plot;
//! - a muted **cartesian grid** with x-axis category labels.
//!
//! ```no_run
//! use glazier::chart::{Chart, Kind, Series};
//! # egui::__run_test_ui(|ui| {
//! Chart::new(Kind::Bar)
//!     .labels(["Jan", "Feb", "Mar", "Apr", "May", "Jun"])
//!     .series(Series::new("Desktop", [186.0, 305.0, 237.0, 73.0, 209.0, 214.0]))
//!     .series(Series::new("Mobile", [80.0, 200.0, 120.0, 190.0, 130.0, 140.0]))
//!     .height(200.0)
//!     .show(ui);
//! # });
//! ```

use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

/// Overridable geometry for [`Chart`] — reach in via [`Chart::sizing`].
#[derive(Clone, Copy, Debug)]
pub struct ChartMetrics {
    /// Width of the left gutter reserved for y-axis tick labels.
    pub y_axis_w: f32,
    /// Legend row height, reserved below the plot when shown.
    pub legend_h: f32,
    /// Axis tick / x-label text size.
    pub axis_text: f32,
    /// Legend swatch+label text size.
    pub legend_text: f32,
    /// Legend swatch edge length.
    pub legend_swatch: f32,
    /// Gap between legend entries.
    pub legend_gap: f32,
    /// Tooltip title/row text size.
    pub tooltip_text: f32,
    /// Tooltip inner padding.
    pub tooltip_pad: f32,
    /// Tooltip row height.
    pub tooltip_row_h: f32,
    /// Tooltip swatch edge length.
    pub tooltip_swatch: f32,
}

impl Default for ChartMetrics {
    fn default() -> Self {
        Self {
            y_axis_w: 30.0,
            legend_h: 22.0,
            axis_text: 10.0,
            legend_text: 11.0,
            legend_swatch: 9.0,
            legend_gap: 14.0,
            tooltip_text: 11.0,
            tooltip_pad: 8.0,
            tooltip_row_h: 16.0,
            tooltip_swatch: 9.0,
        }
    }
}

/// shadcn's default `--chart-1..5` hues (their light palette), plus a couple of
/// extras so larger configs still get distinct colours.
const PALETTE: [Color32; 7] = [
    Color32::from_rgb(0x2a, 0x9d, 0x90), // chart-1 teal
    Color32::from_rgb(0xe7, 0x6e, 0x50), // chart-2 orange
    Color32::from_rgb(0x27, 0x47, 0x54), // chart-3 dark slate
    Color32::from_rgb(0xe8, 0xc4, 0x68), // chart-4 sand
    Color32::from_rgb(0xf4, 0xa2, 0x60), // chart-5 amber
    Color32::from_rgb(0x8b, 0x5c, 0xf6), // extra violet
    Color32::from_rgb(0x06, 0xb6, 0xd4), // extra cyan
];

/// Which mark a [`Chart`] draws.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Kind {
    /// Grouped vertical bars (one cluster per category).
    #[default]
    Bar,
    /// Poly-lines through each series' points.
    Line,
    /// Filled areas under each series' line.
    Area,
}

/// One named metric: a label, a row of values (one per category), and a colour.
#[derive(Clone)]
pub struct Series {
    label: String,
    values: Vec<f32>,
    color: Option<Color32>,
}

impl Series {
    /// Create a series `label` with its per-category `values`.
    pub fn new(label: impl Into<String>, values: impl Into<Vec<f32>>) -> Self {
        Self {
            label: label.into(),
            values: values.into(),
            color: None,
        }
    }

    /// Override the auto-assigned palette colour.
    #[must_use]
    pub const fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }
}

/// A bar / line / area chart.
#[must_use = "charts do nothing unless you show them"]
pub struct Chart {
    kind: Kind,
    labels: Vec<String>,
    series: Vec<Series>,
    height: f32,
    legend: bool,
    y_max: Option<f32>,
    sizing_hook: SizingHook<ChartMetrics>,
}

impl Sizeable<ChartMetrics> for Chart {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<ChartMetrics> {
        &mut self.sizing_hook
    }
}

impl Chart {
    /// Start a chart of the given [`Kind`].
    pub const fn new(kind: Kind) -> Self {
        Self {
            kind,
            labels: Vec::new(),
            series: Vec::new(),
            height: 220.0,
            legend: true,
            y_max: None,
            sizing_hook: SizingHook::new(),
        }
    }

    /// Set the x-axis category labels.
    pub fn labels<S: Into<String>>(mut self, labels: impl IntoIterator<Item = S>) -> Self {
        self.labels = labels.into_iter().map(Into::into).collect();
        self
    }

    /// Add a [`Series`].
    pub fn series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    /// Set the plot height in points (excludes the legend; default 220).
    pub const fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Show or hide the legend (default shown).
    pub const fn legend(mut self, legend: bool) -> Self {
        self.legend = legend;
        self
    }

    /// Pin the y-axis maximum instead of deriving it from the data.
    pub const fn y_max(mut self, max: f32) -> Self {
        self.y_max = Some(max);
        self
    }

    /// The colour for series `i` — its override, else a palette slot.
    fn color_of(&self, i: usize) -> Color32 {
        self.series[i].color.unwrap_or(PALETTE[i % PALETTE.len()])
    }

    /// Largest value across every series (or the pinned max), with headroom.
    fn resolve_max(&self) -> f32 {
        if let Some(m) = self.y_max {
            return m.max(1.0);
        }
        let peak = self
            .series
            .iter()
            .flat_map(|s| s.values.iter().copied())
            .fold(0.0_f32, f32::max);
        if peak <= 0.0 {
            1.0
        } else {
            // Round up to a "nice" ceiling for tidy gridlines.
            nice_ceiling(peak)
        }
    }

    /// Render the chart.
    pub fn show(mut self, ui: &mut Ui) -> egui::Response {
        let tokens = Tokens::get(ui);
        let m = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook));
        let n = self.labels.len();
        let width = ui.available_width();

        // Reserve plot + (optional) legend.
        let legend_h = if self.legend && !self.series.is_empty() {
            m.legend_h
        } else {
            0.0
        };
        let total = Vec2::new(width, self.height + legend_h);
        let (rect, response) = ui.allocate_exact_size(total, Sense::hover());

        if !ui.is_rect_visible(rect) || n == 0 || self.series.is_empty() {
            return response;
        }

        // Plot box: inset a left gutter for the y-axis tick labels and a hair
        // of right margin, leaving a bottom gutter for the x-axis labels and a
        // little top headroom so the tallest point isn't flush to the edge.
        let plot = Rect::from_min_max(
            Pos2::new(rect.left() + m.y_axis_w, rect.top() + 6.0),
            Pos2::new(rect.right() - 4.0, rect.top() + self.height - 20.0),
        );
        let y_max = self.resolve_max();

        Self::paint_grid(ui, tokens, m, plot, y_max);
        self.paint_x_labels(ui, tokens, m, plot, rect);

        let hover = response
            .hover_pos()
            .filter(|p| plot.x_range().contains(p.x));
        let active = hover.map(|p| Self::nearest_category(plot, p.x, n));

        match self.kind {
            Kind::Bar => self.paint_bars(ui, plot, y_max, n, active),
            Kind::Line => self.paint_lines(ui, plot, y_max, n, false, active),
            Kind::Area => self.paint_lines(ui, plot, y_max, n, true, active),
        }

        if legend_h > 0.0 {
            let legend_rect =
                Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - legend_h), rect.max);
            self.paint_legend(ui, tokens, m, legend_rect);
        }

        if let Some(idx) = active {
            self.paint_tooltip(ui, tokens, m, plot, idx, n);
        }

        response
    }

    /// Horizontal gridlines + a baseline, in the muted border colour.
    fn paint_grid(ui: &Ui, tokens: Tokens, m: ChartMetrics, plot: Rect, y_max: f32) {
        let painter = ui.painter();
        let lines = 4;
        let faint = tokens.border.gamma_multiply(0.7);
        for i in 0..=lines {
            #[allow(clippy::cast_precision_loss)]
            let t = i as f32 / lines as f32;
            let y = plot.bottom() - t * plot.height();
            painter.hline(plot.x_range(), y, Stroke::new(1.0, faint));
            // y-axis tick label.
            let val = t * y_max;
            painter.text(
                Pos2::new(plot.left() - 2.0, y),
                Align2::RIGHT_CENTER,
                format_tick(val),
                FontId::proportional(m.axis_text),
                tokens.muted_foreground,
            );
        }
    }

    /// X-axis category labels under the plot.
    fn paint_x_labels(&self, ui: &Ui, tokens: Tokens, m: ChartMetrics, plot: Rect, rect: Rect) {
        let painter = ui.painter();
        let n = self.labels.len();
        for (i, label) in self.labels.iter().enumerate() {
            let x = Self::category_center(plot, i, n);
            painter.text(
                Pos2::new(x, rect.top() + self.height - 14.0),
                Align2::CENTER_TOP,
                label,
                FontId::proportional(m.axis_text),
                tokens.muted_foreground,
            );
        }
    }

    /// The x-centre of category `i`.
    #[allow(clippy::cast_precision_loss)]
    fn category_center(plot: Rect, i: usize, n: usize) -> f32 {
        let band = plot.width() / n as f32;
        band.mul_add(i as f32 + 0.5, plot.left())
    }

    /// Which category a pointer x falls in.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn nearest_category(plot: Rect, x: f32, n: usize) -> usize {
        let band = plot.width() / n as f32;
        (((x - plot.left()) / band).floor() as usize).min(n - 1)
    }

    /// Map a value to a y-coordinate inside the plot.
    fn y_of(plot: Rect, value: f32, y_max: f32) -> f32 {
        (value / y_max)
            .clamp(0.0, 1.0)
            .mul_add(-plot.height(), plot.bottom())
    }

    /// Grouped vertical bars.
    #[allow(clippy::cast_precision_loss)]
    fn paint_bars(&self, ui: &Ui, plot: Rect, y_max: f32, n: usize, active: Option<usize>) {
        let painter = ui.painter();
        let s = self.series.len();
        let band = plot.width() / n as f32;
        let group_w = band * 0.7;
        let bar_w = group_w / s as f32;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let radius = (bar_w * 0.25).min(4.0) as u8;
        for cat in 0..n {
            let group_left = Self::category_center(plot, cat, n) - group_w / 2.0;
            let dim = active.is_some_and(|a| a != cat);
            for (si, series) in self.series.iter().enumerate() {
                let v = series.values.get(cat).copied().unwrap_or(0.0);
                let x0 = group_left + bar_w * si as f32;
                let top = Self::y_of(plot, v, y_max);
                let bar = Rect::from_min_max(
                    Pos2::new(x0 + 1.0, top),
                    Pos2::new(x0 + bar_w - 1.0, plot.bottom()),
                );
                let mut color = self.color_of(si);
                if dim {
                    color = color.gamma_multiply(0.4);
                }
                painter.rect_filled(
                    bar,
                    egui::CornerRadius {
                        nw: radius,
                        ne: radius,
                        sw: 0,
                        se: 0,
                    },
                    color,
                );
            }
        }
    }

    /// Lines (and optionally filled areas) through each series.
    #[allow(clippy::cast_precision_loss)]
    fn paint_lines(
        &self,
        ui: &Ui,
        plot: Rect,
        y_max: f32,
        n: usize,
        fill: bool,
        active: Option<usize>,
    ) {
        let painter = ui.painter();
        for (si, series) in self.series.iter().enumerate() {
            let color = self.color_of(si);
            let pts: Vec<Pos2> = (0..n)
                .map(|cat| {
                    let v = series.values.get(cat).copied().unwrap_or(0.0);
                    Pos2::new(
                        Self::category_center(plot, cat, n),
                        Self::y_of(plot, v, y_max),
                    )
                })
                .collect();

            if fill && pts.len() >= 2 {
                // Fill the band under the line one segment at a time. The area
                // under a poly-line is *concave*, so a single convex_polygon
                // would self-overlap; each segment's quad, however, is convex.
                let wash = color.gamma_multiply(0.18);
                let base = plot.bottom();
                for w in pts.windows(2) {
                    let quad = vec![w[0], w[1], Pos2::new(w[1].x, base), Pos2::new(w[0].x, base)];
                    painter.add(egui::Shape::convex_polygon(quad, wash, Stroke::NONE));
                }
            }

            painter.add(egui::Shape::line(pts.clone(), Stroke::new(2.0, color)));

            // Point dots; the active category's dot is emphasised.
            for (cat, p) in pts.iter().enumerate() {
                let r = if active == Some(cat) { 4.0 } else { 2.5 };
                painter.circle_filled(*p, r, color);
                if active == Some(cat) {
                    painter.circle_stroke(*p, r, Stroke::new(2.0, Tokens::get(ui).widget));
                }
            }
        }
    }

    /// The swatch+label legend row, centred under the plot.
    fn paint_legend(&self, ui: &Ui, tokens: Tokens, m: ChartMetrics, area: Rect) {
        let painter = ui.painter();
        let font = FontId::proportional(m.legend_text);
        // Measure total width to centre the row.
        let gap = m.legend_gap;
        let swatch = m.legend_swatch;
        let mut items = Vec::with_capacity(self.series.len());
        let mut total = 0.0;
        for (i, s) in self.series.iter().enumerate() {
            let g = painter.layout_no_wrap(s.label.clone(), font.clone(), tokens.foreground);
            let w = swatch + 5.0 + g.size().x;
            items.push((i, g, w));
            total += w;
        }
        #[allow(clippy::cast_precision_loss)]
        let span = self.series.len().saturating_sub(1) as f32;
        total += gap * span;

        let mut x = area.center().x - total / 2.0;
        let cy = area.center().y;
        for (i, g, w) in items {
            let sw = Rect::from_center_size(Pos2::new(x + swatch / 2.0, cy), Vec2::splat(swatch));
            painter.rect_filled(sw, 2, self.color_of(i));
            painter.galley(
                Pos2::new(x + swatch + 5.0, cy - g.size().y / 2.0),
                g,
                tokens.foreground,
            );
            x += w + gap;
        }
    }

    /// A floating tooltip card for the hovered category.
    fn paint_tooltip(
        &self,
        ui: &Ui,
        tokens: Tokens,
        m: ChartMetrics,
        plot: Rect,
        cat: usize,
        n: usize,
    ) {
        let painter = ui.painter();
        let cx = Self::category_center(plot, cat, n);

        // Vertical guide.
        painter.vline(
            cx,
            plot.y_range(),
            Stroke::new(1.0, tokens.muted_foreground.gamma_multiply(0.5)),
        );

        let title_font = FontId::proportional(m.tooltip_text);
        let row_font = FontId::proportional(m.tooltip_text);
        let pad = m.tooltip_pad;
        let row_h = m.tooltip_row_h;
        let swatch = m.tooltip_swatch;

        // Lay out rows: "label   value".
        let title = self.labels.get(cat).cloned().unwrap_or_default();
        let title_g = painter.layout_no_wrap(title, title_font, tokens.foreground);

        let mut max_w = title_g.size().x;
        let mut rows = Vec::with_capacity(self.series.len());
        for (si, s) in self.series.iter().enumerate() {
            let v = s.values.get(cat).copied().unwrap_or(0.0);
            let lg =
                painter.layout_no_wrap(s.label.clone(), row_font.clone(), tokens.muted_foreground);
            let vg = painter.layout_no_wrap(format_value(v), row_font.clone(), tokens.foreground);
            let w = swatch + 5.0 + lg.size().x + 16.0 + vg.size().x;
            max_w = max_w.max(w);
            rows.push((si, lg, vg));
        }

        #[allow(clippy::cast_precision_loss)]
        let body_h = (rows.len() as f32).mul_add(row_h, title_g.size().y + 4.0);
        let box_h = 2.0f32.mul_add(pad, body_h);
        let box_w = pad.mul_add(2.0, max_w);

        // Prefer the right of the guide; flip if it would overflow.
        let mut left = cx + 12.0;
        if left + box_w > plot.right() {
            left = cx - 12.0 - box_w;
        }
        let top = plot.top() + 6.0;
        let card = Rect::from_min_size(Pos2::new(left, top), Vec2::new(box_w, box_h));

        // Floating tooltip surface: opaque `widget`, not `background`
        // (translucent-capable app canvas) nor `card` (static containers).
        painter.rect(
            card,
            tokens.radius_md(),
            tokens.widget,
            Stroke::new(1.0, tokens.border),
            StrokeKind::Inside,
        );

        let mut y = card.top() + pad;
        let title_h = title_g.size().y;
        painter.galley(Pos2::new(card.left() + pad, y), title_g, tokens.foreground);
        y += title_h + 4.0;

        for (si, lg, vg) in rows {
            let cy = y + row_h / 2.0;
            let sw = Rect::from_center_size(
                Pos2::new(card.left() + pad + swatch / 2.0, cy),
                Vec2::splat(swatch),
            );
            painter.rect_filled(sw, 2, self.color_of(si));
            painter.galley(
                Pos2::new(card.left() + pad + swatch + 5.0, cy - lg.size().y / 2.0),
                lg,
                tokens.muted_foreground,
            );
            painter.galley(
                Pos2::new(card.right() - pad - vg.size().x, cy - vg.size().y / 2.0),
                vg,
                tokens.foreground,
            );
            y += row_h;
        }
    }
}

/// Round `peak` up to a tidy axis ceiling (1/2/2.5/5 × 10ⁿ).
fn nice_ceiling(peak: f32) -> f32 {
    let mag = 10.0_f32.powf(peak.log10().floor());
    let norm = peak / mag;
    let step = if norm <= 1.0 {
        1.0
    } else if norm <= 2.0 {
        2.0
    } else if norm <= 2.5 {
        2.5
    } else if norm <= 5.0 {
        5.0
    } else {
        10.0
    };
    step * mag
}

/// Compact y-tick formatting (`1.2k`, `350`).
fn format_tick(v: f32) -> String {
    if v >= 1000.0 {
        format!("{:.1}k", v / 1000.0)
    } else if v.fract().abs() < 0.05 {
        format!("{v:.0}")
    } else {
        format!("{v:.1}")
    }
}

/// Tooltip value formatting (integers stay integral).
fn format_value(v: f32) -> String {
    if v.fract().abs() < 0.05 {
        format!("{v:.0}")
    } else {
        format!("{v:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nice_ceilings() {
        assert!((nice_ceiling(186.0) - 200.0).abs() < 0.01);
        assert!((nice_ceiling(73.0) - 100.0).abs() < 0.01);
        assert!((nice_ceiling(305.0) - 500.0).abs() < 0.01);
        assert!((nice_ceiling(21.0) - 25.0).abs() < 0.01);
    }

    #[test]
    fn max_uses_peak_and_pin() {
        let c = Chart::new(Kind::Bar)
            .series(Series::new("a", [10.0, 40.0, 30.0]))
            .series(Series::new("b", [5.0, 12.0, 60.0]));
        assert!((c.resolve_max() - 100.0).abs() < 0.01);
        let pinned = Chart::new(Kind::Bar)
            .series(Series::new("a", [10.0]))
            .y_max(250.0);
        assert!((pinned.resolve_max() - 250.0).abs() < 0.01);
    }

    #[test]
    fn tick_formatting() {
        assert_eq!(format_tick(350.0), "350");
        assert_eq!(format_tick(1200.0), "1.2k");
    }
}
