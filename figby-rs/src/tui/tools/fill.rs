use crate::tui::canvas::{CanvasBuffer, CanvasCell};

pub fn flood_fill(buffer: &mut CanvasBuffer, x: i16, y: i16, replacement: CanvasCell) {
    if x < 0 || y < 0 {
        return;
    }
    let ux = x as usize;
    let uy = y as usize;

    let Some(target) = buffer.get(ux, uy).copied() else {
        return;
    };

    if replacement.ch == target.ch {
        return;
    }

    let w = buffer.width();
    let h = buffer.height();

    let mut stack = vec![(ux, uy)];

    while let Some((cx, cy)) = stack.pop() {
        let Some(cell) = buffer.get_mut(cx, cy) else {
            continue;
        };
        if cell.ch != target.ch {
            continue;
        }
        *cell = replacement;

        if cy > 0 {
            stack.push((cx, cy - 1));
        }
        if cy + 1 < h {
            stack.push((cx, cy + 1));
        }
        if cx > 0 {
            stack.push((cx - 1, cy));
        }
        if cx + 1 < w {
            stack.push((cx + 1, cy));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas_5x5() -> CanvasBuffer {
        CanvasBuffer::new(5, 5)
    }

    fn canvas_7x7() -> CanvasBuffer {
        CanvasBuffer::new(7, 7)
    }

    fn cell_with(ch: char) -> CanvasCell {
        CanvasCell {
            ch,
            fg: None,
            bg: None,
        }
    }

    fn cell_with_color(ch: char, r: u8, g: u8, b: u8) -> CanvasCell {
        CanvasCell {
            ch,
            fg: Some(ratatui::style::Color::Rgb(r, g, b)),
            bg: None,
        }
    }

    fn paint_region(buf: &mut CanvasBuffer, x1: usize, y1: usize, x2: usize, y2: usize, ch: char) {
        for y in y1..=y2 {
            for x in x1..=x2 {
                buf.set(x, y, cell_with(ch));
            }
        }
    }

    #[test]
    fn test_fill_small_region() {
        let mut buf = canvas_5x5();
        paint_region(&mut buf, 1, 1, 3, 3, '@');

        flood_fill(&mut buf, 2, 2, cell_with('#'));

        for y in 0..5 {
            for x in 0..5 {
                let is_inside = (1..=3).contains(&x) && (1..=3).contains(&y);
                let expected = if is_inside { '#' } else { ' ' };
                assert_eq!(
                    buf.get(x, y).unwrap().ch,
                    expected,
                    "cell ({},{}) mismatch",
                    x,
                    y
                );
            }
        }
    }

    #[test]
    fn test_fill_bounded_region() {
        let mut buf = canvas_7x7();

        // Border of X around region of @
        for y in 1..=5 {
            for x in 1..=5 {
                let ch = if x == 1 || x == 5 || y == 1 || y == 5 {
                    'X'
                } else {
                    '@'
                };
                buf.set(x, y, cell_with(ch));
            }
        }

        flood_fill(&mut buf, 3, 3, cell_with('#'));

        for y in 0..7 {
            for x in 0..7 {
                let cell = buf.get(x, y).unwrap();
                let is_interior = (2..=4).contains(&x) && (2..=4).contains(&y);
                let is_border = (x == 1 || x == 5 || y == 1 || y == 5)
                    && (1..=5).contains(&x)
                    && (1..=5).contains(&y);
                if is_interior {
                    assert_eq!(cell.ch, '#', "interior cell ({},{}) should be #", x, y);
                } else if is_border {
                    assert_eq!(cell.ch, 'X', "border cell ({},{}) should be X", x, y);
                } else {
                    assert_eq!(cell.ch, ' ', "outer cell ({},{}) should be space", x, y);
                }
            }
        }
    }

    #[test]
    fn test_fill_unbounded_to_edge() {
        let mut buf = canvas_5x5();
        // Fill a block touching the top edge
        paint_region(&mut buf, 0, 0, 4, 2, 'A');

        flood_fill(&mut buf, 2, 1, cell_with('B'));

        for y in 0..5 {
            for x in 0..5 {
                let in_region = y <= 2;
                let expected = if in_region { 'B' } else { ' ' };
                assert_eq!(
                    buf.get(x, y).unwrap().ch,
                    expected,
                    "cell ({},{}) mismatch",
                    x,
                    y
                );
            }
        }
    }

    #[test]
    fn test_fill_single_cell() {
        let mut buf = canvas_5x5();
        buf.set(2, 2, cell_with('@'));

        flood_fill(&mut buf, 2, 2, cell_with('#'));

        assert_eq!(buf.get(2, 2).unwrap().ch, '#');
        let filled = (0..5)
            .flat_map(|y| (0..5).map(move |x| (x, y)))
            .filter(|&(x, y)| buf.get(x, y).unwrap().ch != ' ')
            .count();
        assert_eq!(filled, 1, "only one cell should be filled");
    }

    #[test]
    fn test_fill_no_match() {
        let mut buf = canvas_5x5();
        buf.set(2, 2, cell_with('@'));

        // Replacement char matches target char — no-op
        flood_fill(&mut buf, 2, 2, cell_with('@'));

        assert_eq!(buf.get(2, 2).unwrap().ch, '@');
        let total = (0..5)
            .flat_map(|y| (0..5).map(move |x| (x, y)))
            .filter(|&(x, y)| buf.get(x, y).unwrap().ch != ' ')
            .count();
        assert_eq!(total, 1, "no new cells should be filled");
    }

    #[test]
    fn test_fill_out_of_bounds() {
        let mut buf = canvas_5x5();
        paint_region(&mut buf, 0, 0, 4, 4, '@');

        // Negative coordinates should not panic and not change anything
        flood_fill(&mut buf, -1, -1, cell_with('#'));

        for y in 0..5 {
            for x in 0..5 {
                assert_eq!(
                    buf.get(x, y).unwrap().ch,
                    '@',
                    "cell ({},{}) should be unchanged",
                    x,
                    y
                );
            }
        }
    }

    #[test]
    fn test_fill_does_not_cross_boundary() {
        let mut buf = canvas_7x7();

        // Left region: @ at columns 1-2, rows 1-4
        paint_region(&mut buf, 1, 1, 2, 4, '@');
        // Wall: X at column 3, rows 1-4
        for row in 1..=4 {
            buf.set(3, row, cell_with('X'));
        }
        // Right region: @ at columns 4-5, rows 1-4
        paint_region(&mut buf, 4, 1, 5, 4, '@');

        // Fill left region only
        flood_fill(&mut buf, 1, 2, cell_with('#'));

        for y in 0..7 {
            for x in 0..7 {
                let cell = buf.get(x, y).unwrap();
                let in_left = (1..=2).contains(&x) && (1..=4).contains(&y);
                let in_wall = x == 3 && (1..=4).contains(&y);
                let in_right = (4..=5).contains(&x) && (1..=4).contains(&y);
                if in_left {
                    assert_eq!(cell.ch, '#', "left region cell ({},{}) should be #", x, y);
                } else if in_wall {
                    assert_eq!(cell.ch, 'X', "wall cell ({},{}) should be X", x, y);
                } else if in_right {
                    assert_eq!(cell.ch, '@', "right region cell ({},{}) should be @", x, y);
                } else {
                    assert_eq!(cell.ch, ' ', "empty cell ({},{}) should be space", x, y);
                }
            }
        }
    }

    #[test]
    fn test_fill_empty_region() {
        let mut buf = canvas_5x5();
        // Paint border with @ to bound the flood fill
        for y in 0..5 {
            for x in 0..5 {
                if x == 0 || x == 4 || y == 0 || y == 4 {
                    buf.set(x, y, cell_with('@'));
                }
            }
        }
        // Fill spaces in the center 3x3 block with '#'
        paint_region(&mut buf, 1, 1, 3, 3, ' ');

        flood_fill(&mut buf, 2, 2, cell_with('#'));

        for y in 0..5 {
            for x in 0..5 {
                let in_region = (1..=3).contains(&x) && (1..=3).contains(&y);
                let expected = if in_region { '#' } else { '@' };
                assert_eq!(
                    buf.get(x, y).unwrap().ch,
                    expected,
                    "cell ({},{}) mismatch",
                    x,
                    y
                );
            }
        }
    }

    #[test]
    fn test_fill_orthogonal_not_diagonal() {
        // 3x3: center cell (1,1) and corners (0,0),(2,0),(0,2),(2,2) are @
        // Arms (1,0),(0,1),(2,1),(1,2) remain space — no orthogonal path
        // from center to corners
        //   @ . @
        //   . @ .
        //   @ . @
        let mut buf = CanvasBuffer::new(3, 3);
        buf.set(1, 1, cell_with('@'));
        buf.set(0, 0, cell_with('@'));
        buf.set(2, 0, cell_with('@'));
        buf.set(0, 2, cell_with('@'));
        buf.set(2, 2, cell_with('@'));

        // Fill at center — only center should change, corners remain @
        flood_fill(&mut buf, 1, 1, cell_with('#'));

        // Center cell should be #
        assert_eq!(buf.get(1, 1).unwrap().ch, '#');

        // Diagonal corners should remain @ — only diagonal to center
        assert_eq!(buf.get(0, 0).unwrap().ch, '@');
        assert_eq!(buf.get(2, 0).unwrap().ch, '@');
        assert_eq!(buf.get(0, 2).unwrap().ch, '@');
        assert_eq!(buf.get(2, 2).unwrap().ch, '@');
    }

    #[test]
    fn test_fill_preserves_fg_bg() {
        let mut buf = canvas_5x5();
        paint_region(&mut buf, 1, 1, 3, 3, '@');

        let replacement = cell_with_color('#', 255, 0, 0);
        flood_fill(&mut buf, 2, 2, replacement);

        for y in 1..=3 {
            for x in 1..=3 {
                let cell = buf.get(x, y).unwrap();
                assert_eq!(cell.ch, '#', "cell ({},{}) should be #", x, y);
                assert_eq!(cell.fg, Some(ratatui::style::Color::Rgb(255, 0, 0)));
            }
        }

        // Border should remain unchanged
        assert_eq!(buf.get(0, 0).unwrap().ch, ' ');
        assert_eq!(buf.get(4, 4).unwrap().ch, ' ');
    }
}
