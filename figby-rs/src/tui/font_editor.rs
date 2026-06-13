use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::font::FIGfont;
use crate::smush::{smush_horizontal, SmushMode};

const SMUSH_RULE_LABELS: [(&str, u32); 6] = [
    ("Equal Char", SmushMode::EQUAL_CHARS),
    ("Underscore", SmushMode::UNDERSCORE),
    ("Hierarchy", SmushMode::HIERARCHY),
    ("Pair", SmushMode::PAIR),
    ("Big X", SmushMode::BIGX),
    ("Hardblank", SmushMode::HARDBLANK),
];

const HEADER_FIELD_LABELS: [&str; 7] = [
    "Hardblank",
    "Char Height",
    "Baseline",
    "Max Length",
    "Full Layout",
    "Print Direction",
    "Comment Lines",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontEditorView {
    Overview,
    CharEditor(u32),
    HeaderEditor,
    SmushRuleEditor,
}

pub struct FontEditor {
    pub font: Option<FIGfont>,
    pub view: FontEditorView,
    pub search_query: String,
    pub search_active: bool,
    pub grid_scroll: u16,
    pub selected_index: usize,
    all_codes: Vec<u32>,
    undo_stack: Vec<Vec<String>>,
    redo_stack: Vec<Vec<String>>,
    pub selected_field: usize,
    pub editing_field: bool,
    pub edit_buffer: String,
    pub error_message: String,
    pub smush_selected: usize,
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
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            selected_field: 0,
            editing_field: false,
            edit_buffer: String::new(),
            error_message: String::new(),
            smush_selected: 0,
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
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.smush_selected = 0;
    }

    pub fn enter_header_editor(&mut self) {
        self.view = FontEditorView::HeaderEditor;
        self.selected_field = 0;
        self.editing_field = false;
        self.edit_buffer.clear();
        self.error_message.clear();
    }

    pub fn enter_smush_editor(&mut self) {
        self.view = FontEditorView::SmushRuleEditor;
        self.smush_selected = 0;
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
            FontEditorView::HeaderEditor => self.render_header_editor(frame, area),
            FontEditorView::SmushRuleEditor => self.render_smush_editor(frame, area),
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

    fn render_smush_editor(&mut self, frame: &mut Frame, area: Rect) {
        let Some(font) = self.font.as_ref() else {
            return;
        };

        let layout = font.full_layout as u32;
        let mode = SmushMode::new(layout);

        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            " Smushing Rules",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        for (i, &(label, bit)) in SMUSH_RULE_LABELS.iter().enumerate() {
            let is_enabled = layout & bit == bit;
            let is_selected = i == self.smush_selected;
            let checkbox = if is_enabled { "[X]" } else { "[ ]" };
            let prefix = if is_selected { ">" } else { " " };
            let text = format!("{} {} {}", prefix, checkbox, label);
            let style = if is_selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(text, style)));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " Preview",
            Style::default().add_modifier(Modifier::BOLD),
        )));

        let result = smush_horizontal('/', '\\', mode, font.hardblank, false);
        let preview = match result {
            Some(ch) => format!("/ + \\ = {}", ch),
            None => "/ + \\ = (no smush)".to_string(),
        };
        lines.push(Line::from(Span::raw(format!(" {}", preview))));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(" Layout value: {} (0b{:08b})", layout, layout),
            Style::default().fg(Color::DarkGray),
        )));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " \u{2191}\u{2193}: Navigate  Enter/Space: Toggle  Esc: Back",
            Style::default().fg(Color::DarkGray),
        )));

        let paragraph = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));
        frame.render_widget(paragraph, area);
    }

    fn render_header_editor(&mut self, frame: &mut Frame, area: Rect) {
        let Some(font) = self.font.as_ref() else {
            return;
        };

        let field_values: [String; 7] = [
            font.hardblank.to_string(),
            font.charheight.to_string(),
            font.baseline.to_string(),
            font.maxlength.to_string(),
            font.full_layout.to_string(),
            font.print_direction.to_string(),
            font.comment_lines.to_string(),
        ];

        let labels = HEADER_FIELD_LABELS;
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            " FIGfont Header Properties",
            Style::default().add_modifier(Modifier::BOLD),
        )));

        for (i, &label) in labels.iter().enumerate() {
            let is_selected = i == self.selected_field;
            let value = if self.editing_field && i == self.selected_field {
                self.edit_buffer.clone()
            } else {
                field_values[i].clone()
            };

            let prefix = if is_selected && !self.editing_field {
                " >"
            } else if is_selected && self.editing_field {
                ">>"
            } else {
                "  "
            };

            let text = format!("{} {}: {}", prefix, label, value);

            let style = if is_selected {
                if self.editing_field {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().add_modifier(Modifier::REVERSED)
                }
            } else {
                Style::default()
            };

            lines.push(Line::from(Span::styled(text, style)));
        }

        if !self.error_message.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!(" Error: {}", self.error_message),
                Style::default().fg(Color::Red),
            )));
        }

        lines.push(Line::from(""));
        if self.editing_field {
            lines.push(Line::from(Span::styled(
                " Enter: Save  Esc: Cancel  Backspace: Delete char",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                " \u{2191}\u{2193}: Navigate  Enter: Edit  Esc: Back",
                Style::default().fg(Color::DarkGray),
            )));
        }

        let paragraph = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));
        frame.render_widget(paragraph, area);
    }

    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers, area_width: u16) -> bool {
        match self.view {
            FontEditorView::Overview => self.handle_key_overview(code, area_width),
            FontEditorView::CharEditor(_) => self.handle_key_char_editor(code, modifiers),
            FontEditorView::HeaderEditor => self.handle_key_header_editor(code),
            FontEditorView::SmushRuleEditor => self.handle_key_smush_editor(code),
        }
    }

    fn handle_key_char_editor(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        match code {
            KeyCode::Esc => {
                self.view = FontEditorView::Overview;
                true
            }
            KeyCode::Char('z') if modifiers.contains(KeyModifiers::CONTROL) => self.undo_char(),
            KeyCode::Char('y') if modifiers.contains(KeyModifiers::CONTROL) => self.redo_char(),
            _ => false,
        }
    }

    fn handle_key_header_editor(&mut self, code: KeyCode) -> bool {
        if self.editing_field {
            match code {
                KeyCode::Enter => {
                    self.error_message.clear();
                    if self.save_current_field() {
                        self.editing_field = false;
                    }
                    true
                }
                KeyCode::Esc => {
                    self.editing_field = false;
                    self.edit_buffer.clear();
                    self.error_message.clear();
                    true
                }
                KeyCode::Char(c) if !c.is_control() => {
                    self.edit_buffer.push(c);
                    self.error_message.clear();
                    true
                }
                KeyCode::Backspace => {
                    self.edit_buffer.pop();
                    self.error_message.clear();
                    true
                }
                _ => false,
            }
        } else {
            match code {
                KeyCode::Up => {
                    if self.selected_field > 0 {
                        self.selected_field -= 1;
                    }
                    self.error_message.clear();
                    true
                }
                KeyCode::Down => {
                    if self.selected_field < 6 {
                        self.selected_field += 1;
                    }
                    self.error_message.clear();
                    true
                }
                KeyCode::Enter => {
                    self.start_editing_field();
                    true
                }
                KeyCode::Esc => {
                    self.view = FontEditorView::Overview;
                    self.error_message.clear();
                    true
                }
                _ => false,
            }
        }
    }

    fn handle_key_smush_editor(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Up => {
                if self.smush_selected > 0 {
                    self.smush_selected -= 1;
                } else {
                    self.smush_selected = SMUSH_RULE_LABELS.len() - 1;
                }
                true
            }
            KeyCode::Down => {
                self.smush_selected = (self.smush_selected + 1) % SMUSH_RULE_LABELS.len();
                true
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let Some(font) = self.font.as_mut() else {
                    return true;
                };
                let bit = SMUSH_RULE_LABELS[self.smush_selected].1;
                font.full_layout ^= bit as i32;
                true
            }
            KeyCode::Esc => {
                self.view = FontEditorView::Overview;
                true
            }
            _ => false,
        }
    }

    fn start_editing_field(&mut self) {
        let Some(font) = &self.font else { return };
        self.editing_field = true;
        self.edit_buffer = match self.selected_field {
            0 => font.hardblank.to_string(),
            1 => font.charheight.to_string(),
            2 => font.baseline.to_string(),
            3 => font.maxlength.to_string(),
            4 => font.full_layout.to_string(),
            5 => font.print_direction.to_string(),
            6 => font.comment_lines.to_string(),
            _ => String::new(),
        };
        self.error_message.clear();
    }

    fn save_current_field(&mut self) -> bool {
        let val = self.edit_buffer.trim().to_string();
        if val.is_empty() {
            self.error_message = "Value cannot be empty".to_string();
            return false;
        }

        let field = self.selected_field;
        let Some(font) = self.font.as_mut() else {
            return false;
        };

        match field {
            0 => {
                let chars: Vec<char> = val.chars().collect();
                if chars.len() == 1 {
                    font.hardblank = chars[0];
                    true
                } else {
                    self.error_message = "Hardblank must be a single character".to_string();
                    false
                }
            }
            1 => match val.parse::<u32>() {
                Ok(v) if v >= 1 => {
                    font.charheight = v;
                    true
                }
                Ok(_) => {
                    self.error_message = "Height must be \u{2265} 1".to_string();
                    false
                }
                Err(_) => {
                    self.error_message = "Invalid number".to_string();
                    false
                }
            },
            2 => match val.parse::<u32>() {
                Ok(v) if v <= font.charheight => {
                    font.baseline = v;
                    true
                }
                Ok(_) => {
                    self.error_message = "Baseline must be \u{2264} height".to_string();
                    false
                }
                Err(_) => {
                    self.error_message = "Invalid number".to_string();
                    false
                }
            },
            3 => match val.parse::<u32>() {
                Ok(v) => {
                    font.maxlength = v;
                    true
                }
                Err(_) => {
                    self.error_message = "Invalid number".to_string();
                    false
                }
            },
            4 => match val.parse::<i32>() {
                Ok(v) => {
                    font.full_layout = v;
                    true
                }
                Err(_) => {
                    self.error_message = "Invalid number".to_string();
                    false
                }
            },
            5 => match val.parse::<i32>() {
                Ok(v) if v == -1 || v == 0 || v == 1 => {
                    font.print_direction = v;
                    true
                }
                Ok(_) => {
                    self.error_message = "Print direction must be -1, 0, or 1".to_string();
                    false
                }
                Err(_) => {
                    self.error_message = "Invalid number".to_string();
                    false
                }
            },
            6 => match val.parse::<u32>() {
                Ok(v) => {
                    font.comment_lines = v;
                    true
                }
                Err(_) => {
                    self.error_message = "Invalid number".to_string();
                    false
                }
            },
            _ => false,
        }
    }

    fn undo_char(&mut self) -> bool {
        let FontEditorView::CharEditor(code) = self.view else {
            return false;
        };
        let Some(font) = self.font.as_mut() else {
            return false;
        };
        let Some(ch) = font.chars.get_mut(&code) else {
            return false;
        };

        if let Some(restored) = self.undo_stack.pop() {
            self.redo_stack.push(ch.rows().to_vec());
            ch.set_rows(restored);
            true
        } else {
            false
        }
    }

    fn redo_char(&mut self) -> bool {
        let FontEditorView::CharEditor(code) = self.view else {
            return false;
        };
        let Some(font) = self.font.as_mut() else {
            return false;
        };
        let Some(ch) = font.chars.get_mut(&code) else {
            return false;
        };

        if let Some(restored) = self.redo_stack.pop() {
            self.undo_stack.push(ch.rows().to_vec());
            ch.set_rows(restored);
            true
        } else {
            false
        }
    }

    pub fn sync_from_canvas(&mut self, code: u32, buffer: &super::canvas::CanvasBuffer) {
        let Some(font) = self.font.as_mut() else {
            return;
        };
        let Some(ch) = font.chars.get_mut(&code) else {
            return;
        };

        let w = buffer.width();
        let h = buffer.height();
        let mut new_rows: Vec<String> = Vec::with_capacity(h);
        for y in 0..h {
            let mut row = String::with_capacity(w);
            for x in 0..w {
                let c = buffer.get(x, y).map_or(' ', |cell| cell.ch);
                row.push(c);
            }
            new_rows.push(row);
        }

        if ch.rows() != new_rows.as_slice() {
            let old = ch.rows().to_vec();
            if self.undo_stack.last() != Some(&old) {
                self.undo_stack.push(old);
            }
            self.redo_stack.clear();
            ch.set_rows(new_rows);
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
            KeyCode::Char('H') => {
                self.enter_header_editor();
                true
            }
            KeyCode::Char('S') => {
                self.enter_smush_editor();
                true
            }
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
            FontEditorView::HeaderEditor => None,
            FontEditorView::SmushRuleEditor => None,
        }
    }
}

impl Default for FontEditor {
    fn default() -> Self {
        Self::new()
    }
}
