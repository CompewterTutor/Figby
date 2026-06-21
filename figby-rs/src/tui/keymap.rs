use crossterm::event::{KeyCode, KeyModifiers};

/// Machine-dispatchable global actions — one-to-one with entries in [`GLOBAL_DISPATCH`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalAction {
    FileOpen,
    FileSave,
    FileSaveAs,
    Export,
    Undo,
    Redo,
    ToggleUndoPanel,
    ToggleRenderMode,
    ToggleZenMode,
    CycleDrawer,
    ToggleKeybindings,
    ToggleTimeline,
    NextMode,
    PrevMode,
    Quit,
}

pub struct KeyDispatch {
    pub modifiers: KeyModifiers,
    pub key_code: KeyCode,
    pub action: GlobalAction,
}

pub static GLOBAL_DISPATCH: &[KeyDispatch] = &[
    // File operations
    KeyDispatch {
        modifiers: KeyModifiers::CONTROL,
        key_code: KeyCode::Char('o'),
        action: GlobalAction::FileOpen,
    },
    KeyDispatch {
        modifiers: KeyModifiers::CONTROL,
        key_code: KeyCode::Char('s'),
        action: GlobalAction::FileSave,
    },
    KeyDispatch {
        modifiers: KeyModifiers::CONTROL.union(KeyModifiers::SHIFT),
        key_code: KeyCode::Char('s'),
        action: GlobalAction::FileSaveAs,
    },
    KeyDispatch {
        modifiers: KeyModifiers::CONTROL,
        key_code: KeyCode::Char('e'),
        action: GlobalAction::Export,
    },
    // Undo / redo
    KeyDispatch {
        modifiers: KeyModifiers::CONTROL,
        key_code: KeyCode::Char('z'),
        action: GlobalAction::Undo,
    },
    KeyDispatch {
        modifiers: KeyModifiers::CONTROL.union(KeyModifiers::SHIFT),
        key_code: KeyCode::Char('z'),
        action: GlobalAction::Redo,
    },
    KeyDispatch {
        modifiers: KeyModifiers::CONTROL,
        key_code: KeyCode::Char('y'),
        action: GlobalAction::Redo,
    },
    // Panels
    KeyDispatch {
        modifiers: KeyModifiers::CONTROL.union(KeyModifiers::SHIFT),
        key_code: KeyCode::Char('h'),
        action: GlobalAction::ToggleUndoPanel,
    },
    KeyDispatch {
        modifiers: KeyModifiers::CONTROL,
        key_code: KeyCode::Char('k'),
        action: GlobalAction::ToggleKeybindings,
    },
    // View toggles
    KeyDispatch {
        modifiers: KeyModifiers::NONE,
        key_code: KeyCode::F(5),
        action: GlobalAction::ToggleRenderMode,
    },
    KeyDispatch {
        modifiers: KeyModifiers::NONE,
        key_code: KeyCode::F(11),
        action: GlobalAction::ToggleZenMode,
    },
    // '?' may arrive with NONE or SHIFT depending on terminal
    KeyDispatch {
        modifiers: KeyModifiers::NONE,
        key_code: KeyCode::Char('?'),
        action: GlobalAction::CycleDrawer,
    },
    KeyDispatch {
        modifiers: KeyModifiers::SHIFT,
        key_code: KeyCode::Char('?'),
        action: GlobalAction::CycleDrawer,
    },
    // Timeline
    KeyDispatch {
        modifiers: KeyModifiers::NONE,
        key_code: KeyCode::Char('T'),
        action: GlobalAction::ToggleTimeline,
    },
    // Mode cycling
    KeyDispatch {
        modifiers: KeyModifiers::NONE,
        key_code: KeyCode::Tab,
        action: GlobalAction::NextMode,
    },
    KeyDispatch {
        modifiers: KeyModifiers::SHIFT,
        key_code: KeyCode::Tab,
        action: GlobalAction::PrevMode,
    },
    KeyDispatch {
        modifiers: KeyModifiers::CONTROL,
        key_code: KeyCode::Tab,
        action: GlobalAction::NextMode,
    },
    KeyDispatch {
        modifiers: KeyModifiers::CONTROL.union(KeyModifiers::SHIFT),
        key_code: KeyCode::Tab,
        action: GlobalAction::PrevMode,
    },
    // Quit — Esc removed; use Q / q / Ctrl+C (SIGINT)
    KeyDispatch {
        modifiers: KeyModifiers::NONE,
        key_code: KeyCode::Char('q'),
        action: GlobalAction::Quit,
    },
    KeyDispatch {
        modifiers: KeyModifiers::NONE,
        key_code: KeyCode::Char('Q'),
        action: GlobalAction::Quit,
    },
    KeyDispatch {
        modifiers: KeyModifiers::CONTROL,
        key_code: KeyCode::Char('q'),
        action: GlobalAction::Quit,
    },
];

/// Look up a global action by exact (modifiers, key_code) match.
pub fn lookup_global(code: KeyCode, modifiers: KeyModifiers) -> Option<GlobalAction> {
    GLOBAL_DISPATCH
        .iter()
        .find(|d| d.key_code == code && d.modifiers == modifiers)
        .map(|d| d.action)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Global,
    Canvas,
    FontOverview,
    FontCharEditor,
    Dialog,
    Timeline,
}

impl Scope {
    pub fn label(self) -> &'static str {
        match self {
            Scope::Global => "Global",
            Scope::Canvas => "Canvas",
            Scope::FontOverview => "Font Overview",
            Scope::FontCharEditor => "Font Char Editor",
            Scope::Dialog => "Dialog",
            Scope::Timeline => "Timeline",
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
    KeyBinding {
        keys: "Ctrl+O",
        scope: Scope::Global,
        description: "Open file",
    },
    KeyBinding {
        keys: "Ctrl+S",
        scope: Scope::Global,
        description: "Save file",
    },
    KeyBinding {
        keys: "Ctrl+Shift+S",
        scope: Scope::Global,
        description: "Save As",
    },
    KeyBinding {
        keys: "Ctrl+E",
        scope: Scope::Global,
        description: "Export",
    },
    KeyBinding {
        keys: "q / Q",
        scope: Scope::Global,
        description: "Quit",
    },
    KeyBinding {
        keys: "Tab",
        scope: Scope::Global,
        description: "Next mode",
    },
    KeyBinding {
        keys: "Shift+Tab",
        scope: Scope::Global,
        description: "Prev mode",
    },
    KeyBinding {
        keys: "F5",
        scope: Scope::Global,
        description: "Toggle render mode (Fast/Dirty)",
    },
    KeyBinding {
        keys: "F11",
        scope: Scope::Global,
        description: "Toggle zen mode (full canvas)",
    },
    KeyBinding {
        keys: "?",
        scope: Scope::Global,
        description: "Toggle side panel (Layers, Props, Tools, Effects)",
    },
    KeyBinding {
        keys: "T",
        scope: Scope::Global,
        description: "Toggle animation timeline",
    },
    KeyBinding {
        keys: "Ctrl+K",
        scope: Scope::Global,
        description: "Toggle keybindings overlay",
    },
    KeyBinding {
        keys: "Ctrl+Shift+H",
        scope: Scope::Global,
        description: "Toggle undo history panel",
    },
    KeyBinding {
        keys: "Alt+F",
        scope: Scope::Global,
        description: "Open File menu",
    },
    KeyBinding {
        keys: "Alt+E",
        scope: Scope::Global,
        description: "Open Edit menu",
    },
    KeyBinding {
        keys: "Alt+V",
        scope: Scope::Global,
        description: "Open View menu",
    },
    KeyBinding {
        keys: "Alt+T",
        scope: Scope::Global,
        description: "Open Tools menu",
    },
    KeyBinding {
        keys: "Alt+H",
        scope: Scope::Global,
        description: "Open Help menu",
    },
    // Canvas
    KeyBinding {
        keys: "Ctrl+Z",
        scope: Scope::Canvas,
        description: "Undo",
    },
    KeyBinding {
        keys: "Ctrl+Y",
        scope: Scope::Canvas,
        description: "Redo",
    },
    KeyBinding {
        keys: "Ctrl+Shift+Z",
        scope: Scope::Canvas,
        description: "Redo (alternate)",
    },
    KeyBinding {
        keys: "+ / -",
        scope: Scope::Canvas,
        description: "Zoom in / out",
    },
    KeyBinding {
        keys: "b/e/l/v/c/p",
        scope: Scope::Canvas,
        description: "Brush/Eraser/Lasso/Select/Circle/Polygon",
    },
    KeyBinding {
        keys: "g/i/d/a/t",
        scope: Scope::Canvas,
        description: "Fill/Line/Eyedropper/Spray/Text",
    },
    KeyBinding {
        keys: "[ / ]",
        scope: Scope::Canvas,
        description: "Brush size down / up",
    },
    KeyBinding {
        keys: "; / '",
        scope: Scope::Canvas,
        description: "Brush density down / up",
    },
    KeyBinding {
        keys: r"\",
        scope: Scope::Canvas,
        description: "Cycle brush shape",
    },
    KeyBinding {
        keys: "M",
        scope: Scope::Canvas,
        description: "Toggle marker sub-mode (brush tool)",
    },
    // Font Overview
    KeyBinding {
        keys: "↑↓←→",
        scope: Scope::FontOverview,
        description: "Navigate glyph grid",
    },
    KeyBinding {
        keys: "Enter",
        scope: Scope::FontOverview,
        description: "Open glyph editor",
    },
    KeyBinding {
        keys: "Type chars",
        scope: Scope::FontOverview,
        description: "Activate search / filter glyphs",
    },
    KeyBinding {
        keys: "Esc",
        scope: Scope::FontOverview,
        description: "Clear search",
    },
    KeyBinding {
        keys: "A",
        scope: Scope::FontOverview,
        description: "Add glyph",
    },
    KeyBinding {
        keys: "D",
        scope: Scope::FontOverview,
        description: "Delete glyph",
    },
    KeyBinding {
        keys: "C",
        scope: Scope::FontOverview,
        description: "Copy glyph",
    },
    KeyBinding {
        keys: "H",
        scope: Scope::FontOverview,
        description: "Header editor",
    },
    KeyBinding {
        keys: "S",
        scope: Scope::FontOverview,
        description: "Smushing rule editor",
    },
    KeyBinding {
        keys: "T",
        scope: Scope::FontOverview,
        description: "Transform editor",
    },
    // Font Char Editor
    KeyBinding {
        keys: "↑↓←→",
        scope: Scope::FontCharEditor,
        description: "Move cursor in glyph",
    },
    KeyBinding {
        keys: "Space",
        scope: Scope::FontCharEditor,
        description: "Toggle cell",
    },
    KeyBinding {
        keys: "M",
        scope: Scope::FontCharEditor,
        description: "Mirror",
    },
    KeyBinding {
        keys: "F",
        scope: Scope::FontCharEditor,
        description: "Flip",
    },
    KeyBinding {
        keys: "G",
        scope: Scope::FontCharEditor,
        description: "Generate from system font",
    },
    // Timeline
    KeyBinding {
        keys: "T",
        scope: Scope::Global,
        description: "Toggle timeline panel",
    },
    KeyBinding {
        keys: "Shift+T",
        scope: Scope::Global,
        description: "Open tween panel",
    },
    KeyBinding {
        keys: "← / →",
        scope: Scope::Timeline,
        description: "Switch frame",
    },
    KeyBinding {
        keys: "A",
        scope: Scope::Timeline,
        description: "Add frame",
    },
    KeyBinding {
        keys: "Delete",
        scope: Scope::Timeline,
        description: "Delete frame",
    },
    KeyBinding {
        keys: "Enter",
        scope: Scope::Timeline,
        description: "Play animation",
    },
    // Dialog
    KeyBinding {
        keys: "Esc",
        scope: Scope::Dialog,
        description: "Close / cancel",
    },
    KeyBinding {
        keys: "Enter",
        scope: Scope::Dialog,
        description: "Confirm",
    },
    KeyBinding {
        keys: "↑↓",
        scope: Scope::Dialog,
        description: "Navigate items",
    },
];
