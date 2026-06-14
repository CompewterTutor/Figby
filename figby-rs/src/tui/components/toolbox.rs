use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::action::Action;
use crate::tui::brush::BrushState;
use crate::tui::component::Component;
pub use crate::tui::toolbox::Tool;
use crate::tui::toolbox::Toolbox;

pub struct ToolboxComponent {
    pub toolbox: Toolbox,
    pub brush: BrushState,
}

impl ToolboxComponent {
    pub fn new() -> Self {
        Self {
            toolbox: Toolbox::new(),
            brush: BrushState::new(),
        }
    }

    pub fn push_undo_snapshot(&mut self, _label: &str) {}
}

impl Default for ToolboxComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for ToolboxComponent {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        let code = key.code;
        let modifiers = key.modifiers;
        match code {
            KeyCode::Char('[') => {
                self.brush.size_down();
                Some(Action::BrushChanged)
            }
            KeyCode::Char(']') => {
                self.brush.size_up();
                Some(Action::BrushChanged)
            }
            KeyCode::Char(';') => {
                self.brush.density_down();
                Some(Action::BrushChanged)
            }
            KeyCode::Char('\'') => {
                self.brush.density_up();
                Some(Action::BrushChanged)
            }
            KeyCode::Char('\\') => {
                self.brush.cycle_shape();
                Some(Action::BrushChanged)
            }
            KeyCode::Char(c) if !modifiers.contains(KeyModifiers::CONTROL) => {
                let lower = c.to_ascii_lowercase();
                for tool in Tool::all() {
                    if let KeyCode::Char(tc) = tool.key_shortcut() {
                        if tc == lower {
                            self.toolbox.selected = *tool;
                            return Some(Action::ToolSelected);
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> std::io::Result<()> {
        if area.width < 2 || area.height < 2 {
            return Ok(());
        }
        self.toolbox.render(frame, area);
        Ok(())
    }
}
