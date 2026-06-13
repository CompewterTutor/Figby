use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Tabs};
use ratatui::Frame;
use std::collections::BTreeMap;
use std::io;
use std::time::Duration;

use crate::font::load_font;
use crate::render::Justification;

pub mod brush;
pub mod canvas;
pub mod font_editor;
pub mod image_editor;
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
    pub font_editor: font_editor::FontEditor,
    pub image_editor: image_editor::ImageEditor,
    pub brush: brush::BrushState,
    pub text_tool: tools::text::TextToolState,
    pub unsaved: bool,
    pub settings: status::CanvasSettings,
    last_canvas_size: (u16, u16),
    canvas_inner_rect: Rect,
    toolbox_area: Rect,
    palette_area: Rect,
    prev_mouse_buf: Option<(i16, i16)>,
    line_start: Option<(i16, i16)>,
    saved_buffer: Option<canvas::CanvasBuffer>,
    selection: Option<tools::selection::Selection>,
    clipboard: Option<tools::selection::Clipboard>,
    selection_drag_origin: Option<(i16, i16)>,
    selection_polygon_points: Vec<(i16, i16)>,
    selection_lasso_points: Vec<(i16, i16)>,
}

impl TuiApp {
    pub fn new() -> Self {
        let icons = serde_yaml::from_str(ICONS_YAML).unwrap_or_default();

        let mut fe = font_editor::FontEditor::new();
        if let Ok(font) = load_font("standard", "fonts") {
            fe.load_font(font);
        }

        Self {
            mode: AppMode::FontEditor,
            should_quit: false,
            _icons: icons,
            toolbox: toolbox::Toolbox::new(),
            canvas: canvas::CanvasWidget::default(),
            palette: palette::Palette::new(),
            brush: brush::BrushState::new(),
            text_tool: tools::text::TextToolState::new("fonts"),
            font_editor: fe,
            image_editor: image_editor::ImageEditor::new(),
            last_canvas_size: (0, 0),
            canvas_inner_rect: Rect::new(0, 0, 0, 0),
            toolbox_area: Rect::new(0, 0, 0, 0),
            palette_area: Rect::new(0, 0, 0, 0),
            prev_mouse_buf: None,
            unsaved: false,
            settings: status::CanvasSettings::new(),
            line_start: None,
            saved_buffer: None,
            selection: None,
            clipboard: None,
            selection_drag_origin: None,
            selection_polygon_points: Vec::new(),
            selection_lasso_points: Vec::new(),
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
        self.toolbox_area = tool_brush_chunks[0];
        self.palette_area = main_chunks[2];
        self.toolbox.render(frame, tool_brush_chunks[0]);
        if self.toolbox.selected == Tool::Text {
            self.text_tool.render_options(frame, tool_brush_chunks[1]);
        } else {
            self.brush.render(frame, tool_brush_chunks[1]);
        }

        let mode_title = match self.mode {
            AppMode::ImageEditor => {
                if self.image_editor.entering_path() {
                    format!(" Image Editor [Path: {}] ", self.image_editor.path_buffer())
                } else if let Some(err) = self.image_editor.error_message() {
                    format!(" Image Editor [Error: {err}] ")
                } else {
                    self.mode.title().to_string()
                }
            }
            _ => self.mode.title().to_string(),
        };
        let block = Block::default().title(mode_title).borders(Borders::ALL);
        let inner = block.inner(main_chunks[1]);

        let is_font_ui_mode = self.mode == AppMode::FontEditor
            && !matches!(
                self.font_editor.view,
                font_editor::FontEditorView::CharEditor(_)
            );

        if is_font_ui_mode {
            frame.render_widget(block, main_chunks[1]);
            self.font_editor.render(frame, inner);
        } else {
            if self.mode == AppMode::FontEditor {
                self.sync_canvas_to_font_char();
            }
            if self.mode == AppMode::ImageEditor {
                self.sync_image_to_canvas();
            }
            let zoom = self.canvas.zoom_level().max(1) as u16;
            let buf_w = self.canvas.buffer.width() as u16;
            let buf_h = self.canvas.buffer.height() as u16;
            let grid_w = (buf_w * zoom).min(inner.width);
            let grid_h = (buf_h * zoom).min(inner.height);
            let centered = Rect {
                x: inner.x + (inner.width.saturating_sub(grid_w) / 2),
                y: inner.y + (inner.height.saturating_sub(grid_h) / 2),
                width: grid_w,
                height: grid_h,
            };
            self.last_canvas_size = (buf_w, buf_h);
            self.canvas_inner_rect = centered;
            self.canvas
                .ensure_cursor_visible(centered.width, centered.height);
            if let Some(ref sel) = self.selection {
                if sel.is_active() {
                    self.canvas.selection_perimeter = Some(sel.perimeter());
                } else {
                    self.canvas.selection_perimeter = None;
                }
            } else {
                self.canvas.selection_perimeter = None;
            }
            self.canvas.polygon_vertices = self.selection_polygon_points.clone();
            frame.render_widget(block, main_chunks[1]);
            if centered.width > 1 && centered.height > 1 {
                let edge = Block::default().borders(Borders::ALL).style(
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::DIM),
                );
                frame.render_widget(edge, centered);
            }
            frame.render_widget(&self.canvas, centered);
        }

        if self.settings.settings_open {
            self.settings.render(frame, main_chunks[2]);
        } else {
            self.palette.render(frame, main_chunks[2]);
        }

        let mode_name = match self.mode {
            AppMode::ImageEditor => {
                let mode_str = match self.image_editor.mode() {
                    image_editor::AsciiMode::Color => "Color",
                    image_editor::AsciiMode::Grayscale => "Grayscale",
                };
                format!("Image Editor [{mode_str}]")
            }
            AppMode::AsciiPreview => "ASCII Preview".to_string(),
            AppMode::FontEditor => {
                if let font_editor::FontEditorView::CharEditor(code) = self.font_editor.view {
                    format!("Font Editor [U+{code:04X}]")
                } else if self.font_editor.view == font_editor::FontEditorView::HeaderEditor {
                    "Font Editor - Header".to_string()
                } else if self.font_editor.view == font_editor::FontEditorView::SmushRuleEditor {
                    "Font Editor - Smushing Rules".to_string()
                } else if self.font_editor.view == font_editor::FontEditorView::TransformEditor {
                    "Font Editor - Transforms".to_string()
                } else {
                    "Font Editor".to_string()
                }
            }
        };
        status::StatusBar::render(
            frame,
            chunks[2],
            self.canvas.cursor(),
            self.canvas.zoom_level(),
            self.toolbox.selected.full_name(),
            &mode_name,
            self.unsaved,
            &self._icons,
        );
    }

    fn sync_canvas_to_font_char(&mut self) {
        if let font_editor::FontEditorView::CharEditor(code) = self.font_editor.view {
            self.font_editor.sync_from_canvas(code, &self.canvas.buffer);
        }
    }

    fn sync_font_char_to_canvas(&mut self) {
        if let Some((_, ch)) = self.font_editor.selected_char() {
            let w = ch.width().max(1);
            let h = ch.rows().len().max(1);
            if self.canvas.buffer.width() != w || self.canvas.buffer.height() != h {
                self.canvas = canvas::CanvasWidget::new(w as u16, h as u16);
            }
            for y in 0..h {
                let row = &ch.rows()[y];
                for (x, c) in row.chars().enumerate() {
                    if x < w {
                        self.canvas.buffer.set(
                            x,
                            y,
                            canvas::CanvasCell {
                                ch: c,
                                fg: None,
                                bg: None,
                            },
                        );
                    }
                }
            }
        }
    }

    fn sync_image_to_canvas(&mut self) {
        if let Some(cells) = self.image_editor.cells() {
            let h = cells.len();
            let w = cells[0].len();
            if self.canvas.buffer.width() != w || self.canvas.buffer.height() != h {
                self.canvas = canvas::CanvasWidget::new(w as u16, h as u16);
            }
            for (y, row) in cells.iter().enumerate() {
                for (x, cell) in row.iter().enumerate() {
                    self.canvas.buffer.set(x, y, *cell);
                }
            }
        }
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

        // Toolbox click: select tool by row
        let tool_count = Tool::all().len() as u16;
        let toolbox_inner_y = self.toolbox_area.y + 1;
        if mouse.kind == MouseEventKind::Down(MouseButton::Left)
            && mouse.column >= self.toolbox_area.x
            && mouse.column < self.toolbox_area.x + self.toolbox_area.width
            && mouse.row >= toolbox_inner_y
            && mouse.row < toolbox_inner_y + tool_count
        {
            let idx = (mouse.row - toolbox_inner_y) as usize;
            let tools = Tool::all();
            if idx < tools.len() {
                self.toolbox.selected = tools[idx];
                self.selection_polygon_points.clear();
            }
            return;
        }

        // Text tool: click to enter text mode at cursor position
        if self.toolbox.selected == Tool::Text {
            if let MouseEventKind::Down(_) = mouse.kind {
                if let Some((bx, by)) = self.screen_to_buffer(mouse.column, mouse.row) {
                    self.text_tool.cursor_position = (bx, by);
                    self.text_tool.entering_text = true;
                    self.text_tool.text_buffer.clear();
                    self.canvas.set_cursor(bx.max(0) as u16, by.max(0) as u16);
                }
            }
            self.prev_mouse_buf = None;
            self.line_start = None;
            self.saved_buffer = None;
            return;
        }

        let is_selection_tool = matches!(
            self.toolbox.selected,
            Tool::Marquee | Tool::Lasso | Tool::CircleSelect | Tool::PolygonSelect
        );

        if !is_selection_tool
            && !matches!(
                self.toolbox.selected,
                Tool::Brush
                    | Tool::Eraser
                    | Tool::Line
                    | Tool::Fill
                    | Tool::Eyedropper
                    | Tool::Spray
            )
        {
            self.prev_mouse_buf = None;
            self.line_start = None;
            self.saved_buffer = None;
            return;
        }

        match mouse.kind {
            MouseEventKind::Down(_) => {
                let Some((bx, by)) = self.screen_to_buffer(mouse.column, mouse.row) else {
                    self.prev_mouse_buf = None;
                    self.line_start = None;
                    return;
                };
                self.canvas.set_cursor(bx.max(0) as u16, by.max(0) as u16);
                self.unsaved = true;

                if is_selection_tool {
                    self.handle_selection_down(bx, by);
                    return;
                }

                if self.toolbox.selected == Tool::Fill {
                    let mut cell = canvas::CanvasCell {
                        ch: self.brush.ch,
                        fg: None,
                        bg: None,
                    };
                    self.palette.apply_to_cell(&mut cell);
                    tools::fill::flood_fill(&mut self.canvas.buffer, bx, by, cell);
                    return;
                }
                if self.toolbox.selected == Tool::Line {
                    self.line_start = Some((bx, by));
                    self.saved_buffer = Some(self.canvas.buffer.clone());
                    return;
                }
                if self.toolbox.selected == Tool::Eraser {
                    tools::eraser::erase_stamp(
                        &mut self.canvas.buffer,
                        bx,
                        by,
                        self.brush.shape,
                        self.brush.size,
                    );
                } else if self.toolbox.selected == Tool::Eyedropper {
                    if let Some(cell) = tools::eyedropper::sample(&self.canvas.buffer, bx, by) {
                        self.brush.ch = cell.ch;
                        if let Some(fg) = cell.fg {
                            self.palette.selected_color = Some(fg);
                            self.palette.push_recent(fg);
                            self.palette.target = palette::ColorTarget::Foreground;
                        }
                    }
                } else if self.toolbox.selected == Tool::Spray {
                    let mut cell = canvas::CanvasCell {
                        ch: self.brush.ch,
                        fg: None,
                        bg: None,
                    };
                    self.palette.apply_to_cell(&mut cell);
                    let mut rng = StdRng::seed_from_u64(rand::thread_rng().gen());
                    tools::spray::spray_stamp(
                        &mut self.canvas.buffer,
                        bx,
                        by,
                        self.brush.size,
                        self.brush.density,
                        cell,
                        &mut rng,
                    );
                } else {
                    let mut cell = canvas::CanvasCell {
                        ch: self.brush.ch,
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

                if is_selection_tool {
                    self.handle_selection_drag(bx, by);
                    return;
                }

                if self.toolbox.selected == Tool::Line {
                    if let (Some((sx, sy)), Some(saved)) = (self.line_start, &self.saved_buffer) {
                        self.canvas.buffer = saved.clone();
                        let mut cell = canvas::CanvasCell {
                            ch: self.brush.ch,
                            fg: None,
                            bg: None,
                        };
                        self.palette.apply_to_cell(&mut cell);
                        tools::line::draw_line_segment(
                            &mut self.canvas.buffer,
                            sx,
                            sy,
                            bx,
                            by,
                            self.brush.shape,
                            self.brush.size,
                            cell,
                        );
                    }
                    return;
                }
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
                    } else if self.toolbox.selected == Tool::Spray {
                        let mut cell = canvas::CanvasCell {
                            ch: self.brush.ch,
                            fg: None,
                            bg: None,
                        };
                        self.palette.apply_to_cell(&mut cell);
                        let mut rng = StdRng::seed_from_u64(rand::thread_rng().gen());
                        tools::spray::spray_line(
                            &mut self.canvas.buffer,
                            px,
                            py,
                            bx,
                            by,
                            self.brush.size,
                            self.brush.density,
                            cell,
                            &mut rng,
                        );
                    } else {
                        let mut cell = canvas::CanvasCell {
                            ch: self.brush.ch,
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
                if is_selection_tool {
                    self.handle_selection_up();
                }
                self.prev_mouse_buf = None;
                self.line_start = None;
                self.saved_buffer = None;
            }
            MouseEventKind::Moved => {
                if let Some((bx, by)) = self.screen_to_buffer(mouse.column, mouse.row) {
                    self.canvas.set_cursor(bx.max(0) as u16, by.max(0) as u16);
                }
            }
            _ => {}
        }
    }

    fn handle_selection_down(&mut self, bx: i16, by: i16) {
        match self.toolbox.selected {
            Tool::Marquee => {
                self.selection = None;
                self.selection_drag_origin = Some((bx, by));
            }
            Tool::CircleSelect => {
                self.selection = None;
                self.selection_drag_origin = Some((bx, by));
            }
            Tool::Lasso => {
                self.selection = None;
                self.selection_lasso_points = vec![(bx, by)];
            }
            Tool::PolygonSelect => {
                let points = &mut self.selection_polygon_points;
                // If click is near first point, close polygon
                if points.len() >= 3 {
                    let (fx, fy) = points[0];
                    let dist = ((bx - fx).abs() + (by - fy).abs()) as f64;
                    if dist < 3.0 {
                        self.selection = Some(tools::selection::Selection::polygon(
                            &self.canvas.buffer,
                            points,
                        ));
                        points.clear();
                        return;
                    }
                }
                points.push((bx, by));
            }
            _ => {}
        }
    }

    fn handle_selection_drag(&mut self, bx: i16, by: i16) {
        match self.toolbox.selected {
            Tool::Marquee => {
                if let Some((ox, oy)) = self.selection_drag_origin {
                    self.selection = Some(tools::selection::Selection::marquee(
                        &self.canvas.buffer,
                        ox,
                        oy,
                        bx,
                        by,
                    ));
                }
            }
            Tool::CircleSelect => {
                if let Some((ox, oy)) = self.selection_drag_origin {
                    let dx = bx - ox;
                    let dy = by - oy;
                    let r = ((dx * dx + dy * dy) as f64).sqrt().round() as i16;
                    self.selection = Some(tools::selection::Selection::circle(
                        &self.canvas.buffer,
                        ox,
                        oy,
                        r,
                    ));
                }
            }
            Tool::Lasso => {
                self.selection_lasso_points.push((bx, by));
            }
            _ => {}
        }
    }

    fn handle_selection_up(&mut self) {
        match self.toolbox.selected {
            Tool::Marquee | Tool::CircleSelect => {
                // Selection already finalized during drag
                self.selection_drag_origin = None;
            }
            Tool::Lasso => {
                let points = std::mem::take(&mut self.selection_lasso_points);
                if points.len() >= 3 {
                    self.selection = Some(tools::selection::Selection::lasso(
                        &self.canvas.buffer,
                        &points,
                    ));
                }
            }
            Tool::PolygonSelect => {
                // Polygon is closed on Enter or click-close, not on mouse up
            }
            _ => {}
        }
    }

    pub fn handle_event(&mut self) -> io::Result<()> {
        if event::poll(Duration::from_millis(100))? {
            loop {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        self.handle_key_event(key);
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse_event(mouse);
                    }
                    _ => {}
                }
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn handle_key_event(&mut self, key: impl Into<KeyEvent>) {
        let key = key.into();
        let code = key.code;
        let modifiers = key.modifiers;

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

        // Font Editor mode: dispatch to font_editor before canvas/tools
        if self.mode == AppMode::FontEditor {
            let area_width = self.canvas_inner_rect.width;
            if self.font_editor.handle_key(code, modifiers, area_width) {
                if self.font_editor.view != font_editor::FontEditorView::Overview {
                    self.sync_font_char_to_canvas();
                }
                return;
            }
        }

        // Image Editor mode: dispatch to image_editor before canvas/tools
        if self.mode == AppMode::ImageEditor && self.image_editor.handle_key(code) {
            self.sync_image_to_canvas();
            return;
        }

        // Text tool: text entry mode (before canvas, captures all keys)
        if self.toolbox.selected == Tool::Text && self.text_tool.entering_text {
            match code {
                KeyCode::Enter => {
                    self.text_tool
                        .render_text_to_buffer(&mut self.canvas.buffer);
                    self.text_tool.text_buffer.clear();
                    self.text_tool.entering_text = false;
                    self.unsaved = true;
                    return;
                }
                KeyCode::Esc => {
                    self.text_tool.text_buffer.clear();
                    self.text_tool.entering_text = false;
                    return;
                }
                KeyCode::Backspace => {
                    self.text_tool.text_buffer.pop();
                    return;
                }
                KeyCode::Char(c) => {
                    self.text_tool.text_buffer.push(c);
                    return;
                }
                _ => {}
            }
        }

        // Text tool: font navigation (before canvas so up/down don't move cursor)
        if self.toolbox.selected == Tool::Text && !self.text_tool.entering_text {
            match code {
                KeyCode::Up => {
                    if !self.text_tool.available_fonts.is_empty() {
                        self.text_tool.font_index = self.text_tool.font_index.saturating_sub(1);
                        self.text_tool.load_selected_font();
                    }
                    return;
                }
                KeyCode::Down => {
                    if !self.text_tool.available_fonts.is_empty() {
                        self.text_tool.font_index = (self.text_tool.font_index + 1)
                            .min(self.text_tool.available_fonts.len() - 1);
                        self.text_tool.load_selected_font();
                    }
                    return;
                }
                _ => {}
            }
        }

        // Selection operations (before canvas cursor movement)
        let selection_active = self.selection.as_ref().is_some_and(|s| s.is_active());

        if selection_active {
            match code {
                // Arrow keys: move selection
                KeyCode::Up => {
                    self.move_selection(0, -1);
                    self.unsaved = true;
                    return;
                }
                KeyCode::Down => {
                    self.move_selection(0, 1);
                    self.unsaved = true;
                    return;
                }
                KeyCode::Left => {
                    self.move_selection(-1, 0);
                    self.unsaved = true;
                    return;
                }
                KeyCode::Right => {
                    self.move_selection(1, 0);
                    self.unsaved = true;
                    return;
                }
                KeyCode::Delete | KeyCode::Backspace => {
                    if let Some(sel) = self.selection.take() {
                        sel.delete_from(&mut self.canvas.buffer);
                        self.unsaved = true;
                    }
                    return;
                }
                _ => {}
            }

            // Ctrl+C/X/V
            if modifiers.contains(KeyModifiers::CONTROL) {
                match code {
                    KeyCode::Char('c') => {
                        if let Some(ref sel) = self.selection {
                            self.clipboard = Some(sel.copy_from(&self.canvas.buffer));
                        }
                        return;
                    }
                    KeyCode::Char('x') => {
                        if let Some(sel) = self.selection.take() {
                            self.clipboard = Some(sel.cut_from(&mut self.canvas.buffer));
                            self.unsaved = true;
                        }
                        return;
                    }
                    KeyCode::Char('v') => {
                        if let Some(ref clip) = self.clipboard {
                            let (cx, cy) = self.canvas.cursor();
                            tools::selection::Selection::paste_into(
                                &mut self.canvas.buffer,
                                clip,
                                cx as i16,
                                cy as i16,
                            );
                            self.unsaved = true;
                        }
                        return;
                    }
                    _ => {}
                }
            }
        }

        // Polygon select tool: Enter closes polygon, Esc cancels
        if self.toolbox.selected == Tool::PolygonSelect && !self.selection_polygon_points.is_empty()
        {
            match code {
                KeyCode::Enter => {
                    let points = std::mem::take(&mut self.selection_polygon_points);
                    if points.len() >= 3 {
                        self.selection = Some(tools::selection::Selection::polygon(
                            &self.canvas.buffer,
                            &points,
                        ));
                    }
                    return;
                }
                KeyCode::Esc => {
                    self.selection_polygon_points.clear();
                    return;
                }
                _ => {}
            }
        }

        // Deselect on Esc (only when a selection exists)
        if self.selection.is_some() && code == KeyCode::Esc {
            self.selection = None;
            return;
        }

        if self
            .canvas
            .handle_key(code, self.last_canvas_size.0, self.last_canvas_size.1)
        {
            return;
        }
        // Text tool settings (not entering text)
        if self.toolbox.selected == Tool::Text && !self.text_tool.entering_text {
            match code {
                KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.text_tool.justification = match self.text_tool.justification {
                        Justification::Left => Justification::Center,
                        Justification::Center => Justification::Right,
                        Justification::Right => Justification::Left,
                    };
                    return;
                }
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    if self.text_tool.scale < 4 {
                        self.text_tool.scale += 1;
                    }
                    return;
                }
                KeyCode::Char('-') | KeyCode::Char('_') => {
                    if self.text_tool.scale > 1 {
                        self.text_tool.scale -= 1;
                    }
                    return;
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    let (cx, cy) = self.canvas.cursor();
                    self.text_tool.cursor_position = (cx as i16, cy as i16);
                    self.text_tool.entering_text = true;
                    self.text_tool.text_buffer.clear();
                    return;
                }
                _ => {}
            }
        }
        // Settings toggle must be before toolbox to avoid 's'/'S' conflict with Spray tool
        if code == KeyCode::Char('S') && !modifiers.contains(KeyModifiers::CONTROL) {
            self.settings.canvas_width = self.canvas.buffer.width() as u16;
            self.settings.canvas_height = self.canvas.buffer.height() as u16;
            self.settings.show_grid = self.canvas.show_grid();
            self.settings.settings_open = true;
            return;
        }
        if self.toolbox.handle_key(code) {
            // Clear polygon points when switching away from PolygonSelect
            if self.toolbox.selected != Tool::PolygonSelect {
                self.selection_polygon_points.clear();
            }
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
            KeyCode::Char(';') => {
                self.brush.density_down();
                return;
            }
            KeyCode::Char('\'') => {
                self.brush.density_up();
                return;
            }
            KeyCode::Char('\\') => {
                self.brush.cycle_shape();
                return;
            }
            _ => {}
        }
        if self.palette.handle_key(code) {
            return;
        }
        // Keyboard painting: Space/Enter paints or erases at cursor
        if matches!(
            self.toolbox.selected,
            Tool::Brush | Tool::Eraser | Tool::Line | Tool::Fill | Tool::Spray
        ) && matches!(code, KeyCode::Char(' ') | KeyCode::Enter)
        {
            let (cx, cy) = self.canvas.cursor();
            if self.toolbox.selected == Tool::Fill {
                let mut cell = canvas::CanvasCell {
                    ch: self.brush.ch,
                    fg: None,
                    bg: None,
                };
                self.palette.apply_to_cell(&mut cell);
                tools::fill::flood_fill(&mut self.canvas.buffer, cx as i16, cy as i16, cell);
            } else if self.toolbox.selected == Tool::Eraser {
                tools::eraser::erase_stamp(
                    &mut self.canvas.buffer,
                    cx as i16,
                    cy as i16,
                    self.brush.shape,
                    self.brush.size,
                );
            } else if self.toolbox.selected == Tool::Spray {
                let mut cell = canvas::CanvasCell {
                    ch: self.brush.ch,
                    fg: None,
                    bg: None,
                };
                self.palette.apply_to_cell(&mut cell);
                let mut rng = StdRng::seed_from_u64(rand::thread_rng().gen());
                tools::spray::spray_stamp(
                    &mut self.canvas.buffer,
                    cx as i16,
                    cy as i16,
                    self.brush.size,
                    self.brush.density,
                    cell,
                    &mut rng,
                );
            } else {
                let mut cell = canvas::CanvasCell {
                    ch: self.brush.ch,
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
            KeyCode::Char('q') if !modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Esc => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    fn move_selection(&mut self, dx: i16, dy: i16) {
        if let Some(ref mut sel) = self.selection {
            if sel.is_active() {
                sel.move_selection(&mut self.canvas.buffer, dx, dy);
            }
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
