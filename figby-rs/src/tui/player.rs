use std::cell::Cell;
use std::time::Duration;

use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

use super::canvas::CanvasCell;

/// A single animation frame: a 2D grid of styled cells.
pub type AnimationFrame = Vec<Vec<CanvasCell>>;

const MIN_SPEED: f64 = 0.25;
const MAX_SPEED: f64 = 4.0;

/// Animation player widget for alternate-screen playback.
///
/// Uses interior mutability (`Cell`) so it can implement `Widget for &AnimationPlayer`.
/// Call `advance(delta)` in the event loop to progress frames based on elapsed time.
pub struct AnimationPlayer {
    frames: Vec<AnimationFrame>,
    fps: u8,
    current_frame: Cell<usize>,
    playing: Cell<bool>,
    loop_: Cell<bool>,
    speed: Cell<f64>,
    accumulator: Cell<f64>,
}

impl std::fmt::Debug for AnimationPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimationPlayer")
            .field("frames", &self.frames.len())
            .field("fps", &self.fps)
            .field("current_frame", &self.current_frame)
            .field("playing", &self.playing)
            .field("loop_", &self.loop_)
            .field("speed", &self.speed)
            .finish()
    }
}

impl AnimationPlayer {
    pub fn new(frames: Vec<AnimationFrame>, fps: u8) -> Self {
        Self {
            frames,
            fps,
            current_frame: Cell::new(0),
            playing: Cell::new(false),
            loop_: Cell::new(false),
            speed: Cell::new(1.0),
            accumulator: Cell::new(0.0),
        }
    }

    pub fn play(&self) {
        self.playing.set(true);
        self.accumulator.set(0.0);
    }

    pub fn pause(&self) {
        self.playing.set(false);
    }

    pub fn toggle_play(&self) {
        if self.playing.get() {
            self.pause();
        } else {
            self.play();
        }
    }

    /// Seek to a specific frame index. Clamps to valid range.
    pub fn seek(&self, idx: usize) {
        let max = self.frames.len().saturating_sub(1);
        self.current_frame.set(idx.min(max));
        self.accumulator.set(0.0);
    }

    /// Set playback speed multiplier. Clamped to [0.25, 4.0].
    pub fn set_speed(&self, mult: f64) {
        self.speed.set(mult.clamp(MIN_SPEED, MAX_SPEED));
        self.accumulator.set(0.0);
    }

    pub fn toggle_loop(&self) {
        self.loop_.set(!self.loop_.get());
    }

    /// Advance animation by `delta` duration.
    /// Returns number of frames advanced (for testing).
    pub fn advance(&self, delta: Duration) -> usize {
        if !self.playing.get() || self.frames.is_empty() {
            return 0;
        }

        let effective_fps = self.fps as f64 * self.speed.get();
        let frame_interval = 1.0 / effective_fps;

        let mut acc = self.accumulator.get() + delta.as_secs_f64();
        let mut advanced = 0u64;

        while acc >= frame_interval {
            acc -= frame_interval;
            advanced += 1;
        }

        if advanced == 0 {
            self.accumulator.set(acc);
            return 0;
        }

        let total = self.frames.len();
        let current = self.current_frame.get();

        if self.loop_.get() {
            let new_frame = (current + advanced as usize) % total;
            self.current_frame.set(new_frame);
        } else {
            let new_frame = (current + advanced as usize).min(total.saturating_sub(1));
            self.current_frame.set(new_frame);
        }

        self.accumulator.set(acc);
        advanced as usize
    }

    pub fn progress(&self) -> (usize, usize) {
        let total = self.frames.len();
        let current = self.current_frame.get().min(total.saturating_sub(1));
        (current, total)
    }

    pub fn is_playing(&self) -> bool {
        self.playing.get()
    }

    pub fn is_looping(&self) -> bool {
        self.loop_.get()
    }

    pub fn total_frames(&self) -> usize {
        self.frames.len()
    }

    pub fn current_frame(&self) -> usize {
        self.current_frame.get()
    }

    pub fn speed_mult(&self) -> f64 {
        self.speed.get()
    }

    /// Handle a key event. Returns `true` if the key was consumed.
    pub fn handle_key(&self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char(' ') => {
                self.toggle_play();
                true
            }
            KeyCode::Left => {
                let cur = self.current_frame.get();
                self.seek(cur.saturating_sub(1));
                true
            }
            KeyCode::Right => {
                let cur = self.current_frame.get();
                self.seek(cur.saturating_add(1));
                true
            }
            KeyCode::Up => {
                let s = self.speed.get() + 0.25;
                self.set_speed(s);
                true
            }
            KeyCode::Down => {
                let s = self.speed.get() - 0.25;
                self.set_speed(s);
                true
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.toggle_loop();
                true
            }
            KeyCode::Esc => {
                self.pause();
                self.seek(0);
                true
            }
            KeyCode::Enter => {
                self.play();
                true
            }
            _ => false,
        }
    }
}

impl Widget for &AnimationPlayer {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 || self.frames.is_empty() {
            return;
        }

        let current = self
            .current_frame
            .get()
            .min(self.frames.len().saturating_sub(1));
        let frame = &self.frames[current];

        let frame_h = frame.len();
        let frame_w = if frame_h > 0 { frame[0].len() } else { 0 };

        if frame_h == 0 || frame_w == 0 {
            return;
        }

        // Reserve bottom row for progress bar
        let frame_render_h = (area.height.saturating_sub(1)).min(frame_h as u16);
        let render_w = frame_w.min(area.width as usize);

        for (row_idx, row) in frame.iter().enumerate().take(frame_render_h as usize) {
            let y = area.y + row_idx as u16;
            for (col_idx, cell) in row.iter().enumerate().take(render_w) {
                let x = area.x + col_idx as u16;
                if let Some(buf_cell) = buf.cell_mut((x, y)) {
                    buf_cell.set_char(cell.ch);
                    let mut style = Style::default();
                    if let Some(fg) = cell.fg {
                        style = style.fg(fg);
                    }
                    if let Some(bg) = cell.bg {
                        style = style.bg(bg);
                    }
                    buf_cell.set_style(style);
                }
            }
        }

        // Progress bar on bottom row
        let progress_y = area.y + area.height.saturating_sub(1);
        self.render_progress_bar(area.x, area.width, progress_y, buf);
    }
}

/// Private helpers
impl AnimationPlayer {
    fn render_progress_bar(&self, origin_x: u16, area_width: u16, y: u16, buf: &mut Buffer) {
        let total = self.frames.len();
        let cur = self.current_frame.get().min(total.saturating_sub(1));

        let play_ch = if self.playing.get() {
            '\u{23F8}'
        } else {
            '\u{25B6}'
        };
        let total_digits = total.to_string().len();
        let counter_str = format!("{:0width$}/{}", cur + 1, total, width = total_digits);
        let speed_str = format!(" {:.2}x", self.speed.get());

        let prefix = format!("{} {}", play_ch, counter_str);
        let prefix_len = prefix.chars().count();
        let suffix = speed_str;
        let suffix_len = suffix.chars().count();

        let bar_available = (area_width as usize).saturating_sub(prefix_len + suffix_len + 3);
        let bar_width = bar_available.clamp(2, 60);

        let mut x = origin_x;

        // Prefix
        for ch in prefix.chars() {
            if x >= origin_x + area_width {
                break;
            }
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(ch);
            }
            x += 1;
        }

        // Space before bar
        if x < origin_x + area_width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(' ');
            }
            x += 1;
        }

        // Opening bracket
        if x < origin_x + area_width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char('[');
            }
            x += 1;
        }

        // Bar fill
        let filled = if total > 1 {
            ((cur as f64) / ((total - 1) as f64) * bar_width as f64).round() as usize
        } else {
            bar_width
        };
        let filled = filled.min(bar_width);

        for i in 0..bar_width {
            if x >= origin_x + area_width {
                break;
            }
            let ch = if i < filled { '\u{2588}' } else { '\u{2591}' };
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(ch);
            }
            x += 1;
        }

        // Closing bracket
        if x < origin_x + area_width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(']');
            }
            x += 1;
        }

        // Suffix (speed)
        for ch in suffix.chars() {
            if x >= origin_x + area_width {
                break;
            }
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(ch);
            }
            x += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_frames(count: usize, w: usize, h: usize) -> Vec<AnimationFrame> {
        (0..count)
            .map(|i| {
                let ch = char::from_u32(b'A' as u32 + (i % 26) as u32).unwrap();
                vec![
                    vec![
                        CanvasCell {
                            ch,
                            fg: None,
                            bg: None
                        };
                        w
                    ];
                    h
                ]
            })
            .collect()
    }

    #[test]
    fn test_player_advance_single_frame() {
        let frames = make_test_frames(10, 3, 2);
        let player = AnimationPlayer::new(frames, 10);
        player.play();
        assert_eq!(player.current_frame(), 0);

        player.advance(Duration::from_millis(100));
        assert_eq!(player.current_frame(), 1);
    }

    #[test]
    fn test_player_advance_multiple_frames() {
        let frames = make_test_frames(10, 3, 2);
        let player = AnimationPlayer::new(frames, 10);
        player.play();

        let n = player.advance(Duration::from_millis(350));
        assert_eq!(n, 3);
        assert_eq!(player.current_frame(), 3);
    }

    #[test]
    fn test_player_does_not_advance_when_paused() {
        let frames = make_test_frames(10, 3, 2);
        let player = AnimationPlayer::new(frames, 10);
        // Not playing, never called play()
        assert!(!player.is_playing());

        player.advance(Duration::from_millis(200));
        assert_eq!(player.current_frame(), 0);
    }

    #[test]
    fn test_player_loops() {
        let frames = make_test_frames(10, 3, 2);
        let player = AnimationPlayer::new(frames, 10);
        player.play();
        player.toggle_loop();
        assert!(player.is_looping());

        // 10fps = 100ms per frame. 1.1s = 11 frames.
        player.advance(Duration::from_millis(1100));
        assert_eq!(player.current_frame(), 1);
    }

    #[test]
    fn test_player_does_not_loop_when_disabled() {
        let frames = make_test_frames(10, 3, 2);
        let player = AnimationPlayer::new(frames, 10);
        player.play();
        assert!(!player.is_looping());

        // 10fps. 2s = 20 frames. Without loop, should clamp at last frame (9).
        player.advance(Duration::from_secs(2));
        assert_eq!(player.current_frame(), 9);
    }

    #[test]
    fn test_player_seek() {
        let frames = make_test_frames(10, 3, 2);
        let player = AnimationPlayer::new(frames, 10);

        player.seek(5);
        assert_eq!(player.current_frame(), 5);

        // Seek past end clamps to last
        player.seek(100);
        assert_eq!(player.current_frame(), 9);

        // Seek before start clamps to first
        player.seek(0);
        assert_eq!(player.current_frame(), 0);
    }

    #[test]
    fn test_player_speed_control() {
        let frames = make_test_frames(10, 3, 2);
        let player = AnimationPlayer::new(frames, 10);
        player.play();
        player.set_speed(2.0);

        // 10fps * 2x = 20fps effective, frame_interval = 50ms.
        // 100ms = 2 frames.
        let n = player.advance(Duration::from_millis(100));
        assert_eq!(n, 2);
        assert_eq!(player.current_frame(), 2);
    }

    #[test]
    fn test_player_speed_clamping() {
        let frames = make_test_frames(10, 3, 2);
        let player = AnimationPlayer::new(frames, 10);

        player.set_speed(0.1);
        assert!((player.speed_mult() - MIN_SPEED).abs() < 1e-6);

        player.set_speed(5.0);
        assert!((player.speed_mult() - MAX_SPEED).abs() < 1e-6);
    }

    #[test]
    fn test_player_render_progress_bar() {
        let frames = make_test_frames(10, 3, 2);
        let player = AnimationPlayer::new(frames, 10);

        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        Widget::render(&player, area, &mut buf);

        // Bottom row should have progress bar content
        let progress_y = area.y + area.height - 1;
        let cell = buf.cell((0, progress_y)).unwrap();
        // First char should be play/pause indicator
        let ch = cell.symbol().chars().next().unwrap();
        assert!(ch == '\u{25B6}' || ch == '\u{23F8}');

        // Should have '/' in the counter portion
        let mut has_slash = false;
        for x in 0..area.width {
            if let Some(c) = buf.cell((x, progress_y)) {
                if c.symbol() == "/" {
                    has_slash = true;
                    break;
                }
            }
        }
        assert!(has_slash);
    }

    #[test]
    fn test_player_render_frame_content() {
        let frame: AnimationFrame = vec![
            vec![
                CanvasCell {
                    ch: 'A',
                    fg: None,
                    bg: None,
                },
                CanvasCell {
                    ch: 'B',
                    fg: None,
                    bg: None,
                },
            ],
            vec![
                CanvasCell {
                    ch: 'C',
                    fg: None,
                    bg: None,
                },
                CanvasCell {
                    ch: 'D',
                    fg: None,
                    bg: None,
                },
            ],
        ];
        let player = AnimationPlayer::new(vec![frame], 10);
        let area = Rect::new(0, 0, 5, 5);
        let mut buf = Buffer::empty(area);
        Widget::render(&player, area, &mut buf);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "A");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "B");
        assert_eq!(buf.cell((0, 1)).unwrap().symbol(), "C");
        assert_eq!(buf.cell((1, 1)).unwrap().symbol(), "D");
    }

    #[test]
    fn test_player_handle_key_play_pause() {
        let frames = make_test_frames(10, 3, 2);
        let player = AnimationPlayer::new(frames, 10);
        assert!(!player.is_playing());

        player.handle_key(KeyCode::Char(' '));
        assert!(player.is_playing());

        player.handle_key(KeyCode::Char(' '));
        assert!(!player.is_playing());
    }

    #[test]
    fn test_player_handle_key_seek() {
        let frames = make_test_frames(10, 3, 2);
        let player = AnimationPlayer::new(frames, 10);
        player.seek(5);
        assert_eq!(player.current_frame(), 5);

        player.handle_key(KeyCode::Left);
        assert_eq!(player.current_frame(), 4);

        player.handle_key(KeyCode::Right);
        assert_eq!(player.current_frame(), 5);
    }

    #[test]
    fn test_player_handle_key_speed() {
        let frames = make_test_frames(10, 3, 2);
        let player = AnimationPlayer::new(frames, 10);

        player.handle_key(KeyCode::Up);
        assert!((player.speed_mult() - 1.25).abs() < 1e-6);

        player.handle_key(KeyCode::Down);
        assert!((player.speed_mult() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_player_handle_key_loop() {
        let frames = make_test_frames(10, 3, 2);
        let player = AnimationPlayer::new(frames, 10);
        assert!(!player.is_looping());

        player.handle_key(KeyCode::Char('L'));
        assert!(player.is_looping());

        player.handle_key(KeyCode::Char('l'));
        assert!(!player.is_looping());
    }

    #[test]
    fn test_player_handle_key_esc_resets() {
        let frames = make_test_frames(10, 3, 2);
        let player = AnimationPlayer::new(frames, 10);
        player.play();
        player.seek(5);
        assert!(player.is_playing());
        assert_eq!(player.current_frame(), 5);

        player.handle_key(KeyCode::Esc);
        assert!(!player.is_playing());
        assert_eq!(player.current_frame(), 0);
    }

    #[test]
    fn test_player_handle_key_enter() {
        let frames = make_test_frames(10, 3, 2);
        let player = AnimationPlayer::new(frames, 10);
        assert!(!player.is_playing());

        player.handle_key(KeyCode::Enter);
        assert!(player.is_playing());
    }

    #[test]
    fn test_player_progress() {
        let frames = make_test_frames(10, 3, 2);
        let player = AnimationPlayer::new(frames, 10);

        let (cur, total) = player.progress();
        assert_eq!(cur, 0);
        assert_eq!(total, 10);

        player.seek(5);
        let (cur, total) = player.progress();
        assert_eq!(cur, 5);
        assert_eq!(total, 10);
    }

    #[test]
    fn test_player_empty_frames_advance() {
        let player = AnimationPlayer::new(vec![], 10);
        player.play();

        assert_eq!(player.advance(Duration::from_secs(1)), 0);
        assert_eq!(player.total_frames(), 0);
    }
}
