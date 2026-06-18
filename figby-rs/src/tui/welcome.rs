use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};
use ratatui::Frame;
use std::path::PathBuf;

use super::theme::Theme;

const MASCOT_RAW: &str = include_str!("../../../assets/img/figby.block.ascii.image.txt");

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
    mascot_lines: Vec<Line<'static>>,
    title_lines: Vec<String>,
}

impl WelcomeScreen {
    pub fn new() -> Self {
        let mascot_lines = parse_ansi_lines(MASCOT_RAW);
        let title_lines = render_title_with_computerist();
        Self {
            show: true,
            mascot_lines,
            title_lines,
        }
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
            .title(" Welcome to Figby ")
            .borders(Borders::ALL)
            .style(Style::default().fg(theme.general.primary));
        let inner = block.inner(welcome_area);
        frame.render_widget(block, welcome_area);

        // Split inner: banner row on top, content below
        let mascot_height = self.mascot_lines.len().max(1) as u16;
        let banner_height = mascot_height;

        let vert = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(banner_height), Constraint::Min(0)])
            .split(inner);

        // Banner: mascot left, FIGBY title right
        let mascot_width = 30u16;
        let horiz = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(mascot_width + 1), Constraint::Min(0)])
            .split(vert[0]);

        let mascot_para = Paragraph::new(self.mascot_lines.clone());
        frame.render_widget(mascot_para, horiz[0]);

        if !self.title_lines.is_empty() {
            let title_color = theme.general.primary;
            let title_para = Paragraph::new(
                self.title_lines
                    .iter()
                    .map(|l| {
                        Line::from(Span::styled(
                            l.clone(),
                            Style::default().fg(title_color),
                        ))
                    })
                    .collect::<Vec<_>>(),
            );
            frame.render_widget(title_para, horiz[1]);
        }

        // Content section below banner
        let content_area = vert[1];
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            format!("  v{version}  —  FIGfont Editor"),
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
        for (key, desc) in &[
            ("  Ctrl+O", "Open font"),
            ("  Ctrl+N", "New font"),
            ("  S      ", "Settings"),
            ("  ?      ", "Keybindings reference"),
            ("  Esc    ", "Dismiss and start editing"),
        ] {
            lines.push(Line::from(vec![
                Span::styled(
                    key.to_string(),
                    Style::default()
                        .fg(theme.general.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  {desc}"),
                    Style::default().fg(theme.general.secondary),
                ),
            ]));
        }

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, content_area);
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
    let w = (area.width / 5 * 3).max(70).min(area.width.saturating_sub(2));
    let h = (area.height / 5 * 3).max(35).min(area.height.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect {
        x,
        y,
        width: w,
        height: h,
    }
}

fn render_title_with_computerist() -> Vec<String> {
    let font_dirs = ["/usr/share/figlet", "/usr/local/share/figlet"];
    for name in &["Computerist-12", "Computerist-20"] {
        if let Ok(font) = crate::font::load_font(name, &font_dirs) {
            let rows = crate::render::render_string(&font, "FIGBY");
            let trimmed: Vec<String> = rows
                .into_iter()
                .map(|l| l.replace('\u{00A0}', " "))
                .collect();
            // strip trailing blank rows
            let last_content = trimmed
                .iter()
                .rposition(|l| l.trim_end().len() > 0)
                .unwrap_or(0);
            return trimmed[..=last_content].to_vec();
        }
    }
    // fallback plain
    vec![
        " _____ _  ____ ______   __".to_string(),
        "|  ___| |/ ___|  _ \\ \\ / /".to_string(),
        "| |_  | | |  _| |_) \\ V / ".to_string(),
        "|  _| | | |_| |  _ < | |  ".to_string(),
        "|_|   |_|\\____|_| \\_\\|_|  ".to_string(),
    ]
}

fn parse_ansi_lines(text: &str) -> Vec<Line<'static>> {
    text.lines()
        .map(|raw| {
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut current_color: Option<Color> = None;
            let mut current_text = String::new();
            let bytes = raw.as_bytes();
            let mut i = 0;

            while i < bytes.len() {
                if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
                    // flush
                    if !current_text.is_empty() {
                        let style = current_color
                            .map_or(Style::default(), |c| Style::default().fg(c));
                        spans.push(Span::styled(current_text.clone(), style));
                        current_text.clear();
                    }
                    i += 2;
                    let start = i;
                    while i < bytes.len() && bytes[i] != b'm' {
                        i += 1;
                    }
                    let seq = std::str::from_utf8(&bytes[start..i]).unwrap_or("");
                    i += 1; // skip 'm'

                    if seq == "0" || seq.is_empty() {
                        current_color = None;
                    } else {
                        let parts: Vec<u8> =
                            seq.split(';').filter_map(|s| s.parse().ok()).collect();
                        if parts.len() == 5 && parts[0] == 38 && parts[1] == 2 {
                            current_color =
                                Some(Color::Rgb(parts[2], parts[3], parts[4]));
                        }
                    }
                } else {
                    // multi-byte UTF-8: find char boundary
                    let ch_start = i;
                    i += 1;
                    while i < bytes.len() && (bytes[i] & 0xC0) == 0x80 {
                        i += 1;
                    }
                    if let Ok(s) = std::str::from_utf8(&bytes[ch_start..i]) {
                        current_text.push_str(s);
                    }
                }
            }

            if !current_text.is_empty() {
                let style =
                    current_color.map_or(Style::default(), |c| Style::default().fg(c));
                spans.push(Span::styled(current_text, style));
            }

            Line::from(spans)
        })
        .collect()
}
