use std::collections::BTreeMap;
use std::path::Path;

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::tui::action::Action;
use crate::tui::component::Component;
use crate::tui::AppMode;

pub struct StatusBarComponent {
    pub cursor: (u16, u16),
    pub zoom: u8,
    pub tool_name: String,
    pub mode_name: String,
    pub mode: AppMode,
    pub unsaved: bool,
    pub icons: BTreeMap<String, String>,
    pub current_path: Option<String>,
    pub throbber_text: String,
    pub undo_count: usize,
    pub fps: f64,
    pub git_branch: Option<String>,
    pub clock_str: String,
    pub layer_count: u8,
    pub animation_frame: u8,
}

impl StatusBarComponent {
    pub fn new(icons: BTreeMap<String, String>) -> Self {
        Self {
            cursor: (0, 0),
            zoom: 1,
            tool_name: String::new(),
            mode_name: String::new(),
            mode: AppMode::FontEditor,
            unsaved: false,
            icons,
            current_path: None,
            throbber_text: String::new(),
            undo_count: 0,
            fps: 0.0,
            git_branch: None,
            clock_str: String::new(),
            layer_count: 1,
            animation_frame: 0,
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
            | Action::ColorChanged(..) => {}
            _ => {}
        }
        None
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> std::io::Result<()> {
        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(&block, area);

        if inner.width < 10 {
            return Ok(());
        }

        let pos_icon = self
            .icons
            .get("status_position")
            .map_or("+", |s| s.as_str());
        let zoom_icon = self.icons.get("status_zoom").map_or("Z", |s| s.as_str());
        let tool_icon = self.icons.get("status_tool").map_or("T", |s| s.as_str());
        let mode_icon = self.icons.get("status_mode").map_or("M", |s| s.as_str());
        let unsaved_icon = self.icons.get("status_unsaved").map_or("!", |s| s.as_str());
        let saved_icon = self.icons.get("status_saved").map_or("*", |s| s.as_str());

        let mode_color = match self.mode {
            AppMode::FontEditor => Color::Blue,
            AppMode::ImageEditor => Color::Green,
            AppMode::AsciiPreview => Color::Yellow,
        };

        let unsaved_dot = if self.unsaved {
            unsaved_icon
        } else {
            saved_icon
        };
        let filename = self
            .current_path
            .as_ref()
            .map(Path::new)
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|n| n.to_string())
            .unwrap_or_else(|| "Untitled".to_string());

        let undo_str = if self.undo_count > 0 {
            format!(" undo:{}", self.undo_count)
        } else {
            String::new()
        };

        let fps_str = format!("FPS:{:.0}", self.fps);
        let branch_str = match &self.git_branch {
            Some(b) => format!(" ⎇ {}", b),
            None => String::new(),
        };
        let throbber_str = if self.throbber_text.is_empty() {
            String::new()
        } else {
            format!(" {} ", self.throbber_text)
        };

        let mode_label = format!(" {} {} ", mode_icon, self.mode_name);
        let cursor_str = format!(" {} X:{} Y:{} ", pos_icon, self.cursor.0, self.cursor.1);
        let zoom_label = format!(" {} {}x", zoom_icon, self.zoom);
        let tool_label = format!(" {}{} ", tool_icon, self.tool_name);
        let center_str = format!(" {} {}{}", unsaved_dot, filename, undo_str);
        let right_str = format!(
            "{} │ L:{} │ F:{} │ {}{}{}",
            fps_str,
            self.layer_count,
            self.animation_frame,
            self.clock_str,
            throbber_str,
            branch_str
        );

        let left_w = mode_label.chars().count()
            + tool_label.chars().count()
            + cursor_str.chars().count()
            + zoom_label.chars().count();
        let right_w = right_str.chars().count();
        let gap = (inner.width as usize).saturating_sub(left_w + right_w + 6);
        let center_trunc: String = center_str.chars().take(gap).collect();

        let mut spans: Vec<Span> = Vec::new();

        spans.push(Span::styled(
            mode_label,
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(format!(
            "{}{}{}",
            tool_label, cursor_str, zoom_label,
        )));
        spans.push(Span::raw(" │ "));
        spans.push(Span::raw(center_trunc));
        spans.push(Span::raw(" │ "));
        spans.push(Span::raw(right_str));

        let paragraph = Paragraph::new(Line::from(spans));
        frame.render_widget(paragraph, inner);

        Ok(())
    }
}
