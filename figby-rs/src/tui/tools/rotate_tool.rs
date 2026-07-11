use crate::tui::canvas::CanvasBuffer;

/// Rotate the cells within `bounds` (`x_min, y_min, x_max, y_max`, inclusive)
/// 90° around the bounds' own center.
///
/// Buffer dimensions never change — unlike a "rotate canvas" operation that
/// would swap width/height, this keeps a fixed-size layer buffer consistent
/// with the rest of the layer stack. Cells that land outside the buffer
/// after rotating are dropped, mirroring `move_tool::translate_buffer`.
/// Uses the same `(dx, dy) -> (-dy, dx)` (clockwise) point transform that
/// `Selection::rotate_90` applies to its mask, so selection content and its
/// mask stay in sync when rotated together.
pub fn rotate_region(
    buffer: &CanvasBuffer,
    bounds: (usize, usize, usize, usize),
    clockwise: bool,
) -> CanvasBuffer {
    let (x_min, y_min, x_max, y_max) = bounds;
    let w = buffer.width();
    let h = buffer.height();
    let mut result = buffer.clone();
    if w == 0 || h == 0 || x_min > x_max || y_min > y_max {
        return result;
    }
    let x_max = x_max.min(w - 1);
    let y_max = y_max.min(h - 1);
    if x_min > x_max || y_min > y_max {
        return result;
    }

    // Clear the source region first — rotated content is written back below.
    for y in y_min..=y_max {
        for x in x_min..=x_max {
            result.set(x, y, Default::default());
        }
    }

    let cx = (x_min + x_max) as f64 / 2.0;
    let cy = (y_min + y_max) as f64 / 2.0;
    for y in y_min..=y_max {
        for x in x_min..=x_max {
            let Some(cell) = buffer.get(x, y) else {
                continue;
            };
            if cell.ch == ' ' && cell.fg.is_none() && cell.bg.is_none() {
                continue;
            }
            let dx = x as f64 - cx;
            let dy = y as f64 - cy;
            let (rdx, rdy) = if clockwise { (-dy, dx) } else { (dy, -dx) };
            let nx = (cx + rdx).round();
            let ny = (cy + rdy).round();
            if nx < 0.0 || ny < 0.0 {
                continue;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            if nx < w && ny < h {
                result.set(nx, ny, *cell);
            }
        }
    }
    result
}

/// Rotate the whole buffer 90° around its own center. Convenience wrapper
/// around [`rotate_region`] for the "no selection" case (Move/Rotate tools
/// act on the whole active layer when nothing is selected).
pub fn rotate_whole_buffer(buffer: &CanvasBuffer, clockwise: bool) -> CanvasBuffer {
    let w = buffer.width();
    let h = buffer.height();
    if w == 0 || h == 0 {
        return buffer.clone();
    }
    rotate_region(buffer, (0, 0, w - 1, h - 1), clockwise)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::canvas::CanvasCell;

    fn make_cell(ch: char) -> CanvasCell {
        CanvasCell {
            ch,
            fg: None,
            bg: None,
            height: None,
        }
    }

    #[test]
    fn test_rotate_whole_buffer_clockwise_square() {
        // 3x3, mark top-middle; clockwise should move it to right-middle.
        let mut buf = CanvasBuffer::new(3, 3);
        buf.set(1, 0, make_cell('X'));
        let rotated = rotate_whole_buffer(&buf, true);
        assert_eq!(rotated.get(2, 1).unwrap().ch, 'X');
        assert_eq!(rotated.get(1, 0).unwrap().ch, ' ', "source cell cleared");
    }

    #[test]
    fn test_rotate_whole_buffer_counterclockwise_square() {
        // 3x3, mark top-middle; counter-clockwise should move it to left-middle.
        let mut buf = CanvasBuffer::new(3, 3);
        buf.set(1, 0, make_cell('X'));
        let rotated = rotate_whole_buffer(&buf, false);
        assert_eq!(rotated.get(0, 1).unwrap().ch, 'X');
    }

    #[test]
    fn test_rotate_four_times_clockwise_returns_to_original() {
        let mut buf = CanvasBuffer::new(5, 5);
        buf.set(1, 1, make_cell('A'));
        buf.set(3, 1, make_cell('B'));
        let mut rotated = buf.clone();
        for _ in 0..4 {
            rotated = rotate_whole_buffer(&rotated, true);
        }
        assert_eq!(rotated.get(1, 1).unwrap().ch, 'A');
        assert_eq!(rotated.get(3, 1).unwrap().ch, 'B');
    }

    #[test]
    fn test_rotate_180_is_point_reflection() {
        let mut buf = CanvasBuffer::new(4, 4);
        buf.set(0, 0, make_cell('A'));
        let mut rotated = buf.clone();
        for _ in 0..2 {
            rotated = rotate_whole_buffer(&rotated, true);
        }
        assert_eq!(rotated.get(3, 3).unwrap().ch, 'A');
    }

    #[test]
    fn test_rotate_region_leaves_outside_untouched() {
        let mut buf = CanvasBuffer::new(6, 6);
        buf.set(1, 1, make_cell('X')); // inside a 0..=2,0..=2 region
        buf.set(5, 5, make_cell('Y')); // outside the region
        let rotated = rotate_region(&buf, (0, 0, 2, 2), true);
        assert_eq!(
            rotated.get(5, 5).unwrap().ch,
            'Y',
            "untouched outside region"
        );
    }

    #[test]
    fn test_rotate_region_drops_cells_pushed_out_of_buffer() {
        // A 3x1 horizontal strip at the buffer's top edge: rotating it 90°
        // needs a 1x3 vertical footprint, which pushes above y=0 — that
        // part must be dropped, not panic or wrap.
        let mut buf = CanvasBuffer::new(3, 3);
        buf.set(0, 0, make_cell('X'));
        let rotated = rotate_region(&buf, (0, 0, 2, 0), true);
        assert_eq!(rotated.width(), 3);
        assert_eq!(rotated.height(), 3);
        for y in 0..3 {
            for x in 0..3 {
                assert_eq!(
                    rotated.get(x, y).unwrap().ch,
                    ' ',
                    "cell rotated out of bounds should vanish, not reappear at ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn test_rotate_preserves_buffer_dimensions_for_nonsquare() {
        let buf = CanvasBuffer::new(8, 3);
        let rotated = rotate_whole_buffer(&buf, true);
        assert_eq!(rotated.width(), 8);
        assert_eq!(rotated.height(), 3);
    }

    #[test]
    fn test_rotate_empty_buffer_no_panic() {
        let buf = CanvasBuffer::new(0, 0);
        let rotated = rotate_whole_buffer(&buf, true);
        assert_eq!(rotated.width(), 0);
    }

    #[test]
    fn test_rotate_preserves_cell_color() {
        use ratatui::style::Color;
        let mut buf = CanvasBuffer::new(3, 3);
        let cell = CanvasCell {
            ch: 'X',
            fg: Some(Color::Rgb(10, 20, 30)),
            bg: None,
            height: None,
        };
        buf.set(1, 0, cell);
        let rotated = rotate_whole_buffer(&buf, true);
        assert_eq!(rotated.get(2, 1).unwrap().fg, Some(Color::Rgb(10, 20, 30)));
    }
}
