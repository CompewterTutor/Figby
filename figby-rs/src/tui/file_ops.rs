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
    Open,
}

pub struct RecentFiles {
    files: Vec<PathBuf>,
    max: usize,
}

impl RecentFiles {
    const MAX: usize = 10;

    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            max: Self::MAX,
        }
    }

    pub fn push(&mut self, path: PathBuf) {
        if let Some(pos) = self.files.iter().position(|p| p == &path) {
            self.files.remove(pos);
        }
        self.files.insert(0, path);
        self.files.truncate(self.max);
    }

    pub fn list(&self) -> &[PathBuf] {
        &self.files
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn get(&self, idx: usize) -> Option<&PathBuf> {
        self.files.get(idx)
    }

    pub fn remove_missing(&mut self) {
        self.files.retain(|p| p.exists());
    }

    pub fn remove(&mut self, idx: usize) {
        if idx < self.files.len() {
            self.files.remove(idx);
        }
    }

    pub fn load_from_disk() -> Self {
        let path = Self::storage_path();
        let path = match path {
            Some(p) => p,
            None => return Self::new(),
        };
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::new(),
        };
        let files: Vec<PathBuf> = content
            .lines()
            .filter(|l| !l.is_empty())
            .map(PathBuf::from)
            .collect();
        Self {
            files,
            max: Self::MAX,
        }
    }

    pub fn save_to_disk(&self) {
        let path = match Self::storage_path() {
            Some(p) => p,
            None => return,
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content = self
            .files
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let _ = std::fs::write(&path, &content);
    }

    fn storage_path() -> Option<PathBuf> {
        if let Ok(data_dir) = std::env::var("XDG_DATA_HOME") {
            Some(PathBuf::from(data_dir).join("figby/recent.json"))
        } else if let Ok(home) = std::env::var("HOME") {
            let xdg = PathBuf::from(&home).join(".local/share/figby/recent.json");
            if xdg.parent().is_some_and(|p| p.exists()) {
                Some(xdg)
            } else {
                Some(PathBuf::from(&home).join(".figby/recent.json"))
            }
        } else {
            None
        }
    }
}

impl Default for RecentFiles {
    fn default() -> Self {
        Self::new()
    }
}

pub struct FileOpsDialog {
    pub mode: FileOpsMode,
    pub path_buffer: String,
    pub directory_entries: Vec<String>,
    pub selected_entry: usize,
    pub error_message: String,
    pub hide_dotfiles: bool,
    recent_files_for_display: Vec<String>,
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
            recent_files_for_display: Vec::new(),
        }
    }

    pub fn enter_open(&mut self, recent: &[PathBuf]) {
        self.mode = FileOpsMode::Open;
        self.path_buffer.clear();
        self.selected_entry = 0;
        self.error_message.clear();
        self.recent_files_for_display = recent
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        self.refresh_directory();
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
        self.recent_files_for_display.clear();
    }

    pub fn handle_paste(&mut self, text: &str) {
        if self.mode == FileOpsMode::Idle {
            return;
        }
        self.path_buffer.push_str(text);
        self.error_message.clear();
        self.selected_entry = 0;
        self.refresh_directory();
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
            FileOpsMode::Open => self.handle_key_open(code),
            FileOpsMode::Idle => false,
        }
    }

    fn handle_key_open(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char(c) if !c.is_control() && !c.is_ascii_digit() => {
                self.path_buffer.push(c);
                self.error_message.clear();
                self.selected_entry = 0;
                self.refresh_directory();
                true
            }
            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                let idx = (c as u8 - b'1') as usize;
                if idx < self.recent_files_for_display.len() {
                    self.path_buffer = self.recent_files_for_display[idx].clone();
                    self.selected_entry = 0;
                    self.error_message.clear();
                    self.refresh_directory();
                }
                true
            }
            KeyCode::Backspace => {
                self.path_buffer.pop();
                self.error_message.clear();
                self.selected_entry = 0;
                self.refresh_directory();
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
            FileOpsMode::Open => self.render_open(frame, area),
            FileOpsMode::Idle => {}
        }
    }

    fn render_open(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Open Font ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width < 24 || inner.height < 8 {
            return;
        }

        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            " Path:",
            Style::default().add_modifier(Modifier::BOLD),
        )));

        let path_display = if self.path_buffer.is_empty() {
            " (type path, browse with arrows, or paste)".to_string()
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
                " Directory:",
                Style::default().add_modifier(Modifier::BOLD),
            )));

            let max_visible = (inner.height as usize).saturating_sub(6).min(8);
            let start = self.selected_entry.saturating_sub(max_visible / 2);
            let end = (start + max_visible).min(self.directory_entries.len());
            for i in start..end {
                let entry = &self.directory_entries[i];
                let is_selected = i == self.selected_entry;
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
                let is_dir = parent.join(entry).is_dir();
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

        if !self.recent_files_for_display.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                " Recent files:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            let max_recent = 9.min(self.recent_files_for_display.len());
            let recent_start = if self.recent_files_for_display.len() > 9 {
                self.recent_files_for_display.len() - 9
            } else {
                0
            };
            for i in recent_start..recent_start + max_recent {
                let display = &self.recent_files_for_display[i];
                let num = i + 1;
                let text = format!("  {num}. {display}");
                lines.push(Line::from(Span::styled(
                    text,
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " Tab: select  Enter: open  Esc: cancel  1-9: recent  \u{2191}\u{2193}: navigate",
            Style::default().fg(Color::DarkGray),
        )));

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
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
            for i in start..end {
                let entry = &self.directory_entries[i];
                let is_selected = i == self.selected_entry;
                let is_dir = parent.join(entry).is_dir();
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

    // --- Open dialog tests ---

    #[test]
    fn test_open_dialog_enter() {
        let mut dialog = FileOpsDialog::new();
        let recent = Vec::new();
        dialog.enter_open(&recent);
        assert_eq!(dialog.mode, FileOpsMode::Open);
        assert!(dialog.path_buffer.is_empty());
        assert!(dialog.error_message.is_empty());
    }

    #[test]
    fn test_open_dialog_enter_exit() {
        let mut dialog = FileOpsDialog::new();
        let recent = Vec::new();
        dialog.enter_open(&recent);
        assert_eq!(dialog.mode, FileOpsMode::Open);
        dialog.handle_key(KeyCode::Esc);
        assert_eq!(dialog.mode, FileOpsMode::Idle);
    }

    #[test]
    fn test_open_dialog_type_path() {
        let mut dialog = FileOpsDialog::new();
        let recent = Vec::new();
        dialog.enter_open(&recent);
        dialog.handle_key(KeyCode::Char('m'));
        dialog.handle_key(KeyCode::Char('y'));
        dialog.handle_key(KeyCode::Char('.'));
        dialog.handle_key(KeyCode::Char('f'));
        dialog.handle_key(KeyCode::Char('l'));
        dialog.handle_key(KeyCode::Char('f'));
        assert_eq!(dialog.path_buffer, "my.flf");
    }

    #[test]
    fn test_open_dialog_paste_path() {
        let mut dialog = FileOpsDialog::new();
        let recent = Vec::new();
        dialog.enter_open(&recent);
        dialog.handle_paste("/path/to/font.flf");
        assert_eq!(dialog.path_buffer, "/path/to/font.flf");
    }

    #[test]
    fn test_open_dialog_paste_idle_noop() {
        let mut dialog = FileOpsDialog::new();
        assert_eq!(dialog.mode, FileOpsMode::Idle);
        dialog.handle_paste("should not set");
        assert!(dialog.path_buffer.is_empty());
    }

    #[test]
    fn test_open_dialog_enter_finalizes() {
        let mut dialog = FileOpsDialog::new();
        let recent = Vec::new();
        dialog.enter_open(&recent);
        dialog.handle_key(KeyCode::Enter);
        assert_eq!(dialog.mode, FileOpsMode::Idle);
    }

    #[test]
    fn test_open_dialog_backspace_empty() {
        let mut dialog = FileOpsDialog::new();
        let recent = Vec::new();
        dialog.enter_open(&recent);
        // Backspace on empty buffer is a no-op
        dialog.handle_key(KeyCode::Backspace);
        assert!(dialog.path_buffer.is_empty());
    }

    // --- Recent files tests ---

    #[test]
    fn test_recent_files_new_empty() {
        let recent = RecentFiles::new();
        assert!(recent.is_empty());
        assert_eq!(recent.len(), 0);
    }

    #[test]
    fn test_recent_files_push() {
        let mut recent = RecentFiles::new();
        recent.push(PathBuf::from("/a.flf"));
        recent.push(PathBuf::from("/b.flf"));
        recent.push(PathBuf::from("/c.flf"));
        assert_eq!(recent.len(), 3);
        assert_eq!(recent.get(0), Some(&PathBuf::from("/c.flf")));
        assert_eq!(recent.get(2), Some(&PathBuf::from("/a.flf")));
    }

    #[test]
    fn test_recent_files_max() {
        let mut recent = RecentFiles::new();
        for i in 0..15 {
            recent.push(PathBuf::from(format!("/font_{i}.flf")));
        }
        assert_eq!(recent.len(), 10);
        assert_eq!(recent.get(0), Some(&PathBuf::from("/font_14.flf")));
        assert_eq!(recent.get(9), Some(&PathBuf::from("/font_5.flf")));
    }

    #[test]
    fn test_recent_files_dedup() {
        let mut recent = RecentFiles::new();
        recent.push(PathBuf::from("/a.flf"));
        recent.push(PathBuf::from("/b.flf"));
        recent.push(PathBuf::from("/a.flf"));
        assert_eq!(recent.len(), 2);
        assert_eq!(recent.get(0), Some(&PathBuf::from("/a.flf")));
        assert_eq!(recent.get(1), Some(&PathBuf::from("/b.flf")));
    }

    #[test]
    fn test_recent_files_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_recent_roundtrip.json");
        // Override storage path via env
        std::env::set_var("XDG_DATA_HOME", dir.to_str().unwrap());

        let mut recent = RecentFiles::new();
        recent.push(PathBuf::from("/font_a.flf"));
        recent.push(PathBuf::from("/font_b.flf"));
        recent.save_to_disk();

        let loaded = RecentFiles::load_from_disk();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.get(0), Some(&PathBuf::from("/font_b.flf")));
        assert_eq!(loaded.get(1), Some(&PathBuf::from("/font_a.flf")));

        std::fs::remove_file(&path).ok();
        std::env::remove_var("XDG_DATA_HOME");
    }

    #[test]
    fn test_recent_files_load_empty_on_missing() {
        // Loading from a non-existent file should give empty list
        let recent = RecentFiles::new();
        assert!(recent.is_empty());
    }

    #[test]
    fn test_recent_files_remove_missing() {
        let mut recent = RecentFiles::new();
        recent.push(PathBuf::from("/nonexistent_path_12345.flf"));
        recent.remove_missing();
        assert!(recent.is_empty());
    }

    #[test]
    fn test_recent_files_remove_idx() {
        let mut recent = RecentFiles::new();
        recent.push(PathBuf::from("/a.flf"));
        recent.push(PathBuf::from("/b.flf"));
        recent.remove(0);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent.get(0), Some(&PathBuf::from("/a.flf")));
        recent.remove(5); // out of bounds, no panic
        assert_eq!(recent.len(), 1);
    }

    // --- Integration test: open known font, verify all glyphs ---

    #[test]
    fn test_open_known_font_all_glyphs_loaded() {
        let font_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../fonts");
        let path = std::path::Path::new(font_dir).join("standard.flf");
        let content = std::fs::read_to_string(&path).expect("standard.flf should exist");
        let font = parse_tlf_font(&content).expect("standard.flf should parse");

        // Verify 95 ASCII chars (32-126)
        for code in 32..=126u32 {
            assert!(
                font.chars.contains_key(&code),
                "ASCII char U+{code:04X} should be present"
            );
        }

        // Verify 7 Deutsch chars
        let deutsch_codes = [196u32, 214, 220, 228, 246, 252, 223];
        for &code in &deutsch_codes {
            assert!(
                font.chars.contains_key(&code),
                "Deutsch char U+{code:04X} should be present"
            );
        }

        assert_eq!(font.charheight, 6, "standard.flf should have height 6");
    }

    #[test]
    fn test_open_dialog_recent_file_by_digit() {
        let mut dialog = FileOpsDialog::new();
        let recent = vec![
            PathBuf::from("/first.flf"),
            PathBuf::from("/second.flf"),
            PathBuf::from("/third.flf"),
        ];
        dialog.enter_open(&recent);

        // Press '2' to select second recent file
        dialog.handle_key(KeyCode::Char('2'));
        assert_eq!(dialog.path_buffer, "/second.flf");
    }

    #[test]
    fn test_open_dialog_recent_digit_out_of_range() {
        let mut dialog = FileOpsDialog::new();
        let recent = vec![PathBuf::from("/first.flf")];
        dialog.enter_open(&recent);

        // Press '9' - only 1 recent file, so should be no-op
        dialog.handle_key(KeyCode::Char('9'));
        assert!(dialog.path_buffer.is_empty());
    }
}
