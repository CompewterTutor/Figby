use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasCell {
    pub ch: char,
    pub fg: Option<Color>,
    pub bg: Option<Color>,
}

impl Default for CanvasCell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: None,
            bg: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CanvasBuffer {
    cells: Vec<Vec<CanvasCell>>,
    width: usize,
    height: usize,
}

impl CanvasBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            cells: vec![vec![CanvasCell::default(); width]; height],
            width,
            height,
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<&CanvasCell> {
        if x < self.width && y < self.height {
            Some(&self.cells[y][x])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut CanvasCell> {
        if x < self.width && y < self.height {
            Some(&mut self.cells[y][x])
        } else {
            None
        }
    }

    pub fn set(&mut self, x: usize, y: usize, cell: CanvasCell) {
        if let Some(c) = self.get_mut(x, y) {
            *c = cell;
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}

pub struct CanvasWidget {
    pub buffer: CanvasBuffer,
    cursor: (u16, u16),
    scroll: (u16, u16),
    zoom: u8,
    show_grid: bool,
}

impl CanvasWidget {
    pub fn new(width: u16, height: u16) -> Self {
        let w = if width > 0 { width as usize } else { 1 };
        let h = if height > 0 { height as usize } else { 1 };
        Self {
            buffer: CanvasBuffer::new(w, h),
            cursor: (0, 0),
            scroll: (0, 0),
            zoom: 1,
            show_grid: false,
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor.1 > 0 {
            self.cursor.1 -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let max_y = self.buffer.height().saturating_sub(1) as u16;
        if self.cursor.1 < max_y {
            self.cursor.1 += 1;
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor.0 > 0 {
            self.cursor.0 -= 1;
        }
    }

    pub fn move_right(&mut self) {
        let max_x = self.buffer.width().saturating_sub(1) as u16;
        if self.cursor.0 < max_x {
            self.cursor.0 += 1;
        }
    }

    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 2).clamp(1, 8);
    }

    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 2).clamp(1, 8);
    }

    pub fn toggle_grid(&mut self) {
        self.show_grid = !self.show_grid;
    }

    pub fn cursor(&self) -> (u16, u16) {
        self.cursor
    }

    pub fn zoom_level(&self) -> u8 {
        self.zoom
    }

    pub fn show_grid(&self) -> bool {
        self.show_grid
    }

    pub fn ensure_cursor_visible(&mut self, inner_width: u16, inner_height: u16) {
        let zoom = self.zoom.max(1) as u16;
        if inner_width == 0 || inner_height == 0 {
            return;
        }
        let vis_w = inner_width / zoom;
        let vis_h = inner_height / zoom;

        if self.cursor.0 < self.scroll.0 {
            self.scroll.0 = self.cursor.0;
        }
        if vis_w > 0 && self.cursor.0 >= self.scroll.0 + vis_w {
            self.scroll.0 = self.cursor.0 - vis_w + 1;
        }
        if self.cursor.1 < self.scroll.1 {
            self.scroll.1 = self.cursor.1;
        }
        if vis_h > 0 && self.cursor.1 >= self.scroll.1 + vis_h {
            self.scroll.1 = self.cursor.1 - vis_h + 1;
        }
    }

    pub fn handle_key(&mut self, code: KeyCode, inner_width: u16, inner_height: u16) -> bool {
        match code {
            KeyCode::Up => {
                self.move_up();
                self.ensure_cursor_visible(inner_width, inner_height);
                true
            }
            KeyCode::Down => {
                self.move_down();
                self.ensure_cursor_visible(inner_width, inner_height);
                true
            }
            KeyCode::Left => {
                self.move_left();
                self.ensure_cursor_visible(inner_width, inner_height);
                true
            }
            KeyCode::Right => {
                self.move_right();
                self.ensure_cursor_visible(inner_width, inner_height);
                true
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.zoom_in();
                self.ensure_cursor_visible(inner_width, inner_height);
                true
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                self.zoom_out();
                self.ensure_cursor_visible(inner_width, inner_height);
                true
            }
            KeyCode::Char('G') => {
                self.toggle_grid();
                true
            }
            _ => false,
        }
    }
}

impl Widget for &CanvasWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let zoom = self.zoom.max(1) as u16;
        let sx = self.scroll.0;
        let sy = self.scroll.1;

        // Fill area with buffer cells
        for row in 0..area.height {
            for col in 0..area.width {
                let bx = sx + col / zoom;
                let by = sy + row / zoom;

                let (ch, style) = if let Some(cell) = self.buffer.get(bx as usize, by as usize) {
                    let mut s = Style::default();
                    if let Some(fg) = cell.fg {
                        s = s.fg(fg);
                    }
                    if let Some(bg) = cell.bg {
                        s = s.bg(bg);
                    }
                    (cell.ch, s)
                } else {
                    (' ', Style::default())
                };

                if let Some(cell) = buf.cell_mut((area.x + col, area.y + row)) {
                    cell.set_char(ch);
                    cell.set_style(style);
                }
            }
        }

        // Grid overlay (before cursor so cursor highlight wins)
        if self.show_grid && zoom > 1 {
            let grid_style = Style::default().add_modifier(Modifier::DIM);
            let max_vis_buf_w =
                (area.width / zoom).min((self.buffer.width() as u16).saturating_sub(sx));
            let max_vis_buf_h =
                (area.height / zoom).min((self.buffer.height() as u16).saturating_sub(sy));

            // Horizontal lines at row boundaries
            for by in 1..max_vis_buf_h {
                let cy = area.y + by * zoom;
                for c in area.x..area.x + area.width {
                    if let Some(cell) = buf.cell_mut((c, cy)) {
                        if (c - area.x).is_multiple_of(zoom) && (c - area.x) > 0 {
                            cell.set_char('┼');
                        } else {
                            cell.set_char('─');
                        }
                        cell.set_style(grid_style);
                    }
                }
            }

            // Vertical lines at column boundaries
            for bx in 1..max_vis_buf_w {
                let cx = area.x + bx * zoom;
                for r in area.y..area.y + area.height {
                    if let Some(cell) = buf.cell_mut((cx, r)) {
                        if (r - area.y).is_multiple_of(zoom) && (r - area.y) > 0 {
                            cell.set_char('┼');
                        } else {
                            cell.set_char('│');
                        }
                        cell.set_style(grid_style);
                    }
                }
            }
        }

        // Cursor highlight (rendered after grid so it's visible on top)
        let vis_w = area.width / zoom;
        let vis_h = area.height / zoom;
        if self.cursor.0 >= sx
            && self.cursor.0 < sx + vis_w
            && self.cursor.1 >= sy
            && self.cursor.1 < sy + vis_h
        {
            let cx = area.x + (self.cursor.0 - sx) * zoom;
            let cy = area.y + (self.cursor.1 - sy) * zoom;
            for r in cy..(cy + zoom).min(area.y + area.height) {
                for c in cx..(cx + zoom).min(area.x + area.width) {
                    if let Some(cell) = buf.cell_mut((c, r)) {
                        cell.set_style(cell.style().reversed());
                    }
                }
            }
        }
    }
}

impl Default for CanvasWidget {
    fn default() -> Self {
        Self::new(40, 20)
    }
}
