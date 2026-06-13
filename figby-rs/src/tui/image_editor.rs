use crossterm::event::KeyCode;
use ratatui::style::Color;

use crate::image_input::{
    bilinear_resize_rgb, load_rgb_matrix, luminance_to_char, RgbPixel, DEFAULT_CHAR_MAP,
};
use crate::tui::canvas::CanvasCell;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsciiMode {
    Color,
    Grayscale,
}

pub struct ImageEditor {
    cells: Vec<Vec<CanvasCell>>,
    mode: AsciiMode,
    source_path: Option<PathBuf>,
    original_rgb: Option<Vec<Vec<RgbPixel>>>,
    target_width: usize,
    entering_path: bool,
    path_buffer: String,
    error_message: Option<String>,
}

impl ImageEditor {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            mode: AsciiMode::Grayscale,
            source_path: None,
            original_rgb: None,
            target_width: 80,
            entering_path: false,
            path_buffer: String::new(),
            error_message: None,
        }
    }

    pub fn cells(&self) -> Option<&Vec<Vec<CanvasCell>>> {
        if self.cells.is_empty() {
            None
        } else {
            Some(&self.cells)
        }
    }

    pub fn mode(&self) -> AsciiMode {
        self.mode
    }

    pub fn entering_path(&self) -> bool {
        self.entering_path
    }

    pub fn path_buffer(&self) -> &str {
        &self.path_buffer
    }

    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    pub fn has_cells(&self) -> bool {
        !self.cells.is_empty()
    }

    pub fn load_from_path(&mut self, path: &str) -> Result<(), String> {
        let path_buf = PathBuf::from(path);

        let rgb = load_rgb_matrix(&path_buf).map_err(|e| format!("Failed to load image: {e}"))?;

        if rgb.is_empty() || rgb[0].is_empty() {
            return Err("Loaded image is empty".to_string());
        }

        let src_h = rgb.len();
        let src_w = rgb[0].len();
        let target_height = ((self.target_width as f64 * src_h as f64 / src_w as f64) * 0.5)
            .ceil()
            .max(1.0) as usize;

        let resized_rgb = bilinear_resize_rgb(&rgb, self.target_width, target_height);
        self.original_rgb = Some(rgb);
        self.source_path = Some(path_buf);
        self.cells = Self::rgb_to_cells(&resized_rgb, self.mode);
        self.error_message = None;
        Ok(())
    }

    fn rgb_to_cells(rgb: &[Vec<RgbPixel>], mode: AsciiMode) -> Vec<Vec<CanvasCell>> {
        let mut cells = Vec::with_capacity(rgb.len());
        for row in rgb {
            let mut cell_row = Vec::with_capacity(row.len());
            for &(r, g, b) in row {
                let luma =
                    (0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64).round() as u8;
                let ch = luminance_to_char(luma, DEFAULT_CHAR_MAP);
                let cell = match mode {
                    AsciiMode::Color => CanvasCell {
                        ch,
                        fg: Some(Color::Rgb(r, g, b)),
                        bg: None,
                    },
                    AsciiMode::Grayscale => CanvasCell {
                        ch,
                        fg: None,
                        bg: None,
                    },
                };
                cell_row.push(cell);
            }
            cells.push(cell_row);
        }
        cells
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            AsciiMode::Color => AsciiMode::Grayscale,
            AsciiMode::Grayscale => AsciiMode::Color,
        };
        if let Some(ref rgb) = self.original_rgb {
            let src_h = rgb.len();
            let src_w = rgb[0].len();
            let target_height = ((self.target_width as f64 * src_h as f64 / src_w as f64) * 0.5)
                .ceil()
                .max(1.0) as usize;
            let resized = bilinear_resize_rgb(rgb, self.target_width, target_height);
            self.cells = Self::rgb_to_cells(&resized, self.mode);
        }
    }

    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        if self.entering_path {
            match code {
                KeyCode::Char(c) => {
                    self.path_buffer.push(c);
                }
                KeyCode::Enter => {
                    let path = std::mem::take(&mut self.path_buffer);
                    if !path.is_empty() {
                        if let Err(e) = self.load_from_path(&path) {
                            self.error_message = Some(e);
                        }
                    }
                    self.entering_path = false;
                }
                KeyCode::Esc => {
                    self.entering_path = false;
                    self.path_buffer.clear();
                }
                KeyCode::Backspace => {
                    self.path_buffer.pop();
                }
                _ => {}
            }
            return true;
        }

        match code {
            KeyCode::Char('o') | KeyCode::Char('O') => {
                self.entering_path = true;
                self.path_buffer.clear();
                self.error_message = None;
                true
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.toggle_mode();
                true
            }
            _ => false,
        }
    }
}

impl Default for ImageEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_input::{color_matrix_to_ascii, load_rgb_matrix, ImageColorConfig};

    const TEST_PNG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/img/figby.png");

    #[test]
    fn test_image_editor_load_grayscale() {
        let mut editor = ImageEditor::new();
        editor.target_width = 40;
        editor.load_from_path(TEST_PNG).expect("failed to load PNG");
        assert!(editor.has_cells(), "should have cells after load");
        let cells = editor.cells().unwrap();
        assert!(!cells.is_empty(), "cells should not be empty");
        assert!(!cells[0].is_empty(), "first row should not be empty");
        for row in cells {
            for cell in row {
                assert!(
                    DEFAULT_CHAR_MAP.contains(cell.ch),
                    "char '{}' not in default map",
                    cell.ch
                );
                assert_eq!(cell.fg, None, "grayscale mode should have no fg color");
            }
        }
    }

    #[test]
    fn test_image_editor_load_color() {
        let mut editor = ImageEditor::new();
        editor.mode = AsciiMode::Color;
        editor.target_width = 40;
        editor.load_from_path(TEST_PNG).expect("failed to load PNG");
        assert!(editor.has_cells(), "should have cells after load");
        let cells = editor.cells().unwrap();
        assert!(!cells.is_empty(), "cells should not be empty");
        for row in cells {
            for cell in row {
                assert!(
                    DEFAULT_CHAR_MAP.contains(cell.ch),
                    "char '{}' not in default map",
                    cell.ch
                );
                assert!(
                    cell.fg.is_some(),
                    "color mode should have fg color for each cell"
                );
            }
        }
    }

    #[test]
    fn test_image_editor_nonexistent_path() {
        let mut editor = ImageEditor::new();
        let result = editor.load_from_path("/nonexistent/path/image.png");
        assert!(result.is_err(), "expected error for nonexistent path");
    }

    #[test]
    fn test_image_editor_mode_toggle() {
        let mut editor = ImageEditor::new();
        editor.target_width = 40;
        editor.load_from_path(TEST_PNG).expect("failed to load PNG");
        assert_eq!(editor.mode, AsciiMode::Grayscale);
        assert!(
            editor
                .cells()
                .unwrap()
                .iter()
                .all(|row| row.iter().all(|c| c.fg.is_none())),
            "all cells should have no fg in grayscale"
        );

        editor.toggle_mode();
        assert_eq!(editor.mode, AsciiMode::Color);
        assert!(
            editor
                .cells()
                .unwrap()
                .iter()
                .all(|row| row.iter().all(|c| c.fg.is_some())),
            "all cells should have fg in color mode"
        );

        editor.toggle_mode();
        assert_eq!(editor.mode, AsciiMode::Grayscale);
        assert!(
            editor
                .cells()
                .unwrap()
                .iter()
                .all(|row| row.iter().all(|c| c.fg.is_none())),
            "all cells should have no fg after toggling back to grayscale"
        );
    }

    #[test]
    fn test_image_editor_matches_cli_output() {
        let mut editor = ImageEditor::new();
        editor.target_width = 40;
        editor.load_from_path(TEST_PNG).expect("failed to load PNG");

        let rgb = load_rgb_matrix(TEST_PNG).expect("failed to load RGB matrix");
        let expected = color_matrix_to_ascii(
            &rgb,
            &ImageColorConfig {
                target_width: Some(40),
                ..Default::default()
            },
        );

        let cells = editor.cells().unwrap();
        let canvas_output: String = cells
            .iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(
            canvas_output, expected,
            "canvas output should match color_matrix_to_ascii output (same RGB→resize→luma pipeline)"
        );
    }

    #[test]
    fn test_image_editor_render_to_canvas() {
        let mut editor = ImageEditor::new();
        editor.target_width = 40;
        editor.load_from_path(TEST_PNG).expect("failed to load PNG");
        let cells = editor.cells().unwrap().clone();
        let h = cells.len();
        let w = cells[0].len();

        let mut canvas = crate::tui::canvas::CanvasBuffer::new(w, h);
        for (y, row) in cells.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                canvas.set(x, y, *cell);
            }
        }

        assert_eq!(canvas.width(), w);
        assert_eq!(canvas.height(), h);
        for (y, row) in cells.iter().enumerate() {
            for (x, src) in row.iter().enumerate() {
                let dst = canvas.get(x, y).expect("cell should exist");
                assert_eq!(src.ch, dst.ch, "char mismatch at ({x},{y})");
                assert_eq!(src.fg, dst.fg, "fg mismatch at ({x},{y})");
            }
        }
    }

    #[test]
    fn test_image_editor_key_path_entry() {
        let mut editor = ImageEditor::new();
        assert!(!editor.entering_path());

        assert!(editor.handle_key(KeyCode::Char('o')));
        assert!(editor.entering_path());
        assert!(editor.path_buffer().is_empty());

        editor.handle_key(KeyCode::Char('t'));
        editor.handle_key(KeyCode::Char('e'));
        editor.handle_key(KeyCode::Char('s'));
        editor.handle_key(KeyCode::Char('t'));
        assert_eq!(editor.path_buffer(), "test");

        editor.handle_key(KeyCode::Backspace);
        assert_eq!(editor.path_buffer(), "tes");

        editor.handle_key(KeyCode::Esc);
        assert!(!editor.entering_path());
        assert!(editor.path_buffer().is_empty());
    }

    #[test]
    fn test_image_editor_key_mode_toggle() {
        let mut editor = ImageEditor::new();
        editor.target_width = 40;
        editor.load_from_path(TEST_PNG).expect("failed to load PNG");
        assert_eq!(editor.mode(), AsciiMode::Grayscale);

        assert!(editor.handle_key(KeyCode::Char('c')));
        assert_eq!(editor.mode(), AsciiMode::Color);

        assert!(editor.handle_key(KeyCode::Char('C')));
        assert_eq!(editor.mode(), AsciiMode::Grayscale);
    }
}
