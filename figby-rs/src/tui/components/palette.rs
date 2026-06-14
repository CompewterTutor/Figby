use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::action::Action;
use crate::tui::component::Component;
pub use crate::tui::palette::ColorTarget;
use crate::tui::palette::Palette;

pub struct PaletteComponent {
    pub palette: Palette,
}

impl PaletteComponent {
    pub fn new() -> Self {
        Self {
            palette: Palette::new(),
        }
    }

    pub fn selected_color(&self) -> Option<ratatui::style::Color> {
        self.palette.selected_color
    }

    pub fn apply_to_cell(&self, cell: &mut crate::tui::canvas::CanvasCell) {
        self.palette.apply_to_cell(cell);
    }
}

impl Default for PaletteComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for PaletteComponent {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        let code = key.code;
        match code {
            KeyCode::Char('x')
            | KeyCode::Char('X')
            | KeyCode::Char('f')
            | KeyCode::Char('F')
            | KeyCode::Char('h')
            | KeyCode::Char('H')
            | KeyCode::Char('z')
            | KeyCode::Char('Z')
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Up
            | KeyCode::Down
            | KeyCode::Enter
            | KeyCode::Backspace
            | KeyCode::Esc => {
                if self.palette.handle_key(code) {
                    let color = self.palette.selected_color;
                    let target = self.palette.target;
                    if let Some(c) = color {
                        return Some(Action::ColorChanged(c, target));
                    }
                    return Some(Action::BrushChanged);
                }
                None
            }
            _ => None,
        }
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> std::io::Result<()> {
        self.palette.render(frame, area);
        Ok(())
    }
}
