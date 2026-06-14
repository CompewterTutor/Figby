use std::path::Path;

use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::action::Action;
use crate::tui::component::Component;
pub use crate::tui::file_ops::FileOpsMode;
use crate::tui::file_ops::{FileOpsDialog, RecentFiles};

pub struct FileOpsComponent {
    pub dialog: FileOpsDialog,
    pub recent_files: RecentFiles,
}

impl FileOpsComponent {
    pub fn new() -> Self {
        Self {
            dialog: FileOpsDialog::new(),
            recent_files: RecentFiles::load_from_disk(),
        }
    }

    pub fn selected_path(&self) -> std::path::PathBuf {
        self.dialog.selected_path()
    }

    pub fn handle_paste(&mut self, text: &str) {
        self.dialog.handle_paste(text);
    }

    pub fn enter_open(&mut self, recent_files: &[std::path::PathBuf]) {
        self.dialog.enter_open(recent_files);
    }

    pub fn enter_save_as(&mut self, current: Option<&Path>) {
        self.dialog.enter_save_as(current);
    }
}

impl Default for FileOpsComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for FileOpsComponent {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        if self.dialog.mode == FileOpsMode::Idle {
            return None;
        }
        let prev_mode = self.dialog.mode;
        self.dialog.handle_key(key.code);
        if self.dialog.mode == FileOpsMode::Idle {
            return match prev_mode {
                FileOpsMode::SaveAs => Some(Action::SaveAsRequested),
                FileOpsMode::Open => Some(Action::OpenRequested),
                FileOpsMode::Idle => None,
            };
        }
        Some(Action::CloseDialog)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> std::io::Result<()> {
        if self.dialog.mode != FileOpsMode::Idle {
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
