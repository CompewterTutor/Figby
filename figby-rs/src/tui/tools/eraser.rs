use crate::tui::brush::BrushShape;
use crate::tui::canvas::CanvasBuffer;
use crate::tui::tools::brush::stamp_offsets;

pub fn erase_stamp(buffer: &mut CanvasBuffer, cx: i16, cy: i16, shape: BrushShape, size: u8) {
    for (dx, dy) in stamp_offsets(shape, size) {
        let x = cx.wrapping_add(dx);
        let y = cy.wrapping_add(dy);
        if x >= 0 && y >= 0 {
            if let Some(c) = buffer.get_mut(x as usize, y as usize) {
                *c = Default::default();
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn erase_line(
    buffer: &mut CanvasBuffer,
    x0: i16,
    y0: i16,
    x1: i16,
    y1: i16,
    shape: BrushShape,
    size: u8,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;
    loop {
        erase_stamp(buffer, x, y, shape, size);
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

    fn canvas_5x5() -> CanvasBuffer {
        CanvasBuffer::new(5, 5)
    }

    fn canvas_10x10() -> CanvasBuffer {
        CanvasBuffer::new(10, 10)
    }

    fn canvas_30x10() -> CanvasBuffer {
        CanvasBuffer::new(30, 10)
    }

    fn filled_cell() -> crate::tui::canvas::CanvasCell {
        crate::tui::canvas::CanvasCell {
            ch: '@',
            fg: None,
            bg: None,
        }
    }

    fn fill_all(buf: &mut CanvasBuffer, cell: crate::tui::canvas::CanvasCell) {
        for y in 0..buf.height() {
            for x in 0..buf.width() {
                buf.set(x, y, cell);
            }
        }
    }

    #[test]
    fn test_erase_square_clears_correct_cells() {
        let mut buf = canvas_5x5();
        fill_all(&mut buf, filled_cell());
        // Size-3 square at (2,2): covers 3x3 block (1..=3, 1..=3)
        erase_stamp(&mut buf, 2, 2, BrushShape::Square, 3);
        for y in 0..5 {
            for x in 0..5 {
                let in_brush = (1..=3).contains(&x) && (1..=3).contains(&y);
                let c = buf.get(x, y).unwrap();
                if in_brush {
                    assert_eq!(c.ch, ' ', "cell ({},{}) should be erased", x, y);
                } else {
                    assert_eq!(c.ch, '@', "cell ({},{}) should remain painted", x, y);
                }
            }
        }
    }

    #[test]
    fn test_erase_circle_round_shape() {
        let mut buf = canvas_10x10();
        fill_all(&mut buf, filled_cell());
        // Size-5 circle at (5,5): radius 2.5
        erase_stamp(&mut buf, 5, 5, BrushShape::Circle, 5);
        // Center cell always erased
        assert_eq!(buf.get(5, 5).unwrap().ch, ' ');
        let erased = (0..10)
            .flat_map(|y| (0..10).map(move |x| (x, y)))
            .filter(|&(x, y)| buf.get(x, y).unwrap().ch == ' ')
            .count();
        let remaining = (0..10)
            .flat_map(|y| (0..10).map(move |x| (x, y)))
            .filter(|&(x, y)| buf.get(x, y).unwrap().ch == '@')
            .count();
        assert!(erased > 0, "circle should erase some cells");
        assert!(remaining > 0, "circle should leave some cells untouched");
    }

    #[test]
    fn test_erase_spray_deterministic() {
        let mut buf_a = canvas_10x10();
        let mut buf_b = canvas_10x10();
        fill_all(&mut buf_a, filled_cell());
        fill_all(&mut buf_b, filled_cell());
        erase_stamp(&mut buf_a, 5, 5, BrushShape::SprayPaint, 7);
        erase_stamp(&mut buf_b, 5, 5, BrushShape::SprayPaint, 7);
        for y in 0..10 {
            for x in 0..10 {
                assert_eq!(
                    buf_a.get(x, y).unwrap().ch,
                    buf_b.get(x, y).unwrap().ch,
                    "spray erase at ({},{}) differs between runs",
                    x,
                    y
                );
            }
        }
    }

    #[test]
    fn test_erase_clips_to_bounds() {
        let mut buf = canvas_5x5();
        fill_all(&mut buf, filled_cell());
        // Stamp at (0,0) with size-5 square: offsets -2..=2 in both axes
        // Some cells will be out of bounds (negative x or y) — should not panic
        erase_stamp(&mut buf, 0, 0, BrushShape::Square, 5);
        // Only cells with x>=0 and y>=0 should be erased
        for y in 0..5 {
            for x in 0..5 {
                let in_brush = x <= 2 && y <= 2;
                let c = buf.get(x, y).unwrap();
                if in_brush {
                    assert_eq!(c.ch, ' ', "cell ({},{}) should be erased", x, y);
                }
            }
        }
    }

    #[test]
    fn test_erase_line_horizontal_no_gaps() {
        let mut buf = canvas_30x10();
        fill_all(&mut buf, filled_cell());
        // Horizontal line at y=5 from x=0 to x=20
        erase_line(&mut buf, 0, 5, 20, 5, BrushShape::Square, 1);
        for x in 0..=20 {
            assert_eq!(
                buf.get(x, 5).unwrap().ch,
                ' ',
                "horizontal cell ({},{}) should be erased",
                x,
                5
            );
        }
    }

    #[test]
    fn test_erase_line_vertical_no_gaps() {
        let mut buf = canvas_30x10();
        fill_all(&mut buf, filled_cell());
        // Vertical line at x=5 from y=0 to y=9
        erase_line(&mut buf, 5, 0, 5, 9, BrushShape::Square, 1);
        for y in 0..=9 {
            assert_eq!(
                buf.get(5, y).unwrap().ch,
                ' ',
                "vertical cell ({},{}) should be erased",
                5,
                y
            );
        }
    }

    #[test]
    fn test_erase_line_diagonal_no_gaps() {
        let mut buf = canvas_10x10();
        fill_all(&mut buf, filled_cell());
        // Diagonal from (0,0) to (9,9)
        erase_line(&mut buf, 0, 0, 9, 9, BrushShape::Square, 1);
        for i in 0..=9 {
            assert_eq!(
                buf.get(i, i).unwrap().ch,
                ' ',
                "diagonal cell ({},{}) should be erased",
                i,
                i
            );
        }
    }

    #[test]
    fn test_erase_line_reverse_direction() {
        let mut buf = canvas_10x10();
        fill_all(&mut buf, filled_cell());
        // Line from (8,8) to (2,2)
        erase_line(&mut buf, 8, 8, 2, 2, BrushShape::Square, 1);
        for i in 2..=8 {
            assert_eq!(
                buf.get(i, i).unwrap().ch,
                ' ',
                "reverse-diagonal cell ({},{}) should be erased",
                i,
                i
            );
        }
    }
}
