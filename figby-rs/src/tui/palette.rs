use super::canvas::CanvasCell;
use super::theme::Theme;
use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

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
/// "deluxe" is listed first as the richest set, combining ASCII printable,
/// blocks, box drawing, dithered, geometric shapes, braille, and ogham.
pub const CHAR_GROUPS: &[CharGroup] = &[
    CharGroup {
        name: "deluxe",
        chars: concat!(
            " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~",
            "▀▁▂▃▄▅▆▇█▉▊▋▌▍▎▏▐░▒▓▔▕▖▗▘▙▚▛▜▝▞▟",
            "─━│┃┄┅┆┇┈┉┊┋┌┍┎┏┐┑┒┓└┕┖┗┘┙┚┛├┝┞┟┠┡┢┣┤┥┦┧┨┩┪┫┬┭┮┯┰┱┲┳┴┵┶┷┸┹┺┻┼┽┾┿╀╁╂╃╄╅╆╇╈╉╊╋╌╍╎╏═║╒╓╔╕╖╗╘╙╚╛╜╝╞╟╠╡╢╣╤╥╦╧╨╩╪╫╬╭╮╯╰╱╲╳╴╵╶╷╸╹╺╻╼╽╾╿",
            "░▒▓",
            "■□▪▫▲△▶▷▼▽◀◁◆◇◈◊○◎●◐◑◦◯",
            "⠀⠁⠂⠃⠄⠅⠆⠇⠈⠉⠊⠋⠌⠍⠎⠏⠐⠑⠒⠓⠔⠕⠖⠗⠘⠙⠚⠛⠜⠝⠞⠟⠠⠡⠢⠣⠤⠥⠦⠧⠨⠩⠪⠫⠬⠭⠮⠯⠰⠱⠲⠳⠴⠵⠶⠷⠸⠹⠺⠻⠼⠽⠾⠿⡀⡁⡂⡃⡄⡅⡆⡇⡈⡉⡊⡋⡌⡍⡎⡏⡐⡑⡒⡓⡔⡕⡖⡗⡘⡙⡚⡛⡜⡝⡞⡟⡠⡡⡢⡣⡤⡥⡦⡧⡨⡩⡪⡫⡬⡭⡮⡯⡰⡱⡲⡳⡴⡵⡶⡷⡸⡹⡺⡻⡼⡽⡾⡿⢀⢁⢂⢃⢄⢅⢆⢇⢈⢉⢊⢋⢌⢍⢎⢏⢐⢑⢒⢓⢔⢕⢖⢗⢘⢙⢚⢛⢜⢝⢞⢟⢠⢡⢢⢣⢤⢥⢦⢧⢨⢩⢪⢫⢬⢭⢮⢯⢰⢱⢲⢳⢴⢵⢶⢷⢸⢹⢺⢻⢼⢽⢾⢿⣀⣁⣂⣃⣄⣅⣆⣇⣈⣉⣊⣋⣌⣍⣎⣏⣐⣑⣒⣓⣔⣕⣖⣗⣘⣙⣚⣛⣜⣝⣞⣟⣠⣡⣢⣣⣤⣥⣦⣧⣨⣩⣪⣫⣬⣭⣮⣯⣰⣱⣲⣳⣴⣵⣶⣷⣸⣹⣺⣻⣼⣽⣾⣿",
            " ᚁᚂᚃᚄᚅᚆᚇᚈᚉᚊᚋᚌᚍᚎᚏᚐᚑᚒᚓᚔᚕᚖᚗᚘᚙᚚ᚛᚜",
        ),
    },
    CharGroup { name: "ascii",   chars: " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~" },
    CharGroup { name: "braille", chars: "⠀⠁⠂⠃⠄⠅⠆⠇⠈⠉⠊⠋⠌⠍⠎⠏⠐⠑⠒⠓⠔⠕⠖⠗⠘⠙⠚⠛⠜⠝⠞⠟⠠⠡⠢⠣⠤⠥⠦⠧⠨⠩⠪⠫⠬⠭⠮⠯⠰⠱⠲⠳⠴⠵⠶⠷⠸⠹⠺⠻⠼⠽⠾⠿⡀⡁⡂⡃⡄⡅⡆⡇⡈⡉⡊⡋⡌⡍⡎⡏⡐⡑⡒⡓⡔⡕⡖⡗⡘⡙⡚⡛⡜⡝⡞⡟⡠⡡⡢⡣⡤⡥⡦⡧⡨⡩⡪⡫⡬⡭⡮⡯⡰⡱⡲⡳⡴⡵⡶⡷⡸⡹⡺⡻⡼⡽⡾⡿⢀⢁⢂⢃⢄⢅⢆⢇⢈⢉⢊⢋⢌⢍⢎⢏⢐⢑⢒⢓⢔⢕⢖⢗⢘⢙⢚⢛⢜⢝⢞⢟⢠⢡⢢⢣⢤⢥⢦⢧⢨⢩⢪⢫⢬⢭⢮⢯⢰⢱⢲⢳⢴⢵⢶⢷⢸⢹⢺⢻⢼⢽⢾⢿⣀⣁⣂⣃⣄⣅⣆⣇⣈⣉⣊⣋⣌⣍⣎⣏⣐⣑⣒⣓⣔⣕⣖⣗⣘⣙⣚⣛⣜⣝⣞⣟⣠⣡⣢⣣⣤⣥⣦⣧⣨⣩⣪⣫⣬⣭⣮⣯⣰⣱⣲⣳⣴⣵⣶⣷⣸⣹⣺⣻⣼⣽⣾⣿" },
    CharGroup { name: "blocks",  chars: "▀▁▂▃▄▅▆▇█▉▊▋▌▍▎▏▐░▒▓▔▕▖▗▘▙▚▛▜▝▞▟" },
    CharGroup { name: "box",     chars: "─━│┃┄┅┆┇┈┉┊┋┌┍┎┏┐┑┒┓└┕┖┗┘┙┚┛├┝┞┟┠┡┢┣┤┥┦┧┨┩┪┫┬┭┮┯┰┱┲┳┴┵┶┷┸┹┺┻┼┽┾┿╀╁╂╃╄╅╆╇╈╉╊╋╌╍╎╏═║╒╓╔╕╖╗╘╙╚╛╜╝╞╟╠╡╢╣╤╥╦╧╨╩╪╫╬╭╮╯╰╱╲╳╴╵╶╷╸╹╺╻╼╽╾╿" },
    CharGroup { name: "dithered", chars: "░▒▓" },
    CharGroup { name: "geometric", chars: "■□▪▫▲△▶▷▼▽◀◁◆◇◈◊○◎●◐◑◦◯" },
    CharGroup { name: "ogham",   chars: " ᚁᚂᚃᚄᚅᚆᚇᚈᚉᚊᚋᚌᚍᚎᚏᚐᚑᚒᚓᚔᚕᚖᚗᚘᚙᚚ᚛᚜" },
];

pub const ANSI_COLOR_NAMES: [&str; 16] = [
    "Black",
    "Red",
    "Green",
    "Yellow",
    "Blue",
    "Magenta",
    "Cyan",
    "White",
    "Bright Black",
    "Bright Red",
    "Bright Green",
    "Bright Yellow",
    "Bright Blue",
    "Bright Magenta",
    "Bright Cyan",
    "Bright White",
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
    pub hover_index: Option<usize>,
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
            hover_index: None,
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

    fn color_name(&self, index: usize) -> String {
        if self.show_extended {
            let abs = 16u16 + self.extended_page as u16 * 16 + index as u16;
            format!("Color {}", abs)
        } else if index < 16 {
            ANSI_COLOR_NAMES[index].to_string()
        } else {
            String::new()
        }
    }

    /// Hit-test a mouse move at terminal coordinates (`col`, `row`) against `area`
    /// and set `hover_index` accordingly. Returns `true` if hover state changed.
    pub fn handle_hover(&mut self, col: u16, row: u16, area: Rect) -> bool {
        let ix = area.x + 1;
        let iy = area.y + 1;
        let iw = area.width.saturating_sub(2);
        let ih = area.height.saturating_sub(2);
        if col < ix || col >= ix + iw || row < iy || row >= iy + ih {
            let changed = self.hover_index.is_some();
            self.hover_index = None;
            return changed;
        }
        let rel_col = col - ix;
        let rel_row = row - iy;

        // Row 0 is FG/BG toggle — not a swatch
        // Swatch rows for standard: 1-2; for extended: 2-3
        let (swatch_start, swatch_end) = if self.show_extended {
            (2u16, 4u16)
        } else {
            (1u16, 3u16)
        };

        if (swatch_start..swatch_end).contains(&rel_row) {
            let swatch_col = (rel_col / 2) as usize;
            if swatch_col < 8 {
                let idx = (rel_row as usize - swatch_start as usize) * 8 + swatch_col;
                if idx < 16 {
                    if self.hover_index != Some(idx) {
                        self.hover_index = Some(idx);
                        return true;
                    }
                    return false;
                }
            }
        }

        let changed = self.hover_index.is_some();
        self.hover_index = None;
        changed
    }

    /// Hit-test a left click at terminal coordinates (`col`, `row`) against `area`
    /// (the full panel rect including border). Returns true if the click landed on
    /// a colour swatch or FG/BG toggle and state changed.
    pub fn handle_click(&mut self, col: u16, row: u16, area: Rect) -> bool {
        // inner area: strip 1-cell border on each side
        let ix = area.x + 1;
        let iy = area.y + 1;
        let iw = area.width.saturating_sub(2);
        let ih = area.height.saturating_sub(2);
        if col < ix || col >= ix + iw || row < iy || row >= iy + ih {
            return false;
        }
        let rel_col = col - ix;
        let rel_row = row - iy;

        // Row 0: " [FG]" (0..4)  " " (5)  " [BG]" (6..10)
        if rel_row == 0 {
            if rel_col < 5 {
                self.target = ColorTarget::Foreground;
                return true;
            } else if (6..11).contains(&rel_col) {
                self.target = ColorTarget::Background;
                return true;
            }
            return false;
        }

        if self.show_extended {
            // Row 1: "Ext pg:N" — not clickable
            // Rows 2-3: 8 swatches each, 2 cols wide
            if (2..=3).contains(&rel_row) {
                let swatch_col = (rel_col / 2) as usize;
                if swatch_col < 8 {
                    let idx = (rel_row as usize - 2) * 8 + swatch_col;
                    self.selected_index = idx;
                    self.selected_color = Some(extended_color(self.extended_page, idx as u8));
                    return true;
                }
            }
        } else {
            // Row 1: ANSI 0-7, Row 2: ANSI 8-15; 2 cols per swatch
            if rel_row == 1 || rel_row == 2 {
                let swatch_col = (rel_col / 2) as usize;
                if swatch_col < 8 {
                    let idx = (rel_row as usize - 1) * 8 + swatch_col;
                    self.select_color(idx);
                    return true;
                }
            }
        }

        false
    }
}

impl Widget for &Palette {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().title(" Palette ").borders(Borders::ALL);
        let inner = block.inner(area);
        Widget::render(block, area, buf);

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

        // Hover tooltip: show color name below swatches
        if let Some(hover) = self.hover_index {
            let name = self.color_name(hover);
            if !name.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!(" {}", name),
                    Style::default().fg(self.theme.general.secondary),
                )));
            }
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
        Widget::render(paragraph, inner, buf);
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Palette;
    use super::CHAR_GROUPS;
    use ratatui::layout::Rect;

    #[test]
    fn test_braille_palette_group_length() {
        let braille = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "braille")
            .expect("braille group should exist");
        let count = braille.chars.chars().count();
        assert_eq!(
            count, 256,
            "braille group should have exactly 256 chars, got {count}"
        );
    }

    #[test]
    fn test_braille_palette_all_in_range() {
        let braille = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "braille")
            .expect("braille group should exist");
        for c in braille.chars.chars() {
            let cp = c as u32;
            assert!(
                (0x2800..=0x28FF).contains(&cp),
                "braille char U+{cp:04X} outside U+2800–U+28FF"
            );
        }
    }

    #[test]
    fn test_braille_palette_all_256_unique() {
        let braille = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "braille")
            .expect("braille group should exist");
        let mut cps: Vec<u32> = braille.chars.chars().map(|c| c as u32).collect();
        assert_eq!(cps.len(), 256, "should have 256 braille chars");
        cps.sort_unstable();
        cps.dedup();
        assert_eq!(cps.len(), 256, "should have 256 unique braille codepoints");
        assert_eq!(cps[0], 0x2800, "first codepoint should be U+2800");
        assert_eq!(cps[255], 0x28FF, "last codepoint should be U+28FF");
        for (i, &cp) in cps.iter().enumerate() {
            assert_eq!(
                cp,
                0x2800 + i as u32,
                "missing codepoint U+{:04X}",
                0x2800 + i as u32
            );
        }
    }

    #[test]
    fn test_blocks_palette_count_32() {
        let blocks = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "blocks")
            .expect("blocks group should exist");
        let count = blocks.chars.chars().count();
        assert_eq!(
            count, 32,
            "blocks group should have exactly 32 chars, got {count}"
        );
    }

    #[test]
    fn test_blocks_palette_all_in_range() {
        let blocks = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "blocks")
            .expect("blocks group should exist");
        for c in blocks.chars.chars() {
            let cp = c as u32;
            assert!(
                (0x2580..=0x259F).contains(&cp),
                "blocks char U+{cp:04X} outside U+2580–U+259F"
            );
        }
    }

    #[test]
    fn test_blocks_palette_all_32_unique() {
        let blocks = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "blocks")
            .expect("blocks group should exist");
        let mut cps: Vec<u32> = blocks.chars.chars().map(|c| c as u32).collect();
        assert_eq!(cps.len(), 32, "should have 32 blocks chars");
        cps.sort_unstable();
        cps.dedup();
        assert_eq!(cps.len(), 32, "should have 32 unique blocks codepoints");
        assert_eq!(cps[0], 0x2580, "first codepoint should be U+2580");
        assert_eq!(cps[31], 0x259F, "last codepoint should be U+259F");
        for (i, &cp) in cps.iter().enumerate() {
            assert_eq!(
                cp,
                0x2580 + i as u32,
                "missing codepoint U+{:04X}",
                0x2580 + i as u32
            );
        }
    }

    // --- Box palette tests ---

    #[test]
    fn test_box_palette_count_128() {
        let box_group = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "box")
            .expect("box group should exist");
        let count = box_group.chars.chars().count();
        assert_eq!(
            count, 128,
            "box group should have exactly 128 chars, got {count}"
        );
    }

    #[test]
    fn test_box_palette_all_in_range() {
        let box_group = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "box")
            .expect("box group should exist");
        for c in box_group.chars.chars() {
            let cp = c as u32;
            assert!(
                (0x2500..=0x257F).contains(&cp),
                "box char U+{cp:04X} outside U+2500–U+257F"
            );
        }
    }

    #[test]
    fn test_box_palette_all_128_unique() {
        let box_group = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "box")
            .expect("box group should exist");
        let mut cps: Vec<u32> = box_group.chars.chars().map(|c| c as u32).collect();
        assert_eq!(cps.len(), 128, "should have 128 box chars");
        cps.sort_unstable();
        cps.dedup();
        assert_eq!(cps.len(), 128, "should have 128 unique box codepoints");
        assert_eq!(cps[0], 0x2500, "first codepoint should be U+2500");
        assert_eq!(cps[127], 0x257F, "last codepoint should be U+257F");
        for (i, &cp) in cps.iter().enumerate() {
            assert_eq!(
                cp,
                0x2500 + i as u32,
                "missing codepoint U+{:04X}",
                0x2500 + i as u32
            );
        }
    }

    // --- Dithered palette tests ---

    #[test]
    fn test_dithered_palette_count_3() {
        let dithered = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "dithered")
            .expect("dithered group should exist");
        let count = dithered.chars.chars().count();
        assert_eq!(
            count, 3,
            "dithered group should have exactly 3 chars, got {count}"
        );
    }

    #[test]
    fn test_dithered_palette_all_in_range() {
        let dithered = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "dithered")
            .expect("dithered group should exist");
        for c in dithered.chars.chars() {
            let cp = c as u32;
            assert!(
                (0x2591..=0x2593).contains(&cp),
                "dithered char U+{cp:04X} outside U+2591–U+2593"
            );
        }
    }

    #[test]
    fn test_dithered_palette_all_unique() {
        let dithered = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "dithered")
            .expect("dithered group should exist");
        let mut cps: Vec<u32> = dithered.chars.chars().map(|c| c as u32).collect();
        assert_eq!(cps.len(), 3, "should have 3 dithered chars");
        cps.sort_unstable();
        cps.dedup();
        assert_eq!(cps.len(), 3, "should have 3 unique dithered codepoints");
        assert_eq!(cps[0], 0x2591, "first should be U+2591");
        assert_eq!(cps[2], 0x2593, "last should be U+2593");
    }

    // --- Geometric palette tests ---

    #[test]
    fn test_geometric_palette_count_23() {
        let geometric = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "geometric")
            .expect("geometric group should exist");
        let count = geometric.chars.chars().count();
        assert_eq!(
            count, 23,
            "geometric group should have exactly 23 chars, got {count}"
        );
    }

    #[test]
    fn test_geometric_palette_all_in_range() {
        let geometric = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "geometric")
            .expect("geometric group should exist");
        for c in geometric.chars.chars() {
            let cp = c as u32;
            assert!(
                (0x25A0..=0x25FF).contains(&cp),
                "geometric char U+{cp:04X} outside U+25A0–U+25FF"
            );
        }
    }

    #[test]
    fn test_geometric_palette_all_unique() {
        let geometric = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "geometric")
            .expect("geometric group should exist");
        let mut cps: Vec<u32> = geometric.chars.chars().map(|c| c as u32).collect();
        assert_eq!(cps.len(), 23, "should have 23 geometric chars");
        cps.sort_unstable();
        cps.dedup();
        assert_eq!(cps.len(), 23, "should have 23 unique geometric codepoints");
    }

    // --- Ogham palette tests ---

    #[test]
    fn test_ogham_palette_count() {
        let ogham = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "ogham")
            .expect("ogham group should exist");
        let count = ogham.chars.chars().count();
        assert_eq!(
            count, 29,
            "ogham group should have exactly 29 chars, got {count}"
        );
    }

    #[test]
    fn test_ogham_palette_all_in_range() {
        let ogham = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "ogham")
            .expect("ogham group should exist");
        for c in ogham.chars.chars() {
            let cp = c as u32;
            assert!(
                (0x1680..=0x169F).contains(&cp),
                "ogham char U+{cp:04X} outside U+1680–U+169F"
            );
        }
    }

    #[test]
    fn test_ogham_palette_all_unique() {
        let ogham = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "ogham")
            .expect("ogham group should exist");
        let mut cps: Vec<u32> = ogham.chars.chars().map(|c| c as u32).collect();
        assert_eq!(cps.len(), 29, "should have 29 ogham chars");
        cps.sort_unstable();
        cps.dedup();
        assert_eq!(cps.len(), 29, "should have 29 unique ogham codepoints");
        assert_eq!(cps[0], 0x1680, "first codepoint should be U+1680");
        assert_eq!(cps[28], 0x169C, "last codepoint should be U+169C");
        for (i, &cp) in cps.iter().enumerate() {
            assert_eq!(
                cp,
                0x1680 + i as u32,
                "missing codepoint U+{:04X}",
                0x1680 + i as u32
            );
        }
    }

    // --- Deluxe palette tests ---

    #[test]
    fn test_deluxe_palette_count() {
        let deluxe = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "deluxe")
            .expect("deluxe group should exist");
        let count = deluxe.chars.chars().count();
        assert_eq!(
            count, 566,
            "deluxe group should have exactly 566 chars, got {count}"
        );
    }

    #[test]
    fn test_deluxe_palette_contains_all_subset_chars() {
        let deluxe = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "deluxe")
            .expect("deluxe group should exist");
        let deluxe_set: std::collections::HashSet<char> = deluxe.chars.chars().collect();
        for group in CHAR_GROUPS.iter().filter(|g| g.name != "deluxe") {
            for c in group.chars.chars() {
                assert!(
                    deluxe_set.contains(&c),
                    "deluxe should contain char U+{:04X} from '{}' group",
                    c as u32,
                    group.name
                );
            }
        }
    }

    #[test]
    fn test_hover_on_swatch() {
        let mut p = Palette::new();
        let area = Rect::new(0, 0, 20, 10);
        // Swatch row 1 (inner row 1), col 0 → index 0
        assert!(p.handle_hover(1, 2, area));
        assert_eq!(p.hover_index, Some(0));
    }

    #[test]
    fn test_hover_outside_clears() {
        let mut p = Palette::new();
        let area = Rect::new(0, 0, 20, 10);
        assert!(p.handle_hover(1, 2, area));
        assert_eq!(p.hover_index, Some(0));
        assert!(p.handle_hover(1, 0, area));
        assert_eq!(p.hover_index, None);
    }

    #[test]
    fn test_hover_outside_palette_clears() {
        let mut p = Palette::new();
        let area = Rect::new(0, 0, 20, 10);
        p.handle_hover(1, 2, area);
        assert!(p.handle_hover(30, 30, area));
        assert_eq!(p.hover_index, None);
    }

    #[test]
    fn test_color_name_standard() {
        let p = Palette::new();
        assert_eq!(p.color_name(0), "Black");
        assert_eq!(p.color_name(9), "Bright Red");
        assert_eq!(p.color_name(15), "Bright White");
    }

    #[test]
    fn test_color_name_extended() {
        let mut p = Palette::new();
        p.show_extended = true;
        let name = p.color_name(0);
        assert!(name.contains("Color 16"));
    }

    #[test]
    fn test_deluxe_palette_all_unique() {
        let deluxe = CHAR_GROUPS
            .iter()
            .find(|g| g.name == "deluxe")
            .expect("deluxe group should exist");
        let mut cps: Vec<u32> = deluxe.chars.chars().map(|c| c as u32).collect();
        assert_eq!(cps.len(), 566, "should have 566 deluxe chars");
        cps.sort_unstable();
        cps.dedup();
        // Dups come from dithered subset of blocks (░▒▓ = U+2591-U+2593)
        assert_eq!(
            cps.len(),
            563,
            "should have 563 unique deluxe codepoints (3 dithered are subset of blocks)"
        );
    }
}
