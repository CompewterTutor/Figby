use crate::tui::canvas::CanvasBuffer;

/// Shift every cell in `buffer` by `(dx, dy)`.
///
/// Cells that would land outside the buffer are dropped; positions nothing
/// lands on become blank (`CanvasCell::default()`). Used by the Move tool
/// when there is no active selection, so the whole active layer is dragged
/// as a unit rather than just the selected region.
pub fn translate_buffer(buffer: &CanvasBuffer, dx: i16, dy: i16) -> CanvasBuffer {
    let w = buffer.width();
    let h = buffer.height();
    let mut result = CanvasBuffer::new(w, h);
    if dx == 0 && dy == 0 {
        return buffer.clone();
    }
    for y in 0..h {
        for x in 0..w {
            let nx = x as i16 + dx;
            let ny = y as i16 + dy;
            if nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h {
                continue;
            }
            if let Some(cell) = buffer.get(x, y) {
                result.set(nx as usize, ny as usize, *cell);
            }
        }
    }
    result
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
    fn test_translate_buffer_zero_offset_is_noop() {
        let mut buf = CanvasBuffer::new(3, 3);
        buf.set(1, 1, make_cell('X'));
        let result = translate_buffer(&buf, 0, 0);
        assert_eq!(result.get(1, 1).unwrap().ch, 'X');
    }

    #[test]
    fn test_translate_buffer_shifts_right_and_down() {
        let mut buf = CanvasBuffer::new(5, 5);
        buf.set(0, 0, make_cell('A'));
        let result = translate_buffer(&buf, 2, 3);
        assert_eq!(result.get(2, 3).unwrap().ch, 'A');
        assert_eq!(result.get(0, 0).unwrap().ch, ' ', "source cell left blank");
    }

    #[test]
    fn test_translate_buffer_shifts_negative() {
        let mut buf = CanvasBuffer::new(5, 5);
        buf.set(4, 4, make_cell('Z'));
        let result = translate_buffer(&buf, -2, -1);
        assert_eq!(result.get(2, 3).unwrap().ch, 'Z');
    }

    #[test]
    fn test_translate_buffer_drops_cells_pushed_off_edge() {
        let mut buf = CanvasBuffer::new(3, 3);
        buf.set(0, 0, make_cell('A'));
        buf.set(2, 2, make_cell('B'));
        let result = translate_buffer(&buf, -1, -1);
        // 'A' at (0,0) would land at (-1,-1): dropped.
        // 'B' at (2,2) lands at (1,1): kept.
        assert_eq!(result.get(1, 1).unwrap().ch, 'B');
        for y in 0..3 {
            for x in 0..3 {
                if (x, y) != (1, 1) {
                    assert_eq!(result.get(x, y).unwrap().ch, ' ');
                }
            }
        }
    }

    #[test]
    fn test_translate_buffer_preserves_dimensions() {
        let buf = CanvasBuffer::new(7, 4);
        let result = translate_buffer(&buf, 1, 1);
        assert_eq!(result.width(), 7);
        assert_eq!(result.height(), 4);
    }

    #[test]
    fn test_translate_buffer_large_offset_clears_buffer() {
        let mut buf = CanvasBuffer::new(3, 3);
        buf.set(1, 1, make_cell('X'));
        let result = translate_buffer(&buf, 100, 100);
        for y in 0..3 {
            for x in 0..3 {
                assert_eq!(result.get(x, y).unwrap().ch, ' ');
            }
        }
    }
}
