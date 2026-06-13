use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseEvent,
    MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Tabs};
use ratatui::Frame;
use std::collections::BTreeMap;
use std::io;
use std::time::Duration;

pub mod brush;
pub mod canvas;
pub mod palette;
pub mod status;
pub mod toolbox;
pub mod tools;

pub use brush::BrushState;
pub use palette::Palette;
pub use status::CanvasSettings;
pub use toolbox::Tool;

const ICONS_YAML: &str = include_str!("../../../assets/tui/icons.yaml");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    FontEditor,
    ImageEditor,
    AsciiPreview,
}

impl AppMode {
    pub fn title(&self) -> &str {
        match self {
            AppMode::FontEditor => " Font Editor ",
            AppMode::ImageEditor => " Image Editor ",
            AppMode::AsciiPreview => " ASCII Preview ",
        }
    }

    fn next(&self) -> Self {
        match self {
            AppMode::FontEditor => AppMode::ImageEditor,
            AppMode::ImageEditor => AppMode::AsciiPreview,
            AppMode::AsciiPreview => AppMode::FontEditor,
        }
    }
}

pub struct TuiApp {
    pub mode: AppMode,
    pub should_quit: bool,
    _icons: BTreeMap<String, String>,
    pub toolbox: toolbox::Toolbox,
    pub canvas: canvas::CanvasWidget,
    pub palette: palette::Palette,
    pub brush: brush::BrushState,
    pub unsaved: bool,
    pub settings: status::CanvasSettings,
    last_canvas_size: (u16, u16),
    canvas_inner_rect: Rect,
    prev_mouse_buf: Option<(i16, i16)>,
}

impl TuiApp {
    pub fn new() -> Self {
        let icons = serde_yaml::from_str(ICONS_YAML).unwrap_or_default();
        Self {
            mode: AppMode::FontEditor,
            should_quit: false,
            _icons: icons,
            toolbox: toolbox::Toolbox::new(),
            canvas: canvas::CanvasWidget::default(),
            palette: palette::Palette::new(),
            brush: brush::BrushState::new(),
            last_canvas_size: (0, 0),
            canvas_inner_rect: Rect::new(0, 0, 0, 0),
            prev_mouse_buf: None,
            unsaved: false,
            settings: status::CanvasSettings::new(),
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        use ratatui::backend::CrosstermBackend;
        use ratatui::Terminal;

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        while !self.should_quit {
            terminal.draw(|f| self.render(f))?;
            self.handle_event()?;
        }

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            DisableMouseCapture,
            LeaveAlternateScreen
        )?;
        terminal.show_cursor()?;
        Ok(())
    }

    pub fn render(&mut self, frame: &mut Frame<'_>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(frame.area());

        let titles = vec![" Font Editor ", " Image Editor ", " ASCII Preview "];
        let selected = match self.mode {
            AppMode::FontEditor => 0,
            AppMode::ImageEditor => 1,
            AppMode::AsciiPreview => 2,
        };
        let tabs = Tabs::new(titles)
            .block(Block::default().title("Mode").borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .select(selected);
        frame.render_widget(tabs, chunks[0]);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(8),
                Constraint::Min(10),
                Constraint::Length(20),
            ])
            .split(chunks[1]);

        let tool_brush_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(10), Constraint::Length(9)])
            .split(main_chunks[0]);
        self.toolbox.render(frame, tool_brush_chunks[0]);
        self.brush.render(frame, tool_brush_chunks[1]);

        let block = Block::default()
            .title(self.mode.title())
            .borders(Borders::ALL);
        let inner = block.inner(main_chunks[1]);
        self.last_canvas_size = (inner.width, inner.height);
        self.canvas_inner_rect = inner;
        self.canvas.ensure_cursor_visible(inner.width, inner.height);
        frame.render_widget(block, main_chunks[1]);
        frame.render_widget(&self.canvas, inner);

        if self.settings.settings_open {
            self.settings.render(frame, main_chunks[2]);
        } else {
            self.palette.render(frame, main_chunks[2]);
        }

        let mode_name = match self.mode {
            AppMode::FontEditor => "Font Editor",
            AppMode::ImageEditor => "Image Editor",
            AppMode::AsciiPreview => "ASCII Preview",
        };
        status::StatusBar::render(
            frame,
            chunks[2],
            self.canvas.cursor(),
            self.canvas.zoom_level(),
            self.toolbox.selected.full_name(),
            mode_name,
            self.unsaved,
            &self._icons,
        );
    }

    fn screen_to_buffer(&self, col: u16, row: u16) -> Option<(i16, i16)> {
        let zoom = self.canvas.zoom_level().max(1) as i16;
        let area = self.canvas_inner_rect;
        if col < area.x || col >= area.x + area.width {
            return None;
        }
        if row < area.y || row >= area.y + area.height {
            return None;
        }
        let (sx, sy) = self.canvas.scroll_offset();
        let bx = sx as i16 + (col as i16 - area.x as i16) / zoom;
        let by = sy as i16 + (row as i16 - area.y as i16) / zoom;
        Some((bx, by))
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        if self.settings.settings_open {
            return;
        }
        if !matches!(self.toolbox.selected, Tool::Brush | Tool::Eraser) {
            self.prev_mouse_buf = None;
            return;
        }
        match mouse.kind {
            MouseEventKind::Down(_) => {
                let Some((bx, by)) = self.screen_to_buffer(mouse.column, mouse.row) else {
                    self.prev_mouse_buf = None;
                    return;
                };
                self.canvas.set_cursor(bx.max(0) as u16, by.max(0) as u16);
                self.unsaved = true;
                if self.toolbox.selected == Tool::Eraser {
                    tools::eraser::erase_stamp(
                        &mut self.canvas.buffer,
                        bx,
                        by,
                        self.brush.shape,
                        self.brush.size,
                    );
                } else {
                    let mut cell = canvas::CanvasCell {
                        ch: '\u{2588}',
                        fg: None,
                        bg: None,
                    };
                    self.palette.apply_to_cell(&mut cell);
                    tools::brush::paint_stamp(
                        &mut self.canvas.buffer,
                        bx,
                        by,
                        self.brush.shape,
                        self.brush.size,
                        cell,
                    );
                }
                self.prev_mouse_buf = Some((bx, by));
            }
            MouseEventKind::Drag(_) => {
                let Some((bx, by)) = self.screen_to_buffer(mouse.column, mouse.row) else {
                    return;
                };
                self.canvas.set_cursor(bx.max(0) as u16, by.max(0) as u16);
                self.unsaved = true;
                if let Some((px, py)) = self.prev_mouse_buf {
                    if self.toolbox.selected == Tool::Eraser {
                        tools::eraser::erase_line(
                            &mut self.canvas.buffer,
                            px,
                            py,
                            bx,
                            by,
                            self.brush.shape,
                            self.brush.size,
                        );
                    } else {
                        let mut cell = canvas::CanvasCell {
                            ch: '\u{2588}',
                            fg: None,
                            bg: None,
                        };
                        self.palette.apply_to_cell(&mut cell);
                        tools::brush::paint_line(
                            &mut self.canvas.buffer,
                            px,
                            py,
                            bx,
                            by,
                            self.brush.shape,
                            self.brush.size,
                            cell,
                        );
                    }
                }
                self.prev_mouse_buf = Some((bx, by));
            }
            MouseEventKind::Up(_) => {
                self.prev_mouse_buf = None;
            }
            _ => {}
        }
    }

    pub fn handle_event(&mut self) -> io::Result<()> {
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    self.handle_key_event(key.code);
                }
                Event::Mouse(mouse) => {
                    self.handle_mouse_event(mouse);
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn handle_key_event(&mut self, code: KeyCode) {
        if self.settings.settings_open {
            if self.settings.handle_key(code) {
                self.apply_settings();
                return;
            }
            if let KeyCode::Char('S') = code {
                self.settings.settings_open = false;
            }
            return;
        }

        if self
            .canvas
            .handle_key(code, self.last_canvas_size.0, self.last_canvas_size.1)
        {
            return;
        }
        if self.toolbox.handle_key(code) {
            return;
        }
        match code {
            KeyCode::Char('[') => {
                self.brush.size_down();
                return;
            }
            KeyCode::Char(']') => {
                self.brush.size_up();
                return;
            }
            KeyCode::Char('\'') => {
                self.brush.cycle_shape();
                return;
            }
            _ => {}
        }
        if self.palette.handle_key(code) {
            return;
        }
        // Keyboard painting: Space/Enter paints or erases at cursor
        if matches!(self.toolbox.selected, Tool::Brush | Tool::Eraser)
            && matches!(code, KeyCode::Char(' ') | KeyCode::Enter)
        {
            let (cx, cy) = self.canvas.cursor();
            if self.toolbox.selected == Tool::Eraser {
                tools::eraser::erase_stamp(
                    &mut self.canvas.buffer,
                    cx as i16,
                    cy as i16,
                    self.brush.shape,
                    self.brush.size,
                );
            } else {
                let mut cell = canvas::CanvasCell {
                    ch: '\u{2588}',
                    fg: None,
                    bg: None,
                };
                self.palette.apply_to_cell(&mut cell);
                tools::brush::paint_stamp(
                    &mut self.canvas.buffer,
                    cx as i16,
                    cy as i16,
                    self.brush.shape,
                    self.brush.size,
                    cell,
                );
            }
            self.unsaved = true;
            return;
        }

        match code {
            KeyCode::Tab => {
                self.mode = self.mode.next();
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Char('S') => {
                self.settings.canvas_width = self.canvas.buffer.width() as u16;
                self.settings.canvas_height = self.canvas.buffer.height() as u16;
                self.settings.show_grid = self.canvas.show_grid();
                self.settings.settings_open = true;
            }
            _ => {}
        }
    }

    fn apply_settings(&mut self) {
        let w = self.settings.canvas_width as usize;
        let h = self.settings.canvas_height as usize;
        if self.canvas.buffer.width() != w || self.canvas.buffer.height() != h {
            self.canvas = canvas::CanvasWidget::new(w as u16, h as u16);
        }
        if self.settings.show_grid != self.canvas.show_grid() {
            self.canvas.toggle_grid();
        }
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}
