use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use ratatui::Frame;
use std::collections::BTreeMap;
use std::io;
use std::time::Duration;

pub mod canvas;
pub mod toolbox;

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
    last_canvas_size: (u16, u16),
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
            last_canvas_size: (0, 0),
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        use ratatui::backend::CrosstermBackend;
        use ratatui::Terminal;

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        while !self.should_quit {
            terminal.draw(|f| self.render(f))?;
            self.handle_event()?;
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        Ok(())
    }

    pub fn render(&mut self, frame: &mut Frame<'_>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
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

        self.toolbox.render(frame, main_chunks[0]);

        let block = Block::default()
            .title(self.mode.title())
            .borders(Borders::ALL);
        let inner = block.inner(main_chunks[1]);
        self.last_canvas_size = (inner.width, inner.height);
        self.canvas.ensure_cursor_visible(inner.width, inner.height);
        frame.render_widget(block, main_chunks[1]);
        frame.render_widget(&self.canvas, inner);

        let palette = Block::default().title(" Palette ").borders(Borders::ALL);
        frame.render_widget(palette, main_chunks[2]);

        let mode_name = match self.mode {
            AppMode::FontEditor => "Font Editor",
            AppMode::ImageEditor => "Image Editor",
            AppMode::AsciiPreview => "ASCII Preview",
        };
        let status = Paragraph::new(format!(" Mode: {} | [Tab] Switch | [q] Quit", mode_name))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(status, chunks[2]);
    }

    pub fn handle_event(&mut self) -> io::Result<()> {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    self.handle_key_event(key.code);
                }
            }
        }
        Ok(())
    }

    pub fn handle_key_event(&mut self, code: KeyCode) {
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
            KeyCode::Tab => {
                self.mode = self.mode.next();
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            _ => {}
        }
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}
