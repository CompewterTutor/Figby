use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;
use std::collections::BTreeMap;
use unicode_width::UnicodeWidthStr;

use super::super::theme::Theme;
use super::super::AppMode;

const POWERLINE_TRIANGLE: &str = "\u{e0b0}";

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

    fn build_left(&self) -> Vec<Span<'a>> {
        let mut spans: Vec<Span> = Vec::new();
        let sep = Style::default().fg(self.theme.statusbar.separator);

        let mode_icon = self.icon("status_mode", "M");
        spans.push(Span::styled(
            format!(" {} {} ", mode_icon, self.mode_name),
            Style::default()
                .fg(self.mode_color())
                .add_modifier(Modifier::BOLD),
        ));

        spans.push(Span::styled(" │ ", sep));

        let tool_icon = self.icon("status_tool", "T");
        spans.push(Span::raw(format!(" {} {} ", tool_icon, self.tool_name)));

        spans.push(Span::styled(" │ ", sep));

        let pos_icon = self.icon("status_position", "+");
        spans.push(Span::raw(format!(
            " {} X:{} Y:{} ",
            pos_icon, self.cursor.0, self.cursor.1
        )));

        let zoom_icon = self.icon("status_zoom", "Z");
        spans.push(Span::styled(
            format!(" {} {}x ", zoom_icon, self.zoom),
            Style::default().fg(self.theme.statusbar.label),
        ));

        spans
    }

    fn build_middle(&self) -> Vec<Span<'a>> {
        let mut spans: Vec<Span> = Vec::new();
        let sep = Style::default().fg(self.theme.statusbar.separator);

        if let Some(name) = self.font_name {
            let font_icon = self.icon("status_font", "F");
            spans.push(Span::styled(
                format!(" {} {} ", font_icon, name),
                Style::default().fg(self.theme.statusbar.font_name),
            ));
        }

        if self.unsaved {
            let unsaved_icon = self.icon("status_unsaved", "!");
            spans.push(Span::styled(
                format!(" {} ", unsaved_icon),
                Style::default().fg(self.theme.statusbar.unsaved),
            ));
        } else if self.font_name.is_some() {
            let saved_icon = self.icon("status_saved", "*");
            spans.push(Span::styled(
                format!(" {} ", saved_icon),
                Style::default().fg(self.theme.statusbar.saved),
            ));
        }

        if self.font_name.is_some() {
            if let Some(count) = self.glyph_count {
                spans.push(Span::styled(" │ ", sep));
                let glyph_icon = self.icon("status_glyph", "#");
                spans.push(Span::styled(
                    format!(" {} {} ", glyph_icon, count),
                    Style::default().fg(self.theme.statusbar.glyph_count),
                ));
            }
        }

        spans
    }

    fn build_right(&self) -> Vec<Span<'a>> {
        let mut spans: Vec<Span> = Vec::new();
        let sep = Style::default().fg(self.theme.statusbar.separator);

        if let Some(branch) = self.git_branch {
            let branch_icon = self.icon("status_git_branch", "⎇");
            spans.push(Span::styled(
                format!(" {} {} ", branch_icon, branch),
                Style::default().fg(self.theme.statusbar.git_branch),
            ));
            spans.push(Span::styled(" │ ", sep));
        }

        spans.push(Span::styled(
            format!(" FPS:{:.0} ", self.fps),
            Style::default().fg(self.theme.statusbar.fps),
        ));

        spans.push(Span::styled(" │ ", sep));

        let clock_icon = self.icon("status_clock", "🕐");
        spans.push(Span::styled(
            format!(" {} {} ", clock_icon, self.clock_str),
            Style::default().fg(self.theme.statusbar.label),
        ));

        if !self.render_mode.is_empty() {
            spans.push(Span::styled(
                format!(" {} ", self.render_mode),
                Style::default().fg(self.theme.statusbar.label),
            ));
        }

        if self.layer_count > 0 {
            let layer_icon = self.icon("status_layer", "L");
            spans.push(Span::styled(
                format!(" {}:{} ", layer_icon, self.layer_count),
                Style::default().fg(self.theme.statusbar.label),
            ));
        }

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

        let area_w = area.width;

        let left_spans = self.build_left();
        let left_w = left_spans.iter().map(|s| s.content.width()).sum::<usize>() as u16;

        if left_w >= area_w {
            let mode_icon = self.icon("status_mode", "M");
            let mode_text = format!(" {} {} ", mode_icon, self.mode_name);
            let truncated: String = mode_text
                .chars()
                .take((area_w as usize).saturating_sub(2))
                .collect();
            let line = Line::from(vec![Span::styled(
                truncated,
                Style::default()
                    .fg(self.mode_color())
                    .add_modifier(Modifier::BOLD),
            )]);
            buf.set_line(area.x, area.y, &line, area_w);
            return;
        }

        let right_spans = self.build_right();
        let right_w = right_spans.iter().map(|s| s.content.width()).sum::<usize>() as u16;

        let needed = left_w + right_w + 2;
        let sep_style = Style::default().fg(self.theme.statusbar.separator);

        if needed >= area_w {
            let mid = Layout::horizontal([
                Constraint::Length(left_w),
                Constraint::Length(1),
                Constraint::Fill(1),
            ])
            .split(area);

            buf.set_line(mid[0].x, area.y, &Line::from(left_spans), mid[0].width);
            buf.set_string(mid[1].x, area.y, POWERLINE_TRIANGLE, sep_style);
            buf.set_line(
                mid[2].x,
                area.y,
                &Line::from(self.build_middle()),
                mid[2].width,
            );
            return;
        }

        let chunks = Layout::horizontal([
            Constraint::Length(left_w),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Length(right_w),
        ])
        .split(area);

        buf.set_line(
            chunks[0].x,
            area.y,
            &Line::from(left_spans),
            chunks[0].width,
        );
        buf.set_string(chunks[1].x, area.y, POWERLINE_TRIANGLE, sep_style);
        buf.set_line(
            chunks[2].x,
            area.y,
            &Line::from(self.build_middle()),
            chunks[2].width,
        );
        buf.set_string(chunks[3].x, area.y, POWERLINE_TRIANGLE, sep_style);
        buf.set_line(
            chunks[4].x,
            area.y,
            &Line::from(right_spans),
            chunks[4].width,
        );
    }
}
