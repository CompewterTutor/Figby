use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::font::FIGfont;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontEditorView {
    Overview,
    CharEditor(u32),
}

pub struct FontEditor {
    pub font: Option<FIGfont>,
    pub view: FontEditorView,
    pub search_query: String,
    pub search_active: bool,
    pub grid_scroll: u16,
    pub selected_index: usize,
    all_codes: Vec<u32>,
}

impl FontEditor {
    pub fn new() -> Self {
        Self {
            font: None,
            view: FontEditorView::Overview,
            search_query: String::new(),
            search_active: false,
            grid_scroll: 0,
            selected_index: 0,
            all_codes: Vec::new(),
        }
    }

    pub fn load_font(&mut self, font: FIGfont) {
        let mut codes: Vec<u32> = font.chars.keys().copied().collect();
        codes.sort();
        self.all_codes = codes;
        self.font = Some(font);
        self.search_active = false;
        self.grid_scroll = 0;
        self.selected_index = 0;
        self.view = FontEditorView::Overview;
    }

    pub fn filtered_codes(&self) -> Vec<u32> {
        let query = self.search_query.trim();
        if query.is_empty() {
            return self.all_codes.clone();
        }
        let q = query.to_lowercase();
        self.all_codes
            .iter()
            .copied()
            .filter(|&code| {
                if format!("{}", code).contains(&q) {
                    return true;
                }
                if let Some(ch) = char::from_u32(code) {
                    if ch.to_lowercase().to_string().contains(&q) {
                        return true;
                    }
                }
                false
            })
            .collect()
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        match self.view {
            FontEditorView::Overview => self.render_overview(frame, area),
            FontEditorView::CharEditor(_) => {}
        }
    }

    fn render_overview(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let search_display = if self.search_active {
            format!(" Search: {}", self.search_query)
        } else {
            " Search (type to filter by code or char)".to_string()
        };
        let search = Paragraph::new(search_display).block(Block::default().borders(Borders::ALL));
        frame.render_widget(search, chunks[0]);

        let grid_area = chunks[1];
        let filtered = self.filtered_codes();

        if filtered.is_empty() {
            let msg = Paragraph::new(" No characters match search.")
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(msg, grid_area);
            return;
        }

        let Some(font) = &self.font else { return };

        let cell_w = (font.maxlength as u16 + 2).max(8) as usize;
        let cell_h = (font.charheight as u16 + 1) as usize;
        let cols = (grid_area.width as usize / cell_w).max(1);

        let start_cell = self.grid_scroll as usize * cols;

        let mut lines: Vec<Line> = Vec::new();
        let mut cell_idx = start_cell;

        while cell_idx < filtered.len() && lines.len() + cell_h <= grid_area.height as usize {
            let end = (cell_idx + cols).min(filtered.len());
            let chunk = &filtered[cell_idx..end];

            let mut code_spans = Vec::new();
            for (ci, &code) in chunk.iter().enumerate() {
                let abs_idx = cell_idx + ci;
                let is_selected = abs_idx == self.selected_index;
                let label = format!("{:>4}", code);
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };
                let mut text = String::with_capacity(cell_w);
                text.push_str(&label);
                text.push(' ');
                while text.len() < cell_w {
                    text.push(' ');
                }
                code_spans.push(Span::styled(text, style));
            }
            lines.push(Line::from(code_spans));

            for row in 0..font.charheight as usize {
                let mut row_spans = Vec::new();
                for &code in chunk {
                    let row_text = font
                        .chars
                        .get(&code)
                        .and_then(|c| c.rows().get(row))
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    let mut text = String::with_capacity(cell_w);
                    text.push(' ');
                    let display_len = row_text.len().min(cell_w.saturating_sub(1));
                    text.push_str(&row_text[..display_len]);
                    while text.len() < cell_w {
                        text.push(' ');
                    }
                    row_spans.push(Span::raw(text));
                }
                lines.push(Line::from(row_spans));
            }

            cell_idx = end;
        }

        let grid = Paragraph::new(lines);
        frame.render_widget(grid, grid_area);
    }

    pub fn handle_key(&mut self, code: KeyCode, area_width: u16) -> bool {
        match self.view {
            FontEditorView::Overview => self.handle_key_overview(code, area_width),
            FontEditorView::CharEditor(_) => match code {
                KeyCode::Esc => {
                    self.view = FontEditorView::Overview;
                    true
                }
                _ => false,
            },
        }
    }

    fn handle_key_overview(&mut self, code: KeyCode, area_width: u16) -> bool {
        let filtered = self.filtered_codes();
        let cols = self.compute_cols(area_width);

        match code {
            // '/' activates search mode
            KeyCode::Char('/') if !self.search_active => {
                self.search_active = true;
                self.grid_scroll = 0;
                self.selected_index = 0;
                true
            }
            // When search active: all printable chars build the query
            KeyCode::Char(c) if !c.is_control() && self.search_active => {
                self.search_query.push(c);
                self.grid_scroll = 0;
                self.selected_index = 0;
                true
            }
            // Backspace: when search active, pop last char
            KeyCode::Backspace if self.search_active => {
                self.search_query.pop();
                self.grid_scroll = 0;
                self.selected_index = 0;
                true
            }
            // Backspace with empty search: no-op (don't fall through to canvas delete)
            KeyCode::Backspace => true,
            // Grid navigation
            KeyCode::Up => {
                if !filtered.is_empty() && self.selected_index >= cols {
                    self.selected_index -= cols;
                }
                true
            }
            KeyCode::Down => {
                if !filtered.is_empty() {
                    let new_idx =
                        (self.selected_index + cols).min(filtered.len().saturating_sub(1));
                    self.selected_index = new_idx;
                }
                true
            }
            KeyCode::Left => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                true
            }
            KeyCode::Right => {
                if !filtered.is_empty() {
                    let new_idx = (self.selected_index + 1).min(filtered.len().saturating_sub(1));
                    self.selected_index = new_idx;
                }
                true
            }
            // Enter selects highlighted char
            KeyCode::Enter => {
                if !filtered.is_empty() && self.selected_index < filtered.len() {
                    self.view = FontEditorView::CharEditor(filtered[self.selected_index]);
                }
                true
            }
            // Esc clears search if active, else falls through
            KeyCode::Esc if self.search_active => {
                self.search_query.clear();
                self.search_active = false;
                self.grid_scroll = 0;
                self.selected_index = 0;
                true
            }
            KeyCode::Esc => false,
            // All other keys fall through to normal handlers
            _ => false,
        }
    }

    fn compute_cols(&self, area_width: u16) -> usize {
        let Some(font) = &self.font else { return 1 };
        let cell_w = (font.maxlength as u16 + 2).max(8);
        (area_width / cell_w).max(1) as usize
    }

    pub fn selected_char(&self) -> Option<(u32, &crate::font::FIGcharacter)> {
        match self.view {
            FontEditorView::CharEditor(code) => self
                .font
                .as_ref()
                .and_then(|f| f.chars.get(&code))
                .map(|ch| (code, ch)),
            FontEditorView::Overview => None,
        }
    }
}

impl Default for FontEditor {
    fn default() -> Self {
        Self::new()
    }
}
