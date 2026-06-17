use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::output::{
    export_cells_to_gif, export_cells_to_png, export_cells_to_txt, ExportError, ExportFormat,
};

use super::canvas::CanvasCell;
use super::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportMode {
    Png,
    Txt,
    Gif,
}

impl ExportMode {
    pub fn label(&self) -> &str {
        match self {
            ExportMode::Png => "PNG",
            ExportMode::Txt => "TXT",
            ExportMode::Gif => "GIF",
        }
    }

    pub fn cycle(&self) -> Self {
        match self {
            ExportMode::Png => ExportMode::Gif,
            ExportMode::Gif => ExportMode::Txt,
            ExportMode::Txt => ExportMode::Png,
        }
    }

    pub fn to_export_format(&self) -> ExportFormat {
        match self {
            ExportMode::Png => ExportFormat::Png,
            ExportMode::Txt => ExportFormat::Txt,
            ExportMode::Gif => ExportFormat::Gif,
        }
    }

    pub fn extension(&self) -> &str {
        match self {
            ExportMode::Png => ".png",
            ExportMode::Txt => ".txt",
            ExportMode::Gif => ".gif",
        }
    }
}

pub struct ExportDialog {
    pub active: bool,
    pub format: ExportMode,
    pub path_buffer: String,
    pub font_size: u8,
    pub error_message: String,
    pub selected_entry: usize,
    pub directory_entries: Vec<String>,
    pub theme: Theme,
}

impl ExportDialog {
    pub fn new() -> Self {
        Self {
            active: false,
            format: ExportMode::Png,
            path_buffer: String::new(),
            font_size: 2,
            error_message: String::new(),
            selected_entry: 0,
            directory_entries: Vec::new(),
            theme: Theme::default(),
        }
    }

    pub fn enter_export(&mut self, mode: ExportMode) {
        self.active = true;
        self.format = mode;
        self.path_buffer = format!("export{}", mode.extension());
        self.error_message.clear();
        self.selected_entry = 0;
        self.refresh_directory();
    }

    pub fn close(&mut self) {
        self.active = false;
        self.path_buffer.clear();
        self.directory_entries.clear();
        self.selected_entry = 0;
        self.error_message.clear();
    }

    fn refresh_directory(&mut self) {
        self.directory_entries.clear();
        self.selected_entry = 0;

        let parent = if self.path_buffer.is_empty() {
            std::path::PathBuf::from(".")
        } else {
            let p = std::path::PathBuf::from(&self.path_buffer);
            if p.is_dir() {
                p
            } else {
                p.parent()
                    .map(|pp| pp.to_path_buf())
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
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
            if name.starts_with('.') {
                continue;
            }
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if is_dir || name.ends_with(".flf") || name.ends_with(".tlf") {
                entries.push(name);
            }
        }

        entries.sort();
        self.directory_entries = entries;
    }

    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char('T') | KeyCode::Char('t') => {
                self.format = self.format.cycle();
                true
            }
            KeyCode::Char(c) if !c.is_control() => {
                self.path_buffer.push(c);
                self.error_message.clear();
                self.selected_entry = 0;
                self.refresh_directory();
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
                self.active = false;
                true
            }
            KeyCode::Esc => {
                self.close();
                true
            }
            _ => false,
        }
    }

    fn select_entry(&mut self) {
        if self.selected_entry >= self.directory_entries.len() {
            return;
        }
        let entry = &self.directory_entries[self.selected_entry];
        let parent = if self.path_buffer.is_empty() {
            std::path::PathBuf::from(".")
        } else {
            let p = std::path::PathBuf::from(&self.path_buffer);
            if p.is_dir() {
                p
            } else {
                p.parent()
                    .map(|pp| pp.to_path_buf())
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
            }
        };
        let abs = parent.join(entry);
        self.path_buffer = abs.to_string_lossy().to_string();
        self.selected_entry = 0;
        self.error_message.clear();
        self.refresh_directory();
    }

    pub fn perform_export(&mut self, cells: &[Vec<CanvasCell>]) -> Result<(), ExportError> {
        if self.path_buffer.is_empty() {
            return Err(ExportError::InvalidCells("no path specified".to_string()));
        }
        let path = std::path::PathBuf::from(&self.path_buffer);
        let bytes: Vec<u8> = match self.format {
            ExportMode::Png => export_cells_to_png(cells, self.font_size)?,
            ExportMode::Txt => export_cells_to_txt(cells).into_bytes(),
            ExportMode::Gif => export_cells_to_gif(&[cells.to_vec()], &[10], self.font_size)?,
        };
        std::fs::write(&path, &bytes).map_err(|e| ExportError::IoError(e.to_string()))?;
        self.error_message.clear();
        Ok(())
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.active {
            return;
        }

        frame.render_widget(Clear, area);
        let block = Block::default()
            .title(" Export ")
            .borders(Borders::ALL)
            .style(Style::default().fg(self.theme.dialog.border_success));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width < 24 || inner.height < 8 {
            return;
        }

        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            format!(" Format: [{}]  (T to cycle)", self.format.label()),
            Style::default().add_modifier(Modifier::BOLD),
        )));

        lines.push(Line::from(Span::styled(
            format!(
                " Size: {}x (z = char at {})",
                self.font_size,
                8 * self.font_size as u16 * 16 * self.font_size as u16
            ),
            Style::default().fg(self.theme.dialog.meta),
        )));

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
            Style::default().fg(self.theme.dialog.border_path),
        )));

        if !self.error_message.is_empty() {
            lines.push(Line::from(Span::styled(
                format!(" Error: {}", self.error_message),
                Style::default().fg(self.theme.dialog.error),
            )));
        }

        lines.push(Line::from(""));

        if self.directory_entries.is_empty() {
            lines.push(Line::from(Span::styled(
                " (empty directory)",
                Style::default().fg(self.theme.dialog.meta),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                " Directory:",
                Style::default().add_modifier(Modifier::BOLD),
            )));

            let max_visible = (inner.height as usize).saturating_sub(7).min(10);
            let start = self.selected_entry.saturating_sub(max_visible / 2);
            let end = (start + max_visible).min(self.directory_entries.len());
            for i in start..end {
                let entry = &self.directory_entries[i];
                let is_selected = i == self.selected_entry;
                let parent = if self.path_buffer.is_empty() {
                    std::path::PathBuf::from(".")
                } else {
                    let p = std::path::PathBuf::from(&self.path_buffer);
                    if p.is_dir() {
                        p
                    } else {
                        p.parent()
                            .map(|pp| pp.to_path_buf())
                            .unwrap_or_else(|| std::path::PathBuf::from("."))
                    }
                };
                let is_dir = parent.join(entry).is_dir();
                let prefix = if is_selected { " >" } else { "  " };
                let suffix = if is_dir { "/" } else { "" };
                let text = format!("{prefix}{entry}{suffix}");
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
            " T: format  Enter: export  Esc: cancel  \u{2191}\u{2193}: navigate  Tab: select",
            Style::default().fg(self.theme.dialog.meta),
        )));

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }
}

impl Default for ExportDialog {
    fn default() -> Self {
        Self::new()
    }
}

use ratatui::buffer::Buffer;
use ratatui::widgets::Widget;

impl Widget for &ExportDialog {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.active {
            return;
        }

        Widget::render(Clear, area, buf);
        let block = Block::default()
            .title(" Export ")
            .borders(Borders::ALL)
            .style(Style::default().fg(self.theme.dialog.border_success));
        let inner = block.inner(area);
        Widget::render(block, area, buf);

        if inner.width < 24 || inner.height < 8 {
            return;
        }

        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            format!(" Format: [{}]  (T to cycle)", self.format.label()),
            Style::default().add_modifier(Modifier::BOLD),
        )));

        lines.push(Line::from(Span::styled(
            format!(
                " Size: {}x (z = char at {})",
                self.font_size,
                8 * self.font_size as u16 * 16 * self.font_size as u16
            ),
            Style::default().fg(self.theme.dialog.meta),
        )));

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
            Style::default().fg(self.theme.dialog.border_path),
        )));

        if !self.error_message.is_empty() {
            lines.push(Line::from(Span::styled(
                format!(" Error: {}", self.error_message),
                Style::default().fg(self.theme.dialog.error),
            )));
        }

        lines.push(Line::from(""));

        if self.directory_entries.is_empty() {
            lines.push(Line::from(Span::styled(
                " (empty directory)",
                Style::default().fg(self.theme.dialog.meta),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                " Directory:",
                Style::default().add_modifier(Modifier::BOLD),
            )));

            let max_visible = (inner.height as usize).saturating_sub(7).min(10);
            let start = self.selected_entry.saturating_sub(max_visible / 2);
            let end = (start + max_visible).min(self.directory_entries.len());
            for i in start..end {
                let entry = &self.directory_entries[i];
                let is_selected = i == self.selected_entry;
                let parent = if self.path_buffer.is_empty() {
                    std::path::PathBuf::from(".")
                } else {
                    let p = std::path::PathBuf::from(&self.path_buffer);
                    if p.is_dir() {
                        p
                    } else {
                        p.parent()
                            .map(|pp| pp.to_path_buf())
                            .unwrap_or_else(|| std::path::PathBuf::from("."))
                    }
                };
                let is_dir = parent.join(entry).is_dir();
                let prefix = if is_selected { " >" } else { "  " };
                let suffix = if is_dir { "/" } else { "" };
                let text = format!("{prefix}{entry}{suffix}");
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
            " T: format  Enter: export  Esc: cancel  \u{2191}\u{2193}: navigate  Tab: select",
            Style::default().fg(self.theme.dialog.meta),
        )));

        let paragraph = Paragraph::new(lines);
        Widget::render(paragraph, inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_dialog_new() {
        let dialog = ExportDialog::new();
        assert!(!dialog.active);
        assert_eq!(dialog.format, ExportMode::Png);
        assert!(dialog.path_buffer.is_empty());
        assert_eq!(dialog.font_size, 2);
    }

    #[test]
    fn test_export_dialog_open_close() {
        let mut dialog = ExportDialog::new();
        dialog.enter_export(ExportMode::Png);
        assert!(dialog.active);
        dialog.close();
        assert!(!dialog.active);
    }

    #[test]
    fn test_export_dialog_format_toggle() {
        let mut dialog = ExportDialog::new();
        dialog.enter_export(ExportMode::Png);
        dialog.handle_key(KeyCode::Char('T'));
        assert_eq!(dialog.format, ExportMode::Gif);
        dialog.handle_key(KeyCode::Char('t'));
        assert_eq!(dialog.format, ExportMode::Txt);
        dialog.handle_key(KeyCode::Char('t'));
        assert_eq!(dialog.format, ExportMode::Png);
    }

    #[test]
    fn test_export_dialog_path_entry() {
        let mut dialog = ExportDialog::new();
        dialog.enter_export(ExportMode::Png);
        dialog.handle_key(KeyCode::Char('m'));
        dialog.handle_key(KeyCode::Char('y'));
        dialog.handle_key(KeyCode::Char('f'));
        assert_eq!(dialog.path_buffer, "export.pngmyf");
    }

    #[test]
    fn test_export_dialog_enter_closes() {
        let mut dialog = ExportDialog::new();
        dialog.enter_export(ExportMode::Png);
        assert!(dialog.active);
        dialog.handle_key(KeyCode::Enter);
        assert!(!dialog.active);
    }

    #[test]
    fn test_export_dialog_esc_closes() {
        let mut dialog = ExportDialog::new();
        dialog.enter_export(ExportMode::Png);
        assert!(dialog.active);
        dialog.handle_key(KeyCode::Esc);
        assert!(!dialog.active);
    }

    #[test]
    fn test_export_dialog_backspace() {
        let mut dialog = ExportDialog::new();
        dialog.enter_export(ExportMode::Png);
        dialog.handle_key(KeyCode::Char('a'));
        dialog.handle_key(KeyCode::Char('b'));
        dialog.handle_key(KeyCode::Backspace);
        assert_eq!(dialog.path_buffer, "export.pnga");
    }

    #[test]
    fn test_export_mode_labels() {
        assert_eq!(ExportMode::Png.label(), "PNG");
        assert_eq!(ExportMode::Txt.label(), "TXT");
        assert_eq!(ExportMode::Gif.label(), "GIF");
    }

    #[test]
    fn test_export_mode_extensions() {
        assert_eq!(ExportMode::Png.extension(), ".png");
        assert_eq!(ExportMode::Txt.extension(), ".txt");
        assert_eq!(ExportMode::Gif.extension(), ".gif");
    }
}
