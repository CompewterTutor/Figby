use std::collections::BTreeMap;
use std::path::Path;

use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::tui::action::Action;
use crate::tui::component::Component;

pub struct StatusBarComponent {
    pub cursor: (u16, u16),
    pub zoom: u8,
    pub tool_name: String,
    pub mode_name: String,
    pub unsaved: bool,
    pub icons: BTreeMap<String, String>,
    pub current_path: Option<String>,
}

impl StatusBarComponent {
    pub fn new(icons: BTreeMap<String, String>) -> Self {
        Self {
            cursor: (0, 0),
            zoom: 1,
            tool_name: String::new(),
            mode_name: String::new(),
            unsaved: false,
            icons,
            current_path: None,
        }
    }
}

impl Component for StatusBarComponent {
    fn update(&mut self, action: &Action) -> Option<Action> {
        match action {
            Action::CanvasModified
            | Action::ToolSelected
            | Action::BrushChanged
            | Action::ModeChanged
            | Action::ColorChanged(..) => {
                // Status bar is updated by TuiApp setting fields directly
            }
            _ => {}
        }
        None
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> std::io::Result<()> {
        let pos_icon = self
            .icons
            .get("status_position")
            .map_or("+", |s| s.as_str());
        let zoom_icon = self.icons.get("status_zoom").map_or("Z", |s| s.as_str());
        let tool_icon = self.icons.get("status_tool").map_or("T", |s| s.as_str());
        let mode_icon = self.icons.get("status_mode").map_or("M", |s| s.as_str());
        let unsaved_icon = self.icons.get("status_unsaved").map_or("!", |s| s.as_str());
        let saved_icon = self.icons.get("status_saved").map_or("*", |s| s.as_str());

        let indicator = if self.unsaved {
            format!(" {} ", unsaved_icon)
        } else {
            format!(" {} ", saved_icon)
        };

        let path_ref = self.current_path.as_ref().map(Path::new);
        let filename = path_ref
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|n| {
                if self.unsaved {
                    format!("*{n}")
                } else {
                    n.to_string()
                }
            })
            .unwrap_or_else(|| {
                if self.unsaved {
                    "*Untitled".to_string()
                } else {
                    "Untitled".to_string()
                }
            });

        let text = format!(
            " {} X:{} Y:{} | {} Zoom:{}x | {} {} | {} {} | {} [{}] | [Tab] Mode | [q] Quit | [S] Settings | ^S Save | ^S+S Save As",
            pos_icon,
            self.cursor.0,
            self.cursor.1,
            zoom_icon,
            self.zoom,
            tool_icon,
            self.tool_name,
            mode_icon,
            self.mode_name,
            indicator,
            filename,
        );

        let paragraph = Paragraph::new(text).block(Block::default().borders(Borders::ALL));
        frame.render_widget(paragraph, area);

        Ok(())
    }
}
