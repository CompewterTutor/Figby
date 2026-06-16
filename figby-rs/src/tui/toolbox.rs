use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget};
use ratatui::Frame;

use super::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Brush,
    Marquee,
    Lasso,
    CircleSelect,
    PolygonSelect,
    Fill,
    Line,
    Eraser,
    Eyedropper,
    Spray,
    Text,
}

impl Tool {
    pub fn display_name(&self) -> &str {
        match self {
            Tool::Brush => "Br",
            Tool::Marquee => "Ma",
            Tool::Lasso => "La",
            Tool::CircleSelect => "Ci",
            Tool::PolygonSelect => "Po",
            Tool::Fill => "Fi",
            Tool::Line => "Li",
            Tool::Eraser => "Er",
            Tool::Eyedropper => "Ey",
            Tool::Spray => "Sp",
            Tool::Text => "Te",
        }
    }

    pub fn full_name(&self) -> &str {
        match self {
            Tool::Brush => "Brush",
            Tool::Marquee => "Select",
            Tool::Lasso => "Lasso",
            Tool::CircleSelect => "Circle",
            Tool::PolygonSelect => "Polygon",
            Tool::Fill => "Fill",
            Tool::Line => "Line",
            Tool::Eraser => "Eraser",
            Tool::Eyedropper => "Eyedrop",
            Tool::Spray => "Spray",
            Tool::Text => "Text",
        }
    }

    pub fn key_shortcut(&self) -> KeyCode {
        match self {
            Tool::Brush => KeyCode::Char('b'),
            Tool::Marquee => KeyCode::Char('v'),
            Tool::Lasso => KeyCode::Char('l'),
            Tool::CircleSelect => KeyCode::Char('c'),
            Tool::PolygonSelect => KeyCode::Char('p'),
            Tool::Fill => KeyCode::Char('g'),
            Tool::Line => KeyCode::Char('i'),
            Tool::Eraser => KeyCode::Char('e'),
            Tool::Eyedropper => KeyCode::Char('d'),
            Tool::Spray => KeyCode::Char('a'),
            Tool::Text => KeyCode::Char('t'),
        }
    }

    pub fn icon_key(&self) -> &str {
        match self {
            Tool::Brush => "tool_brush",
            Tool::Marquee => "tool_marquee",
            Tool::Lasso => "tool_lasso",
            Tool::CircleSelect => "tool_circle",
            Tool::PolygonSelect => "tool_polygon",
            Tool::Fill => "tool_fill",
            Tool::Line => "tool_line",
            Tool::Eraser => "tool_eraser",
            Tool::Eyedropper => "tool_eyedropper",
            Tool::Spray => "tool_spray",
            Tool::Text => "tool_text",
        }
    }

    pub fn all() -> &'static [Tool] {
        &[
            Tool::Brush,
            Tool::Marquee,
            Tool::Lasso,
            Tool::CircleSelect,
            Tool::PolygonSelect,
            Tool::Fill,
            Tool::Line,
            Tool::Eraser,
            Tool::Eyedropper,
            Tool::Spray,
            Tool::Text,
        ]
    }
}

pub struct Toolbox {
    pub selected: Tool,
    pub theme: Theme,
}

impl Toolbox {
    pub fn new() -> Self {
        Self {
            selected: Tool::Brush,
            theme: Theme::default(),
        }
    }

    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        let lower = match code {
            KeyCode::Char(c) => c.to_ascii_lowercase(),
            _ => return false,
        };
        for tool in Tool::all() {
            if let KeyCode::Char(tc) = tool.key_shortcut() {
                if tc == lower {
                    self.selected = *tool;
                    return true;
                }
            }
        }
        false
    }

    pub fn next(&mut self) {
        let all = Tool::all();
        let idx = all.iter().position(|t| *t == self.selected).unwrap_or(0);
        self.selected = all[(idx + 1) % all.len()];
    }

    pub fn prev(&mut self) {
        let all = Tool::all();
        let idx = all.iter().position(|t| *t == self.selected).unwrap_or(0);
        self.selected = all[(idx + all.len() - 1) % all.len()];
    }

    pub fn render(&self, frame: &mut Frame<'_>, area: Rect) {
        frame.render_widget(self, area);
    }
}

impl Widget for &Toolbox {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let tools = Tool::all();
        let items: Vec<ListItem<'_>> = tools
            .iter()
            .map(|t| ListItem::new(format!(" {}", t.display_name())))
            .collect();

        let selected_idx = tools.iter().position(|t| *t == self.selected).unwrap_or(0);

        let list = List::new(items)
            .block(Block::default().title(" Tools ").borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .fg(self.theme.toolbox.selected)
                    .add_modifier(Modifier::BOLD),
            );

        let mut state = ListState::default();
        state.select(Some(selected_idx));
        StatefulWidget::render(list, area, buf, &mut state);
    }
}

impl Toolbox {}

impl Default for Toolbox {
    fn default() -> Self {
        Self::new()
    }
}
