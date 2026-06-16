use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::theme::Theme;
use super::toolbox::Tool;

#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    FileOpen,
    FileSave,
    FileSaveAs,
    FileExport,
    FileQuit,
    EditUndo,
    EditRedo,
    EditCut,
    EditCopy,
    EditPaste,
    ViewZoomIn,
    ViewZoomOut,
    ViewToggleGrid,
    ViewToggleUndoPanel,
    ToolsSelect(Tool),
    HelpAbout,
    HelpKeybindings,
}

struct TopMenu {
    label: &'static str,
    items: Vec<(&'static str, MenuAction)>,
}

pub struct MenuBar {
    menus: Vec<TopMenu>,
    /// Index of the open dropdown, None = closed
    pub active_menu: Option<usize>,
    /// Focused item index within the open dropdown
    focused_item: usize,
    /// Header rects recorded during last draw (one per top-level menu)
    header_rects: Vec<Rect>,
    /// Item rects recorded during last draw for the open dropdown
    item_rects: Vec<Rect>,
    /// Full terminal area from last draw (needed to clamp dropdown)
    frame_area: Rect,
    pub theme: Theme,
    pending_action: Option<MenuAction>,
}

impl MenuBar {
    pub fn new() -> Self {
        Self {
            menus: build_menus(),
            active_menu: None,
            focused_item: 0,
            header_rects: Vec::new(),
            item_rects: Vec::new(),
            frame_area: Rect::default(),
            theme: Theme::default(),
            pending_action: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active_menu.is_some()
    }

    pub fn reset(&mut self) {
        self.active_menu = None;
        self.focused_item = 0;
        self.item_rects.clear();
    }

    fn open_menu(&mut self, idx: usize) {
        self.active_menu = Some(idx);
        self.focused_item = 0;
    }

    fn select_focused(&mut self) {
        let Some(menu_idx) = self.active_menu else {
            return;
        };
        let menu = &self.menus[menu_idx];
        if self.focused_item < menu.items.len() {
            let action = menu.items[self.focused_item].1.clone();
            self.pending_action = Some(action);
        }
        self.reset();
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        let code = key.code;
        let modifiers = key.modifiers;

        // Alt+letter shortcuts work regardless of active state
        if modifiers == KeyModifiers::ALT {
            if let KeyCode::Char(c) = code {
                if let Some(idx) = alt_menu_index(c) {
                    self.open_menu(idx);
                    return true;
                }
            }
        }

        if self.active_menu.is_some() {
            match code {
                KeyCode::Esc => {
                    self.reset();
                    true
                }
                KeyCode::Enter => {
                    self.select_focused();
                    true
                }
                KeyCode::Up => {
                    if self.focused_item > 0 {
                        self.focused_item -= 1;
                    } else if let Some(idx) = self.active_menu {
                        self.focused_item = self.menus[idx].items.len().saturating_sub(1);
                    }
                    true
                }
                KeyCode::Down => {
                    if let Some(idx) = self.active_menu {
                        let max = self.menus[idx].items.len().saturating_sub(1);
                        if self.focused_item < max {
                            self.focused_item += 1;
                        } else {
                            self.focused_item = 0;
                        }
                    }
                    true
                }
                KeyCode::Left => {
                    if let Some(idx) = self.active_menu {
                        let prev = if idx == 0 {
                            self.menus.len() - 1
                        } else {
                            idx - 1
                        };
                        self.open_menu(prev);
                    }
                    true
                }
                KeyCode::Right => {
                    if let Some(idx) = self.active_menu {
                        let next = (idx + 1) % self.menus.len();
                        self.open_menu(next);
                    }
                    true
                }
                _ => true, // swallow all keys while menu is open
            }
        } else {
            false
        }
    }

    pub fn handle_mouse_event(&mut self, col: u16, row: u16, kind: MouseEventKind) -> bool {
        match kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Check dropdown items first (they overlap the frame area)
                if let Some(item_idx) = self.hit_item(col, row) {
                    if let Some(menu_idx) = self.active_menu {
                        let menu = &self.menus[menu_idx];
                        if item_idx < menu.items.len() {
                            let action = menu.items[item_idx].1.clone();
                            self.pending_action = Some(action);
                        }
                    }
                    self.reset();
                    return true;
                }

                // Check headers
                if let Some(header_idx) = self.hit_header(col, row) {
                    if self.active_menu == Some(header_idx) {
                        // Toggle: clicking same header closes menu
                        self.reset();
                    } else {
                        self.open_menu(header_idx);
                    }
                    return true;
                }

                // Click outside while open → close
                if self.active_menu.is_some() {
                    self.reset();
                    return true;
                }

                false
            }
            _ => false,
        }
    }

    fn hit_header(&self, col: u16, row: u16) -> Option<usize> {
        for (i, rect) in self.header_rects.iter().enumerate() {
            if col >= rect.x && col < rect.x + rect.width && row == rect.y {
                return Some(i);
            }
        }
        None
    }

    fn hit_item(&self, col: u16, row: u16) -> Option<usize> {
        for (i, rect) in self.item_rects.iter().enumerate() {
            if col >= rect.x
                && col < rect.x + rect.width
                && row >= rect.y
                && row < rect.y + rect.height
            {
                return Some(i);
            }
        }
        None
    }

    pub fn drain_actions(&mut self) -> Option<MenuAction> {
        self.pending_action.take()
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        self.frame_area = frame.area();
        self.header_rects.clear();

        // Render menu bar background
        let bar_style = Style::default()
            .fg(self.theme.menu.fg)
            .bg(self.theme.menu.bg);
        let bar = Paragraph::new(" ".repeat(area.width as usize)).style(bar_style);
        frame.render_widget(bar, area);

        // Render each header, record rects
        let mut x = area.x + 1;
        for (i, menu) in self.menus.iter().enumerate() {
            let label = format!(" {} ", menu.label);
            let label_w = label.len() as u16;

            if x + label_w > area.x + area.width {
                break;
            }

            let is_open = self.active_menu == Some(i);
            let style = if is_open {
                Style::default()
                    .fg(self.theme.menu.fg)
                    .bg(self.theme.menu.highlight)
                    .add_modifier(Modifier::BOLD)
            } else {
                bar_style
            };

            let header_rect = Rect::new(x, area.y, label_w, 1);
            self.header_rects.push(header_rect);

            let span = Span::styled(label, style);
            let line = Line::from(span);
            frame.render_widget(Paragraph::new(line), header_rect);

            x += label_w;
        }

        // Render open dropdown
        if let Some(menu_idx) = self.active_menu {
            self.render_dropdown(frame, menu_idx);
        }
    }

    fn render_dropdown(&mut self, frame: &mut Frame, menu_idx: usize) {
        let menu = &self.menus[menu_idx];
        let item_count = menu.items.len() as u16;

        // Compute dropdown rect: 2 border chars + max item label width
        let max_label_w: u16 = menu
            .items
            .iter()
            .map(|(label, _)| label.len() as u16)
            .max()
            .unwrap_or(10);
        let dropdown_w = max_label_w + 4; // 1 space padding each side + 2 borders
        let dropdown_h = item_count + 2; // items + top/bottom border

        // Position below the open header
        let Some(&header_rect) = self.header_rects.get(menu_idx) else {
            return;
        };
        let drop_x = header_rect
            .x
            .min(self.frame_area.width.saturating_sub(dropdown_w));
        let drop_y = header_rect.y + 1;

        if drop_y >= self.frame_area.height {
            return;
        }
        let clamp_h = dropdown_h.min(self.frame_area.height.saturating_sub(drop_y));
        let dropdown_rect = Rect::new(drop_x, drop_y, dropdown_w, clamp_h);

        // Clear area and draw bordered box
        frame.render_widget(Clear, dropdown_rect);
        let block = Block::default().borders(Borders::ALL).style(
            Style::default()
                .bg(self.theme.menu.dropdown_bg)
                .fg(self.theme.menu.fg),
        );
        frame.render_widget(block, dropdown_rect);

        // Inner area for items
        let inner_x = dropdown_rect.x + 1;
        let inner_y = dropdown_rect.y + 1;
        let inner_w = dropdown_rect.width.saturating_sub(2);
        let inner_h = dropdown_rect.height.saturating_sub(2);

        self.item_rects.clear();

        for (i, (label, _)) in menu.items.iter().enumerate() {
            let item_row = inner_y + i as u16;
            if i as u16 >= inner_h {
                break;
            }
            let item_rect = Rect::new(inner_x, item_row, inner_w, 1);
            self.item_rects.push(item_rect);

            let is_focused = i == self.focused_item;
            let style = if is_focused {
                Style::default()
                    .fg(self.theme.menu.fg)
                    .bg(self.theme.menu.highlight)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(self.theme.menu.fg)
                    .bg(self.theme.menu.dropdown_bg)
            };

            let padded = format!(
                " {:<width$} ",
                label,
                width = (inner_w as usize).saturating_sub(2)
            );
            frame.render_widget(Paragraph::new(padded).style(style), item_rect);
        }
    }
}

impl Default for MenuBar {
    fn default() -> Self {
        Self::new()
    }
}

fn alt_menu_index(c: char) -> Option<usize> {
    match c {
        'f' | 'F' => Some(0),
        'e' | 'E' => Some(1),
        'v' | 'V' => Some(2),
        't' | 'T' => Some(3),
        'h' | 'H' => Some(4),
        _ => None,
    }
}

fn build_menus() -> Vec<TopMenu> {
    vec![
        TopMenu {
            label: "File",
            items: vec![
                ("Open", MenuAction::FileOpen),
                ("Save", MenuAction::FileSave),
                ("Save As", MenuAction::FileSaveAs),
                ("Export", MenuAction::FileExport),
                ("Quit", MenuAction::FileQuit),
            ],
        },
        TopMenu {
            label: "Edit",
            items: vec![
                ("Undo", MenuAction::EditUndo),
                ("Redo", MenuAction::EditRedo),
                ("Cut", MenuAction::EditCut),
                ("Copy", MenuAction::EditCopy),
                ("Paste", MenuAction::EditPaste),
            ],
        },
        TopMenu {
            label: "View",
            items: vec![
                ("Zoom In", MenuAction::ViewZoomIn),
                ("Zoom Out", MenuAction::ViewZoomOut),
                ("Toggle Grid", MenuAction::ViewToggleGrid),
                ("Toggle Undo Panel", MenuAction::ViewToggleUndoPanel),
            ],
        },
        TopMenu {
            label: "Tools",
            items: vec![
                ("Brush", MenuAction::ToolsSelect(Tool::Brush)),
                ("Eraser", MenuAction::ToolsSelect(Tool::Eraser)),
                ("Line", MenuAction::ToolsSelect(Tool::Line)),
                ("Fill", MenuAction::ToolsSelect(Tool::Fill)),
                ("Marquee", MenuAction::ToolsSelect(Tool::Marquee)),
                ("Lasso", MenuAction::ToolsSelect(Tool::Lasso)),
                ("Circle Select", MenuAction::ToolsSelect(Tool::CircleSelect)),
                (
                    "Polygon Select",
                    MenuAction::ToolsSelect(Tool::PolygonSelect),
                ),
                ("Eyedropper", MenuAction::ToolsSelect(Tool::Eyedropper)),
                ("Spray", MenuAction::ToolsSelect(Tool::Spray)),
                ("Text", MenuAction::ToolsSelect(Tool::Text)),
            ],
        },
        TopMenu {
            label: "Help",
            items: vec![
                ("About", MenuAction::HelpAbout),
                ("Keybindings", MenuAction::HelpKeybindings),
            ],
        },
    ]
}
