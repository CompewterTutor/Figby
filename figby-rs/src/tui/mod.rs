use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Tabs};
use ratatui::Frame;
use std::collections::BTreeMap;
use std::io;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::config;

pub mod brush;
pub mod canvas;
pub mod events;
pub mod export;
pub mod file_ops;
pub mod font_editor;
pub mod image_editor;
pub mod keymap;
pub mod layers;
pub mod layout;
pub mod menu;
pub mod palette;
pub mod render_mode;
pub mod status;
pub mod theme;
pub mod throbber;
pub mod timeline;
pub mod toolbox;
pub mod tools;
pub mod undo;
pub mod undo_panel;
pub mod welcome;

pub use brush::BrushState;
pub use events::AppEvent;
pub use export::ExportMode;
pub use menu::{MenuBar, MenuBarState};
pub use palette::Palette;
pub use render_mode::RenderMode;
pub use status::CanvasSettings;
pub use throbber::ThrobberState;
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

    fn prev(&self) -> Self {
        match self {
            AppMode::FontEditor => AppMode::AsciiPreview,
            AppMode::AsciiPreview => AppMode::ImageEditor,
            AppMode::ImageEditor => AppMode::FontEditor,
        }
    }
}

/// Canvas/tool/undo state — everything needed to edit a document.
pub struct EditorState {
    pub canvas: canvas::CanvasWidget,
    pub toolbox: toolbox::Toolbox,
    pub brush: brush::BrushState,
    pub palette: palette::Palette,
    pub font_editor: font_editor::FontEditor,
    pub image_editor: image_editor::ImageEditor,
    pub text_tool: tools::text::TextToolState,
    pub undo: undo::UndoSystem,
    pub unsaved: bool,
    pub selection: Option<tools::selection::Selection>,
    pub clipboard: Option<tools::selection::Clipboard>,
    pub layer_stack: layers::LayerStack,
    pub layer_panel: layers::LayerPanel,
}

impl EditorState {
    fn recomposite_canvas(&mut self) {
        self.canvas.buffer = self.layer_stack.composite();
    }

    fn push_undo_snapshot(&mut self, label: &str) {
        self.undo.push_snapshot(
            self.layer_stack.active_layer().buffer.clone(),
            label.to_string(),
        );
    }

    fn compute_canvas_rect(&self, inner: Rect) -> Rect {
        let zoom = self.canvas.zoom_level().max(1) as u16;
        let buf_w = self.canvas.buffer.width() as u16;
        let buf_h = self.canvas.buffer.height() as u16;
        let grid_w = (buf_w * zoom).min(inner.width);
        let grid_h = (buf_h * zoom).min(inner.height);
        Rect {
            x: inner.x + (inner.width.saturating_sub(grid_w) / 2),
            y: inner.y + (inner.height.saturating_sub(grid_h) / 2),
            width: grid_w,
            height: grid_h,
        }
    }

    fn screen_to_buffer(&self, col: u16, row: u16, canvas_inner_rect: Rect) -> Option<(i16, i16)> {
        let zoom = self.canvas.zoom_level().max(1) as i16;
        if col < canvas_inner_rect.x || col >= canvas_inner_rect.x + canvas_inner_rect.width {
            return None;
        }
        if row < canvas_inner_rect.y || row >= canvas_inner_rect.y + canvas_inner_rect.height {
            return None;
        }
        let (sx, sy) = self.canvas.scroll_offset();
        let bx = sx as i16 + (col as i16 - canvas_inner_rect.x as i16) / zoom;
        let by = sy as i16 + (row as i16 - canvas_inner_rect.y as i16) / zoom;
        Some((bx, by))
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
                self.layer_stack.resize_all(w, h);
            }
            let mut buf = self.layer_stack.active_layer().buffer.clone();
            for y in 0..h {
                let row = &ch.rows()[y];
                for (x, c) in row.chars().enumerate() {
                    if x < w {
                        buf.set(
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
            *self.layer_stack.active_layer_mut().buffer_mut() = buf;
            self.recomposite_canvas();
        }
    }

    fn sync_image_to_canvas(&mut self) {
        if let Some(cells) = self.image_editor.cells() {
            let h = cells.len();
            let w = cells[0].len();
            if self.canvas.buffer.width() != w || self.canvas.buffer.height() != h {
                self.canvas = canvas::CanvasWidget::new(w as u16, h as u16);
                self.layer_stack.resize_all(w, h);
            }
            let mut buf = self.layer_stack.active_layer().buffer.clone();
            for (y, row) in cells.iter().enumerate() {
                for (x, cell) in row.iter().enumerate() {
                    buf.set(x, y, *cell);
                }
            }
            *self.layer_stack.active_layer_mut().buffer_mut() = buf;
            self.recomposite_canvas();
        }
    }

    fn move_selection(&mut self, dx: i16, dy: i16) {
        if let Some(ref mut sel) = self.selection {
            if sel.is_active() {
                let mut buf = self.layer_stack.active_layer().buffer.clone();
                sel.move_selection(&mut buf, dx, dy);
                *self.layer_stack.active_layer_mut().buffer_mut() = buf;
            }
        }
    }

    fn handle_selection_down(
        &mut self,
        bx: i16,
        by: i16,
        selection_drag_origin: &mut Option<(i16, i16)>,
        selection_polygon_points: &mut Vec<(i16, i16)>,
        selection_lasso_points: &mut Vec<(i16, i16)>,
    ) {
        match self.toolbox.selected {
            Tool::Marquee => {
                self.selection = None;
                *selection_drag_origin = Some((bx, by));
            }
            Tool::CircleSelect => {
                self.selection = None;
                *selection_drag_origin = Some((bx, by));
            }
            Tool::Lasso => {
                self.selection = None;
                *selection_lasso_points = vec![(bx, by)];
            }
            Tool::PolygonSelect => {
                let points = selection_polygon_points;
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

    fn handle_selection_drag(
        &mut self,
        bx: i16,
        by: i16,
        selection_drag_origin: &mut Option<(i16, i16)>,
        selection_lasso_points: &mut Vec<(i16, i16)>,
    ) {
        match self.toolbox.selected {
            Tool::Marquee => {
                if let Some((ox, oy)) = *selection_drag_origin {
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
                if let Some((ox, oy)) = *selection_drag_origin {
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
                selection_lasso_points.push((bx, by));
            }
            _ => {}
        }
    }

    fn handle_selection_up(
        &mut self,
        selection_drag_origin: &mut Option<(i16, i16)>,
        selection_lasso_points: &mut Vec<(i16, i16)>,
    ) {
        match self.toolbox.selected {
            Tool::Marquee | Tool::CircleSelect => {
                *selection_drag_origin = None;
            }
            Tool::Lasso => {
                let points = std::mem::take(selection_lasso_points);
                if points.len() >= 3 {
                    self.selection = Some(tools::selection::Selection::lasso(
                        &self.canvas.buffer,
                        &points,
                    ));
                }
            }
            Tool::PolygonSelect => {}
            _ => {}
        }
    }
}

/// Dialog/overlay state — file ops, export, undo panel, settings panel.
pub struct DialogState {
    pub file_ops: file_ops::FileOpsDialog,
    pub recent_files: file_ops::RecentFiles,
    pub export_dialog: export::ExportDialog,
    pub undo_panel: undo_panel::UndoPanel,
    pub settings: status::CanvasSettings,
}

pub struct TuiApp {
    pub mode: AppMode,
    pub should_quit: bool,
    pub icons: BTreeMap<String, String>,
    pub menu_bar: MenuBar,
    pub menu_bar_state: menu::MenuBarState,
    // Status bar data (inline)
    pub status_cursor: (u16, u16),
    pub status_zoom: u8,
    pub status_tool_name: String,
    pub status_mode_name: String,
    pub status_unsaved: bool,
    pub status_current_path: Option<String>,
    pub status_throbber_text: String,
    pub status_mode: AppMode,
    pub status_undo_count: usize,
    pub status_fps: f64,
    pub status_render_mode: &'static str,
    pub status_git_branch: Option<String>,
    pub status_clock_str: String,
    pub status_layer_count: u8,
    pub status_animation_frame: u8,
    pub status_icons: BTreeMap<String, String>,
    pub status_theme: theme::Theme,
    // Drag state (extracted from EditorState)
    pub selection_drag_origin: Option<(i16, i16)>,
    pub selection_polygon_points: Vec<(i16, i16)>,
    pub selection_lasso_points: Vec<(i16, i16)>,
    pub prev_mouse_buf: Option<(i16, i16)>,
    pub line_start: Option<(i16, i16)>,
    pub saved_buffer: Option<canvas::CanvasBuffer>,
    auto_save_interval: u64,
    last_save_time: Instant,
    pub throbber: ThrobberState,
    async_rx: Option<mpsc::Receiver<AsyncResult>>,
    last_frame_time: Instant,
    fps: f64,
    git_branch: Option<String>,
    pub theme: theme::Theme,
    pub render_mode: RenderMode,
    dirty: bool,
    last_draw_time: Instant,
    pub show_keybindings: bool,
    pub welcome_screen: welcome::WelcomeScreen,
    /// `F11` toggle: canvas fills entire terminal, minimal hint overlay.
    pub zen_mode: bool,
    /// Controls what the right drawer panel shows.
    pub right_drawer: layout::DrawerMode,
    pub editor: EditorState,
    pub dialogs: DialogState,
}

impl TuiApp {
    pub fn new() -> Self {
        let icons: BTreeMap<String, String> = serde_yaml::from_str(ICONS_YAML).unwrap_or_default();
        let config = config::load_config();
        let theme = theme::load_theme(&config.tui.theme);

        let mut brush = brush::BrushState::new();
        if let Some(ref shape) = config.tui.brush.shape {
            brush.shape = match shape.as_str() {
                "square" => brush::BrushShape::Square,
                "circle" => brush::BrushShape::Circle,
                "spray" => brush::BrushShape::SprayPaint,
                "custom" => brush::BrushShape::Custom,
                _ => brush.shape,
            };
        }
        if let Some(size) = config.tui.brush.size {
            brush.set_size(size);
        }
        if let Some(density) = config.tui.brush.density {
            brush.set_density(density);
        }
        if let Some(ref ch_str) = config.tui.brush.ch {
            if let Some(ch) = ch_str.chars().next() {
                brush.ch = ch;
            }
        }

        let mut font_editor = font_editor::FontEditor::new();
        font_editor.theme = theme.clone();

        let mut file_ops = file_ops::FileOpsDialog::new();
        file_ops.theme = theme.clone();
        let mut recent_files = file_ops::RecentFiles::load_from_disk();
        if let Some(max) = config.tui.recent_files_max {
            recent_files.set_max(max);
        }

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

        let render_mode = match config.tui.render_mode.as_deref() {
            Some("fast") | Some("Fast") => RenderMode::Fast,
            _ => RenderMode::Dirty,
        };

        let mut canvas = canvas::CanvasWidget::default();
        canvas.theme = theme.clone();
        let mut palette = palette::Palette::new();
        palette.theme = theme.clone();
        let mut export_dialog = export::ExportDialog::new();
        export_dialog.theme = theme.clone();
        let mut undo_panel = undo_panel::UndoPanel::new();
        undo_panel.theme = theme.clone();
        let mut settings = status::CanvasSettings::new();
        settings.theme = theme.clone();
        let mut toolbox = toolbox::Toolbox::new();
        toolbox.theme = theme.clone();

        let canvas_w = canvas.buffer.width();
        let canvas_h = canvas.buffer.height();
        let layer_stack = layers::LayerStack::new(canvas_w, canvas_h);
        let mut layer_panel = layers::LayerPanel::new();
        layer_panel.theme = theme.clone();

        Self {
            mode: AppMode::FontEditor,
            should_quit: false,
            icons: icons.clone(),
            menu_bar: MenuBar::new(),
            menu_bar_state: menu::MenuBarState::new(),
            status_cursor: (0, 0),
            status_zoom: 1,
            status_tool_name: String::new(),
            status_mode_name: String::new(),
            status_unsaved: false,
            status_current_path: None,
            status_throbber_text: String::new(),
            status_mode: AppMode::FontEditor,
            status_undo_count: 0,
            status_fps: 0.0,
            status_render_mode: "",
            status_git_branch: git_branch.clone(),
            status_clock_str: String::new(),
            status_layer_count: 1,
            status_animation_frame: 0,
            status_icons: icons,
            status_theme: theme.clone(),
            selection_drag_origin: None,
            selection_polygon_points: Vec::new(),
            selection_lasso_points: Vec::new(),
            prev_mouse_buf: None,
            line_start: None,
            saved_buffer: None,
            auto_save_interval: 0,
            last_save_time: Instant::now(),
            throbber: ThrobberState::new(),
            async_rx: None,
            last_frame_time: Instant::now(),
            fps: 0.0,
            git_branch,
            theme: theme.clone(),
            render_mode,
            dirty: true,
            last_draw_time: Instant::now(),
            show_keybindings: false,
            welcome_screen: welcome::WelcomeScreen::new(),
            zen_mode: false,
            right_drawer: layout::DrawerMode::Palette,
            editor: {
                let mut editor = EditorState {
                    canvas,
                    toolbox,
                    brush,
                    palette,
                    font_editor,
                    image_editor: image_editor::ImageEditor::new(),
                    text_tool: tools::text::TextToolState::new("fonts"),
                    undo: undo::UndoSystem::new(config.tui.undo_limit.unwrap_or(50)),
                    unsaved: false,
                    selection: None,
                    clipboard: None,
                    layer_stack,
                    layer_panel,
                };
                editor.recomposite_canvas();
                editor
            },
            dialogs: DialogState {
                file_ops,
                recent_files,
                export_dialog,
                undo_panel,
                settings,
            },
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        let mut terminal = ratatui::init();
        execute!(io::stdout(), EnableBracketedPaste, EnableMouseCapture)?;

        while !self.should_quit {
            self.handle_event()?;

            let now = Instant::now();
            let needs_redraw = match self.render_mode {
                RenderMode::Fast => true,
                RenderMode::Dirty => {
                    self.dirty
                        || (self.throbber.is_active()
                            && now.saturating_duration_since(self.last_draw_time)
                                >= Duration::from_millis(100))
                }
            };

            if needs_redraw {
                terminal.draw(|f| self.render(f))?;
                self.dirty = false;
                self.last_draw_time = now;
            } else {
                std::thread::sleep(Duration::from_millis(5));
            }
        }

        execute!(
            terminal.backend_mut(),
            DisableBracketedPaste,
            DisableMouseCapture
        )?;
        ratatui::restore();
        Ok(())
    }

    fn process_event(&mut self, event: &AppEvent) {
        match event {
            AppEvent::Quit => self.should_quit = true,
            AppEvent::Toolbox(crate::tui::events::ToolboxEvent::ToolSelected)
                if self.editor.toolbox.selected != Tool::PolygonSelect =>
            {
                self.selection_polygon_points.clear();
            }
            AppEvent::Palette(crate::tui::events::PaletteEvent::ColorChanged(color, target)) => {
                self.editor.palette.selected_color = Some(*color);
                match target {
                    palette::ColorTarget::Foreground => {
                        self.editor.palette.target = palette::ColorTarget::Foreground;
                    }
                    palette::ColorTarget::Background => {
                        self.editor.palette.target = palette::ColorTarget::Background;
                    }
                }
            }
            AppEvent::ModeChanged => self.dirty = true,
            AppEvent::RenderModeChanged => self.dirty = true,
            AppEvent::SaveAsRequested => self.perform_save(),
            AppEvent::OpenRequested => self.perform_open(),
            AppEvent::ExportRequested(_) => self.perform_export(),
            AppEvent::Menu(action) => self.handle_menu_action(action.clone()),
            _ => {}
        }
    }

    pub fn render(&mut self, frame: &mut Frame<'_>) {
        self.check_async_completion();
        self.throbber.tick();

        // Welcome screen: full-screen overlay, dismisses on any constructive action
        if self.welcome_screen.show {
            let area = frame.area();
            self.welcome_screen.render(
                frame,
                area,
                self.dialogs.recent_files.list(),
                env!("CARGO_PKG_VERSION"),
                &self.theme,
            );
            self.render_overlays(frame);
            return;
        }

        // Single-pass layout computation — stored for mouse handlers next cycle.
        let fl = layout::FrameLayout::compute(frame.area(), self.zen_mode, self.right_drawer);

        // --- Zen mode: canvas only, hint overlay ---
        if self.zen_mode {
            self.render_canvas_area(frame, fl.canvas);
            // Hint bar at bottom-right corner
            let area = frame.area();
            if area.height > 0 && area.width > 30 {
                let hint = " F11=exit zen  ?=keys  ^K=keybinds ";
                let hint_w = hint.len() as u16;
                let hint_rect = Rect {
                    x: area.width.saturating_sub(hint_w),
                    y: area.height - 1,
                    width: hint_w.min(area.width),
                    height: 1,
                };
                let hint_para = Paragraph::new(hint).style(
                    Style::default()
                        .fg(self.theme.general.secondary)
                        .add_modifier(Modifier::DIM),
                );
                frame.render_widget(hint_para, hint_rect);
            }
            // Still render overlays in zen mode
            self.render_overlays(frame);
            return;
        }

        // --- Normal mode ---

        // Mode tabs
        let mode_labels = [
            ("mode_font_editor", "Font Editor"),
            ("mode_image_editor", "Image Editor"),
            ("mode_ascii_preview", "ASCII Preview"),
        ];
        let titles: Vec<String> = mode_labels
            .iter()
            .map(|(key, name)| {
                let icon = self.icons.get(*key).map(|s| s.as_str()).unwrap_or("");
                format!("{icon}  {name}")
            })
            .collect();
        let selected = match self.mode {
            AppMode::FontEditor => 0,
            AppMode::ImageEditor => 1,
            AppMode::AsciiPreview => 2,
        };
        let titles_refs: Vec<&str> = titles.iter().map(|s| s.as_str()).collect();
        let tabs = Tabs::new(titles_refs)
            .style(Style::default().fg(self.theme.general.secondary))
            .highlight_style(
                Style::default()
                    .fg(self.theme.general.primary)
                    .add_modifier(Modifier::BOLD),
            )
            .select(selected);
        frame.render_widget(tabs, fl.tabs);

        // Toolbox + brush/text options (left panel)
        if let Some(tb_full) = fl.toolbox_full {
            frame.render_widget(&self.editor.toolbox, tb_full);
            if let Some(tb_brush) = fl.toolbox_brush {
                if self.editor.toolbox.selected == Tool::Text {
                    self.editor.text_tool.render_options(frame, tb_brush);
                } else {
                    self.editor.brush.render(frame, tb_brush);
                }
            }
        }

        // Canvas / font editor area
        self.render_canvas_area(frame, fl.canvas);

        // Right drawer
        if let Some(rp) = fl.right_panel {
            match self.right_drawer {
                layout::DrawerMode::Palette => {
                    if self.dialogs.settings.settings_open {
                        frame.render_widget(&self.dialogs.settings, rp);
                    } else {
                        frame.render_widget(&self.editor.palette, rp);
                    }
                }
                layout::DrawerMode::BrushKeys => {
                    self.render_brush_keys_panel(frame, rp);
                }
                layout::DrawerMode::Layers => {
                    self.editor
                        .layer_panel
                        .render_with_stack(frame, rp, &self.editor.layer_stack);
                }
                layout::DrawerMode::Closed => {}
            }
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

        // Status bar (inline rendering)
        self.status_cursor = self.editor.canvas.cursor();
        self.status_zoom = self.editor.canvas.zoom_level();
        self.status_tool_name = self.editor.toolbox.selected.full_name().to_string();
        self.status_mode_name = self.mode_name_string();
        self.status_unsaved = self.editor.unsaved;
        self.status_current_path = self
            .editor
            .font_editor
            .current_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string());
        self.status_throbber_text = self.throbber.render_string();
        self.status_mode = self.mode;
        self.status_undo_count = self.editor.undo.history_len();
        self.status_fps = self.fps;
        self.status_render_mode = self.render_mode.label();
        self.status_git_branch = self.git_branch.clone();
        self.status_clock_str = format_clock();
        self.status_layer_count = self.editor.layer_stack.len() as u8;
        self.status_animation_frame = 0;
        self.render_status_bar(frame, fl.status);

        // Menu bar (rendered last so dropdown overlays main content)
        frame.render_stateful_widget(&self.menu_bar, fl.menu, &mut self.menu_bar_state);

        self.render_overlays(frame);
    }

    /// Render the canvas (or font editor overview) inside `canvas_area`.
    fn render_canvas_area(&mut self, frame: &mut Frame<'_>, canvas_area: Rect) {
        let fl = layout::FrameLayout::compute(frame.area(), self.zen_mode, self.right_drawer);

        let mode_title = match self.mode {
            AppMode::ImageEditor => {
                if self.editor.image_editor.entering_path() {
                    format!(
                        " Image Editor [Path: {}] ",
                        self.editor.image_editor.path_buffer()
                    )
                } else if let Some(err) = self.editor.image_editor.error_message() {
                    format!(" Image Editor [Error: {err}] ")
                } else if self.editor.image_editor.has_cells() {
                    format!(
                        " Image Editor {} ",
                        self.editor.image_editor.adjustment_status()
                    )
                } else {
                    self.mode.title().to_string()
                }
            }
            _ => self.mode.title().to_string(),
        };

        let canvas_borders = if self.zen_mode {
            Borders::NONE
        } else {
            fl.canvas_borders()
        };
        let block = Block::default().title(mode_title).borders(canvas_borders);
        let inner = block.inner(canvas_area);

        let is_font_ui_mode = self.mode == AppMode::FontEditor
            && !matches!(
                self.editor.font_editor.view,
                font_editor::FontEditorView::CharEditor(_)
            );

        if is_font_ui_mode {
            self.editor.font_editor.before_render(inner);
            frame.render_widget(block, canvas_area);
            frame.render_widget(&self.editor.font_editor, inner);
        } else {
            if self.mode == AppMode::FontEditor {
                self.editor.sync_canvas_to_font_char();
            }
            if self.mode == AppMode::ImageEditor {
                self.editor.sync_image_to_canvas();
            }

            // Selection perimeter
            if let Some(ref sel) = self.editor.selection {
                if sel.is_active() {
                    self.editor.canvas.selection_perimeter = Some(sel.perimeter());
                } else {
                    self.editor.canvas.selection_perimeter = None;
                }
            } else {
                self.editor.canvas.selection_perimeter = None;
            }
            self.editor
                .canvas
                .polygon_vertices
                .clone_from(&self.selection_polygon_points);

            // Text overlays
            if self.editor.toolbox.selected == Tool::Text {
                self.editor.canvas.text_overlays = self
                    .editor
                    .text_tool
                    .blocks
                    .iter()
                    .enumerate()
                    .filter_map(|(i, _)| self.editor.text_tool.render_block_to_overlay(i))
                    .collect();
                self.editor.canvas.text_block_perimeter =
                    self.editor.text_tool.selected_block.and_then(|idx| {
                        if idx < self.editor.text_tool.blocks.len() {
                            let (bx, by, bw, bh) = self.editor.text_tool.compute_bounding_box(idx);
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
                self.editor.canvas.text_overlays.clear();
                self.editor.canvas.text_block_perimeter = None;
            }

            frame.render_widget(block, canvas_area);

            let canvas_inner_rect = self.editor.compute_canvas_rect(inner);
            if canvas_inner_rect.width > 1 && canvas_inner_rect.height > 1 {
                let edge = Block::default().borders(Borders::ALL).style(
                    Style::default()
                        .fg(self.theme.canvas.edge)
                        .add_modifier(Modifier::DIM),
                );
                frame.render_widget(edge, canvas_inner_rect);
            }
            // Sync glyph cursor for CharEditor mode
            if self.mode == AppMode::FontEditor
                && matches!(
                    self.editor.font_editor.view,
                    font_editor::FontEditorView::CharEditor(_)
                )
            {
                let gc = &mut self.editor.canvas;
                if gc.glyph_cursor.is_none() {
                    gc.glyph_cursor = Some(canvas::GlyphCursor::new(
                        self.editor.font_editor.glyph_cursor_x,
                        self.editor.font_editor.glyph_cursor_y,
                    ));
                } else if let Some(ref mut g) = gc.glyph_cursor {
                    g.x = self.editor.font_editor.glyph_cursor_x;
                    g.y = self.editor.font_editor.glyph_cursor_y;
                }
                if let Some(ref mut g) = gc.glyph_cursor {
                    g.blink();
                }
            } else {
                self.editor.canvas.glyph_cursor = None;
            }

            frame.render_widget(&self.editor.canvas, canvas_inner_rect);
        }
    }

    /// Render the brush/tool keybind reference in the right drawer.
    fn render_brush_keys_panel(&self, frame: &mut Frame<'_>, area: Rect) {
        let block = Block::default()
            .title(" Brush Keys (? to cycle) ")
            .borders(Borders::ALL)
            .style(
                Style::default()
                    .bg(self.theme.menu.dropdown_bg)
                    .fg(self.theme.menu.fg),
            );
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines: Vec<Line> = vec![
            Line::from(Span::styled(
                " Tools ",
                Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )),
            Line::from("  b  Brush"),
            Line::from("  e  Eraser"),
            Line::from("  l  Lasso"),
            Line::from("  v  Select"),
            Line::from("  c  Circle sel."),
            Line::from("  p  Polygon sel."),
            Line::from("  g  Fill"),
            Line::from("  i  Line"),
            Line::from("  d  Eyedropper"),
            Line::from("  a  Spray"),
            Line::from("  t  Text"),
            Line::from(""),
            Line::from(Span::styled(
                " Brush ",
                Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )),
            Line::from("  [  Size down"),
            Line::from("  ]  Size up"),
            Line::from("  ;  Density down"),
            Line::from("  '  Density up"),
            Line::from(r"  \  Cycle shape"),
            Line::from(""),
            Line::from(Span::styled(
                " View ",
                Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )),
            Line::from("  F11  Zen mode"),
            Line::from("  ?    This panel"),
            Line::from("  ^K   All keybinds"),
        ];
        frame.render_widget(Paragraph::new(lines), inner);
    }

    /// Render the status bar from inline fields.
    fn render_status_bar(&self, frame: &mut Frame<'_>, area: Rect) {
        let block = ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(&block, area);

        if inner.width < 10 {
            return;
        }

        let pos_icon = self
            .icons
            .get("status_position")
            .map_or("+", |s| s.as_str());
        let zoom_icon = self.icons.get("status_zoom").map_or("Z", |s| s.as_str());
        let tool_icon = self.icons.get("status_tool").map_or("T", |s| s.as_str());
        let mode_icon = self.icons.get("status_mode").map_or("M", |s| s.as_str());
        let unsaved_icon = self.icons.get("status_unsaved").map_or("!", |s| s.as_str());
        let saved_icon = self.icons.get("status_saved").map_or("*", |s| s.as_str());

        let mode_color = match self.mode {
            AppMode::FontEditor => self.theme.statusbar.mode_font,
            AppMode::ImageEditor => self.theme.statusbar.mode_image,
            AppMode::AsciiPreview => self.theme.statusbar.mode_ascii,
        };

        let unsaved_dot = if self.status_unsaved {
            unsaved_icon
        } else {
            saved_icon
        };
        let filename = self
            .status_current_path
            .as_ref()
            .map(std::path::Path::new)
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|n| n.to_string())
            .unwrap_or_else(|| "Untitled".to_string());

        let undo_str = if self.status_undo_count > 0 {
            format!(" undo:{}", self.status_undo_count)
        } else {
            String::new()
        };

        let fps_str = format!("FPS:{:.0}", self.status_fps);
        let render_str = if self.status_render_mode.is_empty() {
            String::new()
        } else {
            format!(" Rnd:{}", self.status_render_mode)
        };
        let branch_str = match &self.status_git_branch {
            Some(b) => format!(" ⎇ {}", b),
            None => String::new(),
        };
        let throbber_str = if self.status_throbber_text.is_empty() {
            String::new()
        } else {
            format!(" {} ", self.status_throbber_text)
        };

        let mode_label = format!(" {} {} ", mode_icon, self.status_mode_name);
        let cursor_str = format!(
            " {} X:{} Y:{} ",
            pos_icon, self.status_cursor.0, self.status_cursor.1
        );
        let zoom_label = format!(" {} {}x", zoom_icon, self.status_zoom);
        let tool_label = format!(" {} {} ", tool_icon, self.status_tool_name);
        let center_str = format!(" {} {}{}", unsaved_dot, filename, undo_str);
        let right_str = format!(
            "{}{} │ L:{} │ F:{} │ {}{}{}",
            fps_str,
            render_str,
            self.status_layer_count,
            self.status_animation_frame,
            self.status_clock_str,
            throbber_str,
            branch_str,
        );

        let left_w = mode_label.chars().count()
            + tool_label.chars().count()
            + cursor_str.chars().count()
            + zoom_label.chars().count();
        let right_w = right_str.chars().count();
        let gap = (inner.width as usize).saturating_sub(left_w + right_w + 6);
        let center_trunc: String = center_str.chars().take(gap).collect();

        let mut spans: Vec<Span> = Vec::new();
        spans.push(Span::styled(
            mode_label,
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(format!(
            "{}{}{}",
            tool_label, cursor_str, zoom_label
        )));
        spans.push(Span::raw(" │ "));
        spans.push(Span::raw(center_trunc));
        spans.push(Span::raw(" │ "));
        spans.push(Span::raw(right_str));

        let paragraph = Paragraph::new(Line::from(spans));
        frame.render_widget(paragraph, inner);
    }

    /// Render all floating overlays (dialogs, keybindings, undo panel).
    fn render_overlays(&mut self, frame: &mut Frame<'_>) {
        // Export dialog overlay
        if self.dialogs.export_dialog.active {
            let overlay = centered_overlay(frame.area());
            frame.render_widget(Clear, overlay);
            self.dialogs.export_dialog.render(frame, overlay);
        }

        // File ops overlay
        if self.dialogs.file_ops.mode != file_ops::FileOpsMode::Idle {
            let overlay = centered_overlay(frame.area());
            frame.render_widget(Clear, overlay);
            self.dialogs.file_ops.render(frame, overlay);
        }

        // Keybindings overlay
        if self.show_keybindings {
            let area = frame.area();
            let overlay = Rect {
                x: area.width / 8,
                y: area.height / 8,
                width: area.width * 3 / 4,
                height: area.height * 3 / 4,
            };
            frame.render_widget(Clear, overlay);
            let block = Block::default()
                .title(" Keybindings (Esc to close) ")
                .borders(Borders::ALL)
                .style(
                    Style::default()
                        .bg(self.theme.menu.dropdown_bg)
                        .fg(self.theme.menu.fg),
                );
            let inner = block.inner(overlay);
            frame.render_widget(block, overlay);

            let mut lines: Vec<Line> = Vec::new();
            let mut last_scope: Option<keymap::Scope> = None;
            for binding in keymap::KEYMAP {
                if last_scope != Some(binding.scope) {
                    if last_scope.is_some() {
                        lines.push(Line::from(""));
                    }
                    lines.push(Line::from(Span::styled(
                        format!(" {}", binding.scope.label()),
                        Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    )));
                    last_scope = Some(binding.scope);
                }
                lines.push(Line::from(format!(
                    "  {:<22} {}",
                    binding.keys, binding.description
                )));
            }
            frame.render_widget(Paragraph::new(lines), inner);
        }

        // Undo history panel overlay
        if self.dialogs.undo_panel.open {
            frame.render_widget(Clear, frame.area());
            self.dialogs
                .undo_panel
                .render(frame, frame.area(), self.editor.undo.history_entries());
        }
    }

    /// Build the mode name string for the status bar.
    fn mode_name_string(&self) -> String {
        match self.mode {
            AppMode::ImageEditor => {
                if self.editor.image_editor.has_cells() {
                    format!(
                        "Image Editor {}",
                        self.editor.image_editor.adjustment_status()
                    )
                } else {
                    "Image Editor".to_string()
                }
            }
            AppMode::AsciiPreview => "ASCII Preview".to_string(),
            AppMode::FontEditor => {
                if let font_editor::FontEditorView::CharEditor(code) = self.editor.font_editor.view
                {
                    format!("Font Editor [U+{code:04X}]")
                } else if self.editor.font_editor.view == font_editor::FontEditorView::HeaderEditor
                {
                    "Font Editor - Header".to_string()
                } else if self.editor.font_editor.view
                    == font_editor::FontEditorView::SmushRuleEditor
                {
                    "Font Editor - Smushing Rules".to_string()
                } else if self.editor.font_editor.view
                    == font_editor::FontEditorView::TransformEditor
                {
                    "Font Editor - Transforms".to_string()
                } else {
                    "Font Editor".to_string()
                }
            }
        }
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        // Menu bar mouse event
        if self.menu_bar.handle_mouse_event(
            mouse.column,
            mouse.row,
            mouse.kind,
            &mut self.menu_bar_state,
        ) {
            if let Some(action) = self.menu_bar_state.drain_actions() {
                self.process_event(&AppEvent::Menu(action));
            }
            return;
        }

        if self.dialogs.settings.settings_open {
            return;
        }

        if self.dialogs.file_ops.mode != file_ops::FileOpsMode::Idle {
            return;
        }

        if self.dialogs.export_dialog.active {
            return;
        }

        // Font editor overview: glyph grid mouse click + scroll
        if self.mode == AppMode::FontEditor
            && self.editor.font_editor.view == font_editor::FontEditorView::Overview
        {
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left)
                    if self
                        .editor
                        .font_editor
                        .handle_mouse_click_overview(mouse.column, mouse.row) =>
                {
                    self.dirty = true;
                    return;
                }
                MouseEventKind::ScrollDown => {
                    self.editor.font_editor.handle_mouse_scroll_overview(1);
                    self.dirty = true;
                    return;
                }
                MouseEventKind::ScrollUp => {
                    self.editor.font_editor.handle_mouse_scroll_overview(-1);
                    self.dirty = true;
                    return;
                }
                _ => {}
            }
        }

        // Toolbox click: select tool by row
        let mouse_fl = {
            let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
            layout::FrameLayout::compute(
                Rect::new(0, 0, cols, rows),
                self.zen_mode,
                self.right_drawer,
            )
        };
        let canvas_inner_rect = self.editor.compute_canvas_rect(
            ratatui::widgets::Block::default()
                .borders(mouse_fl.canvas_borders())
                .inner(mouse_fl.canvas),
        );
        if let Some(tb) = mouse_fl.toolbox_list {
            let tool_count = Tool::all().len() as u16;
            let toolbox_inner_y = tb.y + 1;
            if mouse.kind == MouseEventKind::Down(MouseButton::Left)
                && mouse.column >= tb.x
                && mouse.column < tb.x + tb.width
                && mouse.row >= toolbox_inner_y
                && mouse.row < toolbox_inner_y + tool_count
            {
                let idx = (mouse.row - toolbox_inner_y) as usize;
                let tools = Tool::all();
                if idx < tools.len() {
                    self.editor.toolbox.selected = tools[idx];
                    self.selection_polygon_points.clear();
                }
                return;
            }
        }

        // Text tool: hit-test blocks or enter text mode
        if self.editor.toolbox.selected == Tool::Text {
            if let MouseEventKind::Down(_) = mouse.kind {
                if let Some((bx, by)) =
                    self.editor
                        .screen_to_buffer(mouse.column, mouse.row, canvas_inner_rect)
                {
                    if !self.editor.text_tool.entering_text {
                        if let Some(idx) = self.editor.text_tool.hit_test(bx, by) {
                            self.editor.text_tool.selected_block = Some(idx);
                            self.prev_mouse_buf = None;
                            self.line_start = None;
                            self.saved_buffer = None;
                            return;
                        }
                        self.editor.text_tool.cursor_position = (bx, by);
                        self.editor.text_tool.entering_text = true;
                        self.editor.text_tool.text_buffer.clear();
                        self.editor
                            .canvas
                            .set_cursor(bx.max(0) as u16, by.max(0) as u16);
                    } else {
                        self.editor.text_tool.cursor_position = (bx, by);
                        self.editor
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
            self.editor.toolbox.selected,
            Tool::Marquee | Tool::Lasso | Tool::CircleSelect | Tool::PolygonSelect
        );

        if !is_selection_tool
            && !matches!(
                self.editor.toolbox.selected,
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
                let Some((bx, by)) =
                    self.editor
                        .screen_to_buffer(mouse.column, mouse.row, canvas_inner_rect)
                else {
                    self.prev_mouse_buf = None;
                    self.line_start = None;
                    return;
                };
                self.editor
                    .canvas
                    .set_cursor(bx.max(0) as u16, by.max(0) as u16);
                self.editor.unsaved = true;

                if is_selection_tool {
                    self.editor.handle_selection_down(
                        bx,
                        by,
                        &mut self.selection_drag_origin,
                        &mut self.selection_polygon_points,
                        &mut self.selection_lasso_points,
                    );
                    return;
                }

                // Start batch for drag operations, push initial snapshot
                self.editor.undo.begin_batch();
                if self.editor.toolbox.selected == Tool::Fill {
                    self.editor.push_undo_snapshot("Flood fill");
                    let mut cell = canvas::CanvasCell {
                        ch: self.editor.brush.ch,
                        fg: None,
                        bg: None,
                    };
                    self.editor.palette.apply_to_cell(&mut cell);
                    let mut buf = self.editor.layer_stack.active_layer().buffer.clone();
                    tools::fill::flood_fill(&mut buf, bx, by, cell);
                    *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                    self.editor.recomposite_canvas();
                    return;
                }
                if self.editor.toolbox.selected == Tool::Line {
                    self.editor.push_undo_snapshot("Line tool");
                    self.line_start = Some((bx, by));
                    self.saved_buffer = Some(self.editor.layer_stack.active_layer().buffer.clone());
                    return;
                }
                if self.editor.toolbox.selected == Tool::Eraser {
                    self.editor.push_undo_snapshot("Eraser");
                    let shape = self.editor.brush.shape;
                    let size = self.editor.brush.size;
                    let mut buf = self.editor.layer_stack.active_layer().buffer.clone();
                    tools::eraser::erase_stamp(&mut buf, bx, by, shape, size);
                    *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                    self.editor.recomposite_canvas();
                } else if self.editor.toolbox.selected == Tool::Eyedropper {
                    if let Some(cell) =
                        tools::eyedropper::sample(&self.editor.canvas.buffer, bx, by)
                    {
                        self.editor.brush.ch = cell.ch;
                        if let Some(fg) = cell.fg {
                            self.editor.palette.selected_color = Some(fg);
                            self.editor.palette.push_recent(fg);
                            self.editor.palette.target = palette::ColorTarget::Foreground;
                        }
                    }
                } else if self.editor.toolbox.selected == Tool::Spray {
                    self.editor.push_undo_snapshot("Spray");
                    let mut cell = canvas::CanvasCell {
                        ch: self.editor.brush.ch,
                        fg: None,
                        bg: None,
                    };
                    self.editor.palette.apply_to_cell(&mut cell);
                    let mut rng = StdRng::seed_from_u64(rand::thread_rng().gen());
                    let size = self.editor.brush.size;
                    let density = self.editor.brush.density;
                    let mut buf = self.editor.layer_stack.active_layer().buffer.clone();
                    tools::spray::spray_stamp(&mut buf, bx, by, size, density, cell, &mut rng);
                    *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                    self.editor.recomposite_canvas();
                } else {
                    self.editor.push_undo_snapshot("Brush");
                    let mut cell = canvas::CanvasCell {
                        ch: self.editor.brush.ch,
                        fg: None,
                        bg: None,
                    };
                    self.editor.palette.apply_to_cell(&mut cell);
                    let shape = self.editor.brush.shape;
                    let size = self.editor.brush.size;
                    let mut buf = self.editor.layer_stack.active_layer().buffer.clone();
                    tools::brush::paint_stamp(&mut buf, bx, by, shape, size, cell);
                    *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                    self.editor.recomposite_canvas();
                }
                self.prev_mouse_buf = Some((bx, by));
            }
            MouseEventKind::Drag(_) => {
                let Some((bx, by)) =
                    self.editor
                        .screen_to_buffer(mouse.column, mouse.row, canvas_inner_rect)
                else {
                    return;
                };
                self.editor
                    .canvas
                    .set_cursor(bx.max(0) as u16, by.max(0) as u16);
                self.editor.unsaved = true;

                if is_selection_tool {
                    self.editor.handle_selection_drag(
                        bx,
                        by,
                        &mut self.selection_drag_origin,
                        &mut self.selection_lasso_points,
                    );
                    return;
                }

                if self.editor.toolbox.selected == Tool::Line {
                    if let (Some((sx, sy)), Some(ref saved)) =
                        (self.line_start, self.saved_buffer.clone())
                    {
                        let saved_clone = saved.clone();
                        let mut cell = canvas::CanvasCell {
                            ch: self.editor.brush.ch,
                            fg: None,
                            bg: None,
                        };
                        self.editor.palette.apply_to_cell(&mut cell);
                        let shape = self.editor.brush.shape;
                        let size = self.editor.brush.size;
                        let mut buf = saved_clone;
                        tools::line::draw_line_segment(&mut buf, sx, sy, bx, by, shape, size, cell);
                        *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                        self.editor.recomposite_canvas();
                    }
                    return;
                }
                if let Some((px, py)) = self.prev_mouse_buf {
                    if self.editor.toolbox.selected == Tool::Eraser {
                        let shape = self.editor.brush.shape;
                        let size = self.editor.brush.size;
                        let mut buf = self.editor.layer_stack.active_layer().buffer.clone();
                        tools::eraser::erase_line(&mut buf, px, py, bx, by, shape, size);
                        *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                        self.editor.recomposite_canvas();
                    } else if self.editor.toolbox.selected == Tool::Spray {
                        let mut cell = canvas::CanvasCell {
                            ch: self.editor.brush.ch,
                            fg: None,
                            bg: None,
                        };
                        self.editor.palette.apply_to_cell(&mut cell);
                        let mut rng = StdRng::seed_from_u64(rand::thread_rng().gen());
                        let size = self.editor.brush.size;
                        let density = self.editor.brush.density;
                        let mut buf = self.editor.layer_stack.active_layer().buffer.clone();
                        tools::spray::spray_line(
                            &mut buf, px, py, bx, by, size, density, cell, &mut rng,
                        );
                        *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                        self.editor.recomposite_canvas();
                    } else {
                        let mut cell = canvas::CanvasCell {
                            ch: self.editor.brush.ch,
                            fg: None,
                            bg: None,
                        };
                        self.editor.palette.apply_to_cell(&mut cell);
                        let shape = self.editor.brush.shape;
                        let size = self.editor.brush.size;
                        let mut buf = self.editor.layer_stack.active_layer().buffer.clone();
                        tools::brush::paint_line(&mut buf, px, py, bx, by, shape, size, cell);
                        *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                        self.editor.recomposite_canvas();
                    }
                }
                self.prev_mouse_buf = Some((bx, by));
            }
            MouseEventKind::Up(_) => {
                self.editor.undo.end_batch();
                if is_selection_tool {
                    self.editor.handle_selection_up(
                        &mut self.selection_drag_origin,
                        &mut self.selection_lasso_points,
                    );
                }
                self.prev_mouse_buf = None;
                self.line_start = None;
                self.saved_buffer = None;
            }
            MouseEventKind::Moved => {
                if let Some((bx, by)) =
                    self.editor
                        .screen_to_buffer(mouse.column, mouse.row, canvas_inner_rect)
                {
                    self.editor
                        .canvas
                        .set_cursor(bx.max(0) as u16, by.max(0) as u16);
                }
            }
            _ => {}
        }
    }

    fn check_async_completion(&mut self) {
        let rx = match self.async_rx.take() {
            Some(rx) => rx,
            None => return,
        };
        match rx.try_recv() {
            Ok(result) => {
                self.throbber.stop();
                self.dirty = true;
                match result {
                    AsyncResult::SaveComplete(r) => match r {
                        Ok(path) => {
                            self.editor.unsaved = false;
                            self.editor.font_editor.current_path = Some(path);
                            self.last_save_time = Instant::now();
                            self.dialogs.file_ops.error_message.clear();
                        }
                        Err(e) => {
                            self.dialogs.file_ops.error_message = format!("Save failed: {e}");
                        }
                    },
                    AsyncResult::OpenComplete(r) => match r {
                        Ok((font, path)) => {
                            self.editor.unsaved = false;
                            self.editor.undo.clear();
                            self.editor.font_editor.load_font(font);
                            self.editor.font_editor.current_path = Some(path.clone());
                            self.dialogs.recent_files.push(path);
                            self.dialogs.recent_files.save_to_disk();
                            self.dialogs.file_ops.error_message.clear();
                        }
                        Err(e) => {
                            self.dialogs.file_ops.error_message = e;
                            self.dialogs.file_ops.mode = file_ops::FileOpsMode::Open;
                        }
                    },
                    AsyncResult::ExportComplete(r) => match r {
                        Ok(()) => {
                            self.dialogs.export_dialog.active = false;
                        }
                        Err(e) => {
                            self.dialogs.export_dialog.error_message = e;
                            self.dialogs.export_dialog.active = true;
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
                self.dirty = true;
            }
        }
    }

    pub fn handle_event(&mut self) -> io::Result<()> {
        if event::poll(Duration::from_millis(self.render_mode.poll_ms()))? {
            self.dirty = true;
            loop {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        let event = self.handle_key_event(key);
                        if let Some(ref e) = event {
                            self.process_event(e);
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

        self.check_async_completion();

        // Auto-save check
        if self.auto_save_interval > 0
            && self.editor.unsaved
            && self.mode == AppMode::FontEditor
            && !self.throbber.is_active()
        {
            if let Some(ref path) = self.editor.font_editor.current_path {
                if self.last_save_time.elapsed() >= Duration::from_secs(self.auto_save_interval) {
                    if let Some(ref font) = self.editor.font_editor.font {
                        self.last_save_time = Instant::now();
                        let font = font.clone();
                        let path = path.clone();
                        let (tx, rx) = mpsc::channel();
                        self.async_rx = Some(rx);
                        self.throbber.start("Auto-saving...");
                        self.dirty = true;
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

    pub fn handle_key_event(&mut self, key: impl Into<KeyEvent>) -> Option<AppEvent> {
        let key = key.into();
        let code = key.code;
        let modifiers = key.modifiers;

        // Keybindings overlay: Esc closes it, swallow all other keys
        if self.show_keybindings {
            if code == KeyCode::Esc {
                self.show_keybindings = false;
                self.dirty = true;
            }
            return None;
        }

        // File ops dialog active: dispatch all keys to it
        if self.dialogs.file_ops.mode != file_ops::FileOpsMode::Idle {
            let prev_mode = self.dialogs.file_ops.mode;
            self.dialogs.file_ops.handle_key(code);
            if self.dialogs.file_ops.mode == file_ops::FileOpsMode::Idle {
                return match prev_mode {
                    file_ops::FileOpsMode::SaveAs => {
                        self.perform_save();
                        return Some(AppEvent::SaveAsRequested);
                    }
                    file_ops::FileOpsMode::Open => {
                        if self.dialogs.file_ops.is_browsing_zip() {
                            self.perform_open();
                            return Some(AppEvent::OpenRequested);
                        }
                        if self.dialogs.file_ops.path_buffer.trim().is_empty() {
                            return None;
                        }
                        let path = self.dialogs.file_ops.selected_path();
                        if !path.exists() {
                            self.dialogs.file_ops.error_message =
                                format!("File not found: {}", path.display());
                            self.dialogs.file_ops.mode = file_ops::FileOpsMode::Open;
                            return None;
                        }
                        if path.is_dir() {
                            self.dialogs.file_ops.error_message =
                                "Select a .flf or .tlf file, not a directory".to_string();
                            self.dialogs.file_ops.mode = file_ops::FileOpsMode::Open;
                            return None;
                        }
                        self.perform_open();
                        return Some(AppEvent::OpenRequested);
                    }
                    file_ops::FileOpsMode::Idle => None,
                };
            }
            return None;
        }

        // Export dialog active: dispatch all keys to it
        if self.dialogs.export_dialog.active {
            self.dialogs.export_dialog.handle_key(code);
            if !self.dialogs.export_dialog.active {
                self.perform_export();
            }
            return None;
        }

        // Undo history panel open: dispatch to it first
        if self.dialogs.undo_panel.open {
            self.dialogs.undo_panel.handle_key(code);
            return None;
        }

        // Menu bar active: dispatch all keys to it
        if self.menu_bar_state.is_active() {
            self.menu_bar
                .handle_key_event(key, &mut self.menu_bar_state);
            if let Some(action) = self.menu_bar_state.drain_actions() {
                return Some(AppEvent::Menu(action));
            }
            return None;
        }

        // Alt+key: open menu bar
        if modifiers == KeyModifiers::ALT
            && self
                .menu_bar
                .handle_key_event(key, &mut self.menu_bar_state)
        {
            return None;
        }

        // Global key dispatch (early global actions before modal/mode checks)
        if let Some(action) = keymap::lookup_global(code, modifiers) {
            match action {
                keymap::GlobalAction::Undo
                | keymap::GlobalAction::Redo
                | keymap::GlobalAction::ToggleUndoPanel => {
                    return self.dispatch_global(action);
                }
                _ => {}
            }
        }

        if self.dialogs.settings.settings_open {
            if self.dialogs.settings.handle_key(code) {
                self.apply_settings();
                return None;
            }
            if let KeyCode::Char('S') = code {
                self.dialogs.settings.settings_open = false;
            }
            return None;
        }

        // Layer panel: dispatch keys when drawer shows layers
        if self.right_drawer == layout::DrawerMode::Layers
            && self
                .editor
                .layer_panel
                .handle_key(code, &mut self.editor.layer_stack)
        {
            self.editor.recomposite_canvas();
            self.editor.unsaved = true;
            self.dirty = true;
            return None;
        }

        // Welcome screen: intercept before mode-specific dispatch
        if self.welcome_screen.show {
            let recent_count = self.dialogs.recent_files.len();
            if let Some(action) = self
                .welcome_screen
                .handle_key(code, modifiers, recent_count)
            {
                use welcome::WelcomeAction;
                match action {
                    WelcomeAction::Dismiss => {
                        self.welcome_screen.show = false;
                        self.dirty = true;
                    }
                    WelcomeAction::OpenRecent(idx) => {
                        if let Some(path) = self.dialogs.recent_files.get(idx) {
                            self.dialogs.file_ops.path_buffer = path.to_string_lossy().to_string();
                            self.perform_open();
                            self.welcome_screen.show = false;
                            self.dirty = true;
                        }
                    }
                    WelcomeAction::Open => {
                        self.start_open();
                        self.welcome_screen.show = false;
                        self.dirty = true;
                    }
                    WelcomeAction::NewFile => {
                        self.editor.font_editor.font = None;
                        self.editor.font_editor.current_path = None;
                        self.editor.undo.clear();
                        self.editor.canvas = crate::tui::canvas::CanvasWidget::new(32, 16);
                        self.editor.layer_stack = layers::LayerStack::new(32, 16);
                        self.editor.layer_panel = layers::LayerPanel::new();
                        self.editor.layer_panel.theme = self.theme.clone();
                        self.editor.recomposite_canvas();
                        self.welcome_screen.show = false;
                        self.dirty = true;
                    }
                    WelcomeAction::ToggleHelp => {
                        self.show_keybindings = !self.show_keybindings;
                        self.dirty = true;
                    }
                    WelcomeAction::OpenSettings => {
                        self.dialogs.settings.canvas_width =
                            self.editor.canvas.buffer.width() as u16;
                        self.dialogs.settings.canvas_height =
                            self.editor.canvas.buffer.height() as u16;
                        self.dialogs.settings.show_grid = self.editor.canvas.show_grid();
                        self.dialogs.settings.settings_open = true;
                        self.welcome_screen.show = false;
                        self.dirty = true;
                    }
                }
                return None;
            }
        }

        // Font Editor mode: dispatch to font_editor before canvas/tools
        if self.mode == AppMode::FontEditor {
            // Sync brush char for CharEditor cell toggle
            if matches!(
                self.editor.font_editor.view,
                font_editor::FontEditorView::CharEditor(_)
            ) {
                self.editor.font_editor.brush_char = self.editor.brush.ch;
            }
            let area_width = crossterm::terminal::size().unwrap_or((80, 24)).0;
            if self
                .editor
                .font_editor
                .handle_key(key.code, key.modifiers, area_width)
            {
                if self.editor.font_editor.view != font_editor::FontEditorView::Overview {
                    self.editor.sync_font_char_to_canvas();
                }
                return Some(AppEvent::FontEditor(
                    crate::tui::events::FontEditorEvent::Changed(self.editor.font_editor.view),
                ));
            }
        }

        // Image Editor mode: dispatch to image_editor before canvas/tools
        if self.mode == AppMode::ImageEditor {
            let was_entering = self.editor.image_editor.entering_path();
            if self.editor.image_editor.handle_key(code) {
                self.editor.sync_image_to_canvas();
                if was_entering && !self.editor.image_editor.entering_path() {
                    self.editor.undo.clear();
                }
                return Some(AppEvent::ImageEditor);
            }
        }

        // Text tool: text entry mode (before canvas, captures all keys)
        if self.editor.toolbox.selected == Tool::Text && self.editor.text_tool.entering_text {
            match code {
                KeyCode::Enter => {
                    self.editor.push_undo_snapshot("Commit text");
                    self.editor.text_tool.commit_block();
                    self.editor.unsaved = true;
                    return Some(AppEvent::TextCommitted);
                }
                KeyCode::Esc => {
                    self.editor.text_tool.text_buffer.clear();
                    self.editor.text_tool.entering_text = false;
                    return None;
                }
                KeyCode::Backspace => {
                    self.editor.text_tool.text_buffer.pop();
                    return None;
                }
                KeyCode::Char(c) => {
                    self.editor.text_tool.text_buffer.push(c);
                    return None;
                }
                _ => {}
            }
        }

        // Text tool: font navigation
        if self.editor.toolbox.selected == Tool::Text
            && !self.editor.text_tool.entering_text
            && self.editor.text_tool.selected_block.is_none()
        {
            match code {
                KeyCode::Up => {
                    if !self.editor.text_tool.available_fonts.is_empty() {
                        self.editor.text_tool.font_index =
                            self.editor.text_tool.font_index.saturating_sub(1);
                        self.editor.text_tool.load_selected_font();
                    }
                    return None;
                }
                KeyCode::Down => {
                    if !self.editor.text_tool.available_fonts.is_empty() {
                        self.editor.text_tool.font_index = (self.editor.text_tool.font_index + 1)
                            .min(self.editor.text_tool.available_fonts.len() - 1);
                        self.editor.text_tool.load_selected_font();
                    }
                    return None;
                }
                _ => {}
            }
        }

        // Text tool: block operations
        if self.editor.toolbox.selected == Tool::Text
            && !self.editor.text_tool.entering_text
            && self.editor.text_tool.selected_block.is_some()
        {
            match code {
                KeyCode::Up => {
                    self.editor.push_undo_snapshot("Move text block");
                    self.editor.text_tool.move_selected_block(0, -1);
                    self.editor.unsaved = true;
                    return None;
                }
                KeyCode::Down => {
                    self.editor.push_undo_snapshot("Move text block");
                    self.editor.text_tool.move_selected_block(0, 1);
                    self.editor.unsaved = true;
                    return None;
                }
                KeyCode::Left => {
                    self.editor.push_undo_snapshot("Move text block");
                    self.editor.text_tool.move_selected_block(-1, 0);
                    self.editor.unsaved = true;
                    return None;
                }
                KeyCode::Right => {
                    self.editor.push_undo_snapshot("Move text block");
                    self.editor.text_tool.move_selected_block(1, 0);
                    self.editor.unsaved = true;
                    return None;
                }
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    self.editor.push_undo_snapshot("Scale text block");
                    self.editor.text_tool.scale_selected_block(1);
                    self.editor.unsaved = true;
                    return None;
                }
                KeyCode::Char('-') | KeyCode::Char('_') => {
                    self.editor.push_undo_snapshot("Scale text block");
                    self.editor.text_tool.scale_selected_block(-1);
                    self.editor.unsaved = true;
                    return None;
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.editor.push_undo_snapshot("Rotate text block");
                    self.editor.text_tool.rotate_selected_block();
                    self.editor.unsaved = true;
                    return None;
                }
                KeyCode::Delete | KeyCode::Backspace => {
                    self.editor.push_undo_snapshot("Delete text block");
                    self.editor.text_tool.delete_selected_block();
                    self.editor.unsaved = true;
                    return None;
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    if let Some(idx) = self.editor.text_tool.selected_block {
                        self.editor.text_tool.re_edit_block(idx);
                    }
                    return None;
                }
                KeyCode::Esc => {
                    self.editor.text_tool.selected_block = None;
                    return None;
                }
                _ => {}
            }
        }

        // Selection operations (before canvas cursor movement)
        let selection_active = self
            .editor
            .selection
            .as_ref()
            .is_some_and(|s| s.is_active());

        if selection_active {
            match code {
                KeyCode::Up => {
                    self.editor.push_undo_snapshot("Move selection");
                    self.editor.move_selection(0, -1);
                    self.editor.unsaved = true;
                    return None;
                }
                KeyCode::Down => {
                    self.editor.push_undo_snapshot("Move selection");
                    self.editor.move_selection(0, 1);
                    self.editor.unsaved = true;
                    return None;
                }
                KeyCode::Left => {
                    self.editor.push_undo_snapshot("Move selection");
                    self.editor.move_selection(-1, 0);
                    self.editor.unsaved = true;
                    return None;
                }
                KeyCode::Right => {
                    self.editor.push_undo_snapshot("Move selection");
                    self.editor.move_selection(1, 0);
                    self.editor.unsaved = true;
                    return None;
                }
                KeyCode::Delete | KeyCode::Backspace => {
                    self.editor.push_undo_snapshot("Delete selection");
                    if let Some(sel) = self.editor.selection.take() {
                        let mut buf = self.editor.layer_stack.active_layer().buffer.clone();
                        sel.delete_from(&mut buf);
                        *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                        self.editor.recomposite_canvas();
                        self.editor.unsaved = true;
                    }
                    return None;
                }
                _ => {}
            }

            if modifiers.contains(KeyModifiers::CONTROL) {
                match code {
                    KeyCode::Char('c') => {
                        if let Some(ref sel) = self.editor.selection {
                            self.editor.clipboard = Some(sel.copy_from(&self.editor.canvas.buffer));
                        }
                        return None;
                    }
                    KeyCode::Char('x') => {
                        self.editor.push_undo_snapshot("Cut selection");
                        if let Some(sel) = self.editor.selection.take() {
                            let mut buf = self.editor.layer_stack.active_layer().buffer.clone();
                            self.editor.clipboard = Some(sel.cut_from(&mut buf));
                            *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                            self.editor.recomposite_canvas();
                            self.editor.unsaved = true;
                        }
                        return None;
                    }
                    KeyCode::Char('v') => {
                        self.editor.push_undo_snapshot("Paste");
                        if let Some(ref clip) = self.editor.clipboard {
                            let (cx, cy) = self.editor.canvas.cursor();
                            let mut buf = self.editor.layer_stack.active_layer().buffer.clone();
                            tools::selection::Selection::paste_into(
                                &mut buf, clip, cx as i16, cy as i16,
                            );
                            *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                            self.editor.recomposite_canvas();
                            self.editor.unsaved = true;
                        }
                        return None;
                    }
                    _ => {}
                }
            }
        }

        // Polygon select tool: Enter closes polygon, Esc cancels
        if self.editor.toolbox.selected == Tool::PolygonSelect
            && !self.selection_polygon_points.is_empty()
        {
            match code {
                KeyCode::Enter => {
                    let points = std::mem::take(&mut self.selection_polygon_points);
                    if points.len() >= 3 {
                        self.editor.selection = Some(tools::selection::Selection::polygon(
                            &self.editor.canvas.buffer,
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
        if self.editor.selection.is_some() && code == KeyCode::Esc {
            self.editor.selection = None;
            return None;
        }

        // Canvas cursor movement, zoom, grid
        {
            let ck = key.code;
            if matches!(
                ck,
                KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::Char('+')
                    | KeyCode::Char('=')
                    | KeyCode::Char('-')
                    | KeyCode::Char('_')
                    | KeyCode::Char('G')
            ) && self.editor.canvas.handle_key(ck, 0, 0)
            {
                return Some(AppEvent::Canvas(crate::tui::events::CanvasEvent::Modified));
            }
        }

        // Text tool settings (not entering text)
        if self.editor.toolbox.selected == Tool::Text && !self.editor.text_tool.entering_text {
            match code {
                KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.editor.text_tool.justification = match self.editor.text_tool.justification
                    {
                        crate::render::Justification::Left => crate::render::Justification::Center,
                        crate::render::Justification::Center => crate::render::Justification::Right,
                        crate::render::Justification::Right => crate::render::Justification::Left,
                    };
                    return None;
                }
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    if self.editor.text_tool.scale < 4 {
                        self.editor.text_tool.scale += 1;
                    }
                    return None;
                }
                KeyCode::Char('-') | KeyCode::Char('_') => {
                    if self.editor.text_tool.scale > 1 {
                        self.editor.text_tool.scale -= 1;
                    }
                    return None;
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    let (cx, cy) = self.editor.canvas.cursor();
                    self.editor.text_tool.cursor_position = (cx as i16, cy as i16);
                    self.editor.text_tool.entering_text = true;
                    self.editor.text_tool.text_buffer.clear();
                    return None;
                }
                _ => {}
            }
        }

        // Settings toggle
        if code == KeyCode::Char('S') && !modifiers.contains(KeyModifiers::CONTROL) {
            self.dialogs.settings.canvas_width = self.editor.canvas.buffer.width() as u16;
            self.dialogs.settings.canvas_height = self.editor.canvas.buffer.height() as u16;
            self.dialogs.settings.show_grid = self.editor.canvas.show_grid();
            self.dialogs.settings.settings_open = true;
            self.dirty = true;
            return None;
        }

        // Toolbox tool selection + brush adjustments (inline from old ToolboxComponent)
        {
            use crate::tui::events::ToolboxEvent;
            let handled = match code {
                KeyCode::Char('[') => {
                    self.editor.brush.size_down();
                    Some(AppEvent::Toolbox(ToolboxEvent::BrushChanged))
                }
                KeyCode::Char(']') => {
                    self.editor.brush.size_up();
                    Some(AppEvent::Toolbox(ToolboxEvent::BrushChanged))
                }
                KeyCode::Char(';') => {
                    self.editor.brush.density_down();
                    Some(AppEvent::Toolbox(ToolboxEvent::BrushChanged))
                }
                KeyCode::Char('\'') => {
                    self.editor.brush.density_up();
                    Some(AppEvent::Toolbox(ToolboxEvent::BrushChanged))
                }
                KeyCode::Char('\\') => {
                    self.editor.brush.cycle_shape();
                    Some(AppEvent::Toolbox(ToolboxEvent::BrushChanged))
                }
                KeyCode::Char(c) if !modifiers.contains(KeyModifiers::CONTROL) => {
                    let lower = c.to_ascii_lowercase();
                    let mut found = None;
                    for tool in Tool::all() {
                        if let KeyCode::Char(tc) = tool.key_shortcut() {
                            if tc == lower {
                                self.editor.toolbox.selected = *tool;
                                found = Some(AppEvent::Toolbox(ToolboxEvent::ToolSelected));
                                break;
                            }
                        }
                    }
                    found
                }
                _ => None,
            };
            if let Some(action) = handled {
                if self.editor.toolbox.selected != Tool::PolygonSelect {
                    self.selection_polygon_points.clear();
                }
                return Some(action);
            }
        }

        // Palette color selection (inline from old PaletteComponent)
        {
            use crate::tui::events::PaletteEvent;
            let handled = match code {
                KeyCode::Char('x')
                | KeyCode::Char('X')
                | KeyCode::Char('f')
                | KeyCode::Char('F')
                | KeyCode::Char('h')
                | KeyCode::Char('H')
                | KeyCode::Char('z')
                | KeyCode::Char('Z')
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Enter
                | KeyCode::Backspace
                | KeyCode::Esc => {
                    if self.editor.palette.handle_key(code) {
                        let color = self.editor.palette.selected_color;
                        let target = self.editor.palette.target;
                        if let Some(c) = color {
                            Some(AppEvent::Palette(PaletteEvent::ColorChanged(c, target)))
                        } else {
                            Some(AppEvent::Palette(PaletteEvent::BrushChanged))
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(action) = handled {
                return Some(action);
            }
        }

        // Keyboard painting: Space/Enter paints or erases at cursor
        if matches!(
            self.editor.toolbox.selected,
            Tool::Brush | Tool::Eraser | Tool::Line | Tool::Fill | Tool::Spray
        ) && matches!(code, KeyCode::Char(' ') | KeyCode::Enter)
        {
            let (cx, cy) = self.editor.canvas.cursor();
            self.editor.push_undo_snapshot("Keyboard paint");
            if self.editor.toolbox.selected == Tool::Fill {
                let mut cell = canvas::CanvasCell {
                    ch: self.editor.brush.ch,
                    fg: None,
                    bg: None,
                };
                self.editor.palette.apply_to_cell(&mut cell);
                let mut buf = self.editor.layer_stack.active_layer().buffer.clone();
                tools::fill::flood_fill(&mut buf, cx as i16, cy as i16, cell);
                *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                self.editor.recomposite_canvas();
            } else if self.editor.toolbox.selected == Tool::Eraser {
                let shape = self.editor.brush.shape;
                let size = self.editor.brush.size;
                let mut buf = self.editor.layer_stack.active_layer().buffer.clone();
                tools::eraser::erase_stamp(&mut buf, cx as i16, cy as i16, shape, size);
                *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                self.editor.recomposite_canvas();
            } else if self.editor.toolbox.selected == Tool::Spray {
                let mut cell = canvas::CanvasCell {
                    ch: self.editor.brush.ch,
                    fg: None,
                    bg: None,
                };
                self.editor.palette.apply_to_cell(&mut cell);
                let mut rng = StdRng::seed_from_u64(rand::thread_rng().gen());
                let size = self.editor.brush.size;
                let density = self.editor.brush.density;
                let mut buf = self.editor.layer_stack.active_layer().buffer.clone();
                tools::spray::spray_stamp(
                    &mut buf, cx as i16, cy as i16, size, density, cell, &mut rng,
                );
                *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                self.editor.recomposite_canvas();
            } else {
                let mut cell = canvas::CanvasCell {
                    ch: self.editor.brush.ch,
                    fg: None,
                    bg: None,
                };
                self.editor.palette.apply_to_cell(&mut cell);
                let shape = self.editor.brush.shape;
                let size = self.editor.brush.size;
                let mut buf = self.editor.layer_stack.active_layer().buffer.clone();
                tools::brush::paint_stamp(&mut buf, cx as i16, cy as i16, shape, size, cell);
                *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                self.editor.recomposite_canvas();
            }
            self.editor.unsaved = true;
            return None;
        }

        // Global dispatch: file ops, view toggles, mode cycling, quit
        if let Some(action) = keymap::lookup_global(code, modifiers) {
            return self.dispatch_global(action);
        }

        None
    }

    fn dispatch_global(&mut self, action: keymap::GlobalAction) -> Option<AppEvent> {
        use keymap::GlobalAction as GA;
        match action {
            GA::FileOpen => {
                self.start_open();
                None
            }
            GA::FileSave => {
                self.start_save();
                None
            }
            GA::FileSaveAs => {
                self.start_save_as();
                None
            }
            GA::Export => {
                let mode = match self.mode {
                    AppMode::FontEditor => export::ExportMode::Txt,
                    _ => export::ExportMode::Png,
                };
                self.dialogs.export_dialog.enter_export(mode);
                self.dirty = true;
                None
            }
            GA::Undo => {
                let cur = self.editor.layer_stack.active_layer().buffer.clone();
                if let Some((buf, _)) = self.editor.undo.undo(cur) {
                    *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                    self.editor.recomposite_canvas();
                    self.editor.unsaved = true;
                }
                Some(AppEvent::Undo)
            }
            GA::Redo => {
                let cur = self.editor.layer_stack.active_layer().buffer.clone();
                if let Some((buf, _)) = self.editor.undo.redo(cur) {
                    *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                    self.editor.recomposite_canvas();
                    self.editor.unsaved = true;
                }
                Some(AppEvent::Redo)
            }
            GA::ToggleUndoPanel => {
                self.dialogs.undo_panel.toggle();
                Some(AppEvent::UndoPanelToggled)
            }
            GA::ToggleRenderMode => {
                self.render_mode = self.render_mode.toggle();
                self.dirty = true;
                Some(AppEvent::RenderModeChanged)
            }
            GA::ToggleZenMode => {
                self.zen_mode = !self.zen_mode;
                self.dirty = true;
                None
            }
            GA::CycleDrawer => {
                self.right_drawer = self.right_drawer.cycle();
                self.dirty = true;
                None
            }
            GA::ToggleKeybindings => {
                self.show_keybindings = !self.show_keybindings;
                self.dirty = true;
                None
            }
            GA::NextMode => {
                self.mode = self.mode.next();
                self.editor.undo.clear();
                Some(AppEvent::ModeChanged)
            }
            GA::PrevMode => {
                self.mode = self.mode.prev();
                self.editor.undo.clear();
                Some(AppEvent::ModeChanged)
            }
            GA::Quit => {
                self.should_quit = true;
                Some(AppEvent::Quit)
            }
        }
    }

    fn start_save(&mut self) {
        if self.mode != AppMode::FontEditor {
            return;
        }
        if let Some(ref path) = self.editor.font_editor.current_path {
            if let Some(ref font) = self.editor.font_editor.font {
                if self.throbber.is_active() {
                    return;
                }
                let font = font.clone();
                let path = path.clone();
                let (tx, rx) = mpsc::channel();
                self.async_rx = Some(rx);
                self.throbber.start("Saving...");
                self.dirty = true;
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
        self.dialogs
            .file_ops
            .enter_save_as(self.editor.font_editor.current_path.as_deref());
        self.dirty = true;
    }

    fn perform_save(&mut self) {
        if self.throbber.is_active() {
            return;
        }
        let path = self.dialogs.file_ops.selected_path();
        let font = match &self.editor.font_editor.font {
            Some(f) => f.clone(),
            None => return,
        };
        let result_path = path.clone();
        let (tx, rx) = mpsc::channel();
        self.async_rx = Some(rx);
        self.throbber.start("Saving...");
        self.dirty = true;
        std::thread::spawn(move || {
            let result = file_ops::save_font(&font, &result_path)
                .map(|_| result_path)
                .map_err(|e| e.to_string());
            let _ = tx.send(AsyncResult::SaveComplete(result));
        });
    }

    fn handle_paste_event(&mut self, text: String) {
        if self.dialogs.file_ops.mode != file_ops::FileOpsMode::Idle {
            self.dialogs.file_ops.handle_paste(&text);
        }
    }

    fn start_open(&mut self) {
        if self.mode != AppMode::FontEditor {
            return;
        }
        self.dialogs
            .file_ops
            .enter_open(self.dialogs.recent_files.list());
        self.dirty = true;
    }

    fn perform_open(&mut self) {
        if self.throbber.is_active() {
            return;
        }
        let target = self.dialogs.file_ops.resolve_open_target();
        let (tx, rx) = mpsc::channel();
        self.async_rx = Some(rx);
        self.throbber.start("Loading...");
        self.dirty = true;
        match target {
            file_ops::OpenTarget::File(path) => {
                let path_clone = path.clone();
                std::thread::spawn(move || {
                    let result =
                        (|| -> Result<(crate::font::FIGfont, std::path::PathBuf), String> {
                            let content = std::fs::read_to_string(&path_clone)
                                .map_err(|e| format!("Cannot read file: {e}"))?;
                            let font = crate::font::parse_tlf_font(&content)
                                .map_err(|e| format!("Parse error: {e}"))?;
                            Ok((font, path_clone))
                        })();
                    let _ = tx.send(AsyncResult::OpenComplete(result));
                });
            }
            file_ops::OpenTarget::ZipEntry {
                zip_path,
                entry_name,
            } => {
                std::thread::spawn(move || {
                    let result =
                        (|| -> Result<(crate::font::FIGfont, std::path::PathBuf), String> {
                            let bytes = crate::font::read_zip_entry(&zip_path, &entry_name)
                                .map_err(|e| format!("ZIP read error: {e}"))?;
                            let content = String::from_utf8_lossy(&bytes).into_owned();
                            let font = crate::font::parse_tlf_font(&content)
                                .map_err(|e| format!("Parse error: {e}"))?;
                            Ok((font, zip_path))
                        })();
                    let _ = tx.send(AsyncResult::OpenComplete(result));
                });
            }
        }
    }

    fn perform_export(&mut self) {
        if self.throbber.is_active() {
            return;
        }
        let cells: Vec<Vec<canvas::CanvasCell>> = (0..self.editor.canvas.buffer.height())
            .map(|y| {
                (0..self.editor.canvas.buffer.width())
                    .map(|x| {
                        self.editor
                            .canvas
                            .buffer
                            .get(x, y)
                            .copied()
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .collect();
        let format = self.dialogs.export_dialog.format;
        let font_size = self.dialogs.export_dialog.font_size;
        let path_buf = std::path::PathBuf::from(&self.dialogs.export_dialog.path_buffer);
        let (tx, rx) = mpsc::channel();
        self.async_rx = Some(rx);
        self.throbber.start("Exporting...");
        self.dirty = true;
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
        self.dirty = true;
        match action {
            menu::MenuAction::FileOpen => {
                self.start_open();
                self.menu_bar_state.reset();
            }
            menu::MenuAction::FileSave => {
                self.start_save();
                self.menu_bar_state.reset();
            }
            menu::MenuAction::FileSaveAs => {
                self.start_save_as();
                self.menu_bar_state.reset();
            }
            menu::MenuAction::FileExport => {
                let mode = match self.mode {
                    AppMode::FontEditor => export::ExportMode::Txt,
                    _ => export::ExportMode::Png,
                };
                self.dialogs.export_dialog.enter_export(mode);
                self.menu_bar_state.reset();
            }
            menu::MenuAction::FileQuit => {
                self.should_quit = true;
                self.menu_bar_state.reset();
            }
            menu::MenuAction::EditUndo => {
                if self.editor.undo.can_undo() {
                    let cur = self.editor.layer_stack.active_layer().buffer.clone();
                    if let Some((buf, _)) = self.editor.undo.undo(cur) {
                        *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                        self.editor.recomposite_canvas();
                        self.editor.unsaved = true;
                    }
                }
                self.menu_bar_state.reset();
            }
            menu::MenuAction::EditRedo => {
                if self.editor.undo.can_redo() {
                    let cur = self.editor.layer_stack.active_layer().buffer.clone();
                    if let Some((buf, _)) = self.editor.undo.redo(cur) {
                        *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                        self.editor.recomposite_canvas();
                        self.editor.unsaved = true;
                    }
                }
                self.menu_bar_state.reset();
            }
            menu::MenuAction::EditCut => {
                if let Some(ref sel) = self.editor.selection {
                    if sel.is_active() {
                        self.editor.push_undo_snapshot("Cut selection");
                        if let Some(sel_owned) = self.editor.selection.take() {
                            let mut buf = self.editor.layer_stack.active_layer().buffer.clone();
                            self.editor.clipboard = Some(sel_owned.cut_from(&mut buf));
                            *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                            self.editor.recomposite_canvas();
                            self.editor.unsaved = true;
                        }
                    }
                }
                self.menu_bar_state.reset();
            }
            menu::MenuAction::EditCopy => {
                if let Some(ref sel) = self.editor.selection {
                    if sel.is_active() {
                        self.editor.clipboard = Some(sel.copy_from(&self.editor.canvas.buffer));
                    }
                }
                self.menu_bar_state.reset();
            }
            menu::MenuAction::EditPaste => {
                if self.editor.clipboard.is_some() {
                    self.editor.push_undo_snapshot("Paste");
                    let clip = self.editor.clipboard.clone();
                    if let Some(ref clip_data) = clip {
                        let (cx, cy) = self.editor.canvas.cursor();
                        let mut buf = self.editor.layer_stack.active_layer().buffer.clone();
                        tools::selection::Selection::paste_into(
                            &mut buf, clip_data, cx as i16, cy as i16,
                        );
                        *self.editor.layer_stack.active_layer_mut().buffer_mut() = buf;
                        self.editor.recomposite_canvas();
                        self.editor.unsaved = true;
                    }
                }
                self.menu_bar_state.reset();
            }
            menu::MenuAction::ViewZoomIn => {
                if self.editor.canvas.zoom_level() < 8 {
                    self.editor.canvas.zoom_in();
                }
                self.menu_bar_state.reset();
            }
            menu::MenuAction::ViewZoomOut => {
                if self.editor.canvas.zoom_level() > 1 {
                    self.editor.canvas.zoom_out();
                }
                self.menu_bar_state.reset();
            }
            menu::MenuAction::ViewToggleGrid => {
                self.editor.canvas.toggle_grid();
                self.menu_bar_state.reset();
            }
            menu::MenuAction::ViewToggleUndoPanel => {
                self.dialogs.undo_panel.toggle();
                self.menu_bar_state.reset();
            }
            menu::MenuAction::ToolsSelect(tool) => {
                self.editor.toolbox.selected = tool;
                if tool != toolbox::Tool::PolygonSelect {
                    self.selection_polygon_points.clear();
                }
                self.menu_bar_state.reset();
            }
            menu::MenuAction::HelpAbout => {
                self.menu_bar_state.reset();
            }
            menu::MenuAction::HelpKeybindings => {
                self.menu_bar_state.reset();
                self.show_keybindings = true;
                self.dirty = true;
            }
        }
    }

    fn apply_settings(&mut self) {
        let w = self.dialogs.settings.canvas_width as usize;
        let h = self.dialogs.settings.canvas_height as usize;
        if self.editor.canvas.buffer.width() != w || self.editor.canvas.buffer.height() != h {
            self.editor.canvas = canvas::CanvasWidget::new(w as u16, h as u16);
            self.editor.layer_stack.resize_all(w, h);
            self.editor.recomposite_canvas();
            self.editor.undo.clear();
        }
        if self.dialogs.settings.show_grid != self.editor.canvas.show_grid() {
            self.editor.canvas.toggle_grid();
        }
        self.dirty = true;
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

/// Returns a 2/3-width, 2/3-height overlay centered in `area`.
fn centered_overlay(area: Rect) -> Rect {
    Rect {
        x: area.width / 6,
        y: area.height / 6,
        width: area.width * 2 / 3,
        height: area.height * 2 / 3,
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
