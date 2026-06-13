use crate::font::{load_font, FIGfont};
use crate::render::{add_char, Justification};
use crate::smush::SmushMode;
use crate::tui::canvas::{CanvasBuffer, CanvasCell};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

#[derive(Debug, Clone)]
pub struct TextToolState {
    pub entering_text: bool,
    pub text_buffer: String,
    pub font_index: usize,
    pub available_fonts: Vec<String>,
    pub font: Option<FIGfont>,
    pub justification: Justification,
    pub text_color: Option<Color>,
    pub scale: u8,
    pub cursor_position: (i16, i16),
    font_dir: String,
}

impl TextToolState {
    pub fn new(font_dir: &str) -> Self {
        let available = list_available_fonts(font_dir);
        Self {
            entering_text: false,
            text_buffer: String::new(),
            font_index: 0,
            available_fonts: available,
            font: None,
            justification: Justification::Left,
            text_color: None,
            scale: 1,
            cursor_position: (0, 0),
            font_dir: font_dir.to_string(),
        }
    }

    pub fn load_selected_font(&mut self) {
        if self.available_fonts.is_empty() {
            self.font = None;
            return;
        }
        let idx = self.font_index.min(self.available_fonts.len() - 1);
        let name = &self.available_fonts[idx];
        if let Ok(f) = load_font(name, &self.font_dir) {
            self.font = Some(f);
        } else {
            self.font = None;
        }
    }

    pub fn render_text_to_buffer(&mut self, buffer: &mut CanvasBuffer) {
        if self.text_buffer.is_empty() {
            return;
        }
        if self.font.is_none() {
            self.load_selected_font();
        }
        let font = match self.font.as_ref() {
            Some(f) => f,
            None => return,
        };
        if font.chars.is_empty() {
            return;
        }

        let height = font.charheight as usize;
        if height == 0 {
            return;
        }
        let mut output_rows = vec![String::new(); height];
        let mut outlinelen = 0;
        let mut prev_width = 0;
        let mode = if font.full_layout >= 0 {
            SmushMode::new(font.full_layout as u32)
        } else {
            SmushMode::new(SmushMode::KERN)
        };
        let limit = 9999;

        for c in self.text_buffer.chars() {
            let code = c as u32;
            add_char(
                font,
                code,
                &mut output_rows,
                &mut outlinelen,
                &mut prev_width,
                mode,
                false,
                limit,
            );
        }

        let width = output_rows[0].chars().count();
        if width == 0 {
            return;
        }

        let (cx, cy) = self.cursor_position;
        let left_x = match self.justification {
            Justification::Left => cx,
            Justification::Center => cx - (width as i16 / 2),
            Justification::Right => cx - width as i16,
        };

        let scale = self.scale.max(1) as i16;

        for (oy, row) in output_rows.iter().enumerate() {
            for (ox, ch) in row.chars().enumerate() {
                let cell_char = if ch == font.hardblank { ' ' } else { ch };
                if cell_char == ' ' {
                    continue;
                }
                let base_x = left_x + scale * ox as i16;
                let base_y = cy + scale * oy as i16;
                for dy in 0..scale {
                    for dx in 0..scale {
                        let bx = base_x + dx;
                        let by = base_y + dy;
                        if bx >= 0 && by >= 0 {
                            let cell = CanvasCell {
                                ch: cell_char,
                                fg: self.text_color,
                                bg: None,
                            };
                            buffer.set(bx as usize, by as usize, cell);
                        }
                    }
                }
            }
        }
    }

    pub fn render_options(&self, frame: &mut Frame<'_>, area: Rect) {
        let block = Block::default()
            .title(" Text ")
            .borders(Borders::ALL)
            .title_style(Style::default().add_modifier(Modifier::BOLD));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width < 2 || inner.height < 2 {
            return;
        }

        let mut lines: Vec<Line<'_>> = Vec::new();

        let font_name = if self.font_index < self.available_fonts.len() {
            &self.available_fonts[self.font_index]
        } else {
            "?"
        };
        lines.push(Line::from(vec![
            Span::styled("Font:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" {}", font_name)),
        ]));

        let just_str = match self.justification {
            Justification::Left => "Left",
            Justification::Center => "Center",
            Justification::Right => "Right",
        };
        lines.push(Line::from(vec![
            Span::styled("Just:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" {}", just_str)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Scale:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" {}", self.scale)),
        ]));

        let color_str = match self.text_color {
            Some(c) => format!("{:?}", c),
            None => "None".to_string(),
        };
        lines.push(Line::from(vec![
            Span::styled("Color:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" {}", color_str)),
        ]));

        lines.push(Line::from(Span::raw("")));

        if self.entering_text {
            let preview = if self.text_buffer.is_empty() {
                "(type text...)".to_string()
            } else {
                self.text_buffer.clone()
            };
            lines.push(Line::from(vec![Span::styled(
                "Input:",
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(Span::raw(format!(" {}", preview))));
        } else {
            let hint = if self.available_fonts.is_empty() {
                "No fonts found"
            } else {
                "Click canvas to type"
            };
            lines.push(Line::from(Span::raw(format!(" {}", hint))));
        }

        let max_lines = inner.height as usize;
        let truncated: Vec<Line<'_>> = lines.into_iter().take(max_lines).collect();
        let paragraph = Paragraph::new(truncated);
        frame.render_widget(paragraph, inner);
    }
}

impl Default for TextToolState {
    fn default() -> Self {
        Self::new("fonts")
    }
}

pub fn list_available_fonts(font_dir: &str) -> Vec<String> {
    let mut fonts = Vec::new();
    if let Ok(entries) = std::fs::read_dir(font_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "flf" || ext == "tlf" {
                    if let Some(stem) = path.file_stem() {
                        let name = stem.to_string_lossy().to_string();
                        fonts.push(name);
                    }
                }
            }
        }
    }
    fonts.sort();
    fonts.dedup();
    fonts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::canvas::CanvasBuffer;
    use ratatui::style::Color;

    fn test_font_dir() -> &'static str {
        "fonts"
    }

    #[test]
    fn test_list_fonts_nonempty() {
        let fonts = list_available_fonts(test_font_dir());
        assert!(!fonts.is_empty(), "expected at least one font");
        assert!(
            fonts.contains(&"standard".to_string()),
            "expected 'standard' font"
        );
    }

    #[test]
    fn test_list_fonts_nonexistent_dir() {
        let fonts = list_available_fonts("/nonexistent/path");
        assert!(fonts.is_empty());
    }

    #[test]
    fn test_text_tool_initial_state() {
        let state = TextToolState::new(test_font_dir());
        assert!(!state.entering_text);
        assert!(state.text_buffer.is_empty());
        assert_eq!(state.scale, 1);
        assert_eq!(state.justification, Justification::Left);
        assert!(state.text_color.is_none());
    }

    #[test]
    fn test_text_tool_render_single_char() {
        let mut state = TextToolState::new(test_font_dir());
        if !state.available_fonts.contains(&"standard".to_string()) {
            return;
        }
        state.font_index = state
            .available_fonts
            .iter()
            .position(|n| n == "standard")
            .unwrap_or(0);
        state.load_selected_font();
        assert!(state.font.is_some(), "standard font should load");

        state.text_buffer = "A".to_string();
        state.cursor_position = (0, 0);

        let mut buf = CanvasBuffer::new(80, 40);
        state.render_text_to_buffer(&mut buf);

        let has_content = (0..buf.height())
            .any(|y| (0..buf.width()).any(|x| buf.get(x, y).is_some_and(|c| c.ch != ' ')));
        assert!(has_content, "FIGlet 'A' should produce non-space output");
    }

    #[test]
    fn test_text_tool_render_multi_char() {
        let mut state = TextToolState::new(test_font_dir());
        if !state.available_fonts.contains(&"standard".to_string()) {
            return;
        }
        state.font_index = state
            .available_fonts
            .iter()
            .position(|n| n == "standard")
            .unwrap_or(0);
        state.load_selected_font();
        assert!(state.font.is_some());

        state.text_buffer = "Hi".to_string();
        state.cursor_position = (0, 0);

        let mut buf = CanvasBuffer::new(80, 40);
        state.render_text_to_buffer(&mut buf);

        let has_content = (0..buf.height())
            .any(|y| (0..buf.width()).any(|x| buf.get(x, y).is_some_and(|c| c.ch != ' ')));
        assert!(has_content, "FIGlet 'Hi' should produce non-space output");
    }

    #[test]
    fn test_text_tool_justification_left() {
        let mut state = TextToolState::new(test_font_dir());
        if !state.available_fonts.contains(&"standard".to_string()) {
            return;
        }
        state.font_index = state
            .available_fonts
            .iter()
            .position(|n| n == "standard")
            .unwrap_or(0);
        state.load_selected_font();
        state.text_buffer = "A".to_string();
        state.cursor_position = (10, 5);
        state.justification = Justification::Left;

        let mut buf = CanvasBuffer::new(80, 40);
        state.render_text_to_buffer(&mut buf);

        let mut min_x = 999;
        for y in 0..buf.height() {
            for x in 0..buf.width() {
                if buf.get(x, y).is_some_and(|c| c.ch != ' ') {
                    min_x = min_x.min(x);
                }
            }
        }
        assert_eq!(min_x, 10, "left-justified text should start at cursor x");
    }

    #[test]
    fn test_text_tool_justification_right() {
        let mut state = TextToolState::new(test_font_dir());
        if !state.available_fonts.contains(&"standard".to_string()) {
            return;
        }
        state.font_index = state
            .available_fonts
            .iter()
            .position(|n| n == "standard")
            .unwrap_or(0);
        state.load_selected_font();
        state.text_buffer = "A".to_string();
        state.cursor_position = (20, 5);
        state.justification = Justification::Right;

        let mut buf = CanvasBuffer::new(80, 40);
        state.render_text_to_buffer(&mut buf);

        let mut max_x = 0usize;
        for y in 0..buf.height() {
            for x in 0..buf.width() {
                if buf.get(x, y).is_some_and(|c| c.ch != ' ') {
                    max_x = max_x.max(x);
                }
            }
        }
        assert!(
            max_x <= 20,
            "right-justified text should end at or before cursor x (max_x={max_x}, cursor_x=20)"
        );
    }

    #[test]
    fn test_text_tool_justification_center() {
        let mut state = TextToolState::new(test_font_dir());
        if !state.available_fonts.contains(&"standard".to_string()) {
            return;
        }
        state.font_index = state
            .available_fonts
            .iter()
            .position(|n| n == "standard")
            .unwrap_or(0);
        state.load_selected_font();
        state.text_buffer = "A".to_string();
        state.cursor_position = (30, 5);
        state.justification = Justification::Center;

        let mut buf = CanvasBuffer::new(80, 40);
        state.render_text_to_buffer(&mut buf);

        let mut min_x = 999;
        let mut max_x = 0usize;
        for y in 0..buf.height() {
            for x in 0..buf.width() {
                if buf.get(x, y).is_some_and(|c| c.ch != ' ') {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                }
            }
        }
        if max_x >= min_x {
            let center = (min_x + max_x) / 2;
            let diff = (center as i16 - 30).abs();
            assert!(
                diff <= 5,
                "centered text center ({center}) should be near cursor x (30), diff={diff}"
            );
        }
    }

    #[test]
    fn test_text_tool_color_applied() {
        let mut state = TextToolState::new(test_font_dir());
        if !state.available_fonts.contains(&"standard".to_string()) {
            return;
        }
        state.font_index = state
            .available_fonts
            .iter()
            .position(|n| n == "standard")
            .unwrap_or(0);
        state.load_selected_font();
        state.text_buffer = "A".to_string();
        state.cursor_position = (0, 0);
        state.text_color = Some(Color::Red);

        let mut buf = CanvasBuffer::new(80, 40);
        state.render_text_to_buffer(&mut buf);

        let has_red = (0..buf.height()).any(|y| {
            (0..buf.width()).any(|x| buf.get(x, y).is_some_and(|c| c.fg == Some(Color::Red)))
        });
        assert!(has_red, "Some cells should have red foreground");
    }

    #[test]
    fn test_text_tool_font_switching() {
        let mut state = TextToolState::new(test_font_dir());
        let mut buf = CanvasBuffer::new(80, 40);

        if state.available_fonts.is_empty() {
            return;
        }
        state.font_index = 0;
        state.load_selected_font();

        state.font_index = state.available_fonts.len() - 1;
        state.load_selected_font();
        assert!(state.font.is_some(), "last font should load successfully");

        state.text_buffer = "A".to_string();
        state.cursor_position = (0, 0);
        state.render_text_to_buffer(&mut buf);

        let has_content = (0..buf.height())
            .any(|y| (0..buf.width()).any(|x| buf.get(x, y).is_some_and(|c| c.ch != ' ')));
        assert!(has_content, "text should render after font switch");
    }

    #[test]
    fn test_text_tool_scale_factor() {
        let mut state = TextToolState::new(test_font_dir());
        if !state.available_fonts.contains(&"standard".to_string()) {
            return;
        }
        state.font_index = state
            .available_fonts
            .iter()
            .position(|n| n == "standard")
            .unwrap_or(0);
        state.load_selected_font();
        state.text_buffer = "A".to_string();
        state.cursor_position = (0, 0);
        state.scale = 2;

        let mut buf = CanvasBuffer::new(80, 40);
        state.render_text_to_buffer(&mut buf);

        let count = (0..buf.height())
            .flat_map(|y| (0..buf.width()).map(move |x| (x, y)))
            .filter(|&(x, y)| buf.get(x, y).is_some_and(|c| c.ch != ' '))
            .count();
        assert!(count >= 2, "scale=2 should produce at least 2 cells");
    }

    #[test]
    fn test_text_tool_clips_at_buffer_edge() {
        let mut state = TextToolState::new(test_font_dir());
        if !state.available_fonts.contains(&"standard".to_string()) {
            return;
        }
        state.font_index = state
            .available_fonts
            .iter()
            .position(|n| n == "standard")
            .unwrap_or(0);
        state.load_selected_font();
        state.text_buffer = "Hello World!".to_string();
        state.cursor_position = (75, 35);
        let mut buf = CanvasBuffer::new(80, 40);
        state.render_text_to_buffer(&mut buf);
    }

    #[test]
    fn test_text_tool_empty_text_noop() {
        let mut state = TextToolState::new(test_font_dir());
        let mut buf = CanvasBuffer::new(10, 10);
        state.render_text_to_buffer(&mut buf);

        for y in 0..buf.height() {
            for x in 0..buf.width() {
                assert_eq!(buf.get(x, y).unwrap().ch, ' ');
            }
        }
    }

    #[test]
    fn test_text_tool_no_font_no_panic() {
        let mut state = TextToolState::new("/nonexistent/dir");
        state.text_buffer = "Hello".to_string();
        state.cursor_position = (0, 0);
        let mut buf = CanvasBuffer::new(10, 10);
        state.render_text_to_buffer(&mut buf);
    }

    #[test]
    fn test_text_tool_entering_text_state() {
        let mut state = TextToolState::new(test_font_dir());
        assert!(!state.entering_text);
        state.entering_text = true;
        assert!(state.entering_text);
        state.text_buffer.push('H');
        state.text_buffer.push('i');
        assert_eq!(state.text_buffer, "Hi");
        state.entering_text = false;
        assert!(!state.entering_text);
    }
}
