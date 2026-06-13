use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use std::path::PathBuf;

use crate::font::{load_font, FIGfont};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorMode {
    Horizontal,
    Vertical,
    Both,
}

impl MirrorMode {
    fn label(&self) -> &str {
        match self {
            MirrorMode::Horizontal => "Horizontal",
            MirrorMode::Vertical => "Vertical",
            MirrorMode::Both => "Both",
        }
    }

    fn next(&self) -> Self {
        match self {
            MirrorMode::Horizontal => MirrorMode::Vertical,
            MirrorMode::Vertical => MirrorMode::Both,
            MirrorMode::Both => MirrorMode::Horizontal,
        }
    }

    fn prev(&self) -> Self {
        match self {
            MirrorMode::Horizontal => MirrorMode::Both,
            MirrorMode::Vertical => MirrorMode::Horizontal,
            MirrorMode::Both => MirrorMode::Vertical,
        }
    }
}

const TRANSFORM_LABELS: [&str; 6] = [
    "Resize Font",
    "Italicize",
    "Bold",
    "Mirror",
    "Copy Glyph",
    "Rename",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontEditorView {
    Overview,
    CharEditor(u32),
    HeaderEditor,
    SmushRuleEditor,
    TransformEditor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeInputMode {
    Add,
    CopySource,
    CopyDest,
    DeleteConfirm,
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
    pub code_input_active: bool,
    pub code_input_buffer: String,
    pub code_input_mode: CodeInputMode,
    pub copy_source_code: u32,
    pub selected_transform: usize,
    pub input_buffer: String,
    pub input_active: bool,
    pub sub_step: u8,
    pub transform_submode: Option<MirrorMode>,
    pub transform_font_name: String,
    pub font_storage_name: String,
    pub current_path: Option<PathBuf>,
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
            code_input_active: false,
            code_input_buffer: String::new(),
            code_input_mode: CodeInputMode::Add,
            copy_source_code: 0,
            selected_transform: 0,
            input_buffer: String::new(),
            input_active: false,
            sub_step: 0,
            transform_submode: None,
            transform_font_name: String::new(),
            font_storage_name: String::new(),
            current_path: None,
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
        self.code_input_active = false;
        self.code_input_buffer.clear();
        self.selected_transform = 0;
        self.input_buffer.clear();
        self.input_active = false;
        self.sub_step = 0;
        self.transform_submode = None;
        self.transform_font_name.clear();
        self.font_storage_name.clear();
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

    pub fn enter_transform_editor(&mut self) {
        self.view = FontEditorView::TransformEditor;
        self.selected_transform = 0;
        self.input_active = false;
        self.input_buffer.clear();
        self.sub_step = 0;
        self.transform_submode = None;
        self.transform_font_name.clear();
        self.error_message.clear();
    }

    fn is_valid_code(code: u32) -> bool {
        code <= 0x10FFFF && !(0xD800..=0xDFFF).contains(&code)
    }

    fn rebuild_all_codes(&mut self) {
        let Some(font) = self.font.as_ref() else {
            self.all_codes.clear();
            return;
        };
        let mut codes: Vec<u32> = font.chars.keys().copied().collect();
        codes.sort();
        self.all_codes = codes;
    }

    fn ensure_missing_char(&mut self) {
        let Some(font) = self.font.as_mut() else {
            return;
        };
        if !font.chars.contains_key(&0) {
            let rows: Vec<String> = (0..font.charheight)
                .map(|_| " ".repeat(font.maxlength as usize))
                .collect();
            font.chars.insert(0, rows.into());
        }
    }

    pub fn add_char(&mut self, code: u32) {
        if !Self::is_valid_code(code) {
            self.error_message = format!("Invalid code point: U+{code:X}");
            return;
        }
        let Some(font) = self.font.as_mut() else {
            return;
        };
        if font.chars.contains_key(&code) {
            self.error_message = format!("Code U+{code:X} already exists");
            return;
        }
        let rows: Vec<String> = (0..font.charheight)
            .map(|_| " ".repeat(font.maxlength as usize))
            .collect();
        font.chars.insert(code, rows.into());
        self.rebuild_all_codes();
        self.view = FontEditorView::Overview;
        self.selected_index = self.all_codes.iter().position(|&c| c == code).unwrap_or(0);
        self.error_message.clear();
    }

    pub fn delete_char(&mut self, code: u32) {
        let Some(font) = self.font.as_mut() else {
            return;
        };
        if !font.chars.contains_key(&code) {
            self.error_message = format!("Code U+{code:X} not found");
            return;
        }
        font.chars.remove(&code);
        self.ensure_missing_char();
        self.rebuild_all_codes();
        self.view = FontEditorView::Overview;
        if self.selected_index >= self.all_codes.len() {
            self.selected_index = self.all_codes.len().saturating_sub(1);
        }
        self.error_message.clear();
    }

    pub fn copy_char(&mut self, src: u32, dst: u32) {
        if !Self::is_valid_code(dst) {
            self.error_message = format!("Invalid destination code point: U+{dst:X}");
            return;
        }
        let Some(font) = self.font.as_mut() else {
            return;
        };
        let src_rows = font
            .chars
            .get(&src)
            .map(|c| c.rows().to_vec())
            .unwrap_or_else(|| {
                (0..font.charheight)
                    .map(|_| " ".repeat(font.maxlength as usize))
                    .collect()
            });
        font.chars.insert(dst, src_rows.into());
        self.rebuild_all_codes();
        self.view = FontEditorView::Overview;
        self.selected_index = self.all_codes.iter().position(|&c| c == dst).unwrap_or(0);
        self.error_message.clear();
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
            FontEditorView::TransformEditor => self.render_transform_editor(frame, area),
        }
    }

    fn render_overview(&mut self, frame: &mut Frame, area: Rect) {
        let prompt_height: u16 = 3;
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(prompt_height), Constraint::Min(0)].as_ref())
            .split(area);

        if self.code_input_active {
            let prompt = match self.code_input_mode {
                CodeInputMode::Add => format!(" Add code: {}", self.code_input_buffer),
                CodeInputMode::CopySource => {
                    format!(" Copy from code: {}", self.code_input_buffer)
                }
                CodeInputMode::CopyDest => {
                    format!(" Copy to code: {}", self.code_input_buffer)
                }
                CodeInputMode::DeleteConfirm => {
                    let selected_code = self
                        .filtered_codes()
                        .get(self.selected_index)
                        .copied()
                        .unwrap_or(0);
                    format!(
                        " Delete char U+{code:X} ({code})? (Y/N): {buf}",
                        code = selected_code,
                        buf = self.code_input_buffer
                    )
                }
            };
            let search = Paragraph::new(prompt).block(Block::default().borders(Borders::ALL));
            frame.render_widget(search, chunks[0]);
        } else {
            let search_display = if self.search_active {
                format!(" Search: {}", self.search_query)
            } else {
                " Search (type to filter by code or char)".to_string()
            };
            let search =
                Paragraph::new(search_display).block(Block::default().borders(Borders::ALL));
            frame.render_widget(search, chunks[0]);
        }

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

    fn render_transform_editor(&mut self, frame: &mut Frame, area: Rect) {
        let _ = &self.font;
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            " Font Transforms",
            Style::default().add_modifier(Modifier::BOLD),
        )));

        if self.input_active {
            let prompt = match self.selected_transform {
                0 => format!(" New height: {}", self.input_buffer),
                4 => {
                    if self.sub_step == 0 {
                        format!(" Font name: {}", self.input_buffer)
                    } else {
                        format!(" Code point: {}", self.input_buffer)
                    }
                }
                5 => format!(" New name: {}", self.input_buffer),
                _ => String::new(),
            };
            lines.push(Line::from(""));
            lines.push(Line::from(Span::raw(format!(" {}", prompt))));
            if !self.error_message.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!(" Error: {}", self.error_message),
                    Style::default().fg(Color::Red),
                )));
            }
            lines.push(Line::from(Span::styled(
                " Enter: Confirm  Esc: Cancel  Backspace: Delete char",
                Style::default().fg(Color::DarkGray),
            )));
        } else if let Some(mode) = self.transform_submode {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                " Select mirror mode:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for &m in &[
                MirrorMode::Horizontal,
                MirrorMode::Vertical,
                MirrorMode::Both,
            ] {
                let is_selected = m == mode;
                let prefix = if is_selected { " >" } else { "  " };
                let text = format!("{} {}", prefix, m.label());
                let style = if is_selected {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };
                lines.push(Line::from(Span::styled(text, style)));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                " \u{2191}\u{2193}: Navigate  Enter: Apply  Esc: Back",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(""));
            for (i, label) in TRANSFORM_LABELS.iter().enumerate() {
                let is_selected = i == self.selected_transform;
                let prefix = if is_selected { " >" } else { "  " };
                let text = format!("{} {}", prefix, label);
                let style = if is_selected {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };
                lines.push(Line::from(Span::styled(text, style)));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                " \u{2191}\u{2193}: Navigate  Enter: Select  Esc: Back",
                Style::default().fg(Color::DarkGray),
            )));
        }

        if !self.error_message.is_empty() && !self.input_active {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!(" Error: {}", self.error_message),
                Style::default().fg(Color::Red),
            )));
        }

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
            FontEditorView::TransformEditor => self.handle_key_transform_editor(code),
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

    fn handle_key_transform_editor(&mut self, code: KeyCode) -> bool {
        if self.input_active {
            match code {
                KeyCode::Char(c) if !c.is_control() => {
                    self.input_buffer.push(c);
                    true
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                    true
                }
                KeyCode::Esc => {
                    self.input_active = false;
                    self.input_buffer.clear();
                    self.sub_step = 0;
                    self.error_message.clear();
                    true
                }
                KeyCode::Enter => {
                    let buf = self.input_buffer.trim().to_string();
                    match self.selected_transform {
                        0 => {
                            if let Ok(h) = buf.parse::<u32>() {
                                if h == 0 {
                                    self.error_message = "Height must be \u{2265} 1".to_string();
                                } else {
                                    self.transform_resize(h);
                                }
                            } else {
                                self.error_message = "Invalid height".to_string();
                            }
                        }
                        4 => {
                            if self.sub_step == 0 {
                                if buf.is_empty() {
                                    self.error_message = "Font name required".to_string();
                                } else {
                                    self.transform_font_name = buf;
                                    self.sub_step = 1;
                                    self.input_buffer.clear();
                                    self.error_message.clear();
                                    return true;
                                }
                            } else {
                                if let Ok(code) = buf.parse::<u32>() {
                                    let name = self.transform_font_name.clone();
                                    self.transform_copy_glyph_from(&name, "fonts", code);
                                } else {
                                    self.error_message = "Invalid code point".to_string();
                                }
                            }
                        }
                        5 => {
                            if buf.is_empty() {
                                self.error_message = "Name cannot be empty".to_string();
                            } else {
                                self.transform_rename(&buf);
                            }
                        }
                        _ => {}
                    }
                    self.input_active = false;
                    self.input_buffer.clear();
                    self.sub_step = 0;
                    true
                }
                _ => false,
            }
        } else if let Some(current) = self.transform_submode {
            match code {
                KeyCode::Up => {
                    self.transform_submode = Some(current.prev());
                    true
                }
                KeyCode::Down => {
                    self.transform_submode = Some(current.next());
                    true
                }
                KeyCode::Enter => {
                    self.transform_mirror(current);
                    self.transform_submode = None;
                    true
                }
                KeyCode::Esc => {
                    self.transform_submode = None;
                    true
                }
                _ => false,
            }
        } else {
            match code {
                KeyCode::Up => {
                    self.selected_transform = if self.selected_transform > 0 {
                        self.selected_transform - 1
                    } else {
                        TRANSFORM_LABELS.len() - 1
                    };
                    self.error_message.clear();
                    true
                }
                KeyCode::Down => {
                    self.selected_transform =
                        (self.selected_transform + 1) % TRANSFORM_LABELS.len();
                    self.error_message.clear();
                    true
                }
                KeyCode::Enter => {
                    match self.selected_transform {
                        0 => {
                            self.input_active = true;
                            self.input_buffer.clear();
                            self.error_message.clear();
                        }
                        1 => self.transform_italicize(),
                        2 => self.transform_bold(),
                        3 => {
                            self.transform_submode = Some(MirrorMode::Horizontal);
                            self.error_message.clear();
                        }
                        4 => {
                            self.input_active = true;
                            self.input_buffer.clear();
                            self.sub_step = 0;
                            self.error_message.clear();
                        }
                        5 => {
                            self.input_active = true;
                            self.input_buffer.clear();
                            self.error_message.clear();
                        }
                        _ => {}
                    }
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

    fn transform_resize(&mut self, new_height: u32) {
        let Some(font) = self.font.as_mut() else {
            return;
        };
        if new_height == 0 || new_height == font.charheight {
            if new_height == 0 {
                self.error_message = "Height must be \u{2265} 1".to_string();
            }
            return;
        }
        let old_height = font.charheight;
        font.charheight = new_height;
        font.baseline = font.baseline.min(new_height);
        for ch in font.chars.values_mut() {
            let mut rows = ch.rows().to_vec();
            if new_height > old_height {
                while rows.len() < new_height as usize {
                    rows.push(" ".repeat(font.maxlength as usize));
                }
            } else {
                rows.truncate(new_height as usize);
            }
            ch.set_rows(rows);
        }
        let maxlen = font
            .chars
            .values()
            .map(|c| c.width() as u32)
            .max()
            .unwrap_or(1);
        font.maxlength = maxlen;
    }

    fn transform_italicize(&mut self) {
        let Some(font) = self.font.as_mut() else {
            return;
        };
        for ch in font.chars.values_mut() {
            let rows = ch.rows().to_vec();
            let new_rows: Vec<String> = rows
                .iter()
                .enumerate()
                .map(|(i, row)| {
                    let prefix: String = (0..i).map(|_| ' ').collect();
                    format!("{prefix}{row}")
                })
                .collect();
            ch.set_rows(new_rows);
        }
        let maxlen = font
            .chars
            .values()
            .map(|c| c.width() as u32)
            .max()
            .unwrap_or(1);
        font.maxlength = maxlen;
    }

    fn transform_bold(&mut self) {
        let Some(font) = self.font.as_mut() else {
            return;
        };
        for ch in font.chars.values_mut() {
            let rows = ch.rows().to_vec();
            let new_rows: Vec<String> = rows
                .iter()
                .map(|row| row.chars().flat_map(|c| [c, c]).collect())
                .collect();
            ch.set_rows(new_rows);
        }
        let maxlen = font
            .chars
            .values()
            .map(|c| c.width() as u32)
            .max()
            .unwrap_or(1);
        font.maxlength = maxlen;
    }

    fn transform_mirror(&mut self, mode: MirrorMode) {
        let Some(font) = self.font.as_mut() else {
            return;
        };
        for ch in font.chars.values_mut() {
            let rows = ch.rows().to_vec();
            let new_rows: Vec<String> = match mode {
                MirrorMode::Horizontal => rows.iter().map(|r| r.chars().rev().collect()).collect(),
                MirrorMode::Vertical => rows.into_iter().rev().collect(),
                MirrorMode::Both => rows
                    .into_iter()
                    .rev()
                    .map(|r| r.chars().rev().collect())
                    .collect(),
            };
            ch.set_rows(new_rows);
        }
        let maxlen = font
            .chars
            .values()
            .map(|c| c.width() as u32)
            .max()
            .unwrap_or(1);
        font.maxlength = maxlen;
    }

    fn transform_copy_glyph_from(&mut self, font_source: &str, fontdir: &str, code: u32) {
        let external = match load_font(font_source, fontdir) {
            Ok(f) => f,
            Err(_) => {
                self.error_message = format!("Could not load font: {font_source}");
                return;
            }
        };
        let Some(src_ch) = external.chars.get(&code) else {
            self.error_message = format!("Code U+{code:X} not found in font '{font_source}'");
            return;
        };
        let rows: Vec<String> = src_ch.rows().to_vec();
        let Some(font) = self.font.as_mut() else {
            return;
        };
        font.chars.insert(code, rows.into());
        self.rebuild_all_codes();
    }

    fn transform_rename(&mut self, new_name: &str) {
        self.font_storage_name = new_name.to_string();
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

        if self.code_input_active {
            return self.handle_key_code_input(code);
        }

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
            KeyCode::Char('T') => {
                self.enter_transform_editor();
                true
            }
            KeyCode::Char('A') => {
                self.code_input_active = true;
                self.code_input_mode = CodeInputMode::Add;
                self.code_input_buffer.clear();
                self.error_message.clear();
                true
            }
            KeyCode::Char('D') => {
                self.code_input_active = true;
                self.code_input_mode = CodeInputMode::DeleteConfirm;
                self.code_input_buffer.clear();
                self.error_message.clear();
                true
            }
            KeyCode::Char('C') => {
                self.code_input_active = true;
                self.code_input_mode = CodeInputMode::CopySource;
                self.code_input_buffer.clear();
                self.error_message.clear();
                true
            }
            // All other keys fall through to normal handlers
            _ => false,
        }
    }

    fn handle_key_code_input(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char(c)
                if c.is_ascii_digit()
                    || (self.code_input_mode == CodeInputMode::DeleteConfirm
                        && matches!(c, 'y' | 'Y' | 'n' | 'N')) =>
            {
                self.code_input_buffer.push(c);
                true
            }
            KeyCode::Backspace => {
                self.code_input_buffer.pop();
                true
            }
            KeyCode::Esc => {
                self.code_input_active = false;
                self.code_input_buffer.clear();
                self.error_message.clear();
                true
            }
            KeyCode::Enter => {
                self.execute_code_input();
                true
            }
            _ => false,
        }
    }

    fn execute_code_input(&mut self) {
        let buf = self.code_input_buffer.trim().to_string();
        if buf.is_empty() {
            self.error_message = "No code entered".to_string();
            self.code_input_active = false;
            return;
        }

        let code = match buf.parse::<u32>() {
            Ok(v) => v,
            Err(_) => {
                self.error_message = format!("Invalid code: {}", buf);
                self.code_input_active = false;
                return;
            }
        };

        match self.code_input_mode {
            CodeInputMode::Add => {
                self.add_char(code);
            }
            CodeInputMode::CopySource => {
                self.copy_source_code = code;
                self.code_input_mode = CodeInputMode::CopyDest;
                self.code_input_buffer.clear();
                return;
            }
            CodeInputMode::CopyDest => {
                let src = self.copy_source_code;
                self.copy_char(src, code);
            }
            CodeInputMode::DeleteConfirm => {
                let filtered = self.filtered_codes();
                let selected = filtered.get(self.selected_index).copied().unwrap_or(0);
                if buf.to_lowercase() == "y" {
                    self.delete_char(selected);
                }
            }
        }
        self.code_input_active = false;
        self.code_input_buffer.clear();
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
            FontEditorView::TransformEditor => None,
        }
    }
}

impl Default for FontEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::{FIGcharacter, FIGfont};
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
                (
                    66,
                    FIGcharacter::from(vec![
                        "BBB  ".to_string(),
                        "B  B ".to_string(),
                        "BBB  ".to_string(),
                    ]),
                ),
            ]),
            ..Default::default()
        }
    }

    fn make_editor() -> FontEditor {
        let mut editor = FontEditor::new();
        editor.load_font(make_test_font());
        editor
    }

    #[test]
    fn test_add_char() {
        let mut editor = make_editor();
        editor.add_char(999);
        let font = editor.font.as_ref().unwrap();
        assert!(font.chars.contains_key(&999));
        let ch = font.chars.get(&999).unwrap();
        assert_eq!(ch.rows().len(), 3);
        for row in ch.rows() {
            assert_eq!(row.len(), 5);
            assert!(row.chars().all(|c| c == ' '));
        }
        assert!(editor.all_codes.contains(&999));
    }

    #[test]
    fn test_delete_char() {
        let mut editor = make_editor();
        editor.delete_char(65);
        let font = editor.font.as_ref().unwrap();
        assert!(!font.chars.contains_key(&65));
        assert!(!editor.all_codes.contains(&65));
    }

    #[test]
    fn test_delete_fallback() {
        let mut editor = make_editor();
        editor.delete_char(65);
        let font = editor.font.as_ref().unwrap();
        assert!(font.chars.contains_key(&0));
        assert!(font.chars.contains_key(&66));
    }

    #[test]
    fn test_copy_char() {
        let mut editor = make_editor();
        editor.copy_char(65, 999);
        let font = editor.font.as_ref().unwrap();
        let src = font.chars.get(&65).unwrap();
        let dst = font.chars.get(&999).unwrap();
        assert_eq!(src.rows(), dst.rows());
        assert!(editor.all_codes.contains(&999));
    }

    #[test]
    fn test_copy_overwrite() {
        let mut editor = make_editor();
        editor.copy_char(65, 66);
        let font = editor.font.as_ref().unwrap();
        let src = font.chars.get(&65).unwrap();
        let dst = font.chars.get(&66).unwrap();
        assert_eq!(src.rows(), dst.rows());
    }

    #[test]
    fn test_add_duplicate_code() {
        let mut editor = make_editor();
        editor.add_char(65);
        let font = editor.font.as_ref().unwrap();
        assert!(font.chars.contains_key(&65));
        let ch = font.chars.get(&65).unwrap();
        assert_eq!(ch.rows()[0], " AA  ");
    }

    #[test]
    fn test_ensure_missing_char() {
        let font = FIGfont {
            charheight: 3,
            maxlength: 5,
            chars: HashMap::new(),
            ..Default::default()
        };
        let mut editor = FontEditor::new();
        editor.font = Some(font);
        editor.ensure_missing_char();
        let font = editor.font.as_ref().unwrap();
        assert!(font.chars.contains_key(&0));
        let ch = font.chars.get(&0).unwrap();
        assert_eq!(ch.rows().len(), 3);
        for row in ch.rows() {
            assert_eq!(row.len(), 5);
        }
    }

    #[test]
    fn test_delete_unknown() {
        let mut editor = make_editor();
        editor.delete_char(999);
        let font = editor.font.as_ref().unwrap();
        assert_eq!(font.chars.len(), 3);
    }

    #[test]
    fn test_copy_nonexistent_src() {
        let mut editor = make_editor();
        editor.copy_char(999, 888);
        let font = editor.font.as_ref().unwrap();
        let dst = font.chars.get(&888).unwrap();
        assert_eq!(dst.rows().len(), 3);
        for row in dst.rows() {
            assert!(row.chars().all(|c| c == ' '));
        }
    }

    #[test]
    fn test_is_valid_code() {
        assert!(FontEditor::is_valid_code(0));
        assert!(FontEditor::is_valid_code(65));
        assert!(FontEditor::is_valid_code(0x10FFFF));
        assert!(!FontEditor::is_valid_code(0xD800));
        assert!(!FontEditor::is_valid_code(0xDFFF));
        assert!(!FontEditor::is_valid_code(0x110000));
    }

    #[test]
    fn test_rebuild_all_codes() {
        let mut editor = make_editor();
        editor.rebuild_all_codes();
        assert_eq!(editor.all_codes, vec![0, 65, 66]);
        let font = editor.font.as_mut().unwrap();
        font.chars
            .insert(100, FIGcharacter::from(vec![" ".to_string()]));
        editor.rebuild_all_codes();
        assert_eq!(editor.all_codes, vec![0, 65, 66, 100]);
    }

    #[test]
    fn test_code_input_flow_add() {
        let mut editor = make_editor();
        editor.code_input_active = true;
        editor.code_input_mode = CodeInputMode::Add;
        editor.code_input_buffer = "999".to_string();
        editor.execute_code_input();
        assert!(!editor.code_input_active);
        assert!(editor.font.as_ref().unwrap().chars.contains_key(&999));
    }

    #[test]
    fn test_code_input_buffer_management() {
        let mut editor = make_editor();
        let fe = &mut editor;
        fe.code_input_active = true;
        fe.code_input_mode = CodeInputMode::Add;
        assert!(fe.handle_key_code_input(KeyCode::Char('1')));
        assert_eq!(fe.code_input_buffer, "1");
        assert!(fe.handle_key_code_input(KeyCode::Char('2')));
        assert_eq!(fe.code_input_buffer, "12");
        assert!(fe.handle_key_code_input(KeyCode::Backspace));
        assert_eq!(fe.code_input_buffer, "1");
        assert!(fe.handle_key_code_input(KeyCode::Esc));
        assert!(!fe.code_input_active);
        assert!(fe.code_input_buffer.is_empty());
    }

    #[test]
    fn test_code_input_delete_confirm() {
        let mut editor = make_editor();
        editor.code_input_active = true;
        editor.code_input_mode = CodeInputMode::DeleteConfirm;
        editor.code_input_buffer = "y".to_string();
        editor.execute_code_input();
        let font = editor.font.as_ref().unwrap();
        assert!(font.chars.contains_key(&65));
        assert!(font.chars.contains_key(&0));
        assert!(font.chars.contains_key(&66));
    }

    // --- Transform tests ---

    #[test]
    fn test_transform_resize_larger() {
        let mut editor = make_editor();
        editor.transform_resize(5);
        let font = editor.font.as_ref().unwrap();
        assert_eq!(font.charheight, 5);
        for ch in font.chars.values() {
            assert_eq!(ch.rows().len(), 5);
        }
        // Row 0-2 unchanged, row 3-4 are spaced
        let ch0 = font.chars.get(&0).unwrap();
        for row in 3..5 {
            assert!(ch0.rows()[row].chars().all(|c| c == ' '));
        }
        // Char 65 should have its original rows at top
        let ch65 = font.chars.get(&65).unwrap();
        assert_eq!(ch65.rows()[0], " AA  ");
        assert_eq!(ch65.rows()[1], "A  A ");
        assert_eq!(ch65.rows()[2], "AAAA ");
        assert!(ch65.rows()[3].chars().all(|c| c == ' '));
        assert!(ch65.rows()[4].chars().all(|c| c == ' '));
    }

    #[test]
    fn test_transform_resize_smaller() {
        let mut editor = make_editor();
        editor.transform_resize(2);
        let font = editor.font.as_ref().unwrap();
        assert_eq!(font.charheight, 2);
        for ch in font.chars.values() {
            assert_eq!(ch.rows().len(), 2);
        }
        let ch65 = font.chars.get(&65).unwrap();
        assert_eq!(ch65.rows()[0], " AA  ");
        assert_eq!(ch65.rows()[1], "A  A ");
    }

    #[test]
    fn test_transform_bold() {
        let mut editor = make_editor();
        editor.transform_bold();
        let font = editor.font.as_ref().unwrap();
        let ch65 = font.chars.get(&65).unwrap();
        // " AA  " -> "  AAAA    "
        assert_eq!(ch65.rows()[0], "  AAAA    ");
        // "A  A " -> "AA    AA  "
        assert_eq!(ch65.rows()[1], "AA    AA  ");
        // "AAAA " -> "AAAAAAAA  "
        assert_eq!(ch65.rows()[2], "AAAAAAAA  ");
    }

    #[test]
    fn test_transform_bold_updates_maxlength() {
        let mut editor = make_editor();
        assert_eq!(editor.font.as_ref().unwrap().maxlength, 5);
        editor.transform_bold();
        // After bold, char 65 width = 10, char 66 width = 8 (BBB -> BBBBBB)
        assert_eq!(editor.font.as_ref().unwrap().maxlength, 10);
    }

    #[test]
    fn test_transform_italicize() {
        let mut editor = make_editor();
        editor.transform_italicize();
        let font = editor.font.as_ref().unwrap();
        let ch65 = font.chars.get(&65).unwrap();
        // Row 0: no indent
        assert_eq!(ch65.rows()[0], " AA  ");
        // Row 1: 1 space
        assert_eq!(ch65.rows()[1], " A  A ");
        // Row 2: 2 spaces
        assert_eq!(ch65.rows()[2], "  AAAA ");
    }

    #[test]
    fn test_transform_mirror_horizontal() {
        let mut editor = make_editor();
        editor.transform_mirror(MirrorMode::Horizontal);
        let font = editor.font.as_ref().unwrap();
        let ch65 = font.chars.get(&65).unwrap();
        // " AA  " reversed -> "  AA "
        assert_eq!(ch65.rows()[0], "  AA ");
        assert_eq!(ch65.rows()[1], " A  A");
        assert_eq!(ch65.rows()[2], " AAAA");
    }

    #[test]
    fn test_transform_mirror_vertical() {
        let mut editor = make_editor();
        editor.transform_mirror(MirrorMode::Vertical);
        let font = editor.font.as_ref().unwrap();
        let ch65 = font.chars.get(&65).unwrap();
        assert_eq!(ch65.rows()[0], "AAAA ");
        assert_eq!(ch65.rows()[1], "A  A ");
        assert_eq!(ch65.rows()[2], " AA  ");
    }

    #[test]
    fn test_transform_mirror_both() {
        let mut editor = make_editor();
        editor.transform_mirror(MirrorMode::Both);
        let font = editor.font.as_ref().unwrap();
        let ch65 = font.chars.get(&65).unwrap();
        assert_eq!(ch65.rows()[0], " AAAA");
        assert_eq!(ch65.rows()[1], " A  A");
        assert_eq!(ch65.rows()[2], "  AA ");
    }

    #[test]
    fn test_transform_rename() {
        let mut editor = make_editor();
        assert!(editor.font_storage_name.is_empty());
        editor.transform_rename("MyFont");
        assert_eq!(editor.font_storage_name, "MyFont");
        editor.transform_rename("Another Font");
        assert_eq!(editor.font_storage_name, "Another Font");
    }

    #[test]
    fn test_transform_resize_across_all_codes() {
        let mut editor = make_editor();
        editor.transform_resize(4);
        let font = editor.font.as_ref().unwrap();
        assert_eq!(font.charheight, 4);
        for (&code, ch) in &font.chars {
            assert_eq!(
                ch.rows().len(),
                4,
                "char U+{code:X} should have 4 rows after resize"
            );
        }
    }

    #[test]
    fn test_transform_resize_identity() {
        let mut editor = make_editor();
        let orig_height = editor.font.as_ref().unwrap().charheight;
        editor.transform_resize(orig_height);
        assert_eq!(editor.font.as_ref().unwrap().charheight, orig_height);
    }

    #[test]
    fn test_transform_bold_all_chars() {
        let mut editor = make_editor();
        editor.transform_bold();
        let font = editor.font.as_ref().unwrap();
        for (&code, ch) in &font.chars {
            for row in ch.rows() {
                assert_eq!(
                    row.len() % 2,
                    0,
                    "char U+{code:X} should have even width after bold"
                );
            }
        }
    }

    #[test]
    fn test_transform_mirror_horizontal_all_chars() {
        let mut editor = make_editor();
        editor.transform_mirror(MirrorMode::Horizontal);
        let font = editor.font.as_ref().unwrap();
        for ch in font.chars.values() {
            for row in ch.rows() {
                let reversed: String = row.chars().rev().collect();
                // Mirroring twice should give original
                let double: String = reversed.chars().rev().collect();
                assert_eq!(double, *row);
            }
        }
    }

    #[test]
    fn test_transform_italicize_empty_font() {
        let mut editor = FontEditor::new();
        editor.transform_italicize(); // should not panic
    }

    #[test]
    fn test_transform_bold_empty_font() {
        let mut editor = FontEditor::new();
        editor.transform_bold(); // should not panic
    }

    #[test]
    fn test_transform_resize_empty_font() {
        let mut editor = FontEditor::new();
        editor.transform_resize(10); // should not panic
    }

    #[test]
    fn test_enter_transform_editor() {
        let mut editor = make_editor();
        editor.enter_transform_editor();
        assert_eq!(editor.view, FontEditorView::TransformEditor);
        assert_eq!(editor.selected_transform, 0);
        assert!(!editor.input_active);
        assert!(editor.input_buffer.is_empty());
    }

    #[test]
    fn test_transform_editor_navigation() {
        use crossterm::event::KeyCode;
        let mut editor = make_editor();
        editor.enter_transform_editor();

        // Down cycles through transforms
        for i in 0..5 {
            editor.handle_key(KeyCode::Down, KeyModifiers::NONE, 120);
            assert_eq!(editor.selected_transform, (i + 1) % 6);
        }

        // Down again wraps to 0
        editor.handle_key(KeyCode::Down, KeyModifiers::NONE, 120);
        assert_eq!(editor.selected_transform, 0);

        // Up wraps around
        editor.handle_key(KeyCode::Up, KeyModifiers::NONE, 120);
        assert_eq!(editor.selected_transform, 5);
    }

    #[test]
    fn test_transform_editor_esc_returns_to_overview() {
        use crossterm::event::KeyCode;
        let mut editor = make_editor();
        editor.enter_transform_editor();
        assert_eq!(editor.view, FontEditorView::TransformEditor);
        editor.handle_key(KeyCode::Esc, KeyModifiers::NONE, 120);
        assert_eq!(editor.view, FontEditorView::Overview);
    }

    #[test]
    fn test_transform_editor_t_key_opens() {
        use crossterm::event::KeyCode;
        let mut editor = make_editor();
        assert!(editor.handle_key(KeyCode::Char('T'), KeyModifiers::NONE, 120));
        assert_eq!(editor.view, FontEditorView::TransformEditor);
    }

    #[test]
    fn test_transform_editor_resize_flow() {
        use crossterm::event::KeyCode;
        let mut editor = make_editor();
        editor.enter_transform_editor();

        // Select Resize (index 0, already selected)
        editor.handle_key(KeyCode::Enter, KeyModifiers::NONE, 120);
        assert!(editor.input_active);

        // Type height
        editor.handle_key(KeyCode::Char('5'), KeyModifiers::NONE, 120);
        assert_eq!(editor.input_buffer, "5");

        // Confirm
        editor.handle_key(KeyCode::Enter, KeyModifiers::NONE, 120);
        assert!(!editor.input_active);
        assert_eq!(editor.font.as_ref().unwrap().charheight, 5);
    }

    #[test]
    fn test_transform_editor_italicize_flow() {
        use crossterm::event::KeyCode;
        let mut editor = make_editor();
        editor.enter_transform_editor();

        // Navigate to Italicize (index 1)
        editor.handle_key(KeyCode::Down, KeyModifiers::NONE, 120);
        editor.handle_key(KeyCode::Enter, KeyModifiers::NONE, 120);

        let ch65 = editor.font.as_ref().unwrap().chars.get(&65).unwrap();
        assert_eq!(ch65.rows()[1], " A  A ");
    }

    #[test]
    fn test_transform_editor_bold_flow() {
        use crossterm::event::KeyCode;
        let mut editor = make_editor();
        editor.enter_transform_editor();

        // Navigate to Bold (index 2)
        for _ in 0..2 {
            editor.handle_key(KeyCode::Down, KeyModifiers::NONE, 120);
        }
        editor.handle_key(KeyCode::Enter, KeyModifiers::NONE, 120);

        let ch65 = editor.font.as_ref().unwrap().chars.get(&65).unwrap();
        assert_eq!(ch65.rows()[0], "  AAAA    ");
    }

    #[test]
    fn test_transform_editor_mirror_flow() {
        use crossterm::event::KeyCode;
        let mut editor = make_editor();
        editor.enter_transform_editor();

        // Navigate to Mirror (index 3)
        for _ in 0..3 {
            editor.handle_key(KeyCode::Down, KeyModifiers::NONE, 120);
        }
        editor.handle_key(KeyCode::Enter, KeyModifiers::NONE, 120);
        assert!(editor.transform_submode.is_some());

        // Select Enter (Horizontal by default)
        editor.handle_key(KeyCode::Enter, KeyModifiers::NONE, 120);

        let ch65 = editor.font.as_ref().unwrap().chars.get(&65).unwrap();
        assert_eq!(ch65.rows()[0], "  AA ");
    }

    #[test]
    fn test_transform_copy_glyph_from_standard() {
        let mut editor = make_editor();
        let fontdir = concat!(env!("CARGO_MANIFEST_DIR"), "/../fonts");
        editor.transform_copy_glyph_from("standard", fontdir, 65);
        assert!(
            editor.error_message.is_empty(),
            "error message: {}",
            editor.error_message
        );
        let font = editor.font.as_ref().unwrap();
        assert!(
            font.chars.contains_key(&65),
            "code 65 should exist after copy"
        );
    }

    #[test]
    fn test_transform_copy_glyph_from_standard_new_code() {
        let mut editor = make_editor();
        // Copy a glyph from the test font itself using vfs: load standard font,
        // copy code 65 into editor at code 999
        let fontdir = concat!(env!("CARGO_MANIFEST_DIR"), "/../fonts");
        // First copy from standard font to code 65 (overwrite)
        editor.transform_copy_glyph_from("standard", fontdir, 65);
        assert!(editor.error_message.is_empty());
        // Copy code 65 to a new code 999
        editor.copy_char(65, 999);
        let font = editor.font.as_ref().unwrap();
        assert!(
            font.chars.contains_key(&999),
            "code 999 should exist after copy"
        );
        assert_eq!(
            font.chars.get(&65).unwrap().rows(),
            font.chars.get(&999).unwrap().rows()
        );
    }

    #[test]
    fn test_transform_copy_glyph_nonexistent_font() {
        let mut editor = make_editor();
        editor.transform_copy_glyph_from("nonexistent_font_xyz", ".", 65);
        assert!(
            !editor.error_message.is_empty(),
            "should set error for nonexistent font"
        );
    }
}
