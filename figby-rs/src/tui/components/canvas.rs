use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::canvas::CanvasWidget;
use crate::tui::component::Component;
use crate::tui::events::{AppEvent, CanvasEvent};
use crate::tui::theme::Theme;

pub struct CanvasComponent {
    pub canvas: CanvasWidget,
    pub canvas_inner_rect: Rect,
    pub last_canvas_size: (u16, u16),
    pub theme: Theme,
}

impl CanvasComponent {
    pub fn new() -> Self {
        Self {
            canvas: CanvasWidget::default(),
            canvas_inner_rect: Rect::new(0, 0, 0, 0),
            last_canvas_size: (0, 0),
            theme: Theme::default(),
        }
    }

    pub fn set_canvas_size(&mut self, w: u16, h: u16) {
        if self.canvas.buffer.width() != w as usize || self.canvas.buffer.height() != h as usize {
            self.canvas = CanvasWidget::new(w, h);
        }
    }
}

impl Default for CanvasComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for CanvasComponent {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<AppEvent> {
        let code = key.code;
        match code {
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Char('+')
            | KeyCode::Char('=')
            | KeyCode::Char('-')
            | KeyCode::Char('_')
            | KeyCode::Char('G') => {
                if self
                    .canvas
                    .handle_key(code, self.last_canvas_size.0, self.last_canvas_size.1)
                {
                    return Some(AppEvent::Canvas(CanvasEvent::Modified));
                }
                None
            }
            _ => None,
        }
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> std::io::Result<()> {
        let zoom = self.canvas.zoom_level().max(1) as u16;
        let buf_w = self.canvas.buffer.width() as u16;
        let buf_h = self.canvas.buffer.height() as u16;
        let grid_w = (buf_w * zoom).min(area.width);
        let grid_h = (buf_h * zoom).min(area.height);
        let centered = Rect {
            x: area.x + (area.width.saturating_sub(grid_w) / 2),
            y: area.y + (area.height.saturating_sub(grid_h) / 2),
            width: grid_w,
            height: grid_h,
        };
        self.last_canvas_size = (buf_w, buf_h);
        self.canvas_inner_rect = centered;
        self.canvas
            .ensure_cursor_visible(centered.width, centered.height);

        if centered.width > 1 && centered.height > 1 {
            let edge = ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .style(
                    ratatui::style::Style::default()
                        .fg(self.theme.canvas.edge)
                        .add_modifier(ratatui::style::Modifier::DIM),
                );
            frame.render_widget(edge, centered);
        }
        frame.render_widget(&self.canvas, centered);
        Ok(())
    }
}
