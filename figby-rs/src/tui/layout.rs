use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::Borders;

pub const TOOLBOX_BRUSH_HEIGHT: u16 = 10;
const DRAWER_WIDTH: u16 = 22;

/// What the collapsible right drawer shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawerMode {
    /// Show the color palette.
    Palette,
    /// Show brush/tool keybind reference (default).
    BrushKeys,
    /// Show layer panel.
    Layers,
    /// Drawer closed.
    Closed,
}

impl DrawerMode {
    /// Cycle: BrushKeys → Layers → Closed → BrushKeys
    pub fn cycle(self) -> Self {
        match self {
            DrawerMode::Palette => DrawerMode::BrushKeys,
            DrawerMode::BrushKeys => DrawerMode::Layers,
            DrawerMode::Layers => DrawerMode::Closed,
            DrawerMode::Closed => DrawerMode::BrushKeys,
        }
    }

    pub fn is_open(self) -> bool {
        self != DrawerMode::Closed
    }
}

/// Collapsed borders for the toolbox list panel (top portion of left column).
/// Omits BOTTOM — shared with brush/text area below.
pub fn toolbox_list_borders() -> Borders {
    Borders::TOP | Borders::LEFT | Borders::RIGHT
}

/// Collapsed borders for the brush/text panel (bottom portion of left column).
/// Omits TOP — shared with tool list above.
pub fn toolbox_brush_borders() -> Borders {
    Borders::LEFT | Borders::RIGHT | Borders::BOTTOM
}

/// Explicit borders for the right panel column.
/// Same as `Borders::ALL` but documents intent: right panel owns its own edges
/// (canvas omits RIGHT when right panel is open).
pub fn right_panel_borders() -> Borders {
    Borders::ALL
}

/// All widget Rects for one frame, computed in a single pass.
/// Stored on TuiApp so mouse handlers can use last-frame geometry.
#[derive(Debug, Clone, Copy)]
pub struct FrameLayout {
    pub menu: Rect,
    pub tabs: Rect,
    pub main: Rect,
    pub status: Rect,
    /// Full toolbox column (dynamic). None in zen mode.
    pub toolbox_full: Option<Rect>,
    /// Upper portion of toolbox column (for mouse hit-testing tool list items).
    pub toolbox_list: Option<Rect>,
    /// Lower 10 rows of toolbox column (brush / text options).
    pub toolbox_brush: Option<Rect>,
    /// Palette panel below the toolbox in the left column. None in zen mode.
    pub palette: Option<Rect>,
    /// Center canvas area.
    pub canvas: Rect,
    /// Right drawer. None when DrawerMode::Closed or zen mode.
    pub right_panel: Option<Rect>,
}

impl FrameLayout {
    /// Compute layout for the given terminal area.
    ///
    /// Collapsed-border convention (ratatui recipe):
    ///   - Toolbox list block uses `toolbox_list_borders()`: TOP, LEFT, RIGHT
    ///     (omits BOTTOM — shared with brush/text area).
    ///   - Brush/text block uses `toolbox_brush_borders()`: LEFT, RIGHT, BOTTOM
    ///     (omits TOP — shared with tool list).
    ///   - Canvas block omits LEFT when toolbox is visible (shares toolbox's
    ///     right border) and omits RIGHT when right panel is visible.
    ///   - Right panel block uses `right_panel_borders()` = ALL (canvas omits
    ///     its RIGHT when right panel is open, so right panel owns its LEFT).
    ///   - All layouts use `spacing(0)` so adjacent rects share edges exactly.
    pub fn compute(
        area: Rect,
        zen_mode: bool,
        drawer: DrawerMode,
        toolbox_width: u16,
        toolbox_h: u16,
    ) -> Self {
        let vert = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .spacing(0)
        .split(area);

        let menu = vert[0];
        let tabs = vert[1];
        let main = vert[2];
        let status = vert[3];

        if zen_mode {
            return Self {
                menu,
                tabs,
                main,
                status,
                toolbox_full: None,
                toolbox_list: None,
                toolbox_brush: None,
                palette: None,
                canvas: area,
                right_panel: None,
            };
        }

        let h_areas = if drawer.is_open() {
            Layout::horizontal([
                Constraint::Length(toolbox_width),
                Constraint::Fill(1),
                Constraint::Length(DRAWER_WIDTH),
            ])
            .spacing(0)
            .split(main)
        } else {
            Layout::horizontal([Constraint::Length(toolbox_width), Constraint::Fill(1)])
                .spacing(0)
                .split(main)
        };

        let toolbox_full = h_areas[0];
        let canvas = h_areas[1];
        let right_panel = if drawer.is_open() {
            Some(h_areas[2])
        } else {
            None
        };

        // Split the toolbox column: top=toolbox, bottom=palette
        let left_vert = Layout::vertical([Constraint::Length(toolbox_h), Constraint::Min(0)])
            .spacing(0)
            .split(toolbox_full);
        let toolbox_top = left_vert[0];
        let palette_rect = left_vert[1];

        let tb_vert = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(TOOLBOX_BRUSH_HEIGHT),
        ])
        .spacing(0)
        .split(toolbox_top);
        let toolbox_list = tb_vert[0];
        let toolbox_brush = tb_vert[1];

        Self {
            menu,
            tabs,
            main,
            status,
            toolbox_full: Some(toolbox_full),
            toolbox_list: Some(toolbox_list),
            toolbox_brush: Some(toolbox_brush),
            palette: Some(palette_rect),
            canvas,
            right_panel,
        }
    }

    /// Collapsed border flags for the canvas block.
    ///
    /// Omits LEFT when toolbox is present (toolbox provides that edge),
    /// omits RIGHT when right panel is present.
    /// Shared TOP/BOTTOM edges with toolbox/right panel panels are drawn
    /// by both (inherent to ratatui's block model for side-by-side blocks).
    pub fn canvas_borders(&self) -> Borders {
        match (self.toolbox_full.is_some(), self.right_panel.is_some()) {
            (true, true) => Borders::TOP | Borders::BOTTOM,
            (true, false) => Borders::TOP | Borders::RIGHT | Borders::BOTTOM,
            (false, true) => Borders::TOP | Borders::LEFT | Borders::BOTTOM,
            (false, false) => Borders::ALL,
        }
    }
}

impl Default for FrameLayout {
    fn default() -> Self {
        Self::compute(Rect::new(0, 0, 80, 24), false, DrawerMode::BrushKeys, 8, 0)
    }
}
