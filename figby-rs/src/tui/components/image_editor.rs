use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::component::Component;
use crate::tui::events::AppEvent;
use crate::tui::image_editor::ImageEditor;

pub use crate::tui::image_editor::{AdjustmentMode, AsciiMode};

pub struct ImageEditorComponent {
    pub editor: ImageEditor,
}

impl ImageEditorComponent {
    pub fn new() -> Self {
        Self {
            editor: ImageEditor::new(),
        }
    }
}

impl Default for ImageEditorComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for ImageEditorComponent {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<AppEvent> {
        let code = key.code;
        if self.editor.handle_key(code) {
            Some(AppEvent::ImageEditor)
        } else {
            None
        }
    }

    fn draw(&mut self, _frame: &mut Frame, _area: Rect) -> std::io::Result<()> {
        Ok(())
    }
}
