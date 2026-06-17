use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;
use std::collections::BTreeMap;
use unicode_width::UnicodeWidthStr;

use super::super::theme::Theme;
use super::super::AppMode;

pub struct StatusBarWidget<'a> {
    mode: AppMode,
    mode_name: &'a str,
    cursor: (u16, u16),
    zoom: u8,
    tool_name: &'a str,
    unsaved: bool,
    font_name: Option<&'a str>,
    glyph_count: Option<usize>,
    git_branch: Option<&'a str>,
    fps: f64,
    render_mode: &'a str,
    clock_str: &'a str,
    layer_count: u8,
    undo_count: usize,
    throbber_text: &'a str,
    icons: &'a BTreeMap<String, String>,
    theme: &'a Theme,
}

impl<'a> StatusBarWidget<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mode: AppMode,
        mode_name: &'a str,
        cursor: (u16, u16),
        zoom: u8,
        tool_name: &'a str,
        unsaved: bool,
        font_name: Option<&'a str>,
        glyph_count: Option<usize>,
        git_branch: Option<&'a str>,
        fps: f64,
        render_mode: &'a str,
        clock_str: &'a str,
        layer_count: u8,
        undo_count: usize,
        throbber_text: &'a str,
        icons: &'a BTreeMap<String, String>,
        theme: &'a Theme,
    ) -> Self {
        Self {
            mode,
            mode_name,
            cursor,
            zoom,
            tool_name,
            unsaved,
            font_name,
            glyph_count,
            git_branch,
            fps,
            render_mode,
            clock_str,
            layer_count,
            undo_count,
            throbber_text,
            icons,
            theme,
        }
    }

    fn icon(&self, key: &str, fallback: &'static str) -> &str {
        self.icons.get(key).map_or(fallback, |s| s.as_str())
    }

    fn mode_color(&self) -> ratatui::style::Color {
        match self.mode {
            AppMode::FontEditor => self.theme.statusbar.mode_font,
            AppMode::ImageEditor => self.theme.statusbar.mode_image,
            AppMode::AsciiPreview => self.theme.statusbar.mode_ascii,
        }
    }

    fn build_p1(&self) -> Vec<Span<'a>> {
        let mut spans: Vec<Span> = Vec::new();

        // Mode badge
        let mode_icon = self.icon("status_mode", "M");
        let mode_text = format!(" {} {} ", mode_icon, self.mode_name);
        spans.push(Span::styled(
            mode_text,
            Style::default()
                .fg(self.mode_color())
                .add_modifier(Modifier::BOLD),
        ));

        // Separator
        let sep = Style::default().fg(self.theme.statusbar.separator);
        spans.push(Span::styled(" │ ", sep));

        // Tool
        let tool_icon = self.icon("status_tool", "T");
        let tool_text = format!(" {} {} ", tool_icon, self.tool_name);
        spans.push(Span::raw(tool_text));

        // Cursor position
        let pos_icon = self.icon("status_position", "+");
        let cursor_text = format!(" {} X:{} Y:{} ", pos_icon, self.cursor.0, self.cursor.1);
        spans.push(Span::raw(cursor_text));

        // Zoom level
        let zoom_icon = self.icon("status_zoom", "Z");
        let zoom_text = format!(" {} {}x ", zoom_icon, self.zoom);
        spans.push(Span::raw(zoom_text));

        // Unsaved indicator
        if self.unsaved {
            let unsaved_icon = self.icon("status_unsaved", "!");
            spans.push(Span::styled(
                format!(" {} ", unsaved_icon),
                Style::default().fg(self.theme.statusbar.unsaved),
            ));
        } else {
            let saved_icon = self.icon("status_saved", "*");
            spans.push(Span::styled(
                format!(" {} ", saved_icon),
                Style::default().fg(self.theme.statusbar.saved),
            ));
        }

        spans
    }

    fn build_p2(&self) -> Vec<Span<'a>> {
        let mut spans: Vec<Span> = Vec::new();
        let sep = Style::default().fg(self.theme.statusbar.separator);
        spans.push(Span::styled(" │ ", sep));

        // Font name + glyph count
        if let Some(name) = self.font_name {
            let font_icon = self.icon("status_font", "F");
            let glyph_str = self
                .glyph_count
                .map(|c| format!("{}", c))
                .unwrap_or_default();
            let glyph_icon = self.icon("status_glyph", "#");
            spans.push(Span::styled(
                format!(" {} {} {} {}", font_icon, name, glyph_icon, glyph_str),
                Style::default().fg(self.theme.statusbar.font_name),
            ));
        }

        // Git branch
        if let Some(branch) = self.git_branch {
            let branch_icon = self.icon("status_git_branch", "⎇");
            spans.push(Span::styled(
                format!(" {} {} ", branch_icon, branch),
                Style::default().fg(self.theme.statusbar.git_branch),
            ));
        }

        spans
    }

    fn build_p3(&self) -> Vec<Span<'a>> {
        let mut spans: Vec<Span> = Vec::new();
        let sep = Style::default().fg(self.theme.statusbar.separator);
        spans.push(Span::styled(" │ ", sep));

        let fps_str = format!("FPS:{:.0}", self.fps);
        spans.push(Span::styled(
            fps_str,
            Style::default().fg(self.theme.statusbar.fps),
        ));

        if !self.render_mode.is_empty() {
            spans.push(Span::styled(
                format!(" {}", self.render_mode),
                Style::default().fg(self.theme.statusbar.label),
            ));
        }

        spans
    }

    fn build_p4(&self) -> Vec<Span<'a>> {
        let mut spans: Vec<Span> = Vec::new();
        let sep = Style::default().fg(self.theme.statusbar.separator);
        spans.push(Span::styled(" │ ", sep));

        let clock_icon = self.icon("status_clock", "🕐");
        spans.push(Span::styled(
            format!(" {} {} ", clock_icon, self.clock_str),
            Style::default().fg(self.theme.statusbar.label),
        ));

        let layer_icon = self.icon("status_layer", "L");
        spans.push(Span::styled(
            format!(" {}:{} ", layer_icon, self.layer_count),
            Style::default().fg(self.theme.statusbar.label),
        ));

        if self.undo_count > 0 {
            let undo_icon = self.icon("status_undo", "U");
            spans.push(Span::styled(
                format!(" {}:{} ", undo_icon, self.undo_count),
                Style::default().fg(self.theme.statusbar.label),
            ));
        }

        if !self.throbber_text.is_empty() {
            spans.push(Span::raw(format!(" {} ", self.throbber_text)));
        }

        spans
    }
}

impl<'a> Widget for StatusBarWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 {
            return;
        }

        let width = area.width as usize;
        let mut all_spans: Vec<Span<'a>> = Vec::new();

        all_spans.extend(self.build_p1());
        let p1_len: usize = all_spans.iter().map(|s| s.content.width()).sum();

        if p1_len >= width {
            // P1 doesn't even fit — truncate mode badge
            all_spans.clear();
            let mode_icon = self.icon("status_mode", "M");
            let mode_text = format!(" {} {} ", mode_icon, self.mode_name);
            let truncated: String = mode_text.chars().take(width.saturating_sub(2)).collect();
            all_spans.push(Span::styled(
                truncated,
                Style::default()
                    .fg(self.mode_color())
                    .add_modifier(Modifier::BOLD),
            ));
            let line = Line::from(all_spans);
            buf.set_line(area.x, area.y, &line, area.width);
            return;
        }

        // P2 (width >= 60)
        if width >= 60 {
            let p2 = self.build_p2();
            all_spans.extend(p2);
        }

        let sofar: usize = all_spans.iter().map(|s| s.content.width()).sum();
        if sofar < width && width >= 80 {
            let p3 = self.build_p3();
            all_spans.extend(p3);
        }

        let sofar: usize = all_spans.iter().map(|s| s.content.width()).sum();
        if sofar < width && width >= 100 {
            let p4 = self.build_p4();
            all_spans.extend(p4);
        }

        // If anything left over, just render what we have
        let line = Line::from(all_spans);
        buf.set_line(area.x, area.y, &line, area.width);
    }
}
