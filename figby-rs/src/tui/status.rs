use std::collections::BTreeMap;

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::theme::Theme;

pub struct StatusBar;

#[allow(clippy::too_many_arguments)]
impl StatusBar {
    pub fn render(
        frame: &mut Frame<'_>,
        area: Rect,
        cursor: (u16, u16),
        zoom: u8,
        tool_name: &str,
        mode_name: &str,
        unsaved: bool,
        icons: &BTreeMap<String, String>,
        current_path: Option<&std::path::Path>,
    ) {
        let pos_icon = icons.get("status_position").map_or("+", |s| s.as_str());
        let zoom_icon = icons.get("status_zoom").map_or("Z", |s| s.as_str());
        let tool_icon = icons.get("status_tool").map_or("T", |s| s.as_str());
        let mode_icon = icons.get("status_mode").map_or("M", |s| s.as_str());
        let unsaved_icon = icons.get("status_unsaved").map_or("!", |s| s.as_str());
        let saved_icon = icons.get("status_saved").map_or("*", |s| s.as_str());

        let indicator = if unsaved {
            format!(" {} ", unsaved_icon)
        } else {
            format!(" {} ", saved_icon)
        };

        let filename = current_path
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|n| {
                if unsaved {
                    format!("*{n}")
                } else {
                    n.to_string()
                }
            })
            .unwrap_or_else(|| {
                if unsaved {
                    "*Untitled".to_string()
                } else {
                    "Untitled".to_string()
                }
            });

        let text = format!(
            " {} X:{} Y:{} | {} Zoom:{}x | {} {} | {} {} | {} [{}] | [Tab] Mode | [q] Quit | [S] Settings | ^S Save | ^S+S Save As",
            pos_icon, cursor.0, cursor.1,
            zoom_icon, zoom,
            tool_icon, tool_name,
            mode_icon, mode_name,
            indicator, filename,
        );

        let paragraph = Paragraph::new(text).block(Block::default().borders(Borders::ALL));
        frame.render_widget(paragraph, area);
    }
}

#[derive(Debug, Clone)]
pub struct CanvasSettings {
    pub settings_open: bool,
    pub canvas_width: u16,
    pub canvas_height: u16,
    pub font_size: u8,
    pub show_grid: bool,
    pub snap_to_grid: bool,
    selected_field: usize,
    pub theme: Theme,
}

impl CanvasSettings {
    pub fn new() -> Self {
        Self {
            settings_open: false,
            canvas_width: 40,
            canvas_height: 20,
            font_size: 12,
            show_grid: false,
            snap_to_grid: false,
            selected_field: 0,
            theme: Theme::default(),
        }
    }

    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        if !self.settings_open {
            return false;
        }
        match code {
            KeyCode::Up => {
                self.selected_field = self.selected_field.saturating_sub(1);
                true
            }
            KeyCode::Down => {
                if self.selected_field < 4 {
                    self.selected_field += 1;
                }
                true
            }
            KeyCode::Left => {
                match self.selected_field {
                    0 => self.canvas_width = self.canvas_width.saturating_sub(1).max(1),
                    1 => self.canvas_height = self.canvas_height.saturating_sub(1).max(1),
                    2 => self.font_size = self.font_size.saturating_sub(1).max(6),
                    _ => {}
                }
                true
            }
            KeyCode::Right => {
                match self.selected_field {
                    0 => self.canvas_width = self.canvas_width.saturating_add(1).min(200),
                    1 => self.canvas_height = self.canvas_height.saturating_add(1).min(200),
                    2 => self.font_size = self.font_size.saturating_add(1).min(72),
                    _ => {}
                }
                true
            }
            KeyCode::Enter => {
                match self.selected_field {
                    3 => self.show_grid = !self.show_grid,
                    4 => self.snap_to_grid = !self.snap_to_grid,
                    _ => {}
                }
                true
            }
            KeyCode::Esc => {
                self.settings_open = false;
                true
            }
            _ => false,
        }
    }

    pub fn render(&self, frame: &mut Frame<'_>, area: Rect) {
        if !self.settings_open {
            return;
        }

        frame.render_widget(Clear, area);
        let block = Block::default().title(" Settings ").borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width < 10 || inner.height < 4 {
            return;
        }

        let mut lines: Vec<Line<'_>> = Vec::new();

        let fields: [(&str, String); 5] = [
            ("Canvas Width", format!("{}", self.canvas_width)),
            ("Canvas Height", format!("{}", self.canvas_height)),
            ("Font Size", format!("{}", self.font_size)),
            (
                "Grid",
                if self.show_grid {
                    "On".into()
                } else {
                    "Off".into()
                },
            ),
            (
                "Snap-to-Grid",
                if self.snap_to_grid {
                    "On".into()
                } else {
                    "Off".into()
                },
            ),
        ];

        for (i, (label, value)) in fields.iter().enumerate() {
            let line = if i == self.selected_field {
                let style = Style::default()
                    .fg(self.theme.dialog.highlight)
                    .add_modifier(Modifier::REVERSED);
                Line::from(Span::styled(format!(" {}: {} ", label, value), style))
            } else {
                Line::from(Span::raw(format!(" {}: {}", label, value)))
            };
            lines.push(line);
        }

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }
}

impl Default for CanvasSettings {
    fn default() -> Self {
        Self::new()
    }
}
