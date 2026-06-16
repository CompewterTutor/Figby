use ratatui::style::Color;

use super::export::ExportMode;
use super::font_editor::FontEditorView;
use super::menu::MenuAction;
use super::palette::ColorTarget;

#[derive(Debug, Clone, PartialEq)]
pub enum CanvasEvent {
    Modified,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToolboxEvent {
    ToolSelected,
    BrushChanged,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PaletteEvent {
    ColorChanged(Color, ColorTarget),
    BrushChanged,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FontEditorEvent {
    /// Font editor handled a key; current view reported for caller sync.
    Changed(FontEditorView),
}

/// Top-level event returned by components and TuiApp::handle_key_event.
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    Canvas(CanvasEvent),
    Toolbox(ToolboxEvent),
    Palette(PaletteEvent),
    FontEditor(FontEditorEvent),
    ImageEditor,
    Quit,
    ModeChanged,
    RenderModeChanged,
    Undo,
    Redo,
    UndoPanelToggled,
    SaveRequested,
    SaveAsRequested,
    OpenRequested,
    ExportRequested(ExportMode),
    TextCommitted,
    Menu(MenuAction),
}
