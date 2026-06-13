use crate::tui::canvas::{CanvasBuffer, CanvasCell};

pub struct Selection {
    mask: Vec<Vec<bool>>,
    width: usize,
    height: usize,
    bounds: (usize, usize, usize, usize),
}

pub type Clipboard = Vec<Vec<Option<CanvasCell>>>;

impl Selection {
    pub fn new(width: usize, height: usize) -> Self {
        let mask = vec![vec![false; width]; height];
        Self {
            mask,
            width,
            height,
            bounds: (width, height, 0, 0),
        }
    }

    pub fn marquee(buffer: &CanvasBuffer, x1: i16, y1: i16, x2: i16, y2: i16) -> Self {
        let bw = buffer.width();
        let bh = buffer.height();
        let mut sel = Self::new(bw, bh);
        let x_min = x1.min(x2).max(0) as usize;
        let x_max = (x1.max(x2) as usize).min(bw.saturating_sub(1));
        let y_min = y1.min(y2).max(0) as usize;
        let y_max = (y1.max(y2) as usize).min(bh.saturating_sub(1));
        for y in y_min..=y_max {
            for x in x_min..=x_max {
                sel.mask[y][x] = true;
            }
        }
        sel.bounds = (x_min, y_min, x_max, y_max);
        sel
    }

    pub fn circle(buffer: &CanvasBuffer, cx: i16, cy: i16, r: i16) -> Self {
        let bw = buffer.width();
        let bh = buffer.height();
        let mut sel = Self::new(bw, bh);
        if r <= 0 {
            if cx >= 0 && cx < bw as i16 && cy >= 0 && cy < bh as i16 {
                sel.mask[cy as usize][cx as usize] = true;
                sel.bounds = (cx as usize, cy as usize, cx as usize, cy as usize);
            }
            return sel;
        }
        let r2 = r * r;
        let y_min = (cy - r).max(0) as usize;
        let y_max = (cy + r).min(bh as i16 - 1) as usize;
        let mut bx_min = bw;
        let mut bx_max = 0usize;
        for y in y_min..=y_max {
            let dy = (y as i16 - cy).abs();
            let hw = ((r2 - dy * dy) as f64).sqrt() as i16;
            let x_left = (cx - hw).max(0) as usize;
            let x_right = (cx + hw).min(bw as i16 - 1) as usize;
            if x_left <= x_right {
                bx_min = bx_min.min(x_left);
                bx_max = bx_max.max(x_right);
                for x in x_left..=x_right {
                    sel.mask[y][x] = true;
                }
            }
        }
        sel.bounds = (bx_min.min(bw), y_min, bx_max, y_max);
        sel
    }

    pub fn polygon(buffer: &CanvasBuffer, vertices: &[(i16, i16)]) -> Self {
        let bw = buffer.width();
        let bh = buffer.height();
        let mut sel = Self::new(bw, bh);
        if vertices.len() < 3 {
            return sel;
        }
        let y_min = vertices.iter().map(|(_, y)| *y).min().unwrap_or(0).max(0) as usize;
        let y_max = vertices
            .iter()
            .map(|(_, y)| *y)
            .max()
            .unwrap_or(0)
            .min(bh as i16 - 1) as usize;
        let mut bx_min = bw;
        let mut bx_max = 0usize;
        let v: Vec<(f64, f64)> = vertices
            .iter()
            .map(|(x, y)| (*x as f64, *y as f64))
            .collect();
        let len = v.len();
        for y in y_min..=y_max {
            let mut intersections: Vec<f64> = Vec::new();
            let yy = y as f64;
            for i in 0..len {
                let (x1, y1) = v[i];
                let (x2, y2) = v[(i + 1) % len];
                if (y1 <= yy && y2 > yy) || (y2 <= yy && y1 > yy) {
                    let t = (yy - y1) / (y2 - y1);
                    intersections.push(x1 + t * (x2 - x1));
                }
            }
            intersections.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            for chunk in intersections.chunks(2) {
                if chunk.len() == 2 {
                    let x_start = (chunk[0].ceil() as i16).max(0) as usize;
                    let x_end = (chunk[1].floor() as i16).min(bw as i16 - 1) as usize;
                    if x_start <= x_end {
                        bx_min = bx_min.min(x_start);
                        bx_max = bx_max.max(x_end);
                        for x in x_start..=x_end {
                            sel.mask[y][x] = true;
                        }
                    }
                }
            }
        }
        sel.bounds = (bx_min.min(bw), y_min, bx_max, y_max);
        sel
    }

    pub fn lasso(buffer: &CanvasBuffer, points: &[(i16, i16)]) -> Self {
        if points.len() < 3 {
            return Self::new(buffer.width(), buffer.height());
        }
        Self::polygon(buffer, points)
    }

    pub fn is_selected(&self, x: usize, y: usize) -> bool {
        if x < self.width && y < self.height {
            self.mask[y][x]
        } else {
            false
        }
    }

    pub fn bounds(&self) -> (usize, usize, usize, usize) {
        self.bounds
    }

    pub fn is_active(&self) -> bool {
        self.bounds.0 <= self.bounds.2
            && self.bounds.1 <= self.bounds.3
            && self.bounds.0 < self.width
            && self.bounds.1 < self.height
    }

    pub fn copy_from(&self, buffer: &CanvasBuffer) -> Clipboard {
        let (x_min, y_min, x_max, y_max) = self.bounds;
        let cw = x_max.saturating_sub(x_min).saturating_add(1);
        let ch = y_max.saturating_sub(y_min).saturating_add(1);
        let mut clip = vec![vec![None; cw]; ch];
        for (dy, row) in clip.iter_mut().enumerate() {
            let by = y_min + dy;
            for (dx, slot) in row.iter_mut().enumerate() {
                let bx = x_min + dx;
                if self.mask[by][bx] {
                    if let Some(cell) = buffer.get(bx, by) {
                        *slot = Some(*cell);
                    }
                }
            }
        }
        clip
    }

    pub fn cut_from(&self, buffer: &mut CanvasBuffer) -> Clipboard {
        let clip = self.copy_from(buffer);
        self.delete_from(buffer);
        clip
    }

    pub fn delete_from(&self, buffer: &mut CanvasBuffer) {
        let empty = CanvasCell::default();
        for y in 0..self.height {
            for x in 0..self.width {
                if self.mask[y][x] {
                    buffer.set(x, y, empty);
                }
            }
        }
    }

    pub fn paste_into(buffer: &mut CanvasBuffer, clipboard: &Clipboard, dx: i16, dy: i16) {
        for (cy, row) in clipboard.iter().enumerate() {
            for (cx, slot) in row.iter().enumerate() {
                if let Some(cell) = slot {
                    let bx = dx + cx as i16;
                    let by = dy + cy as i16;
                    if bx >= 0 && by >= 0 {
                        buffer.set(bx as usize, by as usize, *cell);
                    }
                }
            }
        }
    }

    pub fn move_selection(&mut self, buffer: &mut CanvasBuffer, dx: i16, dy: i16) {
        let clip = self.cut_from(buffer);
        let (n_min_x, n_min_y, _, _) = self.translate_bounds(dx, dy);
        Self::paste_into(buffer, &clip, n_min_x as i16, n_min_y as i16);
        *self = self.translate_mask(dx, dy);
    }

    fn translate_mask(&self, dx: i16, dy: i16) -> Self {
        let mut new = Self::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                if self.mask[y][x] {
                    let nx = x as i16 + dx;
                    let ny = y as i16 + dy;
                    if nx >= 0 && nx < self.width as i16 && ny >= 0 && ny < self.height as i16 {
                        new.mask[ny as usize][nx as usize] = true;
                    }
                }
            }
        }
        new.recompute_bounds();
        new
    }

    fn translate_bounds(&self, dx: i16, dy: i16) -> (usize, usize, usize, usize) {
        let (x_min, y_min, x_max, y_max) = self.bounds;
        (
            (x_min as i16 + dx).max(0) as usize,
            (y_min as i16 + dy).max(0) as usize,
            (x_max as i16 + dx).min(self.width as i16 - 1).max(0) as usize,
            (y_max as i16 + dy).min(self.height as i16 - 1).max(0) as usize,
        )
    }

    fn recompute_bounds(&mut self) {
        let mut min_x = self.width;
        let mut min_y = self.height;
        let mut max_x = 0usize;
        let mut max_y = 0usize;
        let mut found = false;
        for y in 0..self.height {
            for x in 0..self.width {
                if self.mask[y][x] {
                    found = true;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }
        if found {
            self.bounds = (min_x, min_y, max_x, max_y);
        } else {
            self.bounds = (self.width, self.height, 0, 0);
        }
    }

    pub fn perimeter(&self) -> Vec<(usize, usize)> {
        let mut cells = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if !self.mask[y][x] {
                    continue;
                }
                let is_perimeter = (x == 0 || !self.mask[y][x.saturating_sub(1)])
                    || (x + 1 >= self.width || !self.mask[y][x + 1])
                    || (y == 0 || !self.mask[y.saturating_sub(1)][x])
                    || (y + 1 >= self.height || !self.mask[y + 1][x]);
                if is_perimeter {
                    cells.push((x, y));
                }
            }
        }
        cells
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas_10x10() -> CanvasBuffer {
        CanvasBuffer::new(10, 10)
    }

    fn canvas_20x20() -> CanvasBuffer {
        CanvasBuffer::new(20, 20)
    }

    fn cell_with(ch: char) -> CanvasCell {
        CanvasCell {
            ch,
            fg: None,
            bg: None,
        }
    }

    fn fill_all(buf: &mut CanvasBuffer, ch: char) {
        let cell = cell_with(ch);
        for y in 0..buf.height() {
            for x in 0..buf.width() {
                buf.set(x, y, cell);
            }
        }
    }

    #[test]
    fn test_selection_marquee_mask() {
        let buf = canvas_10x10();
        let sel = Selection::marquee(&buf, 1, 1, 4, 4);
        assert!(sel.is_active());
        for y in 0..10 {
            for x in 0..10 {
                let expected = (1..=4).contains(&x) && (1..=4).contains(&y);
                assert_eq!(sel.is_selected(x, y), expected, "cell ({},{})", x, y);
            }
        }
    }

    #[test]
    fn test_selection_marquee_reversed_coords() {
        let buf = canvas_10x10();
        let sel = Selection::marquee(&buf, 4, 4, 1, 1);
        for y in 0..10 {
            for x in 0..10 {
                let expected = (1..=4).contains(&x) && (1..=4).contains(&y);
                assert_eq!(sel.is_selected(x, y), expected, "cell ({},{})", x, y);
            }
        }
    }

    #[test]
    fn test_selection_circle_mask() {
        let buf = canvas_10x10();
        let sel = Selection::circle(&buf, 5, 5, 3);
        assert!(sel.is_active());
        assert!(sel.is_selected(5, 5), "center should be selected");
        assert!(!sel.is_selected(0, 0), "corner should not be selected");
        assert!(!sel.is_selected(9, 9), "corner should not be selected");
        assert!(!sel.is_selected(0, 5), "far left should not be selected");
    }

    #[test]
    fn test_selection_circle_radius_zero() {
        let buf = canvas_10x10();
        let sel = Selection::circle(&buf, 5, 5, 0);
        assert!(sel.is_selected(5, 5));
        assert!(!sel.is_selected(4, 5));
    }

    #[test]
    fn test_selection_polygon_mask() {
        let buf = canvas_10x10();
        // Triangle with vertices (2,2), (8,2), (5,8) on 10x10
        let verts = [(2, 2), (8, 2), (5, 8)];
        let sel = Selection::polygon(&buf, &verts);
        assert!(sel.is_active());
        // Center of triangle should be selected
        assert!(sel.is_selected(5, 4));
        // Below the triangle (y=9) should not be selected
        assert!(!sel.is_selected(5, 9));
        // Outside left
        assert!(!sel.is_selected(1, 4));
    }

    #[test]
    fn test_selection_polygon_too_few_vertices() {
        let buf = canvas_10x10();
        let sel = Selection::polygon(&buf, &[(1, 1), (5, 5)]);
        assert!(!sel.is_active());
    }

    #[test]
    fn test_selection_lasso_mask() {
        let buf = canvas_10x10();
        let points = [(1, 1), (8, 1), (8, 8), (1, 8)];
        let sel = Selection::lasso(&buf, &points);
        assert!(sel.is_active());
        assert!(sel.is_selected(2, 2));
        assert!(!sel.is_selected(0, 0));
    }

    #[test]
    fn test_selection_copy_paste() {
        let mut buf = canvas_10x10();
        fill_all(&mut buf, '.');
        buf.set(2, 2, cell_with('A'));
        buf.set(3, 2, cell_with('B'));
        let sel = Selection::marquee(&buf, 2, 2, 3, 2);
        let clip = sel.copy_from(&buf);
        // Paste at offset (2, 4)
        Selection::paste_into(&mut buf, &clip, 2, 4);
        assert_eq!(buf.get(2, 4).unwrap().ch, 'A');
        assert_eq!(buf.get(3, 4).unwrap().ch, 'B');
        // Source unchanged
        assert_eq!(buf.get(2, 2).unwrap().ch, 'A');
    }

    #[test]
    fn test_selection_cut() {
        let mut buf = canvas_10x10();
        fill_all(&mut buf, '.');
        buf.set(2, 2, cell_with('X'));
        let sel = Selection::marquee(&buf, 2, 2, 2, 2);
        let clip = sel.cut_from(&mut buf);
        // Original cell cleared
        assert_eq!(buf.get(2, 2).unwrap().ch, ' ');
        // Clipboard has the cell
        assert_eq!(clip[0][0].unwrap().ch, 'X');
    }

    #[test]
    fn test_selection_delete() {
        let mut buf = canvas_10x10();
        fill_all(&mut buf, '.');
        buf.set(1, 1, cell_with('X'));
        buf.set(2, 2, cell_with('Y'));
        let sel = Selection::marquee(&buf, 1, 1, 2, 2);
        sel.delete_from(&mut buf);
        assert_eq!(buf.get(1, 1).unwrap().ch, ' ');
        assert_eq!(buf.get(2, 2).unwrap().ch, ' ');
    }

    #[test]
    fn test_selection_move() {
        let mut buf = canvas_20x20();
        fill_all(&mut buf, '.');
        buf.set(2, 2, cell_with('M'));
        buf.set(3, 2, cell_with('V'));
        let mut sel = Selection::marquee(&buf, 2, 2, 3, 2);
        sel.move_selection(&mut buf, 5, 3);
        // Old location cleared
        assert_eq!(buf.get(2, 2).unwrap().ch, ' ');
        assert_eq!(buf.get(3, 2).unwrap().ch, ' ');
        // New location has content
        assert_eq!(buf.get(7, 5).unwrap().ch, 'M');
        assert_eq!(buf.get(8, 5).unwrap().ch, 'V');
        // Selection bounds updated
        let (x_min, y_min, _, _) = sel.bounds();
        assert!(x_min >= 7);
        assert!(y_min >= 5);
    }

    #[test]
    fn test_selection_clip_to_bounds() {
        let mut buf = canvas_10x10();
        fill_all(&mut buf, '.');
        buf.set(8, 8, cell_with('X'));
        let mut sel = Selection::marquee(&buf, 8, 8, 9, 9);
        // Move partially off-canvas
        sel.move_selection(&mut buf, 5, 5);
        // Should not panic, only in-bounds cells affected
        assert_eq!(buf.get(9, 9).unwrap().ch, ' ');
        // The moved content should be at (13,13) which is out of bounds,
        // but (8,8) should be cleared since original selection was cut
        assert_eq!(buf.get(8, 8).unwrap().ch, ' ');
    }

    #[test]
    fn test_selection_perimeter() {
        let buf = canvas_10x10();
        let sel = Selection::marquee(&buf, 2, 2, 4, 4);
        let perim = sel.perimeter();
        // Each perimeter cell should have at least one unselected neighbor
        for &(x, y) in &perim {
            assert!(sel.is_selected(x, y));
        }
        // Interior cell (3,3) should NOT be in perimeter
        assert!(!perim.contains(&(3, 3)));
        // Corner cells should be in perimeter
        assert!(perim.contains(&(2, 2)));
        assert!(perim.contains(&(4, 4)));
    }

    #[test]
    fn test_selection_empty_on_new() {
        let sel = Selection::new(10, 10);
        assert!(!sel.is_active());
        let perim = sel.perimeter();
        assert!(perim.is_empty());
    }

    #[test]
    fn test_selection_paste_off_canvas() {
        let mut buf = canvas_10x10();
        fill_all(&mut buf, '.');
        let clip = vec![vec![Some(cell_with('X')), Some(cell_with('Y'))]];
        // Paste at negative coordinates — should not panic
        Selection::paste_into(&mut buf, &clip, -5, -5);
        // Positive coordinates unchanged
        assert_eq!(buf.get(0, 0).unwrap().ch, '.');
    }
}
