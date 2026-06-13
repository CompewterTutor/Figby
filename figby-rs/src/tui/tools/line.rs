use crate::tui::brush::BrushShape;
use crate::tui::canvas::{CanvasBuffer, CanvasCell};
use crate::tui::tools::brush::paint_line;

#[allow(clippy::too_many_arguments)]
pub fn draw_line_segment(
    buffer: &mut CanvasBuffer,
    x0: i16,
    y0: i16,
    x1: i16,
    y1: i16,
    shape: BrushShape,
    size: u8,
    cell: CanvasCell,
) {
    paint_line(buffer, x0, y0, x1, y1, shape, size, cell);
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

    fn canvas_10x10() -> CanvasBuffer {
        CanvasBuffer::new(10, 10)
    }

    fn canvas_30x10() -> CanvasBuffer {
        CanvasBuffer::new(30, 10)
    }

    #[test]
    fn test_line_horizontal() {
        let mut buf = canvas_30x10();
        let cell = filled_cell();
        draw_line_segment(&mut buf, 0, 5, 20, 5, BrushShape::Square, 1, cell);
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
    fn test_line_vertical() {
        let mut buf = canvas_30x10();
        let cell = filled_cell();
        draw_line_segment(&mut buf, 5, 0, 5, 9, BrushShape::Square, 1, cell);
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
    fn test_line_diagonal() {
        let mut buf = canvas_10x10();
        let cell = filled_cell();
        draw_line_segment(&mut buf, 0, 0, 9, 9, BrushShape::Square, 1, cell);
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
    fn test_line_reverse_direction() {
        let mut buf = canvas_10x10();
        let cell = filled_cell();
        draw_line_segment(&mut buf, 8, 8, 2, 2, BrushShape::Square, 1, cell);
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

    #[test]
    fn test_line_clips_endpoints() {
        let mut buf = canvas_10x10();
        let cell = filled_cell();
        draw_line_segment(&mut buf, -5, 5, 25, 5, BrushShape::Square, 1, cell);
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
}
