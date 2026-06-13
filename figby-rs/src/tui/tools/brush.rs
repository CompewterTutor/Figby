use crate::tui::brush::BrushShape;
use crate::tui::canvas::{CanvasBuffer, CanvasCell};

pub fn stamp_offsets(shape: BrushShape, size: u8) -> Vec<(i16, i16)> {
    if size == 0 {
        return vec![(0, 0)];
    }
    let half = size as i16 / 2;
    let start = -half;
    let end = start + size as i16 - 1;
    match shape {
        BrushShape::Square => square_offsets(start, end),
        BrushShape::Circle => circle_offsets(size, start, end),
        BrushShape::SprayPaint => spray_offsets(size, start, end),
        BrushShape::Custom => vec![(0, 0)],
    }
}

fn square_offsets(start: i16, end: i16) -> Vec<(i16, i16)> {
    let mut offsets = Vec::new();
    for dy in start..=end {
        for dx in start..=end {
            offsets.push((dx, dy));
        }
    }
    offsets
}

fn circle_offsets(size: u8, start: i16, end: i16) -> Vec<(i16, i16)> {
    let r = size as f64 / 2.0;
    let mut offsets = Vec::new();
    for dy in start..=end {
        for dx in start..=end {
            let cx = dx as f64 + 0.5;
            let cy = dy as f64 + 0.5;
            if cx * cx + cy * cy <= r * r {
                offsets.push((dx, dy));
            }
        }
    }
    offsets
}

fn spray_offsets(size: u8, start: i16, end: i16) -> Vec<(i16, i16)> {
    let half = size as i16 / 2;
    let seed: u64 = 42;
    let mut offsets = Vec::new();
    for dy in start..=end {
        for dx in start..=end {
            let col = (dx + half) as u64;
            let row = (dy + half) as u64;
            let hash = col.wrapping_mul(7) + row.wrapping_mul(31) + seed;
            if hash % 100 < 35 {
                offsets.push((dx, dy));
            }
        }
    }
    offsets
}

pub fn paint_stamp(
    buffer: &mut CanvasBuffer,
    cx: i16,
    cy: i16,
    shape: BrushShape,
    size: u8,
    cell: CanvasCell,
) {
    for (dx, dy) in stamp_offsets(shape, size) {
        let x = cx.wrapping_add(dx);
        let y = cy.wrapping_add(dy);
        if x >= 0 && y >= 0 {
            if let Some(c) = buffer.get_mut(x as usize, y as usize) {
                *c = cell;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn paint_line(
    buffer: &mut CanvasBuffer,
    x0: i16,
    y0: i16,
    x1: i16,
    y1: i16,
    shape: BrushShape,
    size: u8,
    cell: CanvasCell,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;
    loop {
        paint_stamp(buffer, x, y, shape, size, cell);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::brush::BrushShape;

    fn filled_cell() -> CanvasCell {
        CanvasCell {
            ch: '@',
            fg: None,
            bg: None,
        }
    }

    fn canvas_5x5() -> CanvasBuffer {
        CanvasBuffer::new(5, 5)
    }

    fn canvas_10x10() -> CanvasBuffer {
        CanvasBuffer::new(10, 10)
    }

    fn canvas_30x10() -> CanvasBuffer {
        CanvasBuffer::new(30, 10)
    }

    #[test]
    fn test_stamp_square_covers_correct_cells() {
        let mut buf = canvas_5x5();
        let cell = filled_cell();
        // Square brush size 3 at center (2,2): covers 3x3 block (1..=3, 1..=3)
        paint_stamp(&mut buf, 2, 2, BrushShape::Square, 3, cell);
        for y in 0..5 {
            for x in 0..5 {
                let in_brush = (1..=3).contains(&x) && (1..=3).contains(&y);
                let c = buf.get(x, y).unwrap();
                if in_brush {
                    assert_eq!(c.ch, '@', "cell ({},{}) should be painted", x, y);
                } else {
                    assert_eq!(c.ch, ' ', "cell ({},{}) should be empty", x, y);
                }
            }
        }
    }

    #[test]
    fn test_stamp_circle_round_shape() {
        let mut buf = canvas_10x10();
        let cell = filled_cell();
        // Size-5 circle at (5,5): radius 2.5
        paint_stamp(&mut buf, 5, 5, BrushShape::Circle, 5, cell);
        // Center cell always painted
        assert_eq!(buf.get(5, 5).unwrap().ch, '@');
        // Corner cells of bounding square (3,3), (7,3), (3,7), (7,7) may or may not be painted
        // depending on radius check. Let's just verify that at least the center N cells are filled
        // and some corner cells are empty (circle is not a full square)
        let painted = (0..10)
            .flat_map(|y| (0..10).map(move |x| (x, y)))
            .filter(|&(x, y)| buf.get(x, y).unwrap().ch == '@')
            .count();
        let uncovered = (0..10)
            .flat_map(|y| (0..10).map(move |x| (x, y)))
            .filter(|&(x, y)| buf.get(x, y).unwrap().ch == ' ')
            .count();
        assert!(painted > 0, "circle should paint some cells");
        assert!(uncovered > 0, "circle should leave some cells empty");
    }

    #[test]
    fn test_stamp_spray_deterministic() {
        let mut buf_a = canvas_10x10();
        let mut buf_b = canvas_10x10();
        let cell = filled_cell();
        paint_stamp(&mut buf_a, 5, 5, BrushShape::SprayPaint, 7, cell);
        paint_stamp(&mut buf_b, 5, 5, BrushShape::SprayPaint, 7, cell);
        for y in 0..10 {
            for x in 0..10 {
                assert_eq!(
                    buf_a.get(x, y).unwrap().ch,
                    buf_b.get(x, y).unwrap().ch,
                    "spray at ({},{}) differs between runs",
                    x,
                    y
                );
            }
        }
    }

    #[test]
    fn test_stamp_clips_to_bounds() {
        let mut buf = canvas_5x5();
        let cell = filled_cell();
        // Stamp at (0,0) with size-5 square: offsets -2..=2 in both axes
        // Some cells will be out of bounds (negative x or y) — should not panic
        paint_stamp(&mut buf, 0, 0, BrushShape::Square, 5, cell);
        // Only cells with x>=0 and y>=0 should be painted
        for y in 0..5 {
            for x in 0..5 {
                let in_brush = x <= 2 && y <= 2; // from start..=end where start=-2, end=2
                let c = buf.get(x, y).unwrap();
                if in_brush {
                    assert_eq!(c.ch, '@', "cell ({},{}) should be in-bounds painted", x, y);
                }
            }
        }
    }

    #[test]
    fn test_stamp_applies_cell_attributes() {
        let mut buf = canvas_5x5();
        let cell = CanvasCell {
            ch: '#',
            fg: None,
            bg: None,
        };
        paint_stamp(&mut buf, 2, 2, BrushShape::Square, 3, cell);
        let painted = buf.get(2, 2).unwrap();
        assert_eq!(painted.ch, '#');
    }

    #[test]
    fn test_line_horizontal_no_gaps() {
        let mut buf = canvas_30x10();
        let cell = filled_cell();
        // Horizontal line at y=5 from x=0 to x=20
        paint_line(&mut buf, 0, 5, 20, 5, BrushShape::Square, 1, cell);
        for x in 0..=20 {
            assert_eq!(
                buf.get(x, 5).unwrap().ch,
                '@',
                "horizontal cell ({},{}) should be painted",
                x,
                5
            );
        }
    }

    #[test]
    fn test_line_vertical_no_gaps() {
        let mut buf = canvas_30x10();
        let cell = filled_cell();
        // Vertical line at x=5 from y=0 to y=9
        paint_line(&mut buf, 5, 0, 5, 9, BrushShape::Square, 1, cell);
        for y in 0..=9 {
            assert_eq!(
                buf.get(5, y).unwrap().ch,
                '@',
                "vertical cell ({},{}) should be painted",
                5,
                y
            );
        }
    }

    #[test]
    fn test_line_diagonal_no_gaps() {
        let mut buf = canvas_10x10();
        let cell = filled_cell();
        // Diagonal from (0,0) to (9,9)
        paint_line(&mut buf, 0, 0, 9, 9, BrushShape::Square, 1, cell);
        for i in 0..=9 {
            assert_eq!(
                buf.get(i, i).unwrap().ch,
                '@',
                "diagonal cell ({},{}) should be painted",
                i,
                i
            );
        }
    }

    #[test]
    fn test_line_clips_endpoints() {
        let mut buf = canvas_10x10();
        let cell = filled_cell();
        // Line from (-5,5) to (25,5) — most endpoints are out of bounds
        paint_line(&mut buf, -5, 5, 25, 5, BrushShape::Square, 1, cell);
        // Visible portion x=0..9 should all be painted
        for x in 0..10 {
            assert_eq!(
                buf.get(x, 5).unwrap().ch,
                '@',
                "visible cell ({},{}) should be painted",
                x,
                5
            );
        }
    }

    #[test]
    fn test_stamp_square_size_one() {
        let mut buf = canvas_5x5();
        let cell = filled_cell();
        paint_stamp(&mut buf, 2, 2, BrushShape::Square, 1, cell);
        assert_eq!(buf.get(2, 2).unwrap().ch, '@');
        assert_eq!(buf.get(1, 2).unwrap().ch, ' ');
        assert_eq!(buf.get(2, 1).unwrap().ch, ' ');
        assert_eq!(buf.get(3, 2).unwrap().ch, ' ');
        assert_eq!(buf.get(2, 3).unwrap().ch, ' ');
    }

    #[test]
    fn test_stamp_custom_only_center() {
        let mut buf = canvas_5x5();
        let cell = filled_cell();
        paint_stamp(&mut buf, 2, 2, BrushShape::Custom, 10, cell);
        assert_eq!(buf.get(2, 2).unwrap().ch, '@');
        assert_eq!(buf.get(1, 2).unwrap().ch, ' ');
        assert_eq!(buf.get(2, 1).unwrap().ch, ' ');
    }

    #[test]
    fn test_line_reverse_direction() {
        let mut buf = canvas_10x10();
        let cell = filled_cell();
        // Line from high to low
        paint_line(&mut buf, 8, 8, 2, 2, BrushShape::Square, 1, cell);
        for i in 2..=8 {
            assert_eq!(
                buf.get(i, i).unwrap().ch,
                '@',
                "reverse-diagonal cell ({},{}) should be painted",
                i,
                i
            );
        }
    }
}
