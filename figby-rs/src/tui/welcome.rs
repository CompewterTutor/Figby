use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};
use ratatui::Frame;
use std::path::PathBuf;

use super::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelcomeAction {
    Dismiss,
    OpenRecent(usize),
    Open,
    NewFile,
    ToggleHelp,
    OpenSettings,
}

pub struct WelcomeScreen {
    pub show: bool,
}

impl WelcomeScreen {
    pub fn new() -> Self {
        Self { show: true }
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        recent_files: &[PathBuf],
        version: &str,
        theme: &Theme,
    ) {
        frame.render_widget(Clear, area);

        let welcome_area = centered_welcome(area);
        frame.render_widget(Clear, welcome_area);

        let block = Block::default()
            .title(" Welcome ")
            .borders(Borders::ALL)
            .style(Style::default().fg(theme.general.primary));
        let inner = block.inner(welcome_area);
        frame.render_widget(block, welcome_area);

        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(" Figby  {version}"),
            Style::default()
                .fg(theme.general.primary)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            " FIGfont Editor",
            Style::default().fg(theme.general.secondary),
        )));
        lines.push(Line::from(""));

        if !recent_files.is_empty() {
            lines.push(Line::from(Span::styled(
                " Recent files:",
                Style::default()
                    .fg(theme.general.secondary)
                    .add_modifier(Modifier::BOLD),
            )));
            for (i, path) in recent_files.iter().enumerate().take(9) {
                let num = i + 1;
                let display = path.to_string_lossy();
                lines.push(Line::from(Span::styled(
                    format!("  {num}. {display}"),
                    Style::default().fg(theme.dialog.meta),
                )));
            }
            lines.push(Line::from(""));
        }

        lines.push(Line::from(Span::styled(
            " Keybindings:",
            Style::default()
                .fg(theme.general.secondary)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "  Ctrl+O    Open font",
            Style::default().fg(theme.general.secondary),
        )));
        lines.push(Line::from(Span::styled(
            "  Ctrl+N    New font",
            Style::default().fg(theme.general.secondary),
        )));
        lines.push(Line::from(Span::styled(
            "  S         Settings",
            Style::default().fg(theme.general.secondary),
        )));
        lines.push(Line::from(Span::styled(
            "  ?         Keybindings reference",
            Style::default().fg(theme.general.secondary),
        )));
        lines.push(Line::from(Span::styled(
            "  Esc       Dismiss and start editing",
            Style::default().fg(theme.general.secondary),
        )));

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }

    pub fn handle_key(
        &self,
        code: KeyCode,
        modifiers: KeyModifiers,
        recent_count: usize,
    ) -> Option<WelcomeAction> {
        match code {
            KeyCode::Esc if modifiers == KeyModifiers::NONE => Some(WelcomeAction::Dismiss),
            KeyCode::Char('?') => Some(WelcomeAction::ToggleHelp),
            KeyCode::Char('S') if !modifiers.contains(KeyModifiers::CONTROL) => {
                Some(WelcomeAction::OpenSettings)
            }
            KeyCode::Char(c) if modifiers == KeyModifiers::CONTROL => match c {
                'o' | 'O' => Some(WelcomeAction::Open),
                'n' | 'N' => Some(WelcomeAction::NewFile),
                _ => None,
            },
            KeyCode::Char(c)
                if modifiers == KeyModifiers::NONE && c.is_ascii_digit() && c != '0' =>
            {
                let idx = (c as u8 - b'1') as usize;
                if idx < recent_count {
                    Some(WelcomeAction::OpenRecent(idx))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl Widget for &WelcomeScreen {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Widget::render(Clear, area, buf);

        let welcome_area = centered_welcome(area);
        Widget::render(Clear, welcome_area, buf);

        let block = Block::default().title(" Welcome ").borders(Borders::ALL);
        let inner = block.inner(welcome_area);
        Widget::render(block, welcome_area, buf);

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                " Welcome to Figby",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];
        let paragraph = Paragraph::new(lines);
        Widget::render(paragraph, inner, buf);
    }
}

impl Default for WelcomeScreen {
    fn default() -> Self {
        Self::new()
    }
}

pub fn centered_welcome(area: Rect) -> Rect {
    let w = (area.width / 5 * 3).min(60);
    let h = (area.height / 5 * 3).min(30);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect {
        x,
        y,
        width: w,
        height: h,
    }
}
