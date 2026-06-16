use ratatui::backend::TestBackend;
use ratatui::Terminal;

/// Canvas widget round-trip: create, set cells, render, verify buffer.
#[test]
fn regression_tui_canvas_roundtrip() {
    use figby::tui::canvas::{CanvasCell, CanvasWidget};

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
        4,
        2,
        CanvasCell {
            ch: 'Z',
            fg: None,
            bg: None,
        },
    );

    let backend = TestBackend::new(20, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| f.render_widget(&canvas, f.area()))
        .unwrap();

    let buf = terminal.backend().buffer();
    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "A");
    assert_eq!(buf.cell((4, 2)).unwrap().symbol(), "Z");
    assert_eq!(buf.cell((5, 0)).unwrap().symbol(), " ");
}

/// Palette widget round-trip: color selection, apply to cell, render labels.
#[test]
fn regression_tui_palette_roundtrip() {
    use figby::tui::palette::{Palette, ANSI_16_COLORS};

    let mut palette = Palette::new();
    palette.select_color(2);

    let backend = TestBackend::new(30, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| f.render_widget(&palette, f.area()))
        .unwrap();

    let buf = terminal.backend().buffer();
    let output: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(output.contains("FG"), "Palette should show FG label");
    assert!(
        output.contains("Recent"),
        "Palette should show Recent label"
    );
    assert_eq!(palette.selected_color, Some(ANSI_16_COLORS[2]));
}

/// Toolbox widget round-trip: tool selection, render, verify displayed.
#[test]
fn regression_tui_toolbox_roundtrip() {
    use figby::tui::toolbox::{Tool, Toolbox};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut toolbox = Toolbox::new();
    toolbox.handle_key(Tool::Line.key_shortcut());

    let backend = TestBackend::new(20, 15);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| f.render_widget(&toolbox, f.area()))
        .unwrap();

    let buf = terminal.backend().buffer();
    let output: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(output.contains("Li"), "Toolbox should show Line tool label");
    assert_eq!(toolbox.selected, Tool::Line);
}

/// StatusBar widget round-trip: set all fields, render, verify content.
#[test]
fn regression_tui_statusbar_roundtrip() {
    use figby::tui::status::StatusBar;
    use std::collections::BTreeMap;

    let icons: BTreeMap<String, String> = BTreeMap::new();

    let backend = TestBackend::new(80, 3);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            StatusBar::render(
                f,
                f.area(),
                (5, 3),
                2,
                "Brush",
                "Font Editor",
                false,
                &icons,
                None,
            );
        })
        .unwrap();
    let buf = terminal.backend().buffer();
    let output: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(output.contains("Brush"), "StatusBar should show tool name");
    assert!(output.contains("5"), "StatusBar should show X cursor");
    assert!(output.contains("3"), "StatusBar should show Y cursor");
}

/// MenuBar round-trip: construct, render, verify headers.
#[test]
fn regression_tui_menubar_roundtrip() {
    use figby::tui::menu::{MenuBar, MenuBarState};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let bar = MenuBar::new();
    let mut state = MenuBarState::new();

    let backend = TestBackend::new(80, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| f.render_stateful_widget(&bar, f.area(), &mut state))
        .unwrap();

    let buf = terminal.backend().buffer();
    let output: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(output.contains("File"), "MenuBar should show File");
    assert!(output.contains("Edit"), "MenuBar should show Edit");
    assert!(output.contains("View"), "MenuBar should show View");
    assert!(output.contains("Tools"), "MenuBar should show Tools");
    assert!(output.contains("Help"), "MenuBar should show Help");
}

/// Dialog widget round-trip: open FileOpsDialog, render, verify path entry.
#[test]
fn regression_tui_dialog_roundtrip() {
    use crossterm::event::KeyCode;
    use figby::tui::file_ops::FileOpsDialog;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut dialog = FileOpsDialog::new();
    dialog.enter_save_as(None);
    dialog.handle_key(KeyCode::Char('t'));
    dialog.handle_key(KeyCode::Char('e'));
    dialog.handle_key(KeyCode::Char('s'));
    dialog.handle_key(KeyCode::Char('t'));

    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| dialog.render(f, f.area())).unwrap();

    let buf = terminal.backend().buffer();
    let output: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(output.contains("test"), "Dialog should show typed path");
}

/// Full TuiApp render: verify all panels render without panic.
#[test]
fn regression_tui_full_app_render() {
    use figby::tui::TuiApp;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut app = TuiApp::new();
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();

    let buf = terminal.backend().buffer();
    let output: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(output.contains("Font Editor"));
    assert!(output.contains("Image Editor"));
    assert!(output.contains("ASCII Preview"));
    assert!(output.contains("FPS:"));
}

/// Canvas with zoom=2 and colored cells renders correctly.
#[test]
fn regression_tui_canvas_zoom_colored() {
    use crossterm::event::KeyCode;
    use figby::tui::canvas::{CanvasCell, CanvasWidget};
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;
    use ratatui::Terminal;

    let mut canvas = CanvasWidget::new(3, 2);
    canvas.buffer.set(
        0,
        0,
        CanvasCell {
            ch: 'X',
            fg: Some(Color::Red),
            bg: None,
        },
    );
    canvas.handle_key(KeyCode::Char('+'), 20, 10);

    let backend = TestBackend::new(20, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| f.render_widget(&canvas, f.area()))
        .unwrap();

    let buf = terminal.backend().buffer();
    let cell = buf.cell((0, 0)).unwrap();
    assert_eq!(cell.symbol(), "X");
}

/// BrushState render_preview returns expected number of rows.
#[test]
fn regression_tui_brush_preview_rows() {
    use figby::tui::BrushState;

    let brush = BrushState::new();
    let preview = brush.render_preview(10);
    assert_eq!(preview.len(), brush.size as usize);
}
