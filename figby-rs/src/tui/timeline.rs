use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{StatefulWidget, Widget};

#[derive(Debug, Clone)]
pub struct TimelineFrame {
    pub thumbnail: Vec<Vec<char>>,
    pub has_keyframe: bool,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct TimelineTheme {
    pub playhead: Color,
    pub keyframe: Color,
    pub ruler: Color,
    pub thumbnail_border: Color,
    pub thumbnail_bg: Color,
    pub active_frame_border: Color,
}

impl Default for TimelineTheme {
    fn default() -> Self {
        Self {
            playhead: Color::Red,
            keyframe: Color::Yellow,
            ruler: Color::DarkGray,
            thumbnail_border: Color::DarkGray,
            thumbnail_bg: Color::Reset,
            active_frame_border: Color::Cyan,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnimationTimeline {
    pub frame_thumb_width: u16,
    pub frame_thumb_height: u16,
    pub frame_gap: u16,
    pub visible_frames: usize,
    pub theme: TimelineTheme,
}

#[derive(Debug, Clone)]
pub struct TimelineState {
    pub frames: Vec<TimelineFrame>,
    pub current_frame: usize,
    pub scroll_offset: usize,
    pub playing: bool,
    pub fps: u8,
}

impl Default for TimelineState {
    fn default() -> Self {
        Self {
            frames: Vec::new(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
        }
    }
}

impl Widget for &AnimationTimeline {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, area.y)) {
                cell.set_char('─');
                cell.set_style(Style::default().fg(self.theme.ruler));
            }
        }
    }
}

impl StatefulWidget for &AnimationTimeline {
    type State = TimelineState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.width == 0 || area.height == 0 || state.frames.is_empty() {
            return;
        }

        let slot_w = self.frame_thumb_width + self.frame_gap;
        if slot_w == 0 {
            return;
        }

        let total_rows = 1 + self.frame_thumb_height + 1 + 1;
        if area.height < total_rows {
            return;
        }

        let max_frames = self.visible_frames.min((area.width / slot_w) as usize);
        let start = state.scroll_offset.min(state.frames.len());
        let end = (start + max_frames).min(state.frames.len());

        for vis_i in 0..(end - start) {
            let frame_idx = start + vis_i;
            let x_start = area.x + (vis_i as u16) * slot_w;
            let frame = &state.frames[frame_idx];
            let is_active = frame_idx == state.current_frame;

            let ruler_y = area.y;
            if is_active {
                if let Some(cell) = buf.cell_mut((x_start, ruler_y)) {
                    cell.set_char('▼');
                    cell.set_style(Style::default().fg(self.theme.playhead));
                }
            } else {
                let label = format!("{}", frame_idx);
                for (ci, ch) in label.chars().enumerate() {
                    let cx = x_start + ci as u16;
                    if cx < area.x + area.width {
                        if let Some(cell) = buf.cell_mut((cx, ruler_y)) {
                            cell.set_char(ch);
                            cell.set_style(Style::default().fg(self.theme.ruler));
                        }
                    }
                }
            }

            let thumb_y = area.y + 1;
            for ty in 0..self.frame_thumb_height.min(area.height - 1) {
                let cy = thumb_y + ty;
                if cy >= area.y + area.height {
                    break;
                }
                for tx in 0..self.frame_thumb_width {
                    let cx = x_start + tx;
                    if cx >= area.x + area.width {
                        break;
                    }
                    if let Some(cell) = buf.cell_mut((cx, cy)) {
                        let ch = frame
                            .thumbnail
                            .get(ty as usize)
                            .and_then(|row| row.get(tx as usize))
                            .copied()
                            .unwrap_or(' ');
                        cell.set_char(ch);
                        if is_active {
                            cell.set_style(Style::default().fg(self.theme.active_frame_border));
                        }
                    }
                }
            }

            let marker_y = area.y + 1 + self.frame_thumb_height;
            if marker_y < area.y + area.height {
                let marker = if frame.has_keyframe { '◆' } else { '·' };
                if let Some(cell) = buf.cell_mut((x_start, marker_y)) {
                    cell.set_char(marker);
                    if frame.has_keyframe {
                        cell.set_style(Style::default().fg(self.theme.keyframe));
                    }
                }
            }

            let bottom_y = area.y + 1 + self.frame_thumb_height + 1;
            if bottom_y < area.y + area.height {
                for (ci, ch) in frame.label.chars().enumerate() {
                    let cx = x_start + ci as u16;
                    if cx >= area.x + area.width {
                        break;
                    }
                    if let Some(cell) = buf.cell_mut((cx, bottom_y)) {
                        cell.set_char(ch);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_timeline(
        thumb_w: u16,
        thumb_h: u16,
        gap: u16,
        visible: usize,
    ) -> AnimationTimeline {
        AnimationTimeline {
            frame_thumb_width: thumb_w,
            frame_thumb_height: thumb_h,
            frame_gap: gap,
            visible_frames: visible,
            theme: TimelineTheme::default(),
        }
    }

    fn make_frame(thumb: Vec<Vec<char>>, has_kf: bool, label: &str) -> TimelineFrame {
        TimelineFrame {
            thumbnail: thumb,
            has_keyframe: has_kf,
            label: label.to_string(),
        }
    }

    #[test]
    fn test_timeline_basic_render() {
        let timeline = make_test_timeline(3, 2, 1, 5);
        let mut state = TimelineState {
            frames: (0..5)
                .map(|i| make_frame(vec![vec!['X'; 3]; 2], i == 2, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
        };

        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        StatefulWidget::render(&timeline, area, &mut buf, &mut state);

        let cell = buf.cell((0, 0)).unwrap();
        assert_eq!(cell.symbol(), "▼", "playhead should be at frame 0");
    }

    #[test]
    fn test_timeline_playhead_update() {
        let timeline = make_test_timeline(3, 2, 1, 5);
        let mut state = TimelineState {
            frames: (0..5)
                .map(|i| make_frame(vec![vec!['X'; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 3,
            scroll_offset: 0,
            playing: false,
            fps: 12,
        };

        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        StatefulWidget::render(&timeline, area, &mut buf, &mut state);

        let slot_w = timeline.frame_thumb_width + timeline.frame_gap;
        let frame_x = 3 * slot_w;
        let cell = buf.cell((frame_x, 1)).unwrap();
        assert_eq!(
            cell.style().fg,
            Some(Color::Cyan),
            "frame 3 thumbnail should have active style"
        );

        let playhead_cell = buf.cell((frame_x, 0)).unwrap();
        assert_eq!(playhead_cell.symbol(), "▼", "playhead should be at frame 3");
    }

    #[test]
    fn test_timeline_constraints() {
        let thumb_w = 5u16;
        let thumb_h = 3u16;
        let timeline = make_test_timeline(thumb_w, thumb_h, 1, 3);
        let mut state = TimelineState {
            frames: (0..3)
                .map(|i| {
                    make_frame(
                        vec![vec!['A'; thumb_w as usize]; thumb_h as usize],
                        false,
                        &format!("{}", i),
                    )
                })
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
        };

        let slot_w = thumb_w + 1;
        let area = Rect::new(0, 0, 3 * slot_w, 1 + thumb_h + 1 + 1);
        let mut buf = Buffer::empty(area);
        StatefulWidget::render(&timeline, area, &mut buf, &mut state);

        let mut non_empty = 0u32;
        for vis_i in 0..3 {
            for ty in 0..thumb_h {
                for tx in 0..thumb_w {
                    let cx = (vis_i as u16) * slot_w + tx;
                    let cy = 1 + ty;
                    if let Some(cell) = buf.cell((cx, cy)) {
                        if cell.symbol() != " " {
                            non_empty += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(non_empty, 3 * thumb_w as u32 * thumb_h as u32);
    }

    #[test]
    fn test_timeline_scroll() {
        let slot_w = 5u16 + 1;
        let timeline = make_test_timeline(5, 2, 1, 5);
        let mut state = TimelineState {
            frames: (0..20)
                .map(|i| make_frame(vec![vec!['F'; 5]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 10,
            playing: false,
            fps: 12,
        };

        let area = Rect::new(0, 0, 5 * slot_w, 1 + 2 + 1 + 1);
        let mut buf = Buffer::empty(area);
        StatefulWidget::render(&timeline, area, &mut buf, &mut state);

        let bottom_y = area.y + 1 + 2 + 1;
        let label_cell = buf.cell((0, bottom_y)).unwrap();
        assert_eq!(
            label_cell.symbol(),
            "1",
            "frame 10 label should show at leftmost position"
        );

        let label_cell2 = buf.cell((1, bottom_y)).unwrap();
        assert_eq!(label_cell2.symbol(), "0", "frame 10 label should show '10'");

        let frame0_x = -(10i32) * slot_w as i32;
        assert!(
            frame0_x < 0,
            "frame 0 should be scrolled out (negative column)"
        );
    }

    #[test]
    fn test_timeline_keyframe_markers() {
        let timeline = make_test_timeline(3, 2, 1, 3);
        let mut state = TimelineState {
            frames: vec![
                make_frame(vec![vec![' '; 3]; 2], true, "0"),
                make_frame(vec![vec![' '; 3]; 2], false, "1"),
                make_frame(vec![vec![' '; 3]; 2], true, "2"),
            ],
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
        };

        let slot_w = timeline.frame_thumb_width + timeline.frame_gap;
        let area = Rect::new(0, 0, 3 * slot_w, 1 + 2 + 1 + 1);
        let mut buf = Buffer::empty(area);
        StatefulWidget::render(&timeline, area, &mut buf, &mut state);

        let marker_y = area.y + 1 + 2;

        let cell0 = buf.cell((0, marker_y)).unwrap();
        assert_eq!(cell0.symbol(), "◆", "frame 0 should have keyframe marker");

        let cell1 = buf.cell((slot_w, marker_y)).unwrap();
        assert_eq!(
            cell1.symbol(),
            "·",
            "frame 1 should have no-keyframe marker"
        );

        let cell2 = buf.cell((2 * slot_w, marker_y)).unwrap();
        assert_eq!(cell2.symbol(), "◆", "frame 2 should have keyframe marker");
    }

    #[test]
    fn test_timeline_empty() {
        let timeline = make_test_timeline(3, 2, 1, 5);
        let mut state = TimelineState::default();

        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        StatefulWidget::render(&timeline, area, &mut buf, &mut state);

        let cell = buf.cell((0, 0)).unwrap();
        assert_eq!(cell.symbol(), " ", "empty timeline should render nothing");
    }

    #[test]
    fn test_timeline_frame_thumbnail_content() {
        let timeline = make_test_timeline(4, 3, 1, 2);
        let thumb = vec![
            vec!['a', 'b', 'c', 'd'],
            vec!['e', 'f', 'g', 'h'],
            vec!['i', 'j', 'k', 'l'],
        ];
        let mut state = TimelineState {
            frames: vec![make_frame(thumb.clone(), false, "X")],
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
        };

        let area = Rect::new(0, 0, 4, 1 + 3 + 1 + 1);
        let mut buf = Buffer::empty(area);
        StatefulWidget::render(&timeline, area, &mut buf, &mut state);

        for (ty, row) in thumb.iter().enumerate() {
            for (tx, &expected) in row.iter().enumerate() {
                let cx = tx as u16;
                let cy = 1u16 + ty as u16;
                let cell = buf.cell((cx, cy)).unwrap();
                assert_eq!(
                    cell.symbol().chars().next().unwrap(),
                    expected,
                    "cell ({}, {}) should be '{}'",
                    cx,
                    cy,
                    expected
                );
            }
        }
    }
}
