//! Parts shared by the index pickers — [`Select`](crate::Select),
//! [`NativeSelect`](crate::NativeSelect) and [`Combobox`](crate::Combobox):
//! the trigger's value line and the popup's option rows.

use egui::{Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::components::icon::Icon;
use crate::tokens::Tokens;

/// `chevron-down` (lucide) — the trigger's trailing affordance.
const CHEVRON_DOWN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#;
/// `check` (lucide) — marks the active option.
const CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>"#;

/// The trigger's contents: the value (or placeholder) and a trailing chevron.
#[derive(Clone, Copy)]
pub struct ValueLine {
    pub pad_x: f32,
    pub text: f32,
    pub chevron: f32,
    pub chevron_gap: f32,
}

impl ValueLine {
    /// Paint into the trigger `rect`. The value is one line, truncated with
    /// `…` so it never wraps or runs into the chevron.
    pub fn paint(self, ui: &Ui, tokens: Tokens, rect: Rect, label: &str, placeholder: bool) {
        let chevron = Rect::from_min_size(
            egui::pos2(
                rect.right() - self.pad_x - self.chevron,
                rect.center().y - self.chevron / 2.0,
            ),
            Vec2::splat(self.chevron),
        );
        Icon::new(CHEVRON_DOWN)
            .color(tokens.muted_foreground)
            .image(tokens)
            .paint_at(ui, chevron);

        let color = if placeholder {
            tokens.muted_foreground
        } else {
            tokens.foreground
        };
        let avail =
            (self.pad_x.mul_add(-2.0, rect.width()) - self.chevron - self.chevron_gap).max(0.0);
        let mut job = egui::text::LayoutJob::simple(
            label.to_owned(),
            egui::FontId::proportional(self.text),
            color,
            avail,
        );
        job.wrap = egui::text::TextWrapping::truncate_at_width(avail);
        let galley = ui.painter().layout_job(job);
        let pos = egui::pos2(
            rect.left() + self.pad_x,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(pos, galley, color);
    }
}

/// How an option row shows that it is the current value.
#[derive(Clone, Copy)]
pub enum Mark {
    /// A leading check in a column this wide; the text sits after it.
    Check(f32),
    /// The row stays accent-filled, as if hovered.
    Tint,
}

/// One option row in a picker's popup, highlighted on hover.
#[derive(Clone, Copy)]
pub struct OptionRow {
    pub tokens: Tokens,
    pub width: f32,
    pub text: f32,
    pub pad_x: f32,
    pub pad_y: f32,
    pub min_h: f32,
    pub mark: Mark,
}

impl OptionRow {
    pub fn show(&self, ui: &mut Ui, text: &str, active: bool) -> Response {
        let tokens = self.tokens;
        let font = egui::FontId::proportional(self.text);
        let measured =
            ui.painter()
                .layout_no_wrap(text.to_owned(), font.clone(), tokens.foreground);
        let height = self.pad_y.mul_add(2.0, measured.size().y).max(self.min_h);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(self.width, height), Sense::click());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let tinted = matches!(self.mark, Mark::Tint) && active;
        let lit = response.hovered() || tinted;
        let color = if lit {
            ui.painter().rect(
                rect,
                tokens.radius_xl(),
                tokens.accent,
                Stroke::NONE,
                StrokeKind::Inside,
            );
            tokens.accent_foreground
        } else {
            tokens.foreground
        };
        let mut x = rect.left() + self.pad_x;
        if let Mark::Check(col) = self.mark {
            if active {
                let icon =
                    Rect::from_min_size(egui::pos2(x, rect.center().y - 8.0), Vec2::splat(16.0));
                Icon::new(CHECK)
                    .color(color)
                    .image(tokens)
                    .paint_at(ui, icon);
            }
            x += col;
        }
        let galley = ui.painter().layout_no_wrap(text.to_owned(), font, color);
        let pos = egui::pos2(x, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(pos, galley, color);
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::TestUi;

    fn row(mark: Mark) -> OptionRow {
        OptionRow {
            tokens: Tokens::from_visuals(&egui::Visuals::light()),
            width: 200.0,
            text: 14.0,
            pad_x: 8.0,
            pad_y: 6.0,
            min_h: 32.0,
            mark,
        }
    }

    /// Paint fills for one row and return how many accent fills appeared.
    fn accent_fills(mark: Mark, active: bool) -> (usize, f32) {
        let out = TestUi::new().frame(|ui| {
            let _ = row(mark).show(ui, "Option", active);
        });
        let accent = row(mark).tokens.accent;
        let fills = out
            .shapes
            .iter()
            .filter(|c| matches!(&c.shape, egui::Shape::Rect(r) if r.fill == accent))
            .count();
        (fills, TestUi::text_x(&out, "Option"))
    }

    #[test]
    fn a_tint_mark_fills_the_active_row() {
        assert_eq!(accent_fills(Mark::Tint, true).0, 1);
        assert_eq!(accent_fills(Mark::Tint, false).0, 0);
    }

    #[test]
    fn a_check_mark_indents_the_text() {
        let (fills, checked_x) = accent_fills(Mark::Check(22.0), true);
        let (_, plain_x) = accent_fills(Mark::Tint, false);
        assert_eq!(fills, 0);
        assert!((checked_x - plain_x - 22.0).abs() < 1e-3);
    }

    #[test]
    fn long_values_are_truncated() {
        let line = ValueLine {
            pad_x: 12.0,
            text: 14.0,
            chevron: 16.0,
            chevron_gap: 6.0,
        };
        let tokens = Tokens::from_visuals(&egui::Visuals::light());
        let out = TestUi::new().frame(|ui| {
            let rect = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(90.0, 36.0));
            line.paint(ui, tokens, rect, "A very long value that cannot fit", false);
        });
        let text = out.shapes.iter().find_map(|c| match &c.shape {
            egui::Shape::Text(t) => Some(t.galley.clone()),
            _ => None,
        });
        assert!(text.is_some_and(|g| g.size().x <= 90.0 - 24.0 - 22.0 + 0.5));
    }
}
