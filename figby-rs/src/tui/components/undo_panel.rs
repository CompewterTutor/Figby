use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::component::Component;
use crate::tui::events::AppEvent;
use crate::tui::undo::UndoEntry;
use crate::tui::undo_panel::UndoPanel;

pub struct UndoPanelComponent {
    pub panel: UndoPanel,
}

impl UndoPanelComponent {
    pub fn new() -> Self {
        Self {
            panel: UndoPanel::new(),
        }
    }

    pub fn toggle(&mut self) {
        self.panel.toggle();
    }
}

impl Default for UndoPanelComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for UndoPanelComponent {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<AppEvent> {
        if !self.panel.open {
            return None;
        }
        self.panel.handle_key(key.code);
        None
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> std::io::Result<()> {
        if self.panel.open {
            let entries: Vec<UndoEntry> = Vec::new();
            self.panel.render(frame, area, &entries);
        }
        Ok(())
    }
}
