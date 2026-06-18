use std::collections::BTreeMap;

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::layers::{LayerPanel, LayerStack};
use super::theme::Theme;
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

    pub fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        layer_panel: Option<&LayerPanel>,
        layer_stack: Option<&LayerStack>,
        text_tool: Option<&TextToolState>,
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
            let label = if tab_w >= 4 {
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
            let sep_label = "─".repeat(inner.width as usize);
            frame.render_widget(
                Paragraph::new(sep_label).style(sep_style.add_modifier(Modifier::DIM)),
                sep_rect,
            );
        }

        // Content area
        let content_y = inner.y + 2;
        if content_y >= area.y + area.height {
            return;
        }
        let content_area = Rect {
            x: inner.x,
            y: content_y,
            width: inner.width,
            height: (area.y + area.height).saturating_sub(content_y),
        };
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
                Self::render_props_content(frame, content_area, &self.theme);
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

    fn render_props_content(frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let lines: Vec<Line> = vec![
            Line::from(Span::styled(
                " Tools ",
                Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )),
            Line::from("  b  Brush"),
            Line::from("  e  Eraser"),
            Line::from("  l  Lasso"),
            Line::from("  v  Select"),
            Line::from("  c  Circle sel."),
            Line::from("  p  Polygon sel."),
            Line::from("  g  Fill"),
            Line::from("  i  Line"),
            Line::from("  d  Eyedropper"),
            Line::from("  a  Spray"),
            Line::from("  t  Text"),
            Line::from("  m  Emitter"),
            Line::from(""),
            Line::from(Span::styled(
                " Brush ",
                Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )),
            Line::from("  [  Size down"),
            Line::from("  ]  Size up"),
            Line::from("  ;  Density down"),
            Line::from("  '  Density up"),
            Line::from(r"  \  Cycle shape"),
            Line::from(""),
            Line::from(Span::styled(
                " View ",
                Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )),
            Line::from("  F11  Zen mode"),
            Line::from("  ?    Toggle panel"),
            Line::from("  ^K   All keybinds"),
        ];
        frame.render_widget(
            Paragraph::new(lines).style(Style::default().fg(theme.menu.fg)),
            area,
        );
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
