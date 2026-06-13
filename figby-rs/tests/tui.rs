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

    let app = TuiApp::new();
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
