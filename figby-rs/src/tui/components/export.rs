use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::component::Component;
use crate::tui::events::AppEvent;
use crate::tui::export::ExportDialog;

pub use crate::tui::export::ExportMode;

pub struct ExportComponent {
    pub dialog: ExportDialog,
}

impl ExportComponent {
    pub fn new() -> Self {
        Self {
            dialog: ExportDialog::new(),
        }
    }

    pub fn perform_export(
        &mut self,
        cells: &[Vec<crate::tui::canvas::CanvasCell>],
    ) -> Result<(), crate::output::ExportError> {
        self.dialog.perform_export(cells)
    }
}

impl Default for ExportComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for ExportComponent {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<AppEvent> {
        if !self.dialog.active {
            return None;
        }
        self.dialog.handle_key(key.code);
        if !self.dialog.active {
            return Some(AppEvent::ExportRequested(self.dialog.format));
        }
        None
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> std::io::Result<()> {
        if self.dialog.active {
            let overlay = Rect {
                x: area.width / 6,
                y: area.height / 6,
                width: area.width * 2 / 3,
                height: area.height * 2 / 3,
            };
            self.dialog.render(frame, overlay);
        }
        Ok(())
    }
}
