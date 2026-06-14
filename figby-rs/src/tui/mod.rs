use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
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
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::config;

pub mod action;
pub mod brush;
pub mod canvas;
pub mod component;
pub mod components;
pub mod export;
pub mod file_ops;
pub mod font_editor;
pub mod image_editor;
pub mod menu;
pub mod palette;
pub mod status;
pub mod throbber;
pub mod toolbox;
pub mod tools;
pub mod undo;
pub mod undo_panel;

pub use action::Action;
pub use brush::BrushState;
pub use component::Component;
pub use export::ExportMode;
pub use menu::MenuBar;
pub use palette::Palette;
pub use status::CanvasSettings;
pub use throbber::ThrobberState;
pub use toolbox::Tool;

pub use components::canvas::CanvasComponent;
pub use components::export::ExportComponent;
pub use components::file_ops::FileOpsComponent;
pub use components::font_editor::FontEditorComponent;
pub use components::image_editor::ImageEditorComponent;
pub use components::palette::PaletteComponent;
pub use components::status_bar::StatusBarComponent;
pub use components::toolbox::ToolboxComponent;
pub use components::undo_panel::UndoPanelComponent;

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
    pub menu_bar: MenuBar,
    pub toolbox_comp: ToolboxComponent,
    pub canvas_comp: CanvasComponent,
    pub palette_comp: PaletteComponent,
    pub font_editor_comp: FontEditorComponent,
    pub image_editor_comp: ImageEditorComponent,
    pub text_tool: tools::text::TextToolState,
    pub unsaved: bool,
    pub settings: status::CanvasSettings,
    pub file_ops_comp: FileOpsComponent,
    pub export_comp: ExportComponent,
    pub undo_panel_comp: UndoPanelComponent,
    pub undo: undo::UndoSystem,
    pub status_bar_comp: StatusBarComponent,
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
    auto_save_interval: u64,
    last_save_time: Instant,
    pub throbber: ThrobberState,
    async_rx: Option<mpsc::Receiver<AsyncResult>>,
    last_frame_time: Instant,
    fps: f64,
    git_branch: Option<String>,
}

impl TuiApp {
    pub fn new() -> Self {
        let icons: BTreeMap<String, String> = serde_yaml::from_str(ICONS_YAML).unwrap_or_default();
        let config = config::load_config();

        let mut toolbox_comp = ToolboxComponent::new();
        if let Some(ref shape) = config.tui.brush.shape {
            toolbox_comp.brush.shape = match shape.as_str() {
                "square" => brush::BrushShape::Square,
                "circle" => brush::BrushShape::Circle,
                "spray" => brush::BrushShape::SprayPaint,
                "custom" => brush::BrushShape::Custom,
                _ => toolbox_comp.brush.shape,
            };
        }
        if let Some(size) = config.tui.brush.size {
            toolbox_comp.brush.set_size(size);
        }
        if let Some(density) = config.tui.brush.density {
            toolbox_comp.brush.set_density(density);
        }
        if let Some(ref ch_str) = config.tui.brush.ch {
            if let Some(ch) = ch_str.chars().next() {
                toolbox_comp.brush.ch = ch;
            }
        }

        let mut font_editor_comp = FontEditorComponent::new();
        if let Ok(font) = crate::font::load_font("standard", "fonts") {
            font_editor_comp.editor.load_font(font);
        }

        let mut file_ops_comp = FileOpsComponent::new();
        if let Some(max) = config.tui.recent_files_max {
            file_ops_comp.recent_files.set_max(max);
        }

        let status_bar_comp = StatusBarComponent::new(icons.clone());

        let git_branch = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout)
                        .ok()
                        .map(|s| s.trim().to_string())
                } else {
                    None
                }
            });

        Self {
            mode: AppMode::FontEditor,
            should_quit: false,
            _icons: icons,
            menu_bar: MenuBar::new(),
            toolbox_comp,
            canvas_comp: CanvasComponent::new(),
            palette_comp: PaletteComponent::new(),
            text_tool: tools::text::TextToolState::new("fonts"),
            font_editor_comp,
            image_editor_comp: ImageEditorComponent::new(),
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
            file_ops_comp,
            export_comp: ExportComponent::new(),
            auto_save_interval: 0,
            last_save_time: Instant::now(),
            undo: undo::UndoSystem::new(config.tui.undo_limit.unwrap_or(50)),
            undo_panel_comp: UndoPanelComponent::new(),
            status_bar_comp,
            throbber: ThrobberState::new(),
            async_rx: None,
            last_frame_time: Instant::now(),
            fps: 0.0,
            git_branch,
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        let mut terminal = ratatui::init();
        execute!(io::stdout(), EnableBracketedPaste)?;

        while !self.should_quit {
            terminal.draw(|f| self.render(f))?;
            self.handle_event()?;
        }

        execute!(terminal.backend_mut(), DisableBracketedPaste)?;
        ratatui::restore();
        Ok(())
    }

    fn process_action(&mut self, action: &Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::ToolSelected if self.toolbox_comp.toolbox.selected != Tool::PolygonSelect => {
                self.selection_polygon_points.clear();
            }
            Action::ColorChanged(color, target) => {
                self.palette_comp.palette.selected_color = Some(*color);
                match target {
                    palette::ColorTarget::Foreground => {
                        self.palette_comp.palette.target = palette::ColorTarget::Foreground;
                    }
                    palette::ColorTarget::Background => {
                        self.palette_comp.palette.target = palette::ColorTarget::Background;
                    }
                }
            }
            Action::ModeChanged => {}
            Action::SaveAsRequested => self.perform_save(),
            Action::OpenRequested => self.perform_open(),
            Action::ExportRequested(_) => self.perform_export(),
            Action::Menu(action) => self.handle_menu_action(action.clone()),
            _ => {}
        }
    }

    pub fn render(&mut self, frame: &mut Frame<'_>) {
        self.check_async_completion();
        self.throbber.tick();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(frame.area());

        self.menu_bar.draw(frame, chunks[0]);

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
        frame.render_widget(tabs, chunks[1]);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(8),
                Constraint::Min(10),
                Constraint::Length(20),
            ])
            .split(chunks[2]);

        let tool_brush_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(10), Constraint::Length(9)])
            .split(main_chunks[0]);
        self.toolbox_area = tool_brush_chunks[0];
        self.palette_area = main_chunks[2];

        // Toolbox + brush
        let _ = self.toolbox_comp.draw(frame, main_chunks[0]);

        if self.toolbox_comp.toolbox.selected == Tool::Text {
            self.text_tool.render_options(frame, tool_brush_chunks[1]);
        } else {
            self.toolbox_comp.brush.render(frame, tool_brush_chunks[1]);
        }

        let mode_title = match self.mode {
            AppMode::ImageEditor => {
                if self.image_editor_comp.editor.entering_path() {
                    format!(
                        " Image Editor [Path: {}] ",
                        self.image_editor_comp.editor.path_buffer()
                    )
                } else if let Some(err) = self.image_editor_comp.editor.error_message() {
                    format!(" Image Editor [Error: {err}] ")
                } else if self.image_editor_comp.editor.has_cells() {
                    format!(
                        " Image Editor {} ",
                        self.image_editor_comp.editor.adjustment_status()
                    )
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
                self.font_editor_comp.editor.view,
                font_editor::FontEditorView::CharEditor(_)
            );

        if is_font_ui_mode {
            frame.render_widget(block, main_chunks[1]);
            let _ = self.font_editor_comp.draw(frame, inner);
        } else {
            if self.mode == AppMode::FontEditor {
                self.sync_canvas_to_font_char();
            }
            if self.mode == AppMode::ImageEditor {
                self.sync_image_to_canvas();
            }

            // Selection perimeter
            if let Some(ref sel) = self.selection {
                if sel.is_active() {
                    self.canvas_comp.canvas.selection_perimeter = Some(sel.perimeter());
                } else {
                    self.canvas_comp.canvas.selection_perimeter = None;
                }
            } else {
                self.canvas_comp.canvas.selection_perimeter = None;
            }
            self.canvas_comp
                .canvas
                .polygon_vertices
                .clone_from(&self.selection_polygon_points);

            // Text overlays
            if self.toolbox_comp.toolbox.selected == Tool::Text {
                self.canvas_comp.canvas.text_overlays = self
                    .text_tool
                    .blocks
                    .iter()
                    .enumerate()
                    .filter_map(|(i, _)| self.text_tool.render_block_to_overlay(i))
                    .collect();
                self.canvas_comp.canvas.text_block_perimeter =
                    self.text_tool.selected_block.and_then(|idx| {
                        if idx < self.text_tool.blocks.len() {
                            let (bx, by, bw, bh) = self.text_tool.compute_bounding_box(idx);
                            if bw == 0 || bh == 0 {
                                return None;
                            }
                            let mut perim = Vec::new();
                            for x in bx..bx + bw as i16 {
                                perim.push((x.max(0) as usize, by.max(0) as usize));
                                perim.push((
                                    x.max(0) as usize,
                                    (by + bh as i16 - 1).max(0) as usize,
                                ));
                            }
                            for y in (by + 1)..(by + bh as i16 - 1) {
                                if y < 0 {
                                    continue;
                                }
                                perim.push((bx.max(0) as usize, y as usize));
                                perim.push(((bx + bw as i16 - 1).max(0) as usize, y as usize));
                            }
                            Some(perim)
                        } else {
                            None
                        }
                    });
            } else {
                self.canvas_comp.canvas.text_overlays.clear();
                self.canvas_comp.canvas.text_block_perimeter = None;
            }

            let block_inner = block.inner(main_chunks[1]);
            frame.render_widget(block, main_chunks[1]);

            // Render canvas
            let inner_area = block_inner;
            self.canvas_comp.canvas_inner_rect = self.compute_canvas_rect(inner_area);
            if self.canvas_comp.canvas_inner_rect.width > 1
                && self.canvas_comp.canvas_inner_rect.height > 1
            {
                let edge = Block::default().borders(Borders::ALL).style(
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::DIM),
                );
                frame.render_widget(edge, self.canvas_comp.canvas_inner_rect);
            }
            frame.render_widget(&self.canvas_comp.canvas, self.canvas_comp.canvas_inner_rect);
        }

        // Palette or Settings
        if self.settings.settings_open {
            self.settings.render(frame, main_chunks[2]);
        } else {
            let _ = self.palette_comp.draw(frame, main_chunks[2]);
        }

        // FPS tracking
        let now = Instant::now();
        let elapsed = now - self.last_frame_time;
        self.last_frame_time = now;
        let instant_fps = if elapsed.as_secs_f64() > 0.0 {
            1.0 / elapsed.as_secs_f64()
        } else {
            0.0
        };
        self.fps = self.fps * 0.9 + instant_fps * 0.1;

        // Status bar
        let mode_name = match self.mode {
            AppMode::ImageEditor => {
                if self.image_editor_comp.editor.has_cells() {
                    format!(
                        "Image Editor {}",
                        self.image_editor_comp.editor.adjustment_status()
                    )
                } else {
                    "Image Editor".to_string()
                }
            }
            AppMode::AsciiPreview => "ASCII Preview".to_string(),
            AppMode::FontEditor => {
                if let font_editor::FontEditorView::CharEditor(code) =
                    self.font_editor_comp.editor.view
                {
                    format!("Font Editor [U+{code:04X}]")
                } else if self.font_editor_comp.editor.view
                    == font_editor::FontEditorView::HeaderEditor
                {
                    "Font Editor - Header".to_string()
                } else if self.font_editor_comp.editor.view
                    == font_editor::FontEditorView::SmushRuleEditor
                {
                    "Font Editor - Smushing Rules".to_string()
                } else if self.font_editor_comp.editor.view
                    == font_editor::FontEditorView::TransformEditor
                {
                    "Font Editor - Transforms".to_string()
                } else {
                    "Font Editor".to_string()
                }
            }
        };
        self.status_bar_comp.cursor = self.canvas_comp.canvas.cursor();
        self.status_bar_comp.zoom = self.canvas_comp.canvas.zoom_level();
        self.status_bar_comp.tool_name = self.toolbox_comp.toolbox.selected.full_name().to_string();
        self.status_bar_comp.mode_name = mode_name;
        self.status_bar_comp.unsaved = self.unsaved;
        self.status_bar_comp.current_path = self
            .font_editor_comp
            .editor
            .current_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string());
        self.status_bar_comp.throbber_text = self.throbber.render_string();
        self.status_bar_comp.mode = self.mode;
        self.status_bar_comp.undo_count = self.undo.history_len();
        self.status_bar_comp.fps = self.fps;
        self.status_bar_comp.git_branch = self.git_branch.clone();
        self.status_bar_comp.clock_str = format_clock();
        self.status_bar_comp.layer_count = 1;
        self.status_bar_comp.animation_frame = 0;
        let _ = self.status_bar_comp.draw(frame, chunks[3]);

        // Export dialog overlay
        if self.export_comp.dialog.active {
            let overlay = Rect {
                x: frame.area().width / 6,
                y: frame.area().height / 6,
                width: frame.area().width * 2 / 3,
                height: frame.area().height * 2 / 3,
            };
            self.export_comp.dialog.render(frame, overlay);
        }

        // File ops overlay
        if self.file_ops_comp.dialog.mode != file_ops::FileOpsMode::Idle {
            let overlay = Rect {
                x: frame.area().width / 6,
                y: frame.area().height / 6,
                width: frame.area().width * 2 / 3,
                height: frame.area().height * 2 / 3,
            };
            self.file_ops_comp.dialog.render(frame, overlay);
        }

        // Undo history panel overlay
        if self.undo_panel_comp.panel.open {
            self.undo_panel_comp
                .panel
                .render(frame, frame.area(), self.undo.history_entries());
        }
    }

    fn compute_canvas_rect(&self, inner: Rect) -> Rect {
        let zoom = self.canvas_comp.canvas.zoom_level().max(1) as u16;
        let buf_w = self.canvas_comp.canvas.buffer.width() as u16;
        let buf_h = self.canvas_comp.canvas.buffer.height() as u16;
        let grid_w = (buf_w * zoom).min(inner.width);
        let grid_h = (buf_h * zoom).min(inner.height);
        Rect {
            x: inner.x + (inner.width.saturating_sub(grid_w) / 2),
            y: inner.y + (inner.height.saturating_sub(grid_h) / 2),
            width: grid_w,
            height: grid_h,
        }
    }

    fn sync_canvas_to_font_char(&mut self) {
        if let font_editor::FontEditorView::CharEditor(code) = self.font_editor_comp.editor.view {
            self.font_editor_comp
                .sync_from_canvas(code, &self.canvas_comp.canvas.buffer);
        }
    }

    fn sync_font_char_to_canvas(&mut self) {
        if let Some((_, ch)) = self.font_editor_comp.selected_char() {
            let w = ch.width().max(1);
            let h = ch.rows().len().max(1);
            if self.canvas_comp.canvas.buffer.width() != w
                || self.canvas_comp.canvas.buffer.height() != h
            {
                self.canvas_comp.canvas = canvas::CanvasWidget::new(w as u16, h as u16);
            }
            for y in 0..h {
                let row = &ch.rows()[y];
                for (x, c) in row.chars().enumerate() {
                    if x < w {
                        self.canvas_comp.canvas.buffer.set(
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
        if let Some(cells) = self.image_editor_comp.editor.cells() {
            let h = cells.len();
            let w = cells[0].len();
            if self.canvas_comp.canvas.buffer.width() != w
                || self.canvas_comp.canvas.buffer.height() != h
            {
                self.canvas_comp.canvas = canvas::CanvasWidget::new(w as u16, h as u16);
            }
            for (y, row) in cells.iter().enumerate() {
                for (x, cell) in row.iter().enumerate() {
                    self.canvas_comp.canvas.buffer.set(x, y, *cell);
                }
            }
        }
    }

    fn screen_to_buffer(&self, col: u16, row: u16) -> Option<(i16, i16)> {
        let zoom = self.canvas_comp.canvas.zoom_level().max(1) as i16;
        let area = self.canvas_comp.canvas_inner_rect;
        if col < area.x || col >= area.x + area.width {
            return None;
        }
        if row < area.y || row >= area.y + area.height {
            return None;
        }
        let (sx, sy) = self.canvas_comp.canvas.scroll_offset();
        let bx = sx as i16 + (col as i16 - area.x as i16) / zoom;
        let by = sy as i16 + (row as i16 - area.y as i16) / zoom;
        Some((bx, by))
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        // Menu bar mouse event
        if self
            .menu_bar
            .handle_mouse_event(mouse.column, mouse.row, mouse.kind)
        {
            if let Some(action) = self.menu_bar.drain_actions() {
                self.process_action(&Action::Menu(action));
            }
            return;
        }

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
                self.toolbox_comp.toolbox.selected = tools[idx];
                self.selection_polygon_points.clear();
            }
            return;
        }

        // Text tool: hit-test blocks or enter text mode
        if self.toolbox_comp.toolbox.selected == Tool::Text {
            if let MouseEventKind::Down(_) = mouse.kind {
                if let Some((bx, by)) = self.screen_to_buffer(mouse.column, mouse.row) {
                    if !self.text_tool.entering_text {
                        if let Some(idx) = self.text_tool.hit_test(bx, by) {
                            self.text_tool.selected_block = Some(idx);
                            self.prev_mouse_buf = None;
                            self.line_start = None;
                            self.saved_buffer = None;
                            return;
                        }
                        self.text_tool.cursor_position = (bx, by);
                        self.text_tool.entering_text = true;
                        self.text_tool.text_buffer.clear();
                        self.canvas_comp
                            .canvas
                            .set_cursor(bx.max(0) as u16, by.max(0) as u16);
                    } else {
                        self.text_tool.cursor_position = (bx, by);
                        self.canvas_comp
                            .canvas
                            .set_cursor(bx.max(0) as u16, by.max(0) as u16);
                    }
                }
            }
            self.prev_mouse_buf = None;
            self.line_start = None;
            self.saved_buffer = None;
            return;
        }

        let is_selection_tool = matches!(
            self.toolbox_comp.toolbox.selected,
            Tool::Marquee | Tool::Lasso | Tool::CircleSelect | Tool::PolygonSelect
        );

        if !is_selection_tool
            && !matches!(
                self.toolbox_comp.toolbox.selected,
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
                self.canvas_comp
                    .canvas
                    .set_cursor(bx.max(0) as u16, by.max(0) as u16);
                self.unsaved = true;

                if is_selection_tool {
                    self.handle_selection_down(bx, by);
                    return;
                }

                // Start batch for drag operations, push initial snapshot
                self.undo.begin_batch();
                if self.toolbox_comp.toolbox.selected == Tool::Fill {
                    self.push_undo_snapshot("Flood fill");
                    let mut cell = canvas::CanvasCell {
                        ch: self.toolbox_comp.brush.ch,
                        fg: None,
                        bg: None,
                    };
                    self.palette_comp.palette.apply_to_cell(&mut cell);
                    tools::fill::flood_fill(&mut self.canvas_comp.canvas.buffer, bx, by, cell);
                    return;
                }
                if self.toolbox_comp.toolbox.selected == Tool::Line {
                    self.push_undo_snapshot("Line tool");
                    self.line_start = Some((bx, by));
                    self.saved_buffer = Some(self.canvas_comp.canvas.buffer.clone());
                    return;
                }
                if self.toolbox_comp.toolbox.selected == Tool::Eraser {
                    self.push_undo_snapshot("Eraser");
                    tools::eraser::erase_stamp(
                        &mut self.canvas_comp.canvas.buffer,
                        bx,
                        by,
                        self.toolbox_comp.brush.shape,
                        self.toolbox_comp.brush.size,
                    );
                } else if self.toolbox_comp.toolbox.selected == Tool::Eyedropper {
                    if let Some(cell) =
                        tools::eyedropper::sample(&self.canvas_comp.canvas.buffer, bx, by)
                    {
                        self.toolbox_comp.brush.ch = cell.ch;
                        if let Some(fg) = cell.fg {
                            self.palette_comp.palette.selected_color = Some(fg);
                            self.palette_comp.palette.push_recent(fg);
                            self.palette_comp.palette.target = palette::ColorTarget::Foreground;
                        }
                    }
                } else if self.toolbox_comp.toolbox.selected == Tool::Spray {
                    self.push_undo_snapshot("Spray");
                    let mut cell = canvas::CanvasCell {
                        ch: self.toolbox_comp.brush.ch,
                        fg: None,
                        bg: None,
                    };
                    self.palette_comp.palette.apply_to_cell(&mut cell);
                    let mut rng = StdRng::seed_from_u64(rand::thread_rng().gen());
                    tools::spray::spray_stamp(
                        &mut self.canvas_comp.canvas.buffer,
                        bx,
                        by,
                        self.toolbox_comp.brush.size,
                        self.toolbox_comp.brush.density,
                        cell,
                        &mut rng,
                    );
                } else {
                    self.push_undo_snapshot("Brush");
                    let mut cell = canvas::CanvasCell {
                        ch: self.toolbox_comp.brush.ch,
                        fg: None,
                        bg: None,
                    };
                    self.palette_comp.palette.apply_to_cell(&mut cell);
                    tools::brush::paint_stamp(
                        &mut self.canvas_comp.canvas.buffer,
                        bx,
                        by,
                        self.toolbox_comp.brush.shape,
                        self.toolbox_comp.brush.size,
                        cell,
                    );
                }
                self.prev_mouse_buf = Some((bx, by));
            }
            MouseEventKind::Drag(_) => {
                let Some((bx, by)) = self.screen_to_buffer(mouse.column, mouse.row) else {
                    return;
                };
                self.canvas_comp
                    .canvas
                    .set_cursor(bx.max(0) as u16, by.max(0) as u16);
                self.unsaved = true;

                if is_selection_tool {
                    self.handle_selection_drag(bx, by);
                    return;
                }

                if self.toolbox_comp.toolbox.selected == Tool::Line {
                    if let (Some((sx, sy)), Some(saved)) = (self.line_start, &self.saved_buffer) {
                        self.canvas_comp.canvas.buffer = saved.clone();
                        let mut cell = canvas::CanvasCell {
                            ch: self.toolbox_comp.brush.ch,
                            fg: None,
                            bg: None,
                        };
                        self.palette_comp.palette.apply_to_cell(&mut cell);
                        tools::line::draw_line_segment(
                            &mut self.canvas_comp.canvas.buffer,
                            sx,
                            sy,
                            bx,
                            by,
                            self.toolbox_comp.brush.shape,
                            self.toolbox_comp.brush.size,
                            cell,
                        );
                    }
                    return;
                }
                if let Some((px, py)) = self.prev_mouse_buf {
                    if self.toolbox_comp.toolbox.selected == Tool::Eraser {
                        tools::eraser::erase_line(
                            &mut self.canvas_comp.canvas.buffer,
                            px,
                            py,
                            bx,
                            by,
                            self.toolbox_comp.brush.shape,
                            self.toolbox_comp.brush.size,
                        );
                    } else if self.toolbox_comp.toolbox.selected == Tool::Spray {
                        let mut cell = canvas::CanvasCell {
                            ch: self.toolbox_comp.brush.ch,
                            fg: None,
                            bg: None,
                        };
                        self.palette_comp.palette.apply_to_cell(&mut cell);
                        let mut rng = StdRng::seed_from_u64(rand::thread_rng().gen());
                        tools::spray::spray_line(
                            &mut self.canvas_comp.canvas.buffer,
                            px,
                            py,
                            bx,
                            by,
                            self.toolbox_comp.brush.size,
                            self.toolbox_comp.brush.density,
                            cell,
                            &mut rng,
                        );
                    } else {
                        let mut cell = canvas::CanvasCell {
                            ch: self.toolbox_comp.brush.ch,
                            fg: None,
                            bg: None,
                        };
                        self.palette_comp.palette.apply_to_cell(&mut cell);
                        tools::brush::paint_line(
                            &mut self.canvas_comp.canvas.buffer,
                            px,
                            py,
                            bx,
                            by,
                            self.toolbox_comp.brush.shape,
                            self.toolbox_comp.brush.size,
                            cell,
                        );
                    }
                }
                self.prev_mouse_buf = Some((bx, by));
            }
            MouseEventKind::Up(_) => {
                self.undo.end_batch();
                if is_selection_tool {
                    self.handle_selection_up();
                }
                self.prev_mouse_buf = None;
                self.line_start = None;
                self.saved_buffer = None;
            }
            MouseEventKind::Moved => {
                if let Some((bx, by)) = self.screen_to_buffer(mouse.column, mouse.row) {
                    self.canvas_comp
                        .canvas
                        .set_cursor(bx.max(0) as u16, by.max(0) as u16);
                }
            }
            _ => {}
        }
    }

    fn handle_selection_down(&mut self, bx: i16, by: i16) {
        match self.toolbox_comp.toolbox.selected {
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
                if points.len() >= 3 {
                    let (fx, fy) = points[0];
                    let dist = ((bx - fx).abs() + (by - fy).abs()) as f64;
                    if dist < 3.0 {
                        self.selection = Some(tools::selection::Selection::polygon(
                            &self.canvas_comp.canvas.buffer,
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
        match self.toolbox_comp.toolbox.selected {
            Tool::Marquee => {
                if let Some((ox, oy)) = self.selection_drag_origin {
                    self.selection = Some(tools::selection::Selection::marquee(
                        &self.canvas_comp.canvas.buffer,
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
                        &self.canvas_comp.canvas.buffer,
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
        match self.toolbox_comp.toolbox.selected {
            Tool::Marquee | Tool::CircleSelect => {
                self.selection_drag_origin = None;
            }
            Tool::Lasso => {
                let points = std::mem::take(&mut self.selection_lasso_points);
                if points.len() >= 3 {
                    self.selection = Some(tools::selection::Selection::lasso(
                        &self.canvas_comp.canvas.buffer,
                        &points,
                    ));
                }
            }
            Tool::PolygonSelect => {}
            _ => {}
        }
    }

    fn push_undo_snapshot(&mut self, label: &str) {
        self.undo
            .push_snapshot(self.canvas_comp.canvas.buffer.clone(), label.to_string());
    }

    fn check_async_completion(&mut self) {
        let rx = match self.async_rx.take() {
            Some(rx) => rx,
            None => return,
        };
        match rx.try_recv() {
            Ok(result) => {
                self.throbber.stop();
                match result {
                    AsyncResult::SaveComplete(r) => match r {
                        Ok(path) => {
                            self.unsaved = false;
                            self.font_editor_comp.editor.current_path = Some(path);
                            self.last_save_time = Instant::now();
                            self.file_ops_comp.dialog.error_message.clear();
                        }
                        Err(e) => {
                            self.file_ops_comp.dialog.error_message = format!("Save failed: {e}");
                        }
                    },
                    AsyncResult::OpenComplete(r) => match r {
                        Ok((font, path)) => {
                            self.unsaved = false;
                            self.undo.clear();
                            self.font_editor_comp.editor.load_font(font);
                            self.font_editor_comp.editor.current_path = Some(path.clone());
                            self.file_ops_comp.recent_files.push(path);
                            self.file_ops_comp.recent_files.save_to_disk();
                            self.file_ops_comp.dialog.error_message.clear();
                        }
                        Err(e) => {
                            self.file_ops_comp.dialog.error_message = e;
                            self.file_ops_comp.dialog.mode = file_ops::FileOpsMode::Open;
                        }
                    },
                    AsyncResult::ExportComplete(r) => match r {
                        Ok(()) => {
                            self.export_comp.dialog.active = false;
                        }
                        Err(e) => {
                            self.export_comp.dialog.error_message = e;
                            self.export_comp.dialog.active = true;
                        }
                    },
                    AsyncResult::AutoSaveComplete => {}
                }
            }
            Err(mpsc::TryRecvError::Empty) => {
                self.async_rx = Some(rx);
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.throbber.stop();
            }
        }
    }

    pub fn handle_event(&mut self) -> io::Result<()> {
        if event::poll(Duration::from_millis(100))? {
            loop {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        let action = self.handle_key_event(key);
                        if let Some(ref a) = action {
                            self.process_action(a);
                        }
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse_event(mouse);
                    }
                    Event::Paste(text) => {
                        self.handle_paste_event(text);
                    }
                    _ => {}
                }
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }

        // Auto-save check
        if self.auto_save_interval > 0
            && self.unsaved
            && self.mode == AppMode::FontEditor
            && !self.throbber.is_active()
        {
            if let Some(ref path) = self.font_editor_comp.editor.current_path {
                if self.last_save_time.elapsed() >= Duration::from_secs(self.auto_save_interval) {
                    if let Some(ref font) = self.font_editor_comp.editor.font {
                        self.last_save_time = Instant::now();
                        let font = font.clone();
                        let path = path.clone();
                        let (tx, rx) = mpsc::channel();
                        self.async_rx = Some(rx);
                        self.throbber.start("Auto-saving...");
                        std::thread::spawn(move || {
                            let _ = file_ops::save_font(&font, &path);
                            let _ = tx.send(AsyncResult::AutoSaveComplete);
                        });
                    }
                }
            }
        }

        Ok(())
    }

    pub fn handle_key_event(&mut self, key: impl Into<KeyEvent>) -> Option<Action> {
        let key = key.into();
        let code = key.code;
        let modifiers = key.modifiers;

        // File ops dialog active: dispatch all keys to it
        if self.file_ops_comp.dialog.mode != file_ops::FileOpsMode::Idle {
            let prev_mode = self.file_ops_comp.dialog.mode;
            self.file_ops_comp.dialog.handle_key(code);
            if self.file_ops_comp.dialog.mode == file_ops::FileOpsMode::Idle {
                return match prev_mode {
                    file_ops::FileOpsMode::SaveAs => {
                        self.perform_save();
                        return Some(Action::SaveAsRequested);
                    }
                    file_ops::FileOpsMode::Open => {
                        self.perform_open();
                        return Some(Action::OpenRequested);
                    }
                    file_ops::FileOpsMode::Idle => None,
                };
            }
            return None;
        }

        // Export dialog active: dispatch all keys to it
        if self.export_comp.dialog.active {
            self.export_comp.dialog.handle_key(code);
            if !self.export_comp.dialog.active {
                self.perform_export();
            }
            return None;
        }

        // Undo history panel open: dispatch to it first
        if self.undo_panel_comp.panel.open {
            self.undo_panel_comp.panel.handle_key(code);
            return None;
        }

        // Menu bar active: dispatch all keys to it
        if self.menu_bar.is_active() {
            self.menu_bar.handle_key_event(key);
            if let Some(action) = self.menu_bar.drain_actions() {
                return Some(Action::Menu(action));
            }
            return None;
        }

        // Alt+key: open menu bar
        if modifiers == KeyModifiers::ALT && self.menu_bar.handle_key_event(key) {
            return None;
        }

        // Undo/redo: Ctrl+Z, Ctrl+Y, Ctrl+Shift+Z
        if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('z') {
            let empty = canvas::CanvasBuffer::new(1, 1);
            if modifiers.contains(KeyModifiers::SHIFT) {
                let cur = std::mem::replace(&mut self.canvas_comp.canvas.buffer, empty);
                if let Some((buf, _)) = self.undo.redo(cur) {
                    self.canvas_comp.canvas.buffer = buf;
                    self.unsaved = true;
                }
            } else {
                let cur = std::mem::replace(&mut self.canvas_comp.canvas.buffer, empty);
                if let Some((buf, _)) = self.undo.undo(cur) {
                    self.canvas_comp.canvas.buffer = buf;
                    self.unsaved = true;
                }
            }
            return Some(Action::Undo);
        }
        if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('y') {
            let empty = canvas::CanvasBuffer::new(1, 1);
            let cur = std::mem::replace(&mut self.canvas_comp.canvas.buffer, empty);
            if let Some((buf, _)) = self.undo.redo(cur) {
                self.canvas_comp.canvas.buffer = buf;
                self.unsaved = true;
            }
            return Some(Action::Redo);
        }

        // Ctrl+Shift+H: toggle undo history panel
        if modifiers.contains(KeyModifiers::CONTROL)
            && modifiers.contains(KeyModifiers::SHIFT)
            && code == KeyCode::Char('h')
        {
            self.undo_panel_comp.panel.toggle();
            return Some(Action::UndoPanelToggled);
        }

        if self.settings.settings_open {
            if self.settings.handle_key(code) {
                self.apply_settings();
                return None;
            }
            if let KeyCode::Char('S') = code {
                self.settings.settings_open = false;
            }
            return None;
        }

        // Font Editor mode: dispatch to font_editor before canvas/tools
        if self.mode == AppMode::FontEditor {
            if let Some(action) = self.font_editor_comp.handle_key_event(key) {
                if self.font_editor_comp.editor.view != font_editor::FontEditorView::Overview {
                    self.sync_font_char_to_canvas();
                }
                return Some(action);
            }
            // Re-create key for font editor (it consumes it)
            let fe_key = key;
            let area_width = self.canvas_comp.canvas_inner_rect.width;
            if self
                .font_editor_comp
                .editor
                .handle_key(fe_key.code, fe_key.modifiers, area_width)
            {
                if self.font_editor_comp.editor.view != font_editor::FontEditorView::Overview {
                    self.sync_font_char_to_canvas();
                }
                return Some(Action::FontEditorAction);
            }
        }

        // Image Editor mode: dispatch to image_editor before canvas/tools
        if self.mode == AppMode::ImageEditor {
            let was_entering = self.image_editor_comp.editor.entering_path();
            if self.image_editor_comp.editor.handle_key(code) {
                self.sync_image_to_canvas();
                if was_entering && !self.image_editor_comp.editor.entering_path() {
                    self.undo.clear();
                }
                return Some(Action::ImageEditorAction);
            }
        }

        // Text tool: text entry mode (before canvas, captures all keys)
        if self.toolbox_comp.toolbox.selected == Tool::Text && self.text_tool.entering_text {
            match code {
                KeyCode::Enter => {
                    self.push_undo_snapshot("Commit text");
                    self.text_tool.commit_block();
                    self.unsaved = true;
                    return Some(Action::TextCommitted);
                }
                KeyCode::Esc => {
                    self.text_tool.text_buffer.clear();
                    self.text_tool.entering_text = false;
                    return None;
                }
                KeyCode::Backspace => {
                    self.text_tool.text_buffer.pop();
                    return None;
                }
                KeyCode::Char(c) => {
                    self.text_tool.text_buffer.push(c);
                    return None;
                }
                _ => {}
            }
        }

        // Text tool: font navigation
        if self.toolbox_comp.toolbox.selected == Tool::Text
            && !self.text_tool.entering_text
            && self.text_tool.selected_block.is_none()
        {
            match code {
                KeyCode::Up => {
                    if !self.text_tool.available_fonts.is_empty() {
                        self.text_tool.font_index = self.text_tool.font_index.saturating_sub(1);
                        self.text_tool.load_selected_font();
                    }
                    return None;
                }
                KeyCode::Down => {
                    if !self.text_tool.available_fonts.is_empty() {
                        self.text_tool.font_index = (self.text_tool.font_index + 1)
                            .min(self.text_tool.available_fonts.len() - 1);
                        self.text_tool.load_selected_font();
                    }
                    return None;
                }
                _ => {}
            }
        }

        // Text tool: block operations
        if self.toolbox_comp.toolbox.selected == Tool::Text
            && !self.text_tool.entering_text
            && self.text_tool.selected_block.is_some()
        {
            match code {
                KeyCode::Up => {
                    self.push_undo_snapshot("Move text block");
                    self.text_tool.move_selected_block(0, -1);
                    self.unsaved = true;
                    return None;
                }
                KeyCode::Down => {
                    self.push_undo_snapshot("Move text block");
                    self.text_tool.move_selected_block(0, 1);
                    self.unsaved = true;
                    return None;
                }
                KeyCode::Left => {
                    self.push_undo_snapshot("Move text block");
                    self.text_tool.move_selected_block(-1, 0);
                    self.unsaved = true;
                    return None;
                }
                KeyCode::Right => {
                    self.push_undo_snapshot("Move text block");
                    self.text_tool.move_selected_block(1, 0);
                    self.unsaved = true;
                    return None;
                }
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    self.push_undo_snapshot("Scale text block");
                    self.text_tool.scale_selected_block(1);
                    self.unsaved = true;
                    return None;
                }
                KeyCode::Char('-') | KeyCode::Char('_') => {
                    self.push_undo_snapshot("Scale text block");
                    self.text_tool.scale_selected_block(-1);
                    self.unsaved = true;
                    return None;
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.push_undo_snapshot("Rotate text block");
                    self.text_tool.rotate_selected_block();
                    self.unsaved = true;
                    return None;
                }
                KeyCode::Delete | KeyCode::Backspace => {
                    self.push_undo_snapshot("Delete text block");
                    self.text_tool.delete_selected_block();
                    self.unsaved = true;
                    return None;
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    if let Some(idx) = self.text_tool.selected_block {
                        self.text_tool.re_edit_block(idx);
                    }
                    return None;
                }
                KeyCode::Esc => {
                    self.text_tool.selected_block = None;
                    return None;
                }
                _ => {}
            }
        }

        // Selection operations (before canvas cursor movement)
        let selection_active = self.selection.as_ref().is_some_and(|s| s.is_active());

        if selection_active {
            match code {
                KeyCode::Up => {
                    self.push_undo_snapshot("Move selection");
                    self.move_selection(0, -1);
                    self.unsaved = true;
                    return None;
                }
                KeyCode::Down => {
                    self.push_undo_snapshot("Move selection");
                    self.move_selection(0, 1);
                    self.unsaved = true;
                    return None;
                }
                KeyCode::Left => {
                    self.push_undo_snapshot("Move selection");
                    self.move_selection(-1, 0);
                    self.unsaved = true;
                    return None;
                }
                KeyCode::Right => {
                    self.push_undo_snapshot("Move selection");
                    self.move_selection(1, 0);
                    self.unsaved = true;
                    return None;
                }
                KeyCode::Delete | KeyCode::Backspace => {
                    self.push_undo_snapshot("Delete selection");
                    if let Some(sel) = self.selection.take() {
                        sel.delete_from(&mut self.canvas_comp.canvas.buffer);
                        self.unsaved = true;
                    }
                    return None;
                }
                _ => {}
            }

            if modifiers.contains(KeyModifiers::CONTROL) {
                match code {
                    KeyCode::Char('c') => {
                        if let Some(ref sel) = self.selection {
                            self.clipboard = Some(sel.copy_from(&self.canvas_comp.canvas.buffer));
                        }
                        return None;
                    }
                    KeyCode::Char('x') => {
                        self.push_undo_snapshot("Cut selection");
                        if let Some(sel) = self.selection.take() {
                            self.clipboard =
                                Some(sel.cut_from(&mut self.canvas_comp.canvas.buffer));
                            self.unsaved = true;
                        }
                        return None;
                    }
                    KeyCode::Char('v') => {
                        self.push_undo_snapshot("Paste");
                        if let Some(ref clip) = self.clipboard {
                            let (cx, cy) = self.canvas_comp.canvas.cursor();
                            tools::selection::Selection::paste_into(
                                &mut self.canvas_comp.canvas.buffer,
                                clip,
                                cx as i16,
                                cy as i16,
                            );
                            self.unsaved = true;
                        }
                        return None;
                    }
                    _ => {}
                }
            }
        }

        // Polygon select tool: Enter closes polygon, Esc cancels
        if self.toolbox_comp.toolbox.selected == Tool::PolygonSelect
            && !self.selection_polygon_points.is_empty()
        {
            match code {
                KeyCode::Enter => {
                    let points = std::mem::take(&mut self.selection_polygon_points);
                    if points.len() >= 3 {
                        self.selection = Some(tools::selection::Selection::polygon(
                            &self.canvas_comp.canvas.buffer,
                            &points,
                        ));
                    }
                    return None;
                }
                KeyCode::Esc => {
                    self.selection_polygon_points.clear();
                    return None;
                }
                _ => {}
            }
        }

        // Deselect on Esc
        if self.selection.is_some() && code == KeyCode::Esc {
            self.selection = None;
            return None;
        }

        // Canvas cursor movement, zoom, grid
        if let Some(action) = self.canvas_comp.handle_key_event(key) {
            return Some(action);
        }

        // Text tool settings (not entering text)
        if self.toolbox_comp.toolbox.selected == Tool::Text && !self.text_tool.entering_text {
            match code {
                KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.text_tool.justification = match self.text_tool.justification {
                        crate::render::Justification::Left => crate::render::Justification::Center,
                        crate::render::Justification::Center => crate::render::Justification::Right,
                        crate::render::Justification::Right => crate::render::Justification::Left,
                    };
                    return None;
                }
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    if self.text_tool.scale < 4 {
                        self.text_tool.scale += 1;
                    }
                    return None;
                }
                KeyCode::Char('-') | KeyCode::Char('_') => {
                    if self.text_tool.scale > 1 {
                        self.text_tool.scale -= 1;
                    }
                    return None;
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    let (cx, cy) = self.canvas_comp.canvas.cursor();
                    self.text_tool.cursor_position = (cx as i16, cy as i16);
                    self.text_tool.entering_text = true;
                    self.text_tool.text_buffer.clear();
                    return None;
                }
                _ => {}
            }
        }

        // Settings toggle
        if code == KeyCode::Char('S') && !modifiers.contains(KeyModifiers::CONTROL) {
            self.settings.canvas_width = self.canvas_comp.canvas.buffer.width() as u16;
            self.settings.canvas_height = self.canvas_comp.canvas.buffer.height() as u16;
            self.settings.show_grid = self.canvas_comp.canvas.show_grid();
            self.settings.settings_open = true;
            return None;
        }

        // Toolbox tool selection + brush adjustments
        if let Some(action) = self.toolbox_comp.handle_key_event(key) {
            if self.toolbox_comp.toolbox.selected != Tool::PolygonSelect {
                self.selection_polygon_points.clear();
            }
            return Some(action);
        }

        // Palette color selection
        if let Some(action) = self.palette_comp.handle_key_event(key) {
            return Some(action);
        }

        // Keyboard painting: Space/Enter paints or erases at cursor
        if matches!(
            self.toolbox_comp.toolbox.selected,
            Tool::Brush | Tool::Eraser | Tool::Line | Tool::Fill | Tool::Spray
        ) && matches!(code, KeyCode::Char(' ') | KeyCode::Enter)
        {
            let (cx, cy) = self.canvas_comp.canvas.cursor();
            self.push_undo_snapshot("Keyboard paint");
            if self.toolbox_comp.toolbox.selected == Tool::Fill {
                let mut cell = canvas::CanvasCell {
                    ch: self.toolbox_comp.brush.ch,
                    fg: None,
                    bg: None,
                };
                self.palette_comp.palette.apply_to_cell(&mut cell);
                tools::fill::flood_fill(
                    &mut self.canvas_comp.canvas.buffer,
                    cx as i16,
                    cy as i16,
                    cell,
                );
            } else if self.toolbox_comp.toolbox.selected == Tool::Eraser {
                tools::eraser::erase_stamp(
                    &mut self.canvas_comp.canvas.buffer,
                    cx as i16,
                    cy as i16,
                    self.toolbox_comp.brush.shape,
                    self.toolbox_comp.brush.size,
                );
            } else if self.toolbox_comp.toolbox.selected == Tool::Spray {
                let mut cell = canvas::CanvasCell {
                    ch: self.toolbox_comp.brush.ch,
                    fg: None,
                    bg: None,
                };
                self.palette_comp.palette.apply_to_cell(&mut cell);
                let mut rng = StdRng::seed_from_u64(rand::thread_rng().gen());
                tools::spray::spray_stamp(
                    &mut self.canvas_comp.canvas.buffer,
                    cx as i16,
                    cy as i16,
                    self.toolbox_comp.brush.size,
                    self.toolbox_comp.brush.density,
                    cell,
                    &mut rng,
                );
            } else {
                let mut cell = canvas::CanvasCell {
                    ch: self.toolbox_comp.brush.ch,
                    fg: None,
                    bg: None,
                };
                self.palette_comp.palette.apply_to_cell(&mut cell);
                tools::brush::paint_stamp(
                    &mut self.canvas_comp.canvas.buffer,
                    cx as i16,
                    cy as i16,
                    self.toolbox_comp.brush.shape,
                    self.toolbox_comp.brush.size,
                    cell,
                );
            }
            self.unsaved = true;
            return None;
        }

        // Ctrl+O: Open font
        if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('o') {
            self.start_open();
            return None;
        }

        // Ctrl+S: Save (or Save As if no path)
        if modifiers.contains(KeyModifiers::CONTROL)
            && !modifiers.contains(KeyModifiers::SHIFT)
            && code == KeyCode::Char('s')
        {
            self.start_save();
            return None;
        }

        // Ctrl+Shift+S: always Save As
        if modifiers.contains(KeyModifiers::CONTROL)
            && modifiers.contains(KeyModifiers::SHIFT)
            && code == KeyCode::Char('s')
        {
            self.start_save_as();
            return None;
        }

        // Ctrl+E: Open export dialog
        if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('e') {
            let mode = match self.mode {
                AppMode::FontEditor => export::ExportMode::Txt,
                _ => export::ExportMode::Png,
            };
            self.export_comp.dialog.enter_export(mode);
            return None;
        }

        match code {
            KeyCode::Tab => {
                self.mode = self.mode.next();
                self.undo.clear();
                Some(Action::ModeChanged)
            }
            KeyCode::Char('q') if !modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                Some(Action::Quit)
            }
            KeyCode::Esc => {
                self.should_quit = true;
                Some(Action::Quit)
            }
            _ => None,
        }
    }

    fn move_selection(&mut self, dx: i16, dy: i16) {
        if let Some(ref mut sel) = self.selection {
            if sel.is_active() {
                sel.move_selection(&mut self.canvas_comp.canvas.buffer, dx, dy);
            }
        }
    }

    fn start_save(&mut self) {
        if self.mode != AppMode::FontEditor {
            return;
        }
        if let Some(ref path) = self.font_editor_comp.editor.current_path {
            if let Some(ref font) = self.font_editor_comp.editor.font {
                if self.throbber.is_active() {
                    return;
                }
                let font = font.clone();
                let path = path.clone();
                let (tx, rx) = mpsc::channel();
                self.async_rx = Some(rx);
                self.throbber.start("Saving...");
                std::thread::spawn(move || {
                    let result = file_ops::save_font(&font, &path)
                        .map(|_| path)
                        .map_err(|e| e.to_string());
                    let _ = tx.send(AsyncResult::SaveComplete(result));
                });
                return;
            }
        }
        self.start_save_as();
    }

    fn start_save_as(&mut self) {
        if self.mode != AppMode::FontEditor {
            return;
        }
        self.file_ops_comp
            .dialog
            .enter_save_as(self.font_editor_comp.editor.current_path.as_deref());
    }

    fn perform_save(&mut self) {
        if self.throbber.is_active() {
            return;
        }
        let path = self.file_ops_comp.dialog.selected_path();
        let font = match &self.font_editor_comp.editor.font {
            Some(f) => f.clone(),
            None => return,
        };
        let result_path = path.clone();
        let (tx, rx) = mpsc::channel();
        self.async_rx = Some(rx);
        self.throbber.start("Saving...");
        std::thread::spawn(move || {
            let result = file_ops::save_font(&font, &result_path)
                .map(|_| result_path)
                .map_err(|e| e.to_string());
            let _ = tx.send(AsyncResult::SaveComplete(result));
        });
    }

    fn handle_paste_event(&mut self, text: String) {
        if self.file_ops_comp.dialog.mode != file_ops::FileOpsMode::Idle {
            self.file_ops_comp.dialog.handle_paste(&text);
        }
    }

    fn start_open(&mut self) {
        if self.mode != AppMode::FontEditor {
            return;
        }
        self.file_ops_comp
            .dialog
            .enter_open(self.file_ops_comp.recent_files.list());
    }

    fn perform_open(&mut self) {
        if self.throbber.is_active() {
            return;
        }
        let path = self.file_ops_comp.dialog.selected_path();
        let path_clone = path.clone();
        let (tx, rx) = mpsc::channel();
        self.async_rx = Some(rx);
        self.throbber.start("Loading...");
        std::thread::spawn(move || {
            let result = (|| -> Result<(crate::font::FIGfont, std::path::PathBuf), String> {
                let content = std::fs::read_to_string(&path_clone)
                    .map_err(|e| format!("Cannot read file: {e}"))?;
                let font = crate::font::parse_tlf_font(&content)
                    .map_err(|e| format!("Parse error: {e}"))?;
                Ok((font, path_clone))
            })();
            let _ = tx.send(AsyncResult::OpenComplete(result));
        });
    }

    fn perform_export(&mut self) {
        if self.throbber.is_active() {
            return;
        }
        let cells: Vec<Vec<canvas::CanvasCell>> = (0..self.canvas_comp.canvas.buffer.height())
            .map(|y| {
                (0..self.canvas_comp.canvas.buffer.width())
                    .map(|x| {
                        self.canvas_comp
                            .canvas
                            .buffer
                            .get(x, y)
                            .copied()
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .collect();
        let format = self.export_comp.dialog.format;
        let font_size = self.export_comp.dialog.font_size;
        let path_buf = std::path::PathBuf::from(&self.export_comp.dialog.path_buffer);
        let (tx, rx) = mpsc::channel();
        self.async_rx = Some(rx);
        self.throbber.start("Exporting...");
        std::thread::spawn(move || {
            let result = (|| -> Result<(), String> {
                if path_buf.as_os_str().is_empty() {
                    return Err("no path specified".to_string());
                }
                let bytes: Vec<u8> = match format {
                    crate::tui::export::ExportMode::Png => {
                        crate::output::export_cells_to_png(&cells, font_size)
                            .map_err(|e| e.to_string())?
                    }
                    crate::tui::export::ExportMode::Txt => {
                        crate::output::export_cells_to_txt(&cells).into_bytes()
                    }
                    crate::tui::export::ExportMode::Gif => {
                        crate::output::export_cells_to_gif(&[cells], &[10], font_size)
                            .map_err(|e| e.to_string())?
                    }
                };
                std::fs::write(&path_buf, &bytes).map_err(|e| format!("IoError({e})"))?;
                Ok(())
            })();
            let _ = tx.send(AsyncResult::ExportComplete(result));
        });
    }

    fn handle_menu_action(&mut self, action: menu::MenuAction) {
        match action {
            menu::MenuAction::FileOpen => {
                self.start_open();
                self.menu_bar.reset();
            }
            menu::MenuAction::FileSave => {
                self.start_save();
                self.menu_bar.reset();
            }
            menu::MenuAction::FileSaveAs => {
                self.start_save_as();
                self.menu_bar.reset();
            }
            menu::MenuAction::FileExport => {
                let mode = match self.mode {
                    AppMode::FontEditor => export::ExportMode::Txt,
                    _ => export::ExportMode::Png,
                };
                self.export_comp.dialog.enter_export(mode);
                self.menu_bar.reset();
            }
            menu::MenuAction::FileQuit => {
                self.should_quit = true;
                self.menu_bar.reset();
            }
            menu::MenuAction::EditUndo => {
                if self.undo.can_undo() {
                    let empty = canvas::CanvasBuffer::new(1, 1);
                    let cur = std::mem::replace(&mut self.canvas_comp.canvas.buffer, empty);
                    if let Some((buf, _)) = self.undo.undo(cur) {
                        self.canvas_comp.canvas.buffer = buf;
                        self.unsaved = true;
                    }
                }
                self.menu_bar.reset();
            }
            menu::MenuAction::EditRedo => {
                if self.undo.can_redo() {
                    let empty = canvas::CanvasBuffer::new(1, 1);
                    let cur = std::mem::replace(&mut self.canvas_comp.canvas.buffer, empty);
                    if let Some((buf, _)) = self.undo.redo(cur) {
                        self.canvas_comp.canvas.buffer = buf;
                        self.unsaved = true;
                    }
                }
                self.menu_bar.reset();
            }
            menu::MenuAction::EditCut => {
                if let Some(ref sel) = self.selection {
                    if sel.is_active() {
                        self.push_undo_snapshot("Cut selection");
                        if let Some(sel_owned) = self.selection.take() {
                            self.clipboard =
                                Some(sel_owned.cut_from(&mut self.canvas_comp.canvas.buffer));
                            self.unsaved = true;
                        }
                    }
                }
                self.menu_bar.reset();
            }
            menu::MenuAction::EditCopy => {
                if let Some(ref sel) = self.selection {
                    if sel.is_active() {
                        self.clipboard = Some(sel.copy_from(&self.canvas_comp.canvas.buffer));
                    }
                }
                self.menu_bar.reset();
            }
            menu::MenuAction::EditPaste => {
                if self.clipboard.is_some() {
                    self.push_undo_snapshot("Paste");
                    let clip = self.clipboard.clone();
                    if let Some(ref clip_data) = clip {
                        let (cx, cy) = self.canvas_comp.canvas.cursor();
                        tools::selection::Selection::paste_into(
                            &mut self.canvas_comp.canvas.buffer,
                            clip_data,
                            cx as i16,
                            cy as i16,
                        );
                        self.unsaved = true;
                    }
                }
                self.menu_bar.reset();
            }
            menu::MenuAction::ViewZoomIn => {
                if self.canvas_comp.canvas.zoom_level() < 8 {
                    self.canvas_comp.canvas.zoom_in();
                }
                self.menu_bar.reset();
            }
            menu::MenuAction::ViewZoomOut => {
                if self.canvas_comp.canvas.zoom_level() > 1 {
                    self.canvas_comp.canvas.zoom_out();
                }
                self.menu_bar.reset();
            }
            menu::MenuAction::ViewToggleGrid => {
                self.canvas_comp.canvas.toggle_grid();
                self.menu_bar.reset();
            }
            menu::MenuAction::ViewToggleUndoPanel => {
                self.undo_panel_comp.panel.toggle();
                self.menu_bar.reset();
            }
            menu::MenuAction::ToolsSelect(tool) => {
                self.toolbox_comp.toolbox.selected = tool;
                if tool != toolbox::Tool::PolygonSelect {
                    self.selection_polygon_points.clear();
                }
                self.menu_bar.reset();
            }
            menu::MenuAction::HelpAbout => {
                // Show simple about message - deferred to proper dialog
                self.menu_bar.reset();
            }
            menu::MenuAction::HelpKeybindings => {
                // Show keybindings - deferred to proper dialog
                self.menu_bar.reset();
            }
        }
    }

    fn apply_settings(&mut self) {
        let w = self.settings.canvas_width as usize;
        let h = self.settings.canvas_height as usize;
        if self.canvas_comp.canvas.buffer.width() != w
            || self.canvas_comp.canvas.buffer.height() != h
        {
            self.canvas_comp.canvas = canvas::CanvasWidget::new(w as u16, h as u16);
            self.undo.clear();
        }
        if self.settings.show_grid != self.canvas_comp.canvas.show_grid() {
            self.canvas_comp.canvas.toggle_grid();
        }
    }
}

pub enum AsyncResult {
    SaveComplete(Result<std::path::PathBuf, String>),
    OpenComplete(Result<(crate::font::FIGfont, std::path::PathBuf), String>),
    ExportComplete(Result<(), String>),
    AutoSaveComplete,
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}

fn format_clock() -> String {
    let since_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = since_epoch.as_secs();
    let h = (total_secs / 3600) % 24;
    let m = (total_secs / 60) % 60;
    let s = total_secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}
