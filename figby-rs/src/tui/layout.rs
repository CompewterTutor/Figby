use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::Borders;

const TOOLBOX_WIDTH: u16 = 8;
const TOOLBOX_BRUSH_HEIGHT: u16 = 10;
const DRAWER_WIDTH: u16 = 22;

/// What the collapsible right drawer shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawerMode {
    /// Show the color palette (default).
    Palette,
    /// Show brush/tool keybind reference.
    BrushKeys,
    /// Drawer closed.
    Closed,
}

impl DrawerMode {
    /// Cycle: Palette → BrushKeys → Closed → Palette
    pub fn cycle(self) -> Self {
        match self {
            DrawerMode::Palette => DrawerMode::BrushKeys,
            DrawerMode::BrushKeys => DrawerMode::Closed,
            DrawerMode::Closed => DrawerMode::Palette,
        }
    }

    pub fn is_open(self) -> bool {
        self != DrawerMode::Closed
    }
}

/// All widget Rects for one frame, computed in a single pass.
/// Stored on TuiApp so mouse handlers can use last-frame geometry.
#[derive(Debug, Clone, Copy)]
pub struct FrameLayout {
    pub menu: Rect,
    pub tabs: Rect,
    pub main: Rect,
    pub status: Rect,
    /// Full toolbox column (8 wide). None in zen mode.
    pub toolbox_full: Option<Rect>,
    /// Upper portion of toolbox column (for mouse hit-testing tool list items).
    pub toolbox_list: Option<Rect>,
    /// Lower 10 rows of toolbox column (brush / text options).
    pub toolbox_brush: Option<Rect>,
    /// Center canvas area.
    pub canvas: Rect,
    /// Right drawer. None when DrawerMode::Closed or zen mode.
    pub right_panel: Option<Rect>,
}

impl FrameLayout {
    /// Compute layout for the given terminal area.
    ///
    /// Collapsed-border convention (ratatui recipe):
    ///   - Toolbox block uses `Borders::ALL` (provides its own right border).
    ///   - Canvas block omits LEFT border when toolbox is visible (shares
    ///     toolbox's right border) and omits RIGHT when right panel is visible.
    ///   - Right panel block omits LEFT border (shares canvas's right if canvas
    ///     kept it, or toolbox's if canvas omitted both sides).
    pub fn compute(area: Rect, zen_mode: bool, drawer: DrawerMode) -> Self {
        let vert = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(3),
        ])
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
                canvas: area,
                right_panel: None,
            };
        }

        let h_areas = if drawer.is_open() {
            Layout::horizontal([
                Constraint::Length(TOOLBOX_WIDTH),
                Constraint::Fill(1),
                Constraint::Length(DRAWER_WIDTH),
            ])
            .split(main)
        } else {
            Layout::horizontal([Constraint::Length(TOOLBOX_WIDTH), Constraint::Fill(1)]).split(main)
        };

        let toolbox_full = h_areas[0];
        let canvas = h_areas[1];
        let right_panel = if drawer.is_open() {
            Some(h_areas[2])
        } else {
            None
        };

        let tb_vert = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(TOOLBOX_BRUSH_HEIGHT),
        ])
        .split(toolbox_full);
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
            canvas,
            right_panel,
        }
    }

    /// Collapsed border flags for the canvas block.
    ///
    /// Omits LEFT when toolbox is present (toolbox provides that edge),
    /// omits RIGHT when right panel is present.
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
        Self::compute(Rect::new(0, 0, 80, 24), false, DrawerMode::Palette)
    }
}
