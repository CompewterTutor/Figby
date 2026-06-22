use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, StatefulWidget, Widget};

use super::theme::Theme;
use super::toolbox::Tool;

#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    FileOpen,
    FileSave,
    FileSaveAs,
    FileExport,
    FileImportGif,
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
    ViewToggleTimeline,
    ViewToggleSidePanel,
    ViewLoadBuiltinPalette(&'static str),
    ViewPaletteEditor,
    LayerNew,
    LayerDuplicate,
    LayerDelete,
    LayerMergeDown,
    LayerMoveUp,
    LayerMoveDown,
    LayerToggleVisibility,
    LayerToggleLock,
    AnimFrameAdd,
    AnimFrameDelete,
    AnimPlay,
    AnimToggleTimeline,
    ImageResizeCanvas,
    ToolsSelect(Tool),
    HelpAbout,
    HelpKeybindings,
}

struct TopMenu {
    label: &'static str,
    items: Vec<(&'static str, MenuAction)>,
}

/// Persistent + per-frame state for `MenuBar` (held separately so `MenuBar`
/// can implement `StatefulWidget for &MenuBar`).
pub struct MenuBarState {
    /// Index of the open dropdown, None = closed.
    pub active_menu: Option<usize>,
    focused_item: usize,
    /// Header rects recorded during last render pass.
    pub header_rects: Vec<Rect>,
    /// Item rects for the currently open dropdown.
    pub item_rects: Vec<Rect>,
    frame_area: Rect,
    pending_action: Option<MenuAction>,
}

impl MenuBarState {
    pub fn new() -> Self {
        Self {
            active_menu: None,
            focused_item: 0,
            header_rects: Vec::new(),
            item_rects: Vec::new(),
            frame_area: Rect::default(),
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

    pub fn drain_actions(&mut self) -> Option<MenuAction> {
        self.pending_action.take()
    }
}

impl Default for MenuBarState {
    fn default() -> Self {
        Self::new()
    }
}

/// Static configuration for the menu bar (menus + theme).
/// Dynamic state lives in `MenuBarState`.
pub struct MenuBar {
    menus: Vec<TopMenu>,
    pub theme: Theme,
}

impl MenuBar {
    pub fn new() -> Self {
        Self {
            menus: build_menus(),
            theme: Theme::default(),
        }
    }

    pub fn handle_key_event(&self, key: KeyEvent, state: &mut MenuBarState) -> bool {
        let code = key.code;
        let modifiers = key.modifiers;

        if modifiers == KeyModifiers::ALT {
            if let KeyCode::Char(c) = code {
                if let Some(idx) = alt_menu_index(c) {
                    self.open_menu(state, idx);
                    return true;
                }
            }
        }

        if state.active_menu.is_some() {
            match code {
                KeyCode::Esc => {
                    state.reset();
                    true
                }
                KeyCode::Enter => {
                    self.select_focused(state);
                    true
                }
                KeyCode::Up => {
                    if state.focused_item > 0 {
                        state.focused_item -= 1;
                    } else if let Some(idx) = state.active_menu {
                        state.focused_item = self.menus[idx].items.len().saturating_sub(1);
                    }
                    true
                }
                KeyCode::Down => {
                    if let Some(idx) = state.active_menu {
                        let max = self.menus[idx].items.len().saturating_sub(1);
                        if state.focused_item < max {
                            state.focused_item += 1;
                        } else {
                            state.focused_item = 0;
                        }
                    }
                    true
                }
                KeyCode::Left => {
                    if let Some(idx) = state.active_menu {
                        let prev = if idx == 0 {
                            self.menus.len() - 1
                        } else {
                            idx - 1
                        };
                        self.open_menu(state, prev);
                    }
                    true
                }
                KeyCode::Right => {
                    if let Some(idx) = state.active_menu {
                        let next = (idx + 1) % self.menus.len();
                        self.open_menu(state, next);
                    }
                    true
                }
                _ => true,
            }
        } else {
            false
        }
    }

    pub fn handle_mouse_event(
        &self,
        col: u16,
        row: u16,
        kind: MouseEventKind,
        state: &mut MenuBarState,
    ) -> bool {
        match kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(item_idx) = self.hit_item(col, row, state) {
                    if let Some(menu_idx) = state.active_menu {
                        let menu = &self.menus[menu_idx];
                        if item_idx < menu.items.len() {
                            let action = menu.items[item_idx].1.clone();
                            state.pending_action = Some(action);
                        }
                    }
                    state.reset();
                    return true;
                }

                if let Some(header_idx) = self.hit_header(col, row, state) {
                    if state.active_menu == Some(header_idx) {
                        state.reset();
                    } else {
                        self.open_menu(state, header_idx);
                    }
                    return true;
                }

                if state.active_menu.is_some() {
                    state.reset();
                    return true;
                }

                false
            }
            _ => false,
        }
    }

    fn open_menu(&self, state: &mut MenuBarState, idx: usize) {
        state.active_menu = Some(idx);
        state.focused_item = 0;
    }

    fn select_focused(&self, state: &mut MenuBarState) {
        let Some(menu_idx) = state.active_menu else {
            return;
        };
        let menu = &self.menus[menu_idx];
        if state.focused_item < menu.items.len() {
            let action = menu.items[state.focused_item].1.clone();
            state.pending_action = Some(action);
        }
        state.reset();
    }

    fn hit_header(&self, col: u16, row: u16, state: &MenuBarState) -> Option<usize> {
        state
            .header_rects
            .iter()
            .enumerate()
            .find(|(_, r)| col >= r.x && col < r.x + r.width && row == r.y)
            .map(|(i, _)| i)
    }

    fn hit_item(&self, col: u16, row: u16, state: &MenuBarState) -> Option<usize> {
        state
            .item_rects
            .iter()
            .enumerate()
            .find(|(_, r)| col >= r.x && col < r.x + r.width && row >= r.y && row < r.y + r.height)
            .map(|(i, _)| i)
    }

    fn render_dropdown(&self, menu_idx: usize, buf: &mut Buffer, state: &mut MenuBarState) {
        let menu = &self.menus[menu_idx];
        let item_count = menu.items.len() as u16;

        let max_label_w: u16 = menu
            .items
            .iter()
            .map(|(label, _)| label.len() as u16)
            .max()
            .unwrap_or(10);
        let dropdown_w = max_label_w + 4;
        let dropdown_h = item_count + 2;

        let Some(&header_rect) = state.header_rects.get(menu_idx) else {
            return;
        };
        let drop_x = header_rect
            .x
            .min(state.frame_area.width.saturating_sub(dropdown_w));
        let drop_y = header_rect.y + 1;

        if drop_y >= state.frame_area.height {
            return;
        }
        let clamp_h = dropdown_h.min(state.frame_area.height.saturating_sub(drop_y));
        let dropdown_rect = Rect::new(drop_x, drop_y, dropdown_w, clamp_h);

        Widget::render(Clear, dropdown_rect, buf);
        let block = Block::default().borders(Borders::ALL).style(
            Style::default()
                .bg(self.theme.menu.dropdown_bg)
                .fg(self.theme.menu.fg),
        );
        Widget::render(block, dropdown_rect, buf);

        let inner_x = dropdown_rect.x + 1;
        let inner_y = dropdown_rect.y + 1;
        let inner_w = dropdown_rect.width.saturating_sub(2);
        let inner_h = dropdown_rect.height.saturating_sub(2);

        state.item_rects.clear();

        for (i, (label, _)) in menu.items.iter().enumerate() {
            if i as u16 >= inner_h {
                break;
            }
            let item_rect = Rect::new(inner_x, inner_y + i as u16, inner_w, 1);
            state.item_rects.push(item_rect);

            let is_focused = i == state.focused_item;
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
            Widget::render(Paragraph::new(padded).style(style), item_rect, buf);
        }
    }
}

impl Default for MenuBar {
    fn default() -> Self {
        Self::new()
    }
}

impl StatefulWidget for &MenuBar {
    type State = MenuBarState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.frame_area = buf.area;
        state.header_rects.clear();

        let bar_style = Style::default()
            .fg(self.theme.menu.fg)
            .bg(self.theme.menu.bg);

        Widget::render(
            Paragraph::new(Line::from(Span::styled(
                " ".repeat(area.width as usize),
                bar_style,
            ))),
            area,
            buf,
        );

        let mut x = area.x + 1;
        for (i, menu) in self.menus.iter().enumerate() {
            let label = format!(" {} ", menu.label);
            let label_w = label.len() as u16;

            if x + label_w > area.x + area.width {
                break;
            }

            let is_open = state.active_menu == Some(i);
            let style = if is_open {
                Style::default()
                    .fg(self.theme.menu.fg)
                    .bg(self.theme.menu.highlight)
                    .add_modifier(Modifier::BOLD)
            } else {
                bar_style
            };

            let header_rect = Rect::new(x, area.y, label_w, 1);
            state.header_rects.push(header_rect);

            Widget::render(
                Paragraph::new(Line::from(Span::styled(label, style))),
                header_rect,
                buf,
            );

            x += label_w;
        }

        if let Some(menu_idx) = state.active_menu {
            self.render_dropdown(menu_idx, buf, state);
        }
    }
}

fn alt_menu_index(c: char) -> Option<usize> {
    match c {
        'f' | 'F' => Some(0), // File
        'e' | 'E' => Some(1), // Edit
        'v' | 'V' => Some(2), // View
        'i' | 'I' => Some(3), // Image
        't' | 'T' => Some(4), // Tools
        'l' | 'L' => Some(5), // Layers
        'a' | 'A' => Some(6), // Animation
        'h' | 'H' => Some(7), // Help
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
                ("Import GIF", MenuAction::FileImportGif),
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
                ("Toggle Timeline", MenuAction::ViewToggleTimeline),
                ("Toggle Side Panel", MenuAction::ViewToggleSidePanel),
                ("Palette Editor", MenuAction::ViewPaletteEditor),
                (
                    "Palette: Grayscale",
                    MenuAction::ViewLoadBuiltinPalette("Grayscale"),
                ),
                (
                    "Palette: Primary",
                    MenuAction::ViewLoadBuiltinPalette("Primary"),
                ),
                ("Palette: Warm", MenuAction::ViewLoadBuiltinPalette("Warm")),
                ("Palette: Cool", MenuAction::ViewLoadBuiltinPalette("Cool")),
            ],
        },
        TopMenu {
            label: "Image",
            items: vec![("Resize Canvas", MenuAction::ImageResizeCanvas)],
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
                ("Lighting", MenuAction::ToolsSelect(Tool::Lighting)),
            ],
        },
        TopMenu {
            label: "Layers",
            items: vec![
                ("New Layer", MenuAction::LayerNew),
                ("Duplicate Layer", MenuAction::LayerDuplicate),
                ("Delete Layer", MenuAction::LayerDelete),
                ("Merge Down", MenuAction::LayerMergeDown),
                ("Move Up", MenuAction::LayerMoveUp),
                ("Move Down", MenuAction::LayerMoveDown),
                ("Toggle Visibility", MenuAction::LayerToggleVisibility),
                ("Toggle Lock", MenuAction::LayerToggleLock),
            ],
        },
        TopMenu {
            label: "Animation",
            items: vec![
                ("Add Frame", MenuAction::AnimFrameAdd),
                ("Delete Frame", MenuAction::AnimFrameDelete),
                ("Play / Pause", MenuAction::AnimPlay),
                ("Toggle Timeline", MenuAction::AnimToggleTimeline),
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
