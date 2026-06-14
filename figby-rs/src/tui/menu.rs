use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::Frame;
use tui_menu::{Menu, MenuItem, MenuState};

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

pub struct MenuBar {
    state: MenuState<MenuAction>,
    menu_area: Rect,
    pub theme: Theme,
}

impl MenuBar {
    pub fn new() -> Self {
        let items = build_menu_items();
        let state = MenuState::new(items);
        Self {
            state,
            menu_area: Rect::new(0, 0, 0, 0),
            theme: Theme::default(),
        }
    }

    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        let code = key.code;
        let modifiers = key.modifiers;

        if self.state.is_active() {
            match code {
                KeyCode::Esc | KeyCode::Enter if modifiers.is_empty() && code == KeyCode::Esc => {
                    self.state.reset();
                    return true;
                }
                KeyCode::Enter => {
                    self.state.select();
                    return true;
                }
                KeyCode::Up => {
                    self.state.up();
                    return true;
                }
                KeyCode::Down => {
                    self.state.down();
                    return true;
                }
                KeyCode::Left => {
                    self.state.left();
                    return true;
                }
                KeyCode::Right => {
                    self.state.right();
                    return true;
                }
                _ => {}
            }
            if modifiers == KeyModifiers::ALT {
                if let KeyCode::Char(c) = code {
                    if let Some(idx) = match c {
                        'f' | 'F' => Some(0),
                        'e' | 'E' => Some(1),
                        'v' | 'V' => Some(2),
                        't' | 'T' => Some(3),
                        'h' | 'H' => Some(4),
                        _ => None,
                    } {
                        self.state.reset();
                        self.state.activate();
                        for _ in 0..idx {
                            self.state.right();
                        }
                        return true;
                    }
                }
            }
            return true;
        }

        if modifiers == KeyModifiers::ALT {
            if let KeyCode::Char(c) = code {
                match c {
                    'f' | 'F' => {
                        self.state.activate();
                        true
                    }
                    'e' | 'E' => {
                        self.state.activate();
                        self.state.right();
                        true
                    }
                    'v' | 'V' => {
                        self.state.activate();
                        for _ in 0..2 {
                            self.state.right();
                        }
                        true
                    }
                    't' | 'T' => {
                        self.state.activate();
                        for _ in 0..3 {
                            self.state.right();
                        }
                        true
                    }
                    'h' | 'H' => {
                        self.state.activate();
                        for _ in 0..4 {
                            self.state.right();
                        }
                        true
                    }
                    _ => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn handle_mouse_event(&mut self, col: u16, row: u16, kind: MouseEventKind) -> bool {
        if self.menu_area.width == 0 {
            return false;
        }

        let labels = [" File ", " Edit ", " View ", " Tools ", " Help "];

        match kind {
            MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left) => {
                if row == self.menu_area.y && col >= self.menu_area.x {
                    let rel_x = col - self.menu_area.x;
                    let mut x_offset: u16 = 1;
                    for (i, label) in labels.iter().enumerate() {
                        let label_len = label.len() as u16;
                        if rel_x >= x_offset && rel_x < x_offset + label_len {
                            if self.state.is_active() {
                                self.state.reset();
                            }
                            self.state.activate();
                            for _ in 0..i {
                                self.state.right();
                            }
                            if kind == MouseEventKind::Down(MouseButton::Left) {
                                self.state.select();
                            }
                            return true;
                        }
                        x_offset += label_len;
                    }
                }
                if self.state.is_active() && kind == MouseEventKind::Down(MouseButton::Left) {
                    self.state.reset();
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    pub fn drain_actions(&mut self) -> Option<MenuAction> {
        if let Some(event) = self.state.drain_events().next() {
            let tui_menu::MenuEvent::Selected(action) = event;
            return Some(action);
        }
        None
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        self.menu_area = area;
        let menu_widget = Menu::<MenuAction>::new()
            .default_style(
                Style::default()
                    .fg(self.theme.menu.fg)
                    .bg(self.theme.menu.bg),
            )
            .highlight(
                Style::default()
                    .fg(self.theme.menu.fg)
                    .bg(self.theme.menu.highlight)
                    .add_modifier(Modifier::BOLD),
            )
            .dropdown_style(Style::default().bg(self.theme.menu.dropdown_bg))
            .dropdown_width(22);
        frame.render_stateful_widget(menu_widget, area, &mut self.state);
    }

    pub fn reset(&mut self) {
        self.state.reset();
    }
}

impl Default for MenuBar {
    fn default() -> Self {
        Self::new()
    }
}

fn build_menu_items() -> Vec<MenuItem<MenuAction>> {
    vec![
        // File menu
        MenuItem::group(
            "File",
            vec![
                MenuItem::item("Open", MenuAction::FileOpen),
                MenuItem::item("Save", MenuAction::FileSave),
                MenuItem::item("Save As", MenuAction::FileSaveAs),
                MenuItem::item("Export", MenuAction::FileExport),
                MenuItem::item("Quit", MenuAction::FileQuit),
            ],
        ),
        // Edit menu
        MenuItem::group(
            "Edit",
            vec![
                MenuItem::item("Undo", MenuAction::EditUndo),
                MenuItem::item("Redo", MenuAction::EditRedo),
                MenuItem::item("Cut", MenuAction::EditCut),
                MenuItem::item("Copy", MenuAction::EditCopy),
                MenuItem::item("Paste", MenuAction::EditPaste),
            ],
        ),
        // View menu
        MenuItem::group(
            "View",
            vec![
                MenuItem::item("Zoom In", MenuAction::ViewZoomIn),
                MenuItem::item("Zoom Out", MenuAction::ViewZoomOut),
                MenuItem::item("Toggle Grid", MenuAction::ViewToggleGrid),
                MenuItem::item("Toggle Undo Panel", MenuAction::ViewToggleUndoPanel),
            ],
        ),
        // Tools menu
        MenuItem::group(
            "Tools",
            vec![
                MenuItem::item("Brush", MenuAction::ToolsSelect(Tool::Brush)),
                MenuItem::item("Eraser", MenuAction::ToolsSelect(Tool::Eraser)),
                MenuItem::item("Line", MenuAction::ToolsSelect(Tool::Line)),
                MenuItem::item("Fill", MenuAction::ToolsSelect(Tool::Fill)),
                MenuItem::item("Marquee", MenuAction::ToolsSelect(Tool::Marquee)),
                MenuItem::item("Lasso", MenuAction::ToolsSelect(Tool::Lasso)),
                MenuItem::item("Circle Select", MenuAction::ToolsSelect(Tool::CircleSelect)),
                MenuItem::item(
                    "Polygon Select",
                    MenuAction::ToolsSelect(Tool::PolygonSelect),
                ),
                MenuItem::item("Eyedropper", MenuAction::ToolsSelect(Tool::Eyedropper)),
                MenuItem::item("Spray", MenuAction::ToolsSelect(Tool::Spray)),
                MenuItem::item("Text", MenuAction::ToolsSelect(Tool::Text)),
            ],
        ),
        // Help menu
        MenuItem::group(
            "Help",
            vec![
                MenuItem::item("About", MenuAction::HelpAbout),
                MenuItem::item("Keybindings", MenuAction::HelpKeybindings),
            ],
        ),
    ]
}
