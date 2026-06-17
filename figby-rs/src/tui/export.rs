use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::output::{
    export_cells_to_gif, export_cells_to_png, export_cells_to_png_with_alpha, export_cells_to_txt,
    ExportError, ExportFormat,
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
    pub export_layers: bool,
    pub use_transparency: bool,
    pub error_message: String,
    pub selected_entry: usize,
    pub directory_entries: Vec<String>,
    pub theme: Theme,
    // GIF timeline export fields
    pub fps: u8,
    pub loop_count: u16,
    pub frame_delays: Vec<u16>,
    pub preview_frame: usize,
    pub preview_playing: bool,
    pub timeline_available: bool,
    pub timeline_frames: Vec<Vec<Vec<CanvasCell>>>,
}

impl ExportDialog {
    pub fn new() -> Self {
        Self {
            active: false,
            format: ExportMode::Png,
            path_buffer: String::new(),
            font_size: 2,
            export_layers: false,
            use_transparency: false,
            error_message: String::new(),
            selected_entry: 0,
            directory_entries: Vec::new(),
            theme: Theme::default(),
            fps: 12,
            loop_count: 0,
            frame_delays: Vec::new(),
            preview_frame: 0,
            preview_playing: false,
            timeline_available: false,
            timeline_frames: Vec::new(),
        }
    }

    pub fn enter_export(&mut self, mode: ExportMode) {
        self.active = true;
        self.format = mode;
        self.path_buffer = format!("export{}", mode.extension());
        self.error_message.clear();
        self.selected_entry = 0;
        self.clear_timeline();
        self.refresh_directory();
    }

    pub fn close(&mut self) {
        self.active = false;
        self.path_buffer.clear();
        self.directory_entries.clear();
        self.selected_entry = 0;
        self.error_message.clear();
        self.clear_timeline();
    }

    pub fn set_timeline(&mut self, fps: u8, frame_count: usize) {
        self.timeline_available = true;
        self.fps = fps;
        let delay = (100u16 / fps.max(1) as u16).max(1);
        self.frame_delays = vec![delay; frame_count];
        self.preview_frame = 0;
        self.preview_playing = false;
    }

    pub fn clear_timeline(&mut self) {
        self.timeline_available = false;
        self.frame_delays.clear();
        self.timeline_frames.clear();
        self.preview_frame = 0;
        self.preview_playing = false;
    }

    pub fn preview_tick(&mut self) {
        if !self.active
            || !self.preview_playing
            || !self.timeline_available
            || self.format != ExportMode::Gif
        {
            return;
        }
        let count = self.frame_delays.len();
        if count > 0 {
            self.preview_frame = (self.preview_frame + 1) % count;
        }
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

    const FPS_PRESETS: &'static [u8] = &[6, 8, 12, 24, 30, 60];
    const LOOP_PRESETS: &'static [u16] = &[0, 1, 2, 5, 10];

    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        // GIF-specific keys (only when GIF mode + timeline available)
        if self.format == ExportMode::Gif && self.timeline_available {
            match code {
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    let idx = Self::FPS_PRESETS
                        .iter()
                        .position(|&f| f == self.fps)
                        .unwrap_or(0);
                    self.fps = Self::FPS_PRESETS[(idx + 1) % Self::FPS_PRESETS.len()];
                    let delay = (100u16 / self.fps.max(1) as u16).max(1);
                    let count = self.frame_delays.len();
                    self.frame_delays = vec![delay; count];
                    return true;
                }
                KeyCode::Char('L') | KeyCode::Char('l') => {
                    let idx = Self::LOOP_PRESETS
                        .iter()
                        .position(|&l| l == self.loop_count)
                        .unwrap_or(0);
                    self.loop_count = Self::LOOP_PRESETS[(idx + 1) % Self::LOOP_PRESETS.len()];
                    return true;
                }
                KeyCode::Char('P') | KeyCode::Char('p') => {
                    self.preview_playing = !self.preview_playing;
                    return true;
                }
                KeyCode::Char(' ') => {
                    if !self.preview_playing {
                        let count = self.frame_delays.len();
                        if count > 0 {
                            self.preview_frame = (self.preview_frame + 1) % count;
                        }
                    }
                    return true;
                }
                _ => {}
            }
        }

        match code {
            KeyCode::Char('T') | KeyCode::Char('t') => {
                self.format = self.format.cycle();
                true
            }
            KeyCode::Char('L') | KeyCode::Char('l') => {
                self.export_layers = !self.export_layers;
                true
            }
            KeyCode::Char('P') | KeyCode::Char('p') => {
                self.use_transparency = !self.use_transparency;
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
            ExportMode::Gif => {
                let frame_slice: &[Vec<Vec<CanvasCell>>] =
                    if self.timeline_available && !self.timeline_frames.is_empty() {
                        self.timeline_frames.as_slice()
                    } else {
                        &[cells.to_vec()]
                    };
                let delay_slice: &[u16] =
                    if self.timeline_available && !self.frame_delays.is_empty() {
                        self.frame_delays.as_slice()
                    } else {
                        &[10]
                    };
                export_cells_to_gif(frame_slice, delay_slice, self.font_size, self.loop_count)?
            }
        };
        std::fs::write(&path, &bytes).map_err(|e| ExportError::IoError(e.to_string()))?;
        self.error_message.clear();
        Ok(())
    }

    fn sanitize_layer_name(name: &str) -> String {
        let sanitized: String = name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if sanitized.is_empty() {
            "layer".to_string()
        } else {
            sanitized
        }
    }

    pub fn perform_layer_export(
        stack: &super::layers::LayerStack,
        base_path: &std::path::Path,
        font_size: u8,
        use_transparency: bool,
    ) -> Result<(), ExportError> {
        let ext = ".png";
        let mut used_names: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for layer in stack.layers.iter() {
            if !layer.visible {
                continue;
            }

            let cells: Vec<Vec<super::canvas::CanvasCell>> = (0..layer.buffer.height())
                .map(|y| {
                    (0..layer.buffer.width())
                        .map(|x| layer.buffer.get(x, y).copied().unwrap_or_default())
                        .collect()
                })
                .collect();

            let stem = Self::sanitize_layer_name(&layer.name);
            let count = used_names.entry(stem.clone()).or_insert(0);
            let filename = if *count > 0 {
                format!("{}_{}{}", stem, count, ext)
            } else {
                format!("{}{}", stem, ext)
            };
            *count += 1;

            let path = base_path.with_file_name(&filename);
            let bytes = if use_transparency {
                export_cells_to_png_with_alpha(&cells, font_size, true)?
            } else {
                export_cells_to_png(&cells, font_size)?
            };
            std::fs::write(&path, &bytes).map_err(|e| ExportError::IoError(e.to_string()))?;
        }

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

        if self.format == ExportMode::Gif && self.timeline_available {
            lines.push(Line::from(Span::styled(
                format!(" FPS: [{}]  (F to cycle preset)", self.fps),
                Style::default().fg(self.theme.dialog.meta),
            )));
            let loop_label = if self.loop_count == 0 {
                "Infinite".to_string()
            } else {
                format!("{}x", self.loop_count)
            };
            lines.push(Line::from(Span::styled(
                format!(" Loop: [{}]  (L to cycle)", loop_label),
                Style::default().fg(self.theme.dialog.meta),
            )));
            lines.push(Line::from(Span::styled(
                format!(" Frames: [{}]", self.frame_delays.len()),
                Style::default().fg(self.theme.dialog.meta),
            )));
            let play_ch = if self.preview_playing {
                "\u{23F8}"
            } else {
                "\u{25B6}"
            };
            lines.push(Line::from(Span::styled(
                format!(
                    " Preview: {} (frame {}/{})  (P to toggle, Space to step)",
                    play_ch,
                    self.preview_frame + 1,
                    self.frame_delays.len().max(1)
                ),
                Style::default().fg(self.theme.dialog.meta),
            )));
        } else if self.format == ExportMode::Gif {
            lines.push(Line::from(Span::styled(
                " Timeline: No frames available",
                Style::default().fg(self.theme.dialog.error),
            )));
        }

        lines.push(Line::from(Span::styled(
            format!(
                " Size: {}x (z = char at {})",
                self.font_size,
                8 * self.font_size as u16 * 16 * self.font_size as u16
            ),
            Style::default().fg(self.theme.dialog.meta),
        )));

        if self.format != ExportMode::Gif || !self.timeline_available {
            lines.push(Line::from(Span::styled(
                format!(
                    " Layers: [{}]  (L to toggle)",
                    if self.export_layers {
                        "Per-Layer"
                    } else {
                        "Single"
                    }
                ),
                Style::default().fg(self.theme.dialog.meta),
            )));

            lines.push(Line::from(Span::styled(
                format!(
                    " Alpha: [{}]  (P to toggle)",
                    if self.use_transparency {
                        "Transparent"
                    } else {
                        "Opaque"
                    }
                ),
                Style::default().fg(self.theme.dialog.meta),
            )));
        }

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

            let max_visible = (inner.height as usize).saturating_sub(9).min(10);
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
        let gif_hint = if self.format == ExportMode::Gif && self.timeline_available {
            " F:FPS  L:Loop  P:Play  Space:Step  "
        } else {
            ""
        };
        lines.push(Line::from(Span::styled(
            format!(
                " T:format  Enter:export  Esc:cancel  \u{2191}\u{2193}:navigate  Tab:select{}",
                gif_hint
            ),
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

        if self.format == ExportMode::Gif && self.timeline_available {
            lines.push(Line::from(Span::styled(
                format!(" FPS: [{}]  (F to cycle preset)", self.fps),
                Style::default().fg(self.theme.dialog.meta),
            )));
            let loop_label = if self.loop_count == 0 {
                "Infinite".to_string()
            } else {
                format!("{}x", self.loop_count)
            };
            lines.push(Line::from(Span::styled(
                format!(" Loop: [{}]  (L to cycle)", loop_label),
                Style::default().fg(self.theme.dialog.meta),
            )));
            lines.push(Line::from(Span::styled(
                format!(" Frames: [{}]", self.frame_delays.len()),
                Style::default().fg(self.theme.dialog.meta),
            )));
            let play_ch = if self.preview_playing {
                "\u{23F8}"
            } else {
                "\u{25B6}"
            };
            lines.push(Line::from(Span::styled(
                format!(
                    " Preview: {} (frame {}/{})  (P to toggle, Space to step)",
                    play_ch,
                    self.preview_frame + 1,
                    self.frame_delays.len().max(1)
                ),
                Style::default().fg(self.theme.dialog.meta),
            )));
        } else if self.format == ExportMode::Gif {
            lines.push(Line::from(Span::styled(
                " Timeline: No frames available",
                Style::default().fg(self.theme.dialog.error),
            )));
        }

        lines.push(Line::from(Span::styled(
            format!(
                " Size: {}x (z = char at {})",
                self.font_size,
                8 * self.font_size as u16 * 16 * self.font_size as u16
            ),
            Style::default().fg(self.theme.dialog.meta),
        )));

        if self.format != ExportMode::Gif || !self.timeline_available {
            lines.push(Line::from(Span::styled(
                format!(
                    " Layers: [{}]  (L to toggle)",
                    if self.export_layers {
                        "Per-Layer"
                    } else {
                        "Single"
                    }
                ),
                Style::default().fg(self.theme.dialog.meta),
            )));

            lines.push(Line::from(Span::styled(
                format!(
                    " Alpha: [{}]  (P to toggle)",
                    if self.use_transparency {
                        "Transparent"
                    } else {
                        "Opaque"
                    }
                ),
                Style::default().fg(self.theme.dialog.meta),
            )));
        }

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

            let max_visible = (inner.height as usize).saturating_sub(9).min(10);
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
        let gif_hint = if self.format == ExportMode::Gif && self.timeline_available {
            " F:FPS  L:Loop  P:Play  Space:Step  "
        } else {
            ""
        };
        lines.push(Line::from(Span::styled(
            format!(
                " T:format  Enter:export  Esc:cancel  \u{2191}\u{2193}:navigate  Tab:select{}",
                gif_hint
            ),
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

    #[test]
    fn test_export_dialog_toggle_layers() {
        let mut dialog = ExportDialog::new();
        dialog.enter_export(ExportMode::Png);
        assert!(!dialog.export_layers);
        dialog.handle_key(KeyCode::Char('L'));
        assert!(dialog.export_layers);
        dialog.handle_key(KeyCode::Char('l'));
        assert!(!dialog.export_layers);
    }

    #[test]
    fn test_export_dialog_toggle_transparency() {
        let mut dialog = ExportDialog::new();
        dialog.enter_export(ExportMode::Png);
        assert!(!dialog.use_transparency);
        dialog.handle_key(KeyCode::Char('P'));
        assert!(dialog.use_transparency);
        dialog.handle_key(KeyCode::Char('p'));
        assert!(!dialog.use_transparency);
    }

    #[test]
    fn test_sanitize_layer_name_alphanumeric() {
        assert_eq!(ExportDialog::sanitize_layer_name("Layer1"), "Layer1");
    }

    #[test]
    fn test_sanitize_layer_name_strips_special_chars() {
        assert_eq!(
            ExportDialog::sanitize_layer_name("hello/world:test"),
            "helloworldtest"
        );
    }

    #[test]
    fn test_sanitize_layer_name_underscore_and_hyphen() {
        assert_eq!(
            ExportDialog::sanitize_layer_name("my_layer-v2"),
            "my_layer-v2"
        );
    }

    #[test]
    fn test_sanitize_layer_name_empty_fallback() {
        assert_eq!(ExportDialog::sanitize_layer_name("!@#$%"), "layer");
    }

    #[test]
    fn test_export_gif_fps_cycle() {
        let mut dialog = ExportDialog::new();
        dialog.enter_export(ExportMode::Gif);
        dialog.set_timeline(12, 10);
        dialog.handle_key(KeyCode::Char('F'));
        assert_eq!(dialog.fps, 24);
        dialog.handle_key(KeyCode::Char('f'));
        assert_eq!(dialog.fps, 30);
    }

    #[test]
    fn test_export_gif_loop_cycle() {
        let mut dialog = ExportDialog::new();
        dialog.enter_export(ExportMode::Gif);
        dialog.set_timeline(12, 10);
        dialog.handle_key(KeyCode::Char('L'));
        assert_eq!(dialog.loop_count, 1);
        dialog.handle_key(KeyCode::Char('l'));
        assert_eq!(dialog.loop_count, 2);
    }

    #[test]
    fn test_export_gif_preview_toggle() {
        let mut dialog = ExportDialog::new();
        dialog.enter_export(ExportMode::Gif);
        dialog.set_timeline(12, 5);
        assert!(!dialog.preview_playing);
        dialog.handle_key(KeyCode::Char('P'));
        assert!(dialog.preview_playing);
        dialog.handle_key(KeyCode::Char('p'));
        assert!(!dialog.preview_playing);
    }

    #[test]
    fn test_export_gif_frame_delays_from_fps() {
        let mut dialog = ExportDialog::new();
        dialog.enter_export(ExportMode::Gif);
        dialog.set_timeline(12, 10);
        assert_eq!(dialog.frame_delays.len(), 10);
        // FPS 12 = 100/12 = 8 cs per frame
        assert!(dialog.frame_delays.iter().all(|&d| d == 8));
        dialog.handle_key(KeyCode::Char('F'));
        // Now FPS 24 = 100/24 = 4 cs per frame
        assert!(dialog.frame_delays.iter().all(|&d| d == 4));
    }

    #[test]
    fn test_export_gif_set_timeline() {
        let mut dialog = ExportDialog::new();
        dialog.enter_export(ExportMode::Png);
        assert!(!dialog.timeline_available);
        dialog.format = ExportMode::Gif;
        dialog.set_timeline(24, 15);
        assert!(dialog.timeline_available);
        assert_eq!(dialog.fps, 24);
        assert_eq!(dialog.frame_delays.len(), 15);
        assert_eq!(dialog.frame_delays[0], 100 / 24);
    }

    #[test]
    fn test_export_gif_keys_only_in_gif_mode() {
        let mut dialog = ExportDialog::new();
        dialog.enter_export(ExportMode::Png);
        dialog.set_timeline(12, 5);
        // In PNG mode, GIF keys should not work
        dialog.handle_key(KeyCode::Char('F'));
        assert_eq!(dialog.fps, 12); // unchanged
                                    // In GIF mode with timeline, they should work
        dialog.format = ExportMode::Gif;
        dialog.handle_key(KeyCode::Char('F'));
        assert_eq!(dialog.fps, 24);
    }

    #[test]
    fn test_export_gif_preview_tick() {
        let mut dialog = ExportDialog::new();
        dialog.enter_export(ExportMode::Gif);
        dialog.set_timeline(12, 5);
        dialog.preview_playing = true;
        dialog.preview_tick();
        assert_eq!(dialog.preview_frame, 1);
        dialog.preview_tick();
        assert_eq!(dialog.preview_frame, 2);
        // Cycle around
        dialog.preview_frame = 4;
        dialog.preview_tick();
        assert_eq!(dialog.preview_frame, 0);
    }

    #[test]
    fn test_export_gif_preview_tick_only_when_playing() {
        let mut dialog = ExportDialog::new();
        dialog.enter_export(ExportMode::Gif);
        dialog.set_timeline(12, 5);
        dialog.preview_playing = false;
        dialog.preview_tick();
        assert_eq!(dialog.preview_frame, 0);
    }

    #[test]
    fn test_export_gif_space_step_when_paused() {
        let mut dialog = ExportDialog::new();
        dialog.enter_export(ExportMode::Gif);
        dialog.set_timeline(12, 5);
        dialog.preview_playing = false;
        dialog.handle_key(KeyCode::Char(' '));
        assert_eq!(dialog.preview_frame, 1);
        dialog.handle_key(KeyCode::Char(' '));
        assert_eq!(dialog.preview_frame, 2);
    }
}
