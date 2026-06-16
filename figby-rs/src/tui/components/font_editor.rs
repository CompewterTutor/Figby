use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::canvas::CanvasBuffer;
use crate::tui::component::Component;
use crate::tui::events::{AppEvent, FontEditorEvent};
use crate::tui::font_editor::FontEditor;

pub use crate::tui::font_editor::{CodeInputMode, FontEditorView, MirrorMode};

pub struct FontEditorComponent {
    pub editor: FontEditor,
    area_width: u16,
}

impl FontEditorComponent {
    pub fn new() -> Self {
        Self {
            editor: FontEditor::new(),
            area_width: 80,
        }
    }

    pub fn sync_from_canvas(&mut self, code: u32, buffer: &CanvasBuffer) {
        self.editor.sync_from_canvas(code, buffer);
    }

    pub fn selected_char(&self) -> Option<(u32, &crate::font::FIGcharacter)> {
        self.editor.selected_char()
    }

    pub fn set_area_width(&mut self, width: u16) {
        self.area_width = width;
    }
}

impl Default for FontEditorComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for FontEditorComponent {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<AppEvent> {
        let code = key.code;
        let modifiers = key.modifiers;
        if self.editor.handle_key(code, modifiers, self.area_width) {
            Some(AppEvent::FontEditor(FontEditorEvent::Changed(
                self.editor.view,
            )))
        } else {
            None
        }
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> std::io::Result<()> {
        self.area_width = area.width;
        self.editor.render(frame, area);
        Ok(())
    }
}
