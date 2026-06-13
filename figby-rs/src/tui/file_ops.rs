use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::path::{Path, PathBuf};

use crate::font::FIGfont;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOpsMode {
    Idle,
    SaveAs,
}

pub struct FileOpsDialog {
    pub mode: FileOpsMode,
    pub path_buffer: String,
    pub directory_entries: Vec<String>,
    pub selected_entry: usize,
    pub error_message: String,
    pub hide_dotfiles: bool,
}

impl FileOpsDialog {
    pub fn new() -> Self {
        Self {
            mode: FileOpsMode::Idle,
            path_buffer: String::new(),
            directory_entries: Vec::new(),
            selected_entry: 0,
            error_message: String::new(),
            hide_dotfiles: true,
        }
    }

    pub fn enter_save_as(&mut self, current: Option<&Path>) {
        self.mode = FileOpsMode::SaveAs;
        self.path_buffer = current
            .and_then(|p| p.to_str())
            .map(|s| s.to_string())
            .unwrap_or_default();
        self.selected_entry = 0;
        self.error_message.clear();
        self.refresh_directory();
    }

    pub fn close(&mut self) {
        self.mode = FileOpsMode::Idle;
        self.path_buffer.clear();
        self.directory_entries.clear();
        self.selected_entry = 0;
        self.error_message.clear();
    }

    fn refresh_directory(&mut self) {
        self.directory_entries.clear();
        self.selected_entry = 0;

        let parent = if self.path_buffer.is_empty() {
            PathBuf::from(".")
        } else {
            let p = PathBuf::from(&self.path_buffer);
            if p.is_dir() {
                p.clone()
            } else {
                p.parent()
                    .map(|pp| pp.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from("."))
            }
        };

        let read_dir = match std::fs::read_dir(&parent) {
            Ok(rd) => rd,
            Err(_) => {
                self.error_message = format!("Cannot read directory: {}", parent.display());
                return;
            }
        };

        let mut entries: Vec<String> = Vec::new();

        for entry in read_dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if self.hide_dotfiles && name.starts_with('.') {
                continue;
            }
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let is_flf = name.ends_with(".flf") || name.ends_with(".tlf");
            if is_dir || is_flf {
                entries.push(name);
            }
        }

        entries.sort();
        self.directory_entries = entries;
    }

    pub fn selected_path(&self) -> PathBuf {
        let trimmed = self.path_buffer.trim().to_string();
        if trimmed.is_empty() {
            return PathBuf::from("untitled.flf");
        }
        let p = PathBuf::from(&trimmed);
        if p.extension().is_none() {
            let mut with_ext = p;
            with_ext.set_extension("flf");
            with_ext
        } else {
            p
        }
    }

    fn select_entry(&mut self) {
        let entry = &self.directory_entries[self.selected_entry];
        let parent = if self.path_buffer.is_empty() {
            PathBuf::from(".")
        } else {
            let p = PathBuf::from(&self.path_buffer);
            if p.is_dir() {
                p
            } else {
                p.parent()
                    .map(|pp| pp.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from("."))
            }
        };
        let abs = parent.join(entry);
        self.path_buffer = abs.to_string_lossy().to_string();
        self.selected_entry = 0;
        self.error_message.clear();
        self.refresh_directory();
    }

    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        match self.mode {
            FileOpsMode::SaveAs => self.handle_key_save_as(code),
            FileOpsMode::Idle => false,
        }
    }

    fn handle_key_save_as(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char(c) => {
                self.path_buffer.push(c);
                self.error_message.clear();
                true
            }
            KeyCode::Backspace => {
                self.path_buffer.pop();
                self.error_message.clear();
                true
            }
            KeyCode::Up => {
                if !self.directory_entries.is_empty() && self.selected_entry > 0 {
                    self.selected_entry -= 1;
                }
                true
            }
            KeyCode::Down => {
                if !self.directory_entries.is_empty()
                    && self.selected_entry < self.directory_entries.len() - 1
                {
                    self.selected_entry += 1;
                }
                true
            }
            KeyCode::Tab => {
                if !self.directory_entries.is_empty() {
                    self.select_entry();
                }
                true
            }
            KeyCode::Enter => {
                // Finalize save — path is in path_buffer, will be handled by caller
                self.mode = FileOpsMode::Idle;
                true
            }
            KeyCode::Esc => {
                self.close();
                true
            }
            _ => false,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        match self.mode {
            FileOpsMode::SaveAs => self.render_save_as(frame, area),
            FileOpsMode::Idle => {}
        }
    }

    fn render_save_as(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Save Font As ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width < 20 || inner.height < 6 {
            return;
        }

        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            " Path:",
            Style::default().add_modifier(Modifier::BOLD),
        )));

        let path_display = if self.path_buffer.is_empty() {
            " (type path or use arrows to browse)".to_string()
        } else {
            self.path_buffer.clone()
        };
        lines.push(Line::from(Span::styled(
            format!(" {}", path_display),
            Style::default().fg(Color::Cyan),
        )));

        if !self.error_message.is_empty() {
            lines.push(Line::from(Span::styled(
                format!(" Error: {}", self.error_message),
                Style::default().fg(Color::Red),
            )));
        }

        lines.push(Line::from(""));

        if self.directory_entries.is_empty() {
            lines.push(Line::from(Span::styled(
                " (no .flf/.tlf files in directory)",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                " Directory contents:",
                Style::default().add_modifier(Modifier::BOLD),
            )));

            let max_visible = (inner.height as usize).saturating_sub(6).min(10);
            let start = self.selected_entry.saturating_sub(max_visible / 2);
            let end = (start + max_visible).min(self.directory_entries.len());
            for i in start..end {
                let entry = &self.directory_entries[i];
                let is_selected = i == self.selected_entry;
                let is_dir = PathBuf::from(entry).is_dir()
                    || entry.contains('.') && !entry.ends_with(".flf") && !entry.ends_with(".tlf");
                let prefix = if is_selected { " >" } else { "  " };
                let suffix = if is_dir { "/" } else { "" };
                let text = format!("{}{}{}", prefix, entry, suffix);
                let style = if is_selected {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };
                lines.push(Line::from(Span::styled(text, style)));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " Tab: select entry  Enter: save  Esc: cancel  \u{2191}\u{2193}: navigate",
            Style::default().fg(Color::DarkGray),
        )));

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }
}

impl Default for FileOpsDialog {
    fn default() -> Self {
        Self::new()
    }
}

pub fn save_font(font: &FIGfont, path: &Path) -> std::io::Result<()> {
    let content = crate::font_gen::generate_figfont(font);
    let tmp_path = {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("font");
        parent.join(format!(".{}.tmp", stem))
    };

    std::fs::write(&tmp_path, &content)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::{parse_tlf_font, FIGcharacter, FIGfont};
    use std::collections::HashMap;

    fn make_test_font() -> FIGfont {
        FIGfont {
            charheight: 3,
            maxlength: 5,
            chars: HashMap::from([
                (
                    0,
                    FIGcharacter::from(vec![
                        "     ".to_string(),
                        "     ".to_string(),
                        "     ".to_string(),
                    ]),
                ),
                (
                    65,
                    FIGcharacter::from(vec![
                        " AA  ".to_string(),
                        "A  A ".to_string(),
                        "AAAA ".to_string(),
                    ]),
                ),
            ]),
            ..Default::default()
        }
    }

    #[test]
    fn test_save_and_reload_roundtrip() {
        let font = make_test_font();
        let dir = std::env::temp_dir();
        let path = dir.join("test_save_roundtrip.flf");

        save_font(&font, &path).expect("save should succeed");
        assert!(path.exists(), "file should exist after save");

        let content = std::fs::read_to_string(&path).expect("should read saved file");
        let parsed = parse_tlf_font(&content).expect("saved content should parse as FIGfont");

        assert_eq!(parsed.charheight, font.charheight);
        assert_eq!(parsed.maxlength, font.maxlength);

        let a_parsed = parsed.chars.get(&65).expect("char 65 should exist");
        assert_eq!(a_parsed.rows(), &[" AA  ", "A  A ", "AAAA "]);

        let bytes_orig = content.as_bytes();
        let bytes_reload = std::fs::read(&path).expect("should read raw bytes");
        assert_eq!(
            bytes_orig, bytes_reload,
            "reloaded bytes should match saved content"
        );

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_save_creates_valid_flf() {
        let font = make_test_font();
        let dir = std::env::temp_dir();
        let path = dir.join("test_save_valid.flf");

        save_font(&font, &path).expect("save should succeed");

        let content = std::fs::read_to_string(&path).expect("should read saved file");
        let parsed = parse_tlf_font(&content).expect("saved content should parse");

        assert_eq!(parsed.charheight, 3);
        assert!(parsed.chars.contains_key(&32), "space char should exist");
        assert!(parsed.chars.contains_key(&126), "tilde char should exist");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_save_as_error_handling() {
        let font = make_test_font();
        let result = save_font(&font, Path::new("/nonexistent_dir/out.flf"));
        assert!(result.is_err(), "save to invalid dir should error");
    }

    #[test]
    fn test_file_ops_dialog_new() {
        let dialog = FileOpsDialog::new();
        assert_eq!(dialog.mode, FileOpsMode::Idle);
        assert!(dialog.path_buffer.is_empty());
        assert!(dialog.error_message.is_empty());
    }

    #[test]
    fn test_file_ops_enter_save_as() {
        let mut dialog = FileOpsDialog::new();
        dialog.enter_save_as(None);
        assert_eq!(dialog.mode, FileOpsMode::SaveAs);
        assert!(dialog.path_buffer.is_empty());

        dialog.enter_save_as(Some(Path::new("/tmp/test.flf")));
        assert_eq!(dialog.path_buffer, "/tmp/test.flf");
    }

    #[test]
    fn test_file_ops_close() {
        let mut dialog = FileOpsDialog::new();
        dialog.enter_save_as(Some(Path::new("test.flf")));
        dialog.close();
        assert_eq!(dialog.mode, FileOpsMode::Idle);
    }

    #[test]
    fn test_selected_path_adds_extension() {
        let dialog = FileOpsDialog {
            path_buffer: "myfont".to_string(),
            ..FileOpsDialog::new()
        };
        let path = dialog.selected_path();
        assert_eq!(
            path.extension().map(|e| e.to_string_lossy().to_string()),
            Some("flf".to_string())
        );
    }

    #[test]
    fn test_selected_path_keeps_extension() {
        let dialog = FileOpsDialog {
            path_buffer: "myfont.flf".to_string(),
            ..FileOpsDialog::new()
        };
        let path = dialog.selected_path();
        assert_eq!(
            path.file_name().map(|n| n.to_string_lossy().to_string()),
            Some("myfont.flf".to_string())
        );
    }
}
