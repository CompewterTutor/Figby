#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Global,
    Canvas,
    FontOverview,
    FontCharEditor,
    Dialog,
}

impl Scope {
    pub fn label(self) -> &'static str {
        match self {
            Scope::Global => "Global",
            Scope::Canvas => "Canvas",
            Scope::FontOverview => "Font Overview",
            Scope::FontCharEditor => "Font Char Editor",
            Scope::Dialog => "Dialog",
        }
    }
}

pub struct KeyBinding {
    pub keys: &'static str,
    pub scope: Scope,
    pub description: &'static str,
}

pub const KEYMAP: &[KeyBinding] = &[
    // Global
    KeyBinding { keys: "Ctrl+O",       scope: Scope::Global,         description: "Open file" },
    KeyBinding { keys: "Ctrl+S",       scope: Scope::Global,         description: "Save file" },
    KeyBinding { keys: "Ctrl+Shift+S", scope: Scope::Global,         description: "Save As" },
    KeyBinding { keys: "Ctrl+E",       scope: Scope::Global,         description: "Export" },
    KeyBinding { keys: "Ctrl+Q",       scope: Scope::Global,         description: "Quit" },
    KeyBinding { keys: "Tab",          scope: Scope::Global,         description: "Next mode" },
    KeyBinding { keys: "Shift+Tab",    scope: Scope::Global,         description: "Prev mode" },
    KeyBinding { keys: "F5",           scope: Scope::Global,         description: "Toggle render mode (Fast/Dirty)" },
    KeyBinding { keys: "Ctrl+Shift+H", scope: Scope::Global,         description: "Toggle undo history panel" },
    KeyBinding { keys: "Alt+F",        scope: Scope::Global,         description: "Open File menu" },
    KeyBinding { keys: "Alt+E",        scope: Scope::Global,         description: "Open Edit menu" },
    KeyBinding { keys: "Alt+V",        scope: Scope::Global,         description: "Open View menu" },
    KeyBinding { keys: "Alt+T",        scope: Scope::Global,         description: "Open Tools menu" },
    KeyBinding { keys: "Alt+H",        scope: Scope::Global,         description: "Open Help menu" },
    // Canvas
    KeyBinding { keys: "Ctrl+Z",       scope: Scope::Canvas,         description: "Undo" },
    KeyBinding { keys: "Ctrl+Y",       scope: Scope::Canvas,         description: "Redo" },
    KeyBinding { keys: "Ctrl+Shift+Z", scope: Scope::Canvas,         description: "Redo (alternate)" },
    KeyBinding { keys: "+ / -",        scope: Scope::Canvas,         description: "Zoom in / out" },
    // Font Overview
    KeyBinding { keys: "↑↓←→",         scope: Scope::FontOverview,   description: "Navigate glyph grid" },
    KeyBinding { keys: "Enter",        scope: Scope::FontOverview,   description: "Open glyph editor" },
    KeyBinding { keys: "Type chars",   scope: Scope::FontOverview,   description: "Activate search / filter glyphs" },
    KeyBinding { keys: "Esc",          scope: Scope::FontOverview,   description: "Clear search" },
    KeyBinding { keys: "A",            scope: Scope::FontOverview,   description: "Add glyph" },
    KeyBinding { keys: "D",            scope: Scope::FontOverview,   description: "Delete glyph" },
    KeyBinding { keys: "C",            scope: Scope::FontOverview,   description: "Copy glyph" },
    KeyBinding { keys: "H",            scope: Scope::FontOverview,   description: "Header editor" },
    KeyBinding { keys: "S",            scope: Scope::FontOverview,   description: "Smushing rule editor" },
    KeyBinding { keys: "T",            scope: Scope::FontOverview,   description: "Transform editor" },
    // Font Char Editor
    KeyBinding { keys: "↑↓←→",         scope: Scope::FontCharEditor, description: "Move cursor in glyph" },
    KeyBinding { keys: "Space",        scope: Scope::FontCharEditor, description: "Toggle cell" },
    KeyBinding { keys: "M",            scope: Scope::FontCharEditor, description: "Mirror" },
    KeyBinding { keys: "F",            scope: Scope::FontCharEditor, description: "Flip" },
    KeyBinding { keys: "G",            scope: Scope::FontCharEditor, description: "Generate from system font" },
    // Dialog
    KeyBinding { keys: "Esc",          scope: Scope::Dialog,         description: "Close / cancel" },
    KeyBinding { keys: "Enter",        scope: Scope::Dialog,         description: "Confirm" },
    KeyBinding { keys: "↑↓",           scope: Scope::Dialog,         description: "Navigate items" },
];
