use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::canvas::CanvasCell;
use super::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorTarget {
    Foreground,
    Background,
}

impl ColorTarget {
    fn toggle(&mut self) {
        *self = match self {
            ColorTarget::Foreground => ColorTarget::Background,
            ColorTarget::Background => ColorTarget::Foreground,
        };
    }
}

/// Named character groups for the palette char picker.
pub struct CharGroup {
    pub name: &'static str,
    pub chars: &'static str,
}

/// All palette char groups, ordered for display.
/// "deluxe" is listed first as the richest set.
pub const CHAR_GROUPS: &[CharGroup] = &[
    CharGroup { name: "deluxe",  chars: "Combines ASCII + blocks + box + braille + ogham (see font_gen)" },
    CharGroup { name: "ascii",   chars: " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~" },
    CharGroup { name: "braille", chars: "⠀⠁⠂⠃⠄⠅⠆⠇⠈⠉⠊⠋⠌⠍⠎⠏⠐⠑⠒⠓⠔⠕⠖⠗⠘⠙⠚⠛⠜⠝⠞⠟⠠⠡⠢⠣⠤⠥⠦⠧⠨⠩⠪⠫⠬⠭⠮⠯⠰⠱⠲⠳⠴⠵⠶⠷⠸⠹⠺⠻⠼⠽⠾⠿⡀⡁⡂⡃⡄⡅⡆⡇⡈⡉⡊⡋⡌⡍⡎⡏⡐⡑⡒⡓⡔⡕⡖⡗⡘⡙⡚⡛⡜⡝⡞⡟⡠⡡⡢⡣⡤⡥⡦⡧⡨⡩⡪⡫⡬⡭⡮⡯⡰⡱⡲⡳⡴⡵⡶⡷⡸⡹⡺⡻⡼⡽⡾⡿⢀⢁⢂⢃⢄⢅⢆⢇⢈⢉⢊⢋⢌⢍⢎⢏⢐⢑⢒⢓⢔⢕⢖⢗⢘⢙⢚⢛⢜⢝⢞⢟⢠⢡⢢⢣⢤⢥⢦⢧⢨⢩⢪⢫⢬⢭⢮⢯⢰⢱⢲⢳⢴⢵⢶⢷⢸⢹⢺⢻⢼⢽⢾⢿⣀⣁⣂⣃⣄⣅⣆⣇⣈⣉⣊⣋⣌⣍⣎⣏⣐⣑⣒⣓⣔⣕⣖⣗⣘⣙⣚⣛⣜⣝⣞⣟⣠⣡⣢⣣⣤⣥⣦⣧⣨⣩⣪⣫⣬⣭⣮⣯⣰⣱⣲⣳⣴⣵⣶⣷⣸⣹⣺⣻⣼⣽⣾⣿" },
    CharGroup { name: "blocks",  chars: "░▒▓▁▂▃▄▅▆▇█▌▐▀▄▖▗▘▙▚▛▜▝▞▟" },
    CharGroup { name: "box",     chars: "─│┌┐└┘├┤┬┴┼═║╔╗╚╝╠╣╦╩╬━┃┏┓┗┛┣┫┳┻╋╴╵╶╷" },
    CharGroup { name: "ogham",   chars: " ᚁᚂᚃᚄᚅᚆᚇᚈᚉᚊᚋᚌᚍᚎᚏᚐᚑᚒᚓᚔᚕᚖᚗᚘᚙᚚ᚛᚜" },
];

pub const ANSI_16_COLORS: [Color; 16] = [
    Color::Indexed(0),
    Color::Indexed(1),
    Color::Indexed(2),
    Color::Indexed(3),
    Color::Indexed(4),
    Color::Indexed(5),
    Color::Indexed(6),
    Color::Indexed(7),
    Color::Indexed(8),
    Color::Indexed(9),
    Color::Indexed(10),
    Color::Indexed(11),
    Color::Indexed(12),
    Color::Indexed(13),
    Color::Indexed(14),
    Color::Indexed(15),
];

fn extended_color(page: u8, offset: u8) -> Color {
    let idx = 16u16 + page as u16 * 16 + offset as u16;
    Color::Indexed(idx.min(255) as u8)
}

const MAX_RECENT: usize = 8;

pub struct Palette {
    pub target: ColorTarget,
    pub selected_color: Option<Color>,
    pub recent: Vec<Color>,
    selected_index: usize,
    custom_hex: String,
    custom_mode: bool,
    show_extended: bool,
    extended_page: u8,
    pub theme: Theme,
}

impl Palette {
    pub fn new() -> Self {
        Self {
            target: ColorTarget::Foreground,
            selected_color: None,
            selected_index: 0,
            recent: Vec::with_capacity(MAX_RECENT),
            custom_hex: String::new(),
            custom_mode: false,
            show_extended: false,
            extended_page: 0,
            theme: Theme::default(),
        }
    }

    pub fn toggle_target(&mut self) {
        self.target.toggle();
    }

    pub fn select_color(&mut self, index: usize) {
        let color = self.current_color(index);
        self.selected_index = index;
        self.selected_color = Some(color);
        self.push_recent(color);
    }

    fn current_color(&self, index: usize) -> Color {
        if self.show_extended {
            if index < 16 {
                extended_color(self.extended_page, index as u8)
            } else {
                ANSI_16_COLORS[0]
            }
        } else if index < 16 {
            ANSI_16_COLORS[index]
        } else {
            ANSI_16_COLORS[0]
        }
    }

    fn visible_count(&self) -> usize {
        16
    }

    pub fn push_recent(&mut self, color: Color) {
        self.recent.retain(|c| *c != color);
        self.recent.push(color);
        if self.recent.len() > MAX_RECENT {
            self.recent.remove(0);
        }
    }

    pub fn set_custom_hex(&mut self, hex: &str) -> bool {
        self.custom_hex.clear();
        self.custom_hex.push_str(hex);
        if hex.len() == 7 && hex.starts_with('#') {
            let r = u8::from_str_radix(&hex[1..3], 16);
            let g = u8::from_str_radix(&hex[3..5], 16);
            let b = u8::from_str_radix(&hex[5..7], 16);
            if let (Ok(r), Ok(g), Ok(b)) = (r, g, b) {
                let color = Color::Rgb(r, g, b);
                self.selected_color = Some(color);
                self.push_recent(color);
                return true;
            }
        }
        false
    }

    pub fn apply_to_cell(&self, cell: &mut CanvasCell) {
        if let Some(color) = self.selected_color {
            match self.target {
                ColorTarget::Foreground => cell.fg = Some(color),
                ColorTarget::Background => cell.bg = Some(color),
            }
        }
    }

    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        if self.custom_mode {
            return match code {
                KeyCode::Char(c) if c.is_ascii_hexdigit() || c == '#' => {
                    if self.custom_hex.len() < 7 {
                        self.custom_hex.push(c);
                    }
                    true
                }
                KeyCode::Backspace => {
                    self.custom_hex.pop();
                    true
                }
                KeyCode::Enter => {
                    let plain = self.custom_hex.trim_start_matches('#');
                    let hex = format!("#{:0>6}", plain);
                    self.custom_hex = hex.clone();
                    self.set_custom_hex(&hex);
                    self.custom_mode = false;
                    true
                }
                KeyCode::Esc => {
                    self.custom_mode = false;
                    true
                }
                _ => false,
            };
        }
        match code {
            KeyCode::Char('x') | KeyCode::Char('X') => {
                self.toggle_target();
                true
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.target = ColorTarget::Foreground;
                true
            }
            KeyCode::Char('h') | KeyCode::Char('H') => {
                self.custom_mode = true;
                self.custom_hex.clear();
                true
            }
            KeyCode::Char('z') | KeyCode::Char('Z') => {
                self.show_extended = !self.show_extended;
                self.selected_index = 0;
                true
            }
            KeyCode::Left => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                true
            }
            KeyCode::Right => {
                let max_idx = self.visible_count().saturating_sub(1);
                if self.selected_index < max_idx {
                    self.selected_index += 1;
                }
                true
            }
            KeyCode::Up => {
                if self.selected_index >= 8 {
                    self.selected_index -= 8;
                }
                true
            }
            KeyCode::Down => {
                let max_idx = self.visible_count().saturating_sub(1);
                if self.selected_index + 8 <= max_idx {
                    self.selected_index += 8;
                }
                true
            }
            KeyCode::Enter => {
                self.select_color(self.selected_index);
                true
            }
            _ => false,
        }
    }

    pub fn render(&self, frame: &mut Frame<'_>, area: Rect) {
        let block = Block::default().title(" Palette ").borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width < 4 || inner.height < 2 {
            return;
        }

        let mut lines: Vec<Line<'_>> = Vec::new();

        let active_style = Style::default()
            .fg(self.theme.palette.active_target)
            .bg(self.theme.palette.cell_bg);
        let fg_label = if self.target == ColorTarget::Foreground {
            Span::styled(" [FG]", active_style)
        } else {
            Span::styled(" [FG]", Style::default())
        };
        let bg_label = if self.target == ColorTarget::Background {
            Span::styled(" [BG]", active_style)
        } else {
            Span::styled(" [BG]", Style::default())
        };
        lines.push(Line::from(vec![fg_label, Span::raw(" "), bg_label]));

        if self.show_extended {
            lines.push(Line::from(Span::raw(format!(
                " Ext pg:{}",
                self.extended_page + 1
            ))));

            for row in 0..2 {
                let mut spans = Vec::new();
                for col in 0..8 {
                    let idx = row * 8 + col;
                    if idx < 16 {
                        let color = extended_color(self.extended_page, idx as u8);
                        let swatch = if idx == self.selected_index {
                            Span::styled(
                                "██",
                                Style::default()
                                    .bg(color)
                                    .fg(self.theme.palette.swatch_indicator),
                            )
                        } else {
                            Span::styled("  ", Style::default().bg(color))
                        };
                        spans.push(swatch);
                    }
                }
                lines.push(Line::from(spans));
            }
            lines.push(Line::from(Span::raw(" < > pages")));
        } else {
            let mut row1 = Vec::new();
            for (col, color) in ANSI_16_COLORS.iter().enumerate().take(8) {
                let swatch = if col == self.selected_index {
                    Span::styled(
                        "██",
                        Style::default()
                            .bg(*color)
                            .fg(self.theme.palette.swatch_indicator),
                    )
                } else {
                    Span::styled("  ", Style::default().bg(*color))
                };
                row1.push(swatch);
            }
            lines.push(Line::from(row1));

            let mut row2 = Vec::new();
            for (col, color) in ANSI_16_COLORS.iter().enumerate().skip(8) {
                let swatch = if col == self.selected_index {
                    Span::styled(
                        "██",
                        Style::default()
                            .bg(*color)
                            .fg(self.theme.palette.swatch_indicator),
                    )
                } else {
                    Span::styled("  ", Style::default().bg(*color))
                };
                row2.push(swatch);
            }
            lines.push(Line::from(row2));
        }

        if self.custom_mode {
            let hex_display = format!(" #{}", self.custom_hex);
            lines.push(Line::from(Span::raw(hex_display)));
        } else {
            let custom_display = match self.selected_color {
                Some(Color::Rgb(r, g, b)) => format!(" #{:02X}{:02X}{:02X}", r, g, b),
                _ => " Cst:#......".to_string(),
            };
            lines.push(Line::from(Span::raw(custom_display)));
        }

        if !self.recent.is_empty() {
            lines.push(Line::from(Span::raw(" Recent:")));
            let mut recent_spans = Vec::new();
            for color in self.recent.iter().rev().take(8) {
                recent_spans.push(Span::styled("  ", Style::default().bg(*color)));
            }
            lines.push(Line::from(recent_spans));
        }

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::new()
    }
}
