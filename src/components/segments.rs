//! Rounding for a row of joined segments, shared by
//! [`ButtonGroup`](crate::ButtonGroup) and [`ToggleGroup`](crate::ToggleGroup).

use egui::CornerRadius;

/// A row of `count` segments drawn flush against each other.
#[derive(Clone, Copy, Debug)]
pub struct Segments {
    pub count: usize,
}

impl Segments {
    /// Round only the row's outer corners: the first segment rounds its left
    /// side, the last its right side, interior segments stay square.
    #[must_use]
    pub const fn radius(self, i: usize, r: u8) -> CornerRadius {
        let left = if i == 0 { r } else { 0 };
        let right = if i + 1 == self.count { r } else { 0 };
        CornerRadius {
            nw: left,
            sw: left,
            ne: right,
            se: right,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_outer_corners_round() {
        let row = Segments { count: 3 };
        assert_eq!(
            row.radius(0, 8),
            CornerRadius {
                nw: 8,
                sw: 8,
                ne: 0,
                se: 0
            }
        );
        assert_eq!(row.radius(1, 8), CornerRadius::ZERO);
        assert_eq!(
            row.radius(2, 8),
            CornerRadius {
                nw: 0,
                sw: 0,
                ne: 8,
                se: 8
            }
        );
        assert_eq!(Segments { count: 1 }.radius(0, 8), CornerRadius::same(8));
    }
}
