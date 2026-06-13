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
    assert!(output.contains("Mode"), "status bar missing");
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

#[test]
fn test_palette_default_target_foreground() {
    use figby::tui::palette::{ColorTarget, Palette};
    let palette = Palette::new();
    assert_eq!(palette.target, ColorTarget::Foreground);
}

#[test]
fn test_palette_fg_bg_toggle() {
    use crossterm::event::KeyCode;
    use figby::tui::palette::{ColorTarget, Palette};

    let mut palette = Palette::new();
    assert_eq!(palette.target, ColorTarget::Foreground);

    palette.handle_key(KeyCode::Char('x'));
    assert_eq!(palette.target, ColorTarget::Background);

    palette.handle_key(KeyCode::Char('x'));
    assert_eq!(palette.target, ColorTarget::Foreground);

    palette.handle_key(KeyCode::Char('X'));
    assert_eq!(palette.target, ColorTarget::Background);

    palette.handle_key(KeyCode::Char('X'));
    assert_eq!(palette.target, ColorTarget::Foreground);
}

#[test]
fn test_palette_select_color_updates_selected() {
    use figby::tui::palette::{Palette, ANSI_16_COLORS};

    let mut palette = Palette::new();
    palette.select_color(1);
    assert_eq!(palette.selected_color, Some(ANSI_16_COLORS[1]));

    palette.select_color(7);
    assert_eq!(palette.selected_color, Some(ANSI_16_COLORS[7]));
}

#[test]
fn test_palette_select_pushes_recent() {
    use figby::tui::palette::{Palette, ANSI_16_COLORS};

    let mut palette = Palette::new();
    assert!(palette.recent.is_empty());

    palette.select_color(1);
    assert_eq!(palette.recent.len(), 1);
    assert_eq!(palette.recent[0], ANSI_16_COLORS[1]);

    palette.select_color(5);
    assert_eq!(palette.recent.len(), 2);
    assert_eq!(palette.recent[1], ANSI_16_COLORS[5]);

    palette.select_color(1);
    assert_eq!(palette.recent.len(), 2);
    assert_eq!(palette.recent[1], ANSI_16_COLORS[1]);
}

#[test]
fn test_palette_custom_hex_applies() {
    use figby::tui::palette::Palette;
    use ratatui::style::Color;

    let mut palette = Palette::new();
    let result = palette.set_custom_hex("#FF8800");
    assert!(result);
    assert_eq!(palette.selected_color, Some(Color::Rgb(255, 136, 0)));
}

#[test]
fn test_palette_apply_to_cell_fg() {
    use figby::tui::canvas::CanvasCell;
    use figby::tui::palette::{Palette, ANSI_16_COLORS};

    let mut palette = Palette::new();
    palette.select_color(2);
    let mut cell = CanvasCell::default();
    palette.apply_to_cell(&mut cell);
    assert_eq!(cell.fg, Some(ANSI_16_COLORS[2]));
    assert_eq!(cell.bg, None);
}

#[test]
fn test_palette_apply_to_cell_bg() {
    use figby::tui::canvas::CanvasCell;
    use figby::tui::palette::{Palette, ANSI_16_COLORS};

    let mut palette = Palette::new();
    palette.toggle_target();
    palette.select_color(4);
    let mut cell = CanvasCell::default();
    palette.apply_to_cell(&mut cell);
    assert_eq!(cell.fg, None);
    assert_eq!(cell.bg, Some(ANSI_16_COLORS[4]));
}

#[test]
fn test_brush_default_shape() {
    use figby::tui::brush::BrushShape;
    use figby::tui::BrushState;
    let brush = BrushState::new();
    assert_eq!(brush.shape, BrushShape::Square);
}

#[test]
fn test_brush_default_size() {
    use figby::tui::BrushState;
    let brush = BrushState::new();
    assert_eq!(brush.size, 3);
}

#[test]
fn test_brush_cycle_shape() {
    use figby::tui::brush::BrushShape;
    use figby::tui::BrushState;
    let mut brush = BrushState::new();
    assert_eq!(brush.shape, BrushShape::Square);
    brush.cycle_shape();
    assert_eq!(brush.shape, BrushShape::Circle);
    brush.cycle_shape();
    assert_eq!(brush.shape, BrushShape::SprayPaint);
    brush.cycle_shape();
    assert_eq!(brush.shape, BrushShape::Custom);
    brush.cycle_shape();
    assert_eq!(brush.shape, BrushShape::Square);
}

#[test]
fn test_brush_size_up_down_key() {
    use crossterm::event::KeyCode;
    use figby::tui::TuiApp;

    let mut app = TuiApp::new();
    assert_eq!(app.brush.size, 3);

    app.handle_key_event(KeyCode::Char(']'));
    assert_eq!(app.brush.size, 4);

    app.handle_key_event(KeyCode::Char('['));
    assert_eq!(app.brush.size, 3);
}

#[test]
fn test_brush_shape_cycle_key() {
    use crossterm::event::KeyCode;
    use figby::tui::brush::BrushShape;
    use figby::tui::TuiApp;

    let mut app = TuiApp::new();
    assert_eq!(app.brush.shape, BrushShape::Square);

    app.handle_key_event(KeyCode::Char('\''));
    assert_eq!(app.brush.shape, BrushShape::Circle);

    app.handle_key_event(KeyCode::Char('\''));
    assert_eq!(app.brush.shape, BrushShape::SprayPaint);
}

#[test]
fn test_brush_preview_square_integration() {
    use figby::tui::BrushState;

    let brush = BrushState {
        shape: figby::tui::brush::BrushShape::Square,
        size: 3,
        ch: '\u{2588}',
        density: 35,
    };
    let preview = brush.render_preview(10);
    assert_eq!(preview.len(), 3);
    for row in &preview {
        assert_eq!(row.chars().filter(|&c| c == '@').count(), 3);
    }
}

#[test]
fn test_brush_preview_circle_integration() {
    use figby::tui::brush::BrushShape;
    use figby::tui::BrushState;

    let brush = BrushState {
        shape: BrushShape::Circle,
        size: 5,
        ch: '\u{2588}',
        density: 35,
    };
    let preview = brush.render_preview(10);
    assert_eq!(preview.len(), 5);
    for row in &preview {
        assert_eq!(row.len(), 5);
    }
}

#[test]
fn test_brush_preview_spray_deterministic() {
    use figby::tui::brush::BrushShape;
    use figby::tui::BrushState;

    let a = BrushState {
        shape: BrushShape::SprayPaint,
        size: 7,
        ch: '\u{2588}',
        density: 35,
    };
    let b = BrushState {
        shape: BrushShape::SprayPaint,
        size: 7,
        ch: '\u{2588}',
        density: 35,
    };
    assert_eq!(a.render_preview(10), b.render_preview(10));
}

#[test]
fn test_brush_preview_custom_center() {
    use figby::tui::brush::BrushShape;
    use figby::tui::BrushState;

    let brush = BrushState {
        shape: BrushShape::Custom,
        size: 5,
        ch: '\u{2588}',
        density: 35,
    };
    let preview = brush.render_preview(10);
    assert_eq!(preview[2].as_bytes()[2] as char, '+');
}

#[test]
fn test_brush_preview_respects_max_size() {
    use figby::tui::BrushState;

    let mut brush = BrushState::new();
    brush.set_size(15);
    let preview = brush.render_preview(5);
    assert_eq!(preview.len(), 5);
    for row in &preview {
        assert_eq!(row.len(), 5);
    }
}

#[test]
fn test_brush_render_contains_shape_name() {
    use figby::tui::TuiApp;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut app = TuiApp::new();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
    let buffer = terminal.backend().buffer();
    let output: String = buffer.content().iter().map(|c| c.symbol()).collect();
    assert!(output.contains("Brush"), "brush panel missing");
    assert!(output.contains("Square"), "brush shape name missing");
}

#[test]
fn test_palette_render_contains_labels() {
    use figby::tui::TuiApp;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut app = TuiApp::new();
    app.palette.select_color(0);
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
    let buffer = terminal.backend().buffer();
    let output: String = buffer.content().iter().map(|c| c.symbol()).collect();
    assert!(output.contains("FG"), "palette missing FG indicator");
    assert!(output.contains("BG"), "palette missing BG indicator");
    assert!(output.contains("Recent"), "palette missing Recent label");
}

#[test]
fn test_status_bar_shows_cursor_position() {
    use crossterm::event::KeyCode;
    use figby::tui::TuiApp;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut app = TuiApp::new();
    app.handle_key_event(KeyCode::Right);
    app.handle_key_event(KeyCode::Down);

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
    let buffer = terminal.backend().buffer();
    let output: String = buffer.content().iter().map(|c| c.symbol()).collect();
    assert!(output.contains("X:1"), "status bar missing X cursor");
    assert!(output.contains("Y:1"), "status bar missing Y cursor");
}

#[test]
fn test_status_bar_shows_zoom_level() {
    use crossterm::event::KeyCode;
    use figby::tui::TuiApp;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut app = TuiApp::new();
    app.handle_key_event(KeyCode::Char('+'));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
    let buffer = terminal.backend().buffer();
    let output: String = buffer.content().iter().map(|c| c.symbol()).collect();
    assert!(output.contains("Zoom:2x"), "status bar missing zoom level");
}

#[test]
fn test_status_bar_shows_tool_name() {
    use crossterm::event::KeyCode;
    use figby::tui::TuiApp;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut app = TuiApp::new();
    app.handle_key_event(KeyCode::Char('e'));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
    let buffer = terminal.backend().buffer();
    let output: String = buffer.content().iter().map(|c| c.symbol()).collect();
    assert!(output.contains("Eraser"), "status bar missing tool name");
}

#[test]
fn test_status_bar_shows_mode_name() {
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
        "status bar missing mode name"
    );
}

#[test]
fn test_status_bar_unsaved_indicator() {
    use figby::tui::TuiApp;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut app = TuiApp::new();
    app.unsaved = true;
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
    let buffer = terminal.backend().buffer();
    let output: String = buffer.content().iter().map(|c| c.symbol()).collect();
    assert!(
        output.contains("exclamation"),
        "unsaved indicator missing when unsaved=true"
    );

    app.unsaved = false;
    let backend2 = TestBackend::new(80, 24);
    let mut terminal2 = Terminal::new(backend2).unwrap();
    terminal2.draw(|f| app.render(f)).unwrap();
    let buffer2 = terminal2.backend().buffer();
    let output2: String = buffer2.content().iter().map(|c| c.symbol()).collect();
    assert!(
        output2.contains("nf-fa-check"),
        "saved indicator missing when unsaved=false"
    );
}

#[test]
fn test_settings_toggle_visibility() {
    use crossterm::event::KeyCode;
    use figby::tui::TuiApp;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut app = TuiApp::new();
    app.handle_key_event(KeyCode::Char('S'));
    assert!(app.settings.settings_open, "settings should open on S");

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
    let buffer = terminal.backend().buffer();
    let output: String = buffer.content().iter().map(|c| c.symbol()).collect();
    assert!(output.contains("Settings"), "settings panel title missing");

    app.handle_key_event(KeyCode::Char('S'));
    assert!(!app.settings.settings_open, "settings should close on S");
}

#[test]
fn test_settings_changes_canvas_width() {
    use crossterm::event::KeyCode;
    use figby::tui::TuiApp;

    let mut app = TuiApp::new();
    assert_eq!(app.canvas.buffer.width(), 40);
    app.handle_key_event(KeyCode::Char('S'));
    assert!(app.settings.settings_open);
    app.handle_key_event(KeyCode::Right);
    assert_eq!(
        app.canvas.buffer.width(),
        41,
        "canvas width should increase"
    );
}

#[test]
fn test_settings_toggle_grid() {
    use crossterm::event::KeyCode;
    use figby::tui::TuiApp;

    let mut app = TuiApp::new();
    app.handle_key_event(KeyCode::Char('S'));
    for _ in 0..3 {
        app.handle_key_event(KeyCode::Down);
    }
    app.handle_key_event(KeyCode::Enter);
    assert!(app.canvas.show_grid(), "grid should be toggled on");
}

#[test]
fn test_settings_toggle_snap_to_grid() {
    use crossterm::event::KeyCode;
    use figby::tui::TuiApp;

    let mut app = TuiApp::new();
    assert!(!app.settings.snap_to_grid);
    app.handle_key_event(KeyCode::Char('S'));
    for _ in 0..4 {
        app.handle_key_event(KeyCode::Down);
    }
    app.handle_key_event(KeyCode::Enter);
    assert!(
        app.settings.snap_to_grid,
        "snap-to-grid should be toggled on"
    );
}

#[test]
fn test_fill_tool_keyboard() {
    use crossterm::event::KeyCode;
    use figby::tui::canvas::CanvasCell;
    use figby::tui::{Tool, TuiApp};

    let mut app = TuiApp::new();

    // Select Fill tool via keyboard shortcut
    app.handle_key_event(KeyCode::Char('g'));
    assert_eq!(app.toolbox.selected, Tool::Fill);

    // Draw a 2x2 region of @
    app.canvas.buffer.set(
        1,
        1,
        CanvasCell {
            ch: '@',
            fg: None,
            bg: None,
        },
    );
    app.canvas.buffer.set(
        1,
        2,
        CanvasCell {
            ch: '@',
            fg: None,
            bg: None,
        },
    );
    app.canvas.buffer.set(
        2,
        1,
        CanvasCell {
            ch: '@',
            fg: None,
            bg: None,
        },
    );
    app.canvas.buffer.set(
        2,
        2,
        CanvasCell {
            ch: '@',
            fg: None,
            bg: None,
        },
    );

    // Move cursor to (1, 1)
    app.canvas.set_cursor(1, 1);

    // Press Space to flood fill
    app.handle_key_event(KeyCode::Char(' '));

    // The filled region should have been replaced with full block
    assert_eq!(
        app.canvas.buffer.get(1, 1).unwrap().ch,
        '\u{2588}',
        "filled cell (1,1)"
    );
    assert_eq!(
        app.canvas.buffer.get(2, 2).unwrap().ch,
        '\u{2588}',
        "filled cell (2,2)"
    );
    assert_eq!(
        app.canvas.buffer.get(0, 0).unwrap().ch,
        ' ',
        "outside fill should remain space"
    );
}
