use std::collections::BTreeMap;

const ICONS_YAML: &str = include_str!("../../assets/tui/icons.yaml");

#[test]
fn test_icons_yaml_all_keys_present() {
    let map: BTreeMap<String, String> =
        serde_yaml::from_str(ICONS_YAML).expect("failed to parse icons.yaml");

    assert!(
        map.len() >= 120,
        "expected at least 120 icon entries, got {}",
        map.len()
    );

    for (key, value) in &map {
        assert!(!key.is_empty(), "empty key found");
        assert!(
            value.starts_with("nf-"),
            "icon value for '{}' does not start with 'nf-': got '{}'",
            key,
            value
        );
    }
}

#[test]
fn test_tui_smoke_all_panels_render() {
    use figby::tui::TuiApp;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut app = TuiApp::new();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
    let buffer = terminal.backend().buffer();
    let output: String = buffer.content().iter().map(|c| c.symbol()).collect();
    assert!(
        output.contains("Font Editor"),
        "toolbar missing Font Editor tab"
    );
    assert!(
        output.contains("Image Editor"),
        "toolbar missing Image Editor tab"
    );
    assert!(
        output.contains("ASCII Preview"),
        "toolbar missing ASCII Preview tab"
    );
    assert!(output.contains("Palette"), "palette sidebar missing");
    assert!(output.contains("Mode:"), "status bar missing");
}

#[test]
fn test_tui_mode_switching() {
    use crossterm::event::KeyCode;
    use figby::tui::{AppMode, TuiApp};

    let mut app = TuiApp::new();
    assert_eq!(app.mode, AppMode::FontEditor);

    app.handle_key_event(KeyCode::Tab);
    assert_eq!(app.mode, AppMode::ImageEditor);

    app.handle_key_event(KeyCode::Tab);
    assert_eq!(app.mode, AppMode::AsciiPreview);

    app.handle_key_event(KeyCode::Tab);
    assert_eq!(app.mode, AppMode::FontEditor);

    app.handle_key_event(KeyCode::Char('q'));
    assert!(app.should_quit);

    let mut app2 = TuiApp::new();
    app2.handle_key_event(KeyCode::Esc);
    assert!(app2.should_quit);
}

#[test]
fn test_tui_app_default_mode() {
    use figby::tui::{AppMode, TuiApp};
    let app = TuiApp::new();
    assert_eq!(app.mode, AppMode::FontEditor);
    assert!(!app.should_quit);
}

#[test]
fn test_tool_default_is_brush() {
    use figby::tui::{Tool, TuiApp};
    let app = TuiApp::new();
    assert_eq!(app.toolbox.selected, Tool::Brush);
}

#[test]
fn test_tool_selection_roundtrip() {
    use crossterm::event::KeyCode;
    use figby::tui::{Tool, TuiApp};

    let mut app = TuiApp::new();
    assert_eq!(app.toolbox.selected, Tool::Brush);

    app.handle_key_event(KeyCode::Char('v'));
    assert_eq!(app.toolbox.selected, Tool::Marquee);

    app.handle_key_event(KeyCode::Char('b'));
    assert_eq!(app.toolbox.selected, Tool::Brush);

    app.handle_key_event(KeyCode::Char('l'));
    assert_eq!(app.toolbox.selected, Tool::Lasso);

    app.handle_key_event(KeyCode::Char('c'));
    assert_eq!(app.toolbox.selected, Tool::CircleSelect);

    app.handle_key_event(KeyCode::Char('p'));
    assert_eq!(app.toolbox.selected, Tool::PolygonSelect);

    app.handle_key_event(KeyCode::Char('g'));
    assert_eq!(app.toolbox.selected, Tool::Fill);

    app.handle_key_event(KeyCode::Char('i'));
    assert_eq!(app.toolbox.selected, Tool::Line);

    app.handle_key_event(KeyCode::Char('e'));
    assert_eq!(app.toolbox.selected, Tool::Eraser);

    app.handle_key_event(KeyCode::Char('d'));
    assert_eq!(app.toolbox.selected, Tool::Eyedropper);

    app.handle_key_event(KeyCode::Char('t'));
    assert_eq!(app.toolbox.selected, Tool::Text);

    app.handle_key_event(KeyCode::Char('B'));
    assert_eq!(app.toolbox.selected, Tool::Brush);
}

#[test]
fn test_toolbox_renders_tool_names() {
    use figby::tui::TuiApp;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut app = TuiApp::new();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
    let buffer = terminal.backend().buffer();
    let output: String = buffer.content().iter().map(|c| c.symbol()).collect();
    assert!(output.contains(" Br"), "toolbox missing Brush tool");
    assert!(output.contains(" Er"), "toolbox missing Eraser tool");
}

#[test]
fn test_canvas_render_empty() {
    use figby::tui::canvas::CanvasWidget;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let canvas = CanvasWidget::new(10, 5);
    let backend = TestBackend::new(20, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| f.render_widget(&canvas, f.area()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    for cell in buffer.content() {
        assert_eq!(cell.symbol(), " ");
    }
}

#[test]
fn test_canvas_render_cells() {
    use figby::tui::canvas::{CanvasCell, CanvasWidget};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut canvas = CanvasWidget::new(10, 5);
    canvas.buffer.set(
        0,
        0,
        CanvasCell {
            ch: 'A',
            fg: None,
            bg: None,
        },
    );
    canvas.buffer.set(
        2,
        1,
        CanvasCell {
            ch: 'B',
            fg: None,
            bg: None,
        },
    );

    let backend = TestBackend::new(20, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| f.render_widget(&canvas, f.area()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let output: String = buffer.content().iter().map(|c| c.symbol()).collect();
    assert!(output.contains('A'), "expected 'A' in canvas output");
    assert!(output.contains('B'), "expected 'B' in canvas output");
}

#[test]
fn test_canvas_cursor_movement() {
    use crossterm::event::KeyCode;
    use figby::tui::canvas::CanvasWidget;

    let mut canvas = CanvasWidget::new(10, 5);
    assert_eq!(canvas.cursor(), (0, 0));

    canvas.handle_key(KeyCode::Right, 20, 10);
    assert_eq!(canvas.cursor(), (1, 0));

    canvas.handle_key(KeyCode::Down, 20, 10);
    assert_eq!(canvas.cursor(), (1, 1));

    canvas.handle_key(KeyCode::Left, 20, 10);
    assert_eq!(canvas.cursor(), (0, 1));

    canvas.handle_key(KeyCode::Up, 20, 10);
    assert_eq!(canvas.cursor(), (0, 0));
}

#[test]
fn test_canvas_zoom_in_out() {
    use crossterm::event::KeyCode;
    use figby::tui::canvas::CanvasWidget;

    let mut canvas = CanvasWidget::new(10, 5);
    assert_eq!(canvas.zoom_level(), 1);

    canvas.handle_key(KeyCode::Char('+'), 20, 10);
    assert_eq!(canvas.zoom_level(), 2);

    canvas.handle_key(KeyCode::Char('='), 20, 10);
    assert_eq!(canvas.zoom_level(), 4);

    canvas.handle_key(KeyCode::Char('+'), 20, 10);
    assert_eq!(canvas.zoom_level(), 8);

    canvas.handle_key(KeyCode::Char('+'), 20, 10);
    assert_eq!(canvas.zoom_level(), 8);

    canvas.handle_key(KeyCode::Char('-'), 20, 10);
    assert_eq!(canvas.zoom_level(), 4);

    canvas.handle_key(KeyCode::Char('_'), 20, 10);
    assert_eq!(canvas.zoom_level(), 2);

    canvas.handle_key(KeyCode::Char('-'), 20, 10);
    assert_eq!(canvas.zoom_level(), 1);

    canvas.handle_key(KeyCode::Char('-'), 20, 10);
    assert_eq!(canvas.zoom_level(), 1);
}

#[test]
fn test_canvas_cursor_visible() {
    use figby::tui::canvas::{CanvasCell, CanvasWidget};
    use ratatui::backend::TestBackend;
    use ratatui::style::Modifier;
    use ratatui::Terminal;

    let mut canvas = CanvasWidget::new(10, 5);
    canvas.buffer.set(
        0,
        0,
        CanvasCell {
            ch: 'X',
            fg: None,
            bg: None,
        },
    );

    let backend = TestBackend::new(20, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| f.render_widget(&canvas, f.area()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let cell = buffer.cell((0, 0)).expect("cell at (0,0)");
    assert_eq!(cell.symbol(), "X");
    assert!(
        cell.style().add_modifier.contains(Modifier::REVERSED),
        "cursor cell should have REVERSED modifier"
    );
}

#[test]
fn test_canvas_zoom_shows_grid() {
    use crossterm::event::KeyCode;
    use figby::tui::canvas::{CanvasCell, CanvasWidget};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut canvas = CanvasWidget::new(5, 3);
    canvas.buffer.set(
        0,
        0,
        CanvasCell {
            ch: 'A',
            fg: None,
            bg: None,
        },
    );
    canvas.buffer.set(
        1,
        0,
        CanvasCell {
            ch: 'B',
            fg: None,
            bg: None,
        },
    );

    canvas.handle_key(KeyCode::Char('+'), 20, 10);
    canvas.handle_key(KeyCode::Char('G'), 20, 10);

    let backend = TestBackend::new(20, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| f.render_widget(&canvas, f.area()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let output: String = buffer.content().iter().map(|c| c.symbol()).collect();
    assert!(output.contains('│'), "expected vertical grid line │");
    assert!(output.contains('─'), "expected horizontal grid line ─");
}
