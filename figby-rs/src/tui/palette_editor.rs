use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::palette::Palette;
use super::theme::Theme;
use crate::palette_import::{self, ImportFormat, Swatch};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaletteFile {
    pub name: String,
    pub swatches: Vec<Swatch>,
}

enum PanelMode {
    Idle,
    Naming,
    Loading,
    ChoosingFormat,
}

pub struct PaletteEditor {
    pub open: bool,
    pub name_buffer: String,
    pub swatches: Vec<Swatch>,
    pub selected: usize,
    mode: PanelMode,
    pub file_list: Vec<PathBuf>,
    pub file_scroll: usize,
    pub message: Option<String>,
    pub modified: bool,
    pub import_format: Option<ImportFormat>,
    format_index: usize,
}

impl PaletteEditor {
    pub fn new() -> Self {
        Self {
            open: false,
            name_buffer: String::new(),
            swatches: Vec::new(),
            selected: 0,
            mode: PanelMode::Idle,
            file_list: Vec::new(),
            file_scroll: 0,
            message: None,
            modified: false,
            import_format: None,
            format_index: 0,
        }
    }

    pub fn load_current_from_palette(&mut self, palette: &Palette) {
        self.swatches.clear();
        self.name_buffer.clear();
        self.modified = true;
        let names = super::palette::ANSI_COLOR_NAMES;
        if palette.recent.is_empty() {
            for (i, color) in super::palette::ANSI_16_COLORS.iter().enumerate() {
                let name = names.get(i).copied().unwrap_or("").to_string();
                let hex = color_to_hex(*color);
                self.swatches.push(Swatch { name, hex });
            }
        } else {
            for (i, color) in palette.recent.iter().enumerate() {
                let name = names.get(i).copied().unwrap_or("Custom").to_string();
                let hex = color_to_hex(*color);
                self.swatches.push(Swatch { name, hex });
            }
        }
    }

    fn palettes_dir() -> Option<PathBuf> {
        let base = if let Ok(val) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(val)
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config")
        } else {
            return None;
        };
        let path = base.join("figby").join("palettes");
        if let Err(e) = std::fs::create_dir_all(&path) {
            eprintln!("Failed to create palettes dir: {e}");
            return None;
        }
        Some(path)
    }

    pub fn available_palettes(&mut self, format: Option<ImportFormat>) {
        self.file_list.clear();
        self.file_scroll = 0;
        if let Some(dir) = Self::palettes_dir() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|s| s.to_ascii_lowercase());
                    match format {
                        Some(ImportFormat::AdobeAse) => {
                            if ext.as_deref() == Some("ase") {
                                self.file_list.push(path);
                            }
                        }
                        Some(
                            ImportFormat::Native
                            | ImportFormat::PalettyJson
                            | ImportFormat::WezTermJson
                            | ImportFormat::WindowsTerminalJson,
                        ) => {
                            if ext.as_deref() == Some("json") {
                                self.file_list.push(path);
                            }
                        }
                        None => {
                            if ext.as_deref() == Some("json") || ext.as_deref() == Some("ase") {
                                self.file_list.push(path);
                            }
                        }
                    }
                }
                self.file_list.sort();
            }
        }
    }

    pub fn save(&mut self) -> Result<(), String> {
        let name = self.name_buffer.trim();
        if name.is_empty() {
            return Err("Palette name is empty".to_string());
        }
        if name.contains('/') || name.contains("..") || name.contains('\\') {
            return Err("Invalid palette name".to_string());
        }
        let dir = Self::palettes_dir().ok_or("Cannot find config directory")?;
        let path = dir.join(format!("{}.json", name));
        let file = PaletteFile {
            name: name.to_string(),
            swatches: self.swatches.clone(),
        };
        let json =
            serde_json::to_string_pretty(&file).map_err(|e| format!("Serialization error: {e}"))?;
        std::fs::write(&path, &json).map_err(|e| format!("Write error: {e}"))?;
        Ok(())
    }

    pub fn load_file(&mut self, path: &Path) -> Result<(), String> {
        let format = self.import_format;
        self.load_file_with_format(path, format)
    }

    fn load_file_with_format(
        &mut self,
        path: &Path,
        format: Option<ImportFormat>,
    ) -> Result<(), String> {
        let content = std::fs::read(path).map_err(|e| format!("Read error: {e}"))?;
        let format = match format {
            Some(f) => f,
            None => {
                let ext = path.extension().and_then(|e| e.to_str());
                palette_import::auto_detect_format(&content, ext).unwrap_or(ImportFormat::Native)
            }
        };
        let swatches = match format {
            ImportFormat::Native => {
                let s = String::from_utf8(content).map_err(|e| format!("UTF-8 error: {e}"))?;
                let file: PaletteFile =
                    serde_json::from_str(&s).map_err(|e| format!("Parse error: {e}"))?;
                self.name_buffer = file.name;
                file.swatches
            }
            _ => palette_import::import_swatches(&content, format)?,
        };
        self.swatches = swatches;
        if self.name_buffer.is_empty() {
            self.name_buffer = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
        }
        self.message = Some(format!(
            "Loaded {}",
            path.file_stem().unwrap_or_default().to_string_lossy()
        ));
        self.modified = true;
        Ok(())
    }

    pub fn duplicate(&mut self, new_name: &str) -> Result<(), String> {
        let name = new_name.trim();
        if name.is_empty() {
            return Err("New name is empty".to_string());
        }
        self.name_buffer = name.to_string();
        self.save()
    }

    pub fn apply_to_palette(&self, palette: &mut Palette) {
        palette.recent.clear();
        for swatch in &self.swatches {
            if let Some(color) = hex_to_color(&swatch.hex) {
                palette.push_recent(color);
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.open {
            return;
        }

        let rect = super::layout::palette_editor_overlay(area);
        frame.render_widget(Clear, rect);

        let mut lines: Vec<Line> = Vec::new();

        let name_display = if self.name_buffer.is_empty() {
            " <unnamed>".to_string()
        } else {
            format!(" {}", self.name_buffer)
        };
        lines.push(Line::from(Span::styled(
            name_display,
            Style::default().fg(theme.menu.fg),
        )));
        lines.push(Line::from(""));

        for (i, swatch) in self.swatches.iter().enumerate() {
            let indicator = if i == self.selected { ">" } else { " " };
            let swatch_bg = hex_to_color(&swatch.hex).unwrap_or(Color::Reset);
            let fg = luminance(swatch_bg).map_or(Color::White, |l| {
                if l > 128 {
                    Color::Black
                } else {
                    Color::White
                }
            });
            let color_span = Span::styled(
                format!(" {} ", swatch.hex),
                Style::default().bg(swatch_bg).fg(fg),
            );
            lines.push(Line::from(vec![
                Span::styled(indicator, Style::default().fg(theme.dialog.highlight)),
                color_span,
                Span::raw(format!(" {} ({})", swatch.name, swatch.hex)),
            ]));
        }

        lines.push(Line::from(""));

        if let PanelMode::ChoosingFormat = self.mode {
            lines.push(Line::from(Span::styled(
                " Select import format:",
                Style::default().fg(theme.dialog.label),
            )));
            lines.push(Line::from(vec![
                Span::styled(
                    if self.format_index == 0 { ">" } else { " " },
                    Style::default().fg(theme.dialog.highlight),
                ),
                Span::raw(" [Auto-Detect]"),
            ]));
            for (i, fmt) in ImportFormat::all().iter().enumerate() {
                let indicator = if self.format_index == i + 1 { ">" } else { " " };
                lines.push(Line::from(vec![
                    Span::styled(indicator, Style::default().fg(theme.dialog.highlight)),
                    Span::raw(format!(" {}", fmt.display_name())),
                ]));
            }
        }

        if let PanelMode::Loading = self.mode {
            lines.push(Line::from(Span::styled(
                " Available palettes:",
                Style::default().fg(theme.dialog.label),
            )));
            for (i, path) in self.file_list.iter().enumerate() {
                let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                let indicator = if i == self.file_scroll { ">" } else { " " };
                lines.push(Line::from(vec![
                    Span::styled(indicator, Style::default().fg(theme.dialog.highlight)),
                    Span::raw(format!(" {} ", stem)),
                ]));
            }
        }

        if let Some(ref msg) = self.message {
            let msg_style = if msg.starts_with("Save error")
                || msg.starts_with("Read error")
                || msg.starts_with("Parse error")
                || msg.starts_with("Write error")
                || msg.starts_with("Serialization error")
            {
                Style::default().fg(theme.general.error)
            } else {
                Style::default().fg(theme.general.success)
            };
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(format!(" {}", msg), msg_style)));
        }

        lines.push(Line::from(""));
        let format_hint = match self.import_format {
            Some(f) => format!(
                " [S]ave  [L]oad ({})  [D]uplicate  Esc=close",
                f.display_name()
            ),
            None => " [S]ave  [L]oad  [D]uplicate  Esc=close".to_string(),
        };
        lines.push(Line::from(Span::styled(
            format_hint,
            Style::default().fg(theme.dialog.meta),
        )));

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .title(" Palette Editor ")
                .borders(Borders::ALL),
        );
        frame.render_widget(paragraph, rect);
    }

    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        match self.mode {
            PanelMode::Naming => match code {
                KeyCode::Char(c) => {
                    self.name_buffer.push(c);
                    true
                }
                KeyCode::Backspace => {
                    self.name_buffer.pop();
                    true
                }
                KeyCode::Enter => {
                    let name = self.name_buffer.trim().to_string();
                    if !name.is_empty() {
                        let result = self.duplicate(&name);
                        if let Err(e) = result {
                            self.message = Some(format!("Duplicate error: {e}"));
                        } else {
                            self.message = Some("Duplicated".to_string());
                        }
                    }
                    self.mode = PanelMode::Idle;
                    true
                }
                KeyCode::Esc => {
                    self.mode = PanelMode::Idle;
                    true
                }
                _ => false,
            },
            PanelMode::ChoosingFormat => match code {
                KeyCode::Up => {
                    self.format_index = self.format_index.saturating_sub(1);
                    true
                }
                KeyCode::Down => {
                    let max = ImportFormat::all().len(); // 5 formats
                    self.format_index = self.format_index.saturating_add(1).min(max);
                    true
                }
                KeyCode::Enter => {
                    let selected_format = if self.format_index == 0 {
                        None
                    } else {
                        Some(ImportFormat::all()[self.format_index - 1])
                    };
                    self.import_format = selected_format;
                    self.available_palettes(selected_format);
                    if self.file_list.is_empty() {
                        let hint = match selected_format {
                            Some(f) => format!("No {} files found", f.display_name()),
                            None => "No palette files found".to_string(),
                        };
                        self.message = Some(hint);
                        self.mode = PanelMode::Idle;
                    } else {
                        self.mode = PanelMode::Loading;
                        self.file_scroll = 0;
                    }
                    true
                }
                KeyCode::Esc => {
                    self.mode = PanelMode::Idle;
                    true
                }
                _ => false,
            },
            PanelMode::Loading => match code {
                KeyCode::Up => {
                    self.file_scroll = self.file_scroll.saturating_sub(1);
                    true
                }
                KeyCode::Down => {
                    self.file_scroll = self
                        .file_scroll
                        .saturating_add(1)
                        .min(self.file_list.len().saturating_sub(1));
                    true
                }
                KeyCode::Enter => {
                    if let Some(path) = self.file_list.get(self.file_scroll).cloned() {
                        if let Err(e) = self.load_file(&path) {
                            self.message = Some(e);
                        }
                    }
                    self.mode = PanelMode::Idle;
                    true
                }
                KeyCode::Esc => {
                    self.mode = PanelMode::Idle;
                    true
                }
                _ => false,
            },
            PanelMode::Idle => match code {
                KeyCode::Esc => {
                    self.open = false;
                    true
                }
                KeyCode::Up => {
                    self.selected = self.selected.saturating_sub(1);
                    true
                }
                KeyCode::Down => {
                    self.selected = self
                        .selected
                        .saturating_add(1)
                        .min(self.swatches.len().saturating_sub(1));
                    true
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    match self.save() {
                        Ok(()) => self.message = Some("Saved".to_string()),
                        Err(e) => self.message = Some(format!("Save error: {e}")),
                    }
                    true
                }
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    self.mode = PanelMode::ChoosingFormat;
                    self.format_index = 0;
                    true
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.mode = PanelMode::Naming;
                    self.name_buffer.clear();
                    self.message = Some("Enter new name for duplicate".to_string());
                    true
                }
                _ => false,
            },
        }
    }
}

impl Default for PaletteEditor {
    fn default() -> Self {
        Self::new()
    }
}

fn color_to_hex(color: Color) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("#{:02X}{:02X}{:02X}", r, g, b),
        Color::Indexed(i) => {
            // Approximate indexed color to RGB for storage
            let (r, g, b) = ansi_to_rgb(i);
            format!("#{:02X}{:02X}{:02X}", r, g, b)
        }
        _ => "#000000".to_string(),
    }
}

fn hex_to_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

fn ansi_to_rgb(index: u8) -> (u8, u8, u8) {
    match index {
        0 => (0, 0, 0),
        1 => (128, 0, 0),
        2 => (0, 128, 0),
        3 => (128, 128, 0),
        4 => (0, 0, 128),
        5 => (128, 0, 128),
        6 => (0, 128, 128),
        7 => (192, 192, 192),
        8 => (128, 128, 128),
        9 => (255, 0, 0),
        10 => (0, 255, 0),
        11 => (255, 255, 0),
        12 => (0, 0, 255),
        13 => (255, 0, 255),
        14 => (0, 255, 255),
        15 => (255, 255, 255),
        // Extended colors: 216-color cube (6x6x6) + grayscale ramp
        _ if index < 232 => {
            let n = index - 16;
            let r = n / 36;
            let g = (n % 36) / 6;
            let b = n % 6;
            let to_byte = |v: u8| -> u8 {
                match v {
                    0 => 0,
                    1 => 95,
                    2 => 135,
                    3 => 175,
                    4 => 215,
                    5 => 255,
                    _ => 0,
                }
            };
            (to_byte(r), to_byte(g), to_byte(b))
        }
        _ => {
            let gray = 8 + (index - 232) * 10;
            (gray, gray, gray)
        }
    }
}

fn luminance(color: Color) -> Option<u8> {
    match color {
        Color::Rgb(r, g, b) => Some(((r as u16 + g as u16 + b as u16) / 3) as u8),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::palette::Palette;

    #[test]
    fn test_palette_file_roundtrip() {
        let swatches = vec![
            Swatch {
                name: "Red".to_string(),
                hex: "#FF0000".to_string(),
            },
            Swatch {
                name: "Green".to_string(),
                hex: "#00FF00".to_string(),
            },
        ];
        let file = PaletteFile {
            name: "test".to_string(),
            swatches: swatches.clone(),
        };
        let json = serde_json::to_string(&file).unwrap();
        let decoded: PaletteFile = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.name, "test");
        assert_eq!(decoded.swatches.len(), 2);
        assert_eq!(decoded.swatches[0].hex, "#FF0000");
        assert_eq!(decoded.swatches[1].name, "Green");
    }

    #[test]
    fn test_palette_duplicate_independence() {
        let swatches_a = vec![Swatch {
            name: "Red".to_string(),
            hex: "#FF0000".to_string(),
        }];
        let mut a = PaletteFile {
            name: "A".to_string(),
            swatches: swatches_a,
        };
        let b = PaletteFile {
            name: "B".to_string(),
            swatches: a.swatches.clone(),
        };
        a.swatches[0].hex = "#00FF00".to_string();
        assert_eq!(b.swatches[0].hex, "#FF0000");
    }

    #[test]
    fn test_save_load_disk_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let orig = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::set_var("XDG_CONFIG_HOME", dir.path());

        let mut editor = PaletteEditor::new();
        editor.swatches = vec![Swatch {
            name: "Blue".to_string(),
            hex: "#0000FF".to_string(),
        }];
        editor.name_buffer = "test_palette".to_string();

        // Save
        editor.save().unwrap();

        // Load into a fresh editor
        let palettes_dir = PaletteEditor::palettes_dir().unwrap();
        let path = palettes_dir.join("test_palette.json");
        let mut editor2 = PaletteEditor::new();
        editor2.load_file(&path).unwrap();
        assert_eq!(editor2.swatches.len(), 1);
        assert_eq!(editor2.swatches[0].hex, "#0000FF");
        assert_eq!(editor2.swatches[0].name, "Blue");

        // Cleanup
        match orig {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }

    #[test]
    fn test_load_current_from_palette() {
        let mut palette = Palette::new();
        palette.push_recent(Color::Rgb(255, 0, 0));
        palette.push_recent(Color::Rgb(0, 255, 0));

        let mut editor = PaletteEditor::new();
        editor.load_current_from_palette(&palette);
        assert_eq!(editor.swatches.len(), 2);
        assert_eq!(editor.swatches[0].hex, "#FF0000");
        assert_eq!(editor.swatches[1].hex, "#00FF00");
    }

    #[test]
    fn test_palettes_dir_creates() {
        let dir = tempfile::tempdir().unwrap();
        let orig = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::set_var("XDG_CONFIG_HOME", dir.path());

        let path = PaletteEditor::palettes_dir().unwrap();
        assert!(path.exists());
        assert!(path.to_string_lossy().contains("figby/palettes"));

        match orig {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }

    #[test]
    fn test_hex_color_conversion_roundtrip() {
        let hex = "#AABBCC";
        let color = hex_to_color(hex).unwrap();
        let hex2 = color_to_hex(color);
        assert_eq!(hex, hex2);
    }

    #[test]
    fn test_apply_to_palette() {
        let mut editor = PaletteEditor::new();
        editor.swatches = vec![Swatch {
            name: "Custom".to_string(),
            hex: "#FF00FF".to_string(),
        }];

        let mut palette = Palette::new();
        editor.apply_to_palette(&mut palette);
        assert_eq!(palette.recent.len(), 1);
    }

    #[test]
    fn test_save_rejects_path_traversal() {
        let mut editor = PaletteEditor::new();
        editor.name_buffer = "../evil".to_string();
        assert!(editor.save().is_err());
        editor.name_buffer = "good/name".to_string();
        assert!(editor.save().is_err());
        editor.name_buffer = "".to_string();
        assert!(editor.save().is_err());
    }
}
