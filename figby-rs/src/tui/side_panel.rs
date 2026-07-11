use std::collections::BTreeMap;

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::brush::BrushState;
use super::canvas::CanvasCell;
use super::layers::{LayerPanel, LayerStack};
use super::light_panel::LightPanel;
use super::lighting::Scene;
use super::particles::ParticleConfig;
use super::theme::Theme;
use super::toolbox::Tool;
use super::tools::text::TextToolState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabId {
    Layers,
    Props,
    Text,
    Libraries,
    Effects,
}

impl TabId {
    pub fn icon_key(&self) -> &str {
        match self {
            TabId::Layers => "layer_new",
            TabId::Props => "settings_open",
            TabId::Text => "tool_text",
            TabId::Libraries => "brush_shape_custom",
            TabId::Effects => "image_contrast",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            TabId::Layers => "Layers",
            TabId::Props => "Props",
            TabId::Text => "Text",
            TabId::Libraries => "Libraries",
            TabId::Effects => "Effects",
        }
    }

    pub fn all() -> &'static [TabId] {
        &[
            TabId::Layers,
            TabId::Props,
            TabId::Text,
            TabId::Libraries,
            TabId::Effects,
        ]
    }

    pub fn next(self) -> Self {
        match self {
            TabId::Layers => TabId::Props,
            TabId::Props => TabId::Text,
            TabId::Text => TabId::Libraries,
            TabId::Libraries => TabId::Effects,
            TabId::Effects => TabId::Layers,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            TabId::Layers => TabId::Effects,
            TabId::Props => TabId::Layers,
            TabId::Text => TabId::Props,
            TabId::Libraries => TabId::Text,
            TabId::Effects => TabId::Libraries,
        }
    }
}

pub struct SidePanel {
    pub open: bool,
    pub active_tab: TabId,
    pub icons: BTreeMap<String, String>,
    pub theme: Theme,
}

impl SidePanel {
    pub fn new(icons: BTreeMap<String, String>, theme: Theme) -> Self {
        Self {
            open: false,
            active_tab: TabId::Layers,
            icons,
            theme,
        }
    }

    pub fn toggle_open(&mut self) {
        self.open = !self.open;
    }

    pub fn set_active_tab(&mut self, id: TabId) {
        self.active_tab = id;
    }

    pub fn cycle_tab(&mut self, forward: bool) {
        self.active_tab = if forward {
            self.active_tab.next()
        } else {
            self.active_tab.prev()
        };
    }

    pub fn tab_rects(&self, area: Rect) -> Vec<Rect> {
        let tab_count = TabId::all().len() as u16;
        if tab_count == 0 || area.width < 2 {
            return Vec::new();
        }
        let tab_w = (area.width - 2).max(tab_count) / tab_count;
        (0..tab_count)
            .map(|i| Rect {
                x: area.x + 1 + i * tab_w,
                y: area.y + 1,
                width: tab_w,
                height: 1,
            })
            .collect()
    }

    pub fn tab_at_pos(&self, col: u16, row: u16, area: Rect) -> Option<TabId> {
        let rects = self.tab_rects(area);
        for (i, r) in rects.iter().enumerate() {
            if col >= r.x && col < r.x + r.width && row == r.y {
                return TabId::all().get(i).copied();
            }
        }
        None
    }

    /// Compute the content rect below the tab header + separator, matching
    /// the layout `render` actually draws into. Callers that need to
    /// translate mouse coordinates into a tab's content area (e.g. the
    /// layers panel's click handling) must use this instead of the raw
    /// panel `area`, since `render` draws its own border plus a two-row
    /// tab bar before handing off to the per-tab content.
    pub fn content_area(&self, area: Rect) -> Rect {
        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        let content_y = inner.y + 2;
        if content_y >= area.y + area.height {
            return Rect {
                x: inner.x,
                y: content_y,
                width: inner.width,
                height: 0,
            };
        }
        Rect {
            x: inner.x,
            y: content_y,
            width: inner.width,
            height: (area.y + area.height).saturating_sub(content_y),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        layer_panel: Option<&mut LayerPanel>,
        layer_stack: Option<&LayerStack>,
        active_tool: Tool,
        brush: &BrushState,
        text_tool: Option<&TextToolState>,
        eyedropper_sample: Option<CanvasCell>,
        fill_threshold: u8,
        emitter_config: Option<&ParticleConfig>,
        canvas_width: u16,
        canvas_height: u16,
        font_name: Option<&str>,
        zoom: u8,
        lighting_scene: Option<&Scene>,
        lighting_panel: Option<&LightPanel>,
    ) {
        let block = Block::default().borders(Borders::ALL).style(
            Style::default()
                .bg(self.theme.menu.dropdown_bg)
                .fg(self.theme.menu.fg),
        );
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width < 4 || inner.height < 3 {
            return;
        }

        // Tab header row
        let tabs = TabId::all();
        let tab_count = tabs.len() as u16;
        let total_tab_w = inner.width.saturating_sub(2);
        let tab_w = total_tab_w / tab_count;

        let mut tab_x = inner.x;
        for tab in tabs {
            let icon = self
                .icons
                .get(tab.icon_key())
                .map(|s| s.as_str())
                .unwrap_or("");
            let label = if tab_w >= 6 {
                format!("{} {}", icon, tab.display_name())
            } else if tab_w >= 2 {
                icon.to_string()
            } else {
                String::new()
            };
            let is_active = *tab == self.active_tab;
            let style = if is_active {
                Style::default()
                    .fg(self.theme.general.primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(self.theme.general.secondary)
            };
            let tab_rect = Rect {
                x: tab_x,
                y: inner.y,
                width: tab_w.min(inner.x + inner.width - tab_x),
                height: 1,
            };
            if tab_rect.width >= label.len() as u16 {
                frame.render_widget(Paragraph::new(label).style(style), tab_rect);
            }
            tab_x += tab_w;
        }

        // Separator line below tabs
        let sep_style = Style::default().fg(self.theme.general.secondary);
        let sep_rect = Rect {
            x: inner.x,
            y: inner.y + 1,
            width: inner.width,
            height: 1,
        };
        if inner.width > 0 && inner.y + 1 < area.y + area.height {
            let hint = " ←/→ tabs ";
            let sep_label = if inner.width as usize >= hint.len() + 4 {
                format!("{:─^width$}", hint, width = inner.width as usize)
            } else {
                "─".repeat(inner.width as usize)
            };
            frame.render_widget(
                Paragraph::new(sep_label).style(sep_style.add_modifier(Modifier::DIM)),
                sep_rect,
            );
        }

        // Content area
        let content_area = self.content_area(area);
        if content_area.height == 0 {
            return;
        }

        match self.active_tab {
            TabId::Layers => {
                if let (Some(panel), Some(stack)) = (layer_panel, layer_stack) {
                    panel.render_with_stack(frame, content_area, stack);
                }
            }
            TabId::Props => {
                Self::render_tool_props(
                    frame,
                    content_area,
                    &self.theme,
                    active_tool,
                    brush,
                    text_tool,
                    eyedropper_sample,
                    fill_threshold,
                    emitter_config,
                    canvas_width,
                    canvas_height,
                    font_name,
                    zoom,
                    lighting_scene,
                    lighting_panel,
                );
            }
            TabId::Text => {
                if let Some(tt) = text_tool {
                    Self::render_text_content(frame, content_area, tt, &self.theme);
                }
            }
            TabId::Libraries => {
                Self::render_placeholder(frame, content_area, "Libraries", &self.theme);
            }
            TabId::Effects => {
                Self::render_placeholder(frame, content_area, "Effects", &self.theme);
            }
        }
    }

    /// Dispatch tool property content based on active tool.
    /// Image/font info always rendered at bottom.
    #[allow(clippy::too_many_arguments)]
    fn render_tool_props(
        frame: &mut Frame<'_>,
        area: Rect,
        theme: &Theme,
        active_tool: Tool,
        brush: &BrushState,
        text_tool: Option<&TextToolState>,
        eyedropper_sample: Option<CanvasCell>,
        fill_threshold: u8,
        emitter_config: Option<&ParticleConfig>,
        canvas_width: u16,
        canvas_height: u16,
        font_name: Option<&str>,
        zoom: u8,
        lighting_scene: Option<&Scene>,
        lighting_panel: Option<&LightPanel>,
    ) {
        let mut lines: Vec<Line> = Vec::new();

        match active_tool {
            Tool::Brush | Tool::Spray | Tool::Eraser => {
                Self::add_brush_props(&mut lines, brush);
            }
            Tool::Text => {
                if let Some(tt) = text_tool {
                    Self::add_text_props(&mut lines, tt);
                }
            }
            Tool::Eyedropper => {
                Self::add_eyedropper_props(&mut lines, eyedropper_sample);
            }
            Tool::Fill => {
                Self::add_fill_props(&mut lines, fill_threshold);
            }
            Tool::Emitter => {
                Self::add_emitter_props(&mut lines, emitter_config);
            }
            Tool::Lighting => {
                Self::add_lighting_props(&mut lines, lighting_scene, lighting_panel, theme);
            }
            _ => {
                Self::add_tool_keybinds(&mut lines);
            }
        }

        // Separator + image/font info at bottom
        lines.push(Line::from(""));
        Self::add_image_font_info(&mut lines, canvas_width, canvas_height, font_name, zoom);

        let para = Paragraph::new(lines).style(Style::default().fg(theme.menu.fg));
        frame.render_widget(para, area);
    }

    fn add_brush_props(lines: &mut Vec<Line>, brush: &BrushState) {
        lines.push(Line::from(Span::styled(
            " Brush ",
            Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )));
        lines.push(Line::from(vec![
            Span::styled("Size:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" {}", brush.size)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Shape:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" {}", brush.shape.name())),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Mode:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" {}", brush.sub_mode.name())),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Density:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" {}%", brush.density)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Char:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" '{}'", brush.ch)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::raw(" [  Size down")));
        lines.push(Line::from(Span::raw(" ]  Size up")));
        lines.push(Line::from(Span::raw(r" \  Cycle shape")));
        if matches!(brush.shape, crate::tui::brush::BrushShape::SprayPaint) {
            lines.push(Line::from(Span::raw(" ;  Density down")));
            lines.push(Line::from(Span::raw(" '  Density up")));
        }
    }

    fn add_text_props(lines: &mut Vec<Line>, tt: &TextToolState) {
        lines.push(Line::from(Span::styled(
            " Text Tool ",
            Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )));
        let font_name = if tt.font_index < tt.available_fonts.len() {
            &tt.available_fonts[tt.font_index]
        } else {
            "?"
        };
        lines.push(Line::from(vec![
            Span::styled("Font:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" {}", font_name)),
        ]));
        let just_str = match tt.justification {
            crate::render::Justification::Left => "Left",
            crate::render::Justification::Center => "Center",
            crate::render::Justification::Right => "Right",
        };
        lines.push(Line::from(vec![
            Span::styled("Just:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" {}", just_str)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Scale:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" {}", tt.scale)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::raw(" Click canvas to type")));
    }

    fn add_eyedropper_props(lines: &mut Vec<Line>, sample: Option<CanvasCell>) {
        lines.push(Line::from(Span::styled(
            " Eyedropper ",
            Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )));
        match sample {
            Some(cell) => {
                lines.push(Line::from(vec![
                    Span::styled("Char:", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(format!(" '{}'", cell.ch)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("FG:", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(format!(" {:?}", cell.fg)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("BG:", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(format!(" {:?}", cell.bg)),
                ]));
            }
            None => {
                lines.push(Line::from(Span::raw(" No sample yet")));
                lines.push(Line::from(Span::raw(" Click canvas to sample")));
            }
        }
    }

    fn add_fill_props(lines: &mut Vec<Line>, threshold: u8) {
        lines.push(Line::from(Span::styled(
            " Fill ",
            Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )));
        lines.push(Line::from(vec![
            Span::styled("Threshold:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" {}", threshold)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::raw(" Click canvas to flood fill")));
    }

    fn add_emitter_props(lines: &mut Vec<Line>, config: Option<&ParticleConfig>) {
        lines.push(Line::from(Span::styled(
            " Emitter ",
            Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )));
        if let Some(cfg) = config {
            lines.push(Line::from(vec![
                Span::styled("Rate:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!(" {:.1}/s", cfg.spawn_rate)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Lifetime:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!(" {:.1}–{:.1}s", cfg.lifetime_min, cfg.lifetime_max)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Shape:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!(" {}", cfg.emission_shape.display_name())),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Char:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!(" '{}'", cfg.character)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Size:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!(" {}", cfg.size)),
            ]));
        } else {
            lines.push(Line::from(Span::raw(" No config")));
        }
    }

    fn add_lighting_props(
        lines: &mut Vec<Line>,
        scene: Option<&Scene>,
        panel: Option<&LightPanel>,
        theme: &Theme,
    ) {
        lines.push(Line::from(Span::styled(
            " Lighting ",
            Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )));
        match (scene, panel) {
            (Some(scene), Some(panel)) => {
                lines.extend(LightPanel::build_lines(
                    scene,
                    panel.selected_index(),
                    theme,
                ));
            }
            _ => {
                lines.push(Line::from(Span::raw(" No scene yet")));
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::raw(" A/D/P  Add Amb/Dir/Point")));
        lines.push(Line::from(Span::raw(" ↑/↓  Select light")));
        lines.push(Line::from(Span::raw(" ←/→  Move (Point)")));
        lines.push(Line::from(Span::raw(" -/+  Intensity")));
        lines.push(Line::from(Span::raw(" Del  Remove light")));
    }

    fn add_tool_keybinds(lines: &mut Vec<Line>) {
        lines.push(Line::from(Span::styled(
            " Tools ",
            Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )));
        lines.push(Line::from("  b  Brush"));
        lines.push(Line::from("  u  Move"));
        lines.push(Line::from("  e  Eraser"));
        lines.push(Line::from("  l  Lasso"));
        lines.push(Line::from("  v  Select"));
        lines.push(Line::from("  c  Circle sel."));
        lines.push(Line::from("  p  Polygon sel."));
        lines.push(Line::from("  g  Fill"));
        lines.push(Line::from("  i  Line"));
        lines.push(Line::from("  d  Eyedropper"));
        lines.push(Line::from("  a  Spray"));
        lines.push(Line::from("  t  Text"));
        lines.push(Line::from("  m  Emitter"));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " View ",
            Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )));
        lines.push(Line::from("  F11  Zen mode"));
        lines.push(Line::from("  ?    Toggle panel"));
        lines.push(Line::from("  ^K   All keybinds"));
    }

    fn add_image_font_info(
        lines: &mut Vec<Line>,
        canvas_width: u16,
        canvas_height: u16,
        font_name: Option<&str>,
        zoom: u8,
    ) {
        lines.push(Line::from(Span::styled(
            " Image / Font Info ",
            Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )));
        lines.push(Line::from(vec![
            Span::styled("Size:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" {}×{}", canvas_width, canvas_height)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Zoom:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" {}×", zoom)),
        ]));
        if let Some(name) = font_name {
            lines.push(Line::from(vec![
                Span::styled("Font:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!(" {}", name)),
            ]));
        }
    }

    fn render_text_content(frame: &mut Frame<'_>, area: Rect, tt: &TextToolState, _theme: &Theme) {
        let font_name = if tt.font_index < tt.available_fonts.len() {
            &tt.available_fonts[tt.font_index]
        } else {
            "?"
        };
        let just_str = match tt.justification {
            crate::render::Justification::Left => "Left",
            crate::render::Justification::Center => "Center",
            crate::render::Justification::Right => "Right",
        };
        let lines: Vec<Line> = vec![
            Line::from(vec![
                Span::styled("Font:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!(" {}", font_name)),
            ]),
            Line::from(vec![
                Span::styled("Just:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!(" {}", just_str)),
            ]),
            Line::from(vec![
                Span::styled("Scale:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!(" {}", tt.scale)),
            ]),
        ];
        frame.render_widget(Paragraph::new(lines), area);
    }

    fn render_placeholder(frame: &mut Frame<'_>, area: Rect, name: &str, theme: &Theme) {
        let text = format!(" {name} — coming soon ");
        let para = Paragraph::new(text).style(
            Style::default()
                .fg(theme.general.secondary)
                .add_modifier(Modifier::DIM),
        );
        frame.render_widget(para, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_area_matches_render_layout() {
        // area: 30x20 panel. Outer border (1) + tab row (1) + separator (1)
        // = content starts 3 rows down from the panel's own y, 1 col in.
        let panel = SidePanel::new(BTreeMap::new(), Theme::default());
        let area = Rect::new(0, 0, 30, 20);
        let content = panel.content_area(area);
        assert_eq!(content.x, 1);
        assert_eq!(content.y, 3);
        assert_eq!(content.width, 28);
        assert_eq!(content.height, 17);
    }

    #[test]
    fn test_content_area_too_small_yields_zero_height() {
        let panel = SidePanel::new(BTreeMap::new(), Theme::default());
        let area = Rect::new(0, 0, 30, 3);
        let content = panel.content_area(area);
        assert_eq!(content.height, 0);
    }
}
