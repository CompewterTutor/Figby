use ratatui::style::Color;

use super::export::ExportMode;
use super::menu::MenuAction;
use super::palette::ColorTarget;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Quit,
    ToolSelected,
    ColorChanged(Color, ColorTarget),
    BrushChanged,
    Undo,
    Redo,
    ModeChanged,
    FileOpened(String),
    FileSaved(String),
    FontCharSelected(u32),
    ExportRequested(ExportMode),
    SettingsToggled,
    UndoPanelToggled,
    CanvasModified,
    TextCommitted,
    CloseDialog,
    SaveRequested,
    OpenRequested,
    SaveAsRequested,
    Message(String),
    FontEditorAction,
    ImageEditorAction,
    Menu(MenuAction),
    RenderModeChanged,
}
