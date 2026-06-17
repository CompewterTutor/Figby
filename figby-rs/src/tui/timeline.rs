use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, StatefulWidget, Widget};
use ratatui::Frame;

use super::canvas::CanvasBuffer;
use super::layers::BlendMode;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayerKeyframe {
    pub position_offset: (i16, i16),
    pub opacity: u8,
    pub blend_mode: BlendMode,
}

impl Default for LayerKeyframe {
    fn default() -> Self {
        Self {
            position_offset: (0, 0),
            opacity: 255,
            blend_mode: BlendMode::Normal,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimelineFrame {
    pub thumbnail: Vec<Vec<char>>,
    pub has_keyframe: bool,
    pub label: String,
    pub layer_state: Option<CanvasBuffer>,
    pub layer_keyframes: Vec<Option<LayerKeyframe>>,
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

#[derive(Debug, Clone, Default)]
pub struct KeyframeEditState {
    pub open: bool,
    pub selected_layer: usize,
    pub selected_property: usize,
    pub edit_mode: bool,
    pub edit_buffer: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EasingFunction {
    Linear,
    EaseIn,
    EaseOut,
    Bounce,
}

impl EasingFunction {
    pub fn apply(&self, t: f64) -> f64 {
        match self {
            EasingFunction::Linear => t,
            EasingFunction::EaseIn => t * t * t,
            EasingFunction::EaseOut => 1.0 - (1.0 - t).powi(3),
            EasingFunction::Bounce => ease_bounce(t),
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            EasingFunction::Linear => "Linear",
            EasingFunction::EaseIn => "Ease In",
            EasingFunction::EaseOut => "Ease Out",
            EasingFunction::Bounce => "Bounce",
        }
    }

    pub fn cycle(&self) -> Self {
        match self {
            EasingFunction::Linear => EasingFunction::EaseIn,
            EasingFunction::EaseIn => EasingFunction::EaseOut,
            EasingFunction::EaseOut => EasingFunction::Bounce,
            EasingFunction::Bounce => EasingFunction::Linear,
        }
    }

    pub fn cycle_back(&self) -> Self {
        match self {
            EasingFunction::Linear => EasingFunction::Bounce,
            EasingFunction::EaseIn => EasingFunction::Linear,
            EasingFunction::EaseOut => EasingFunction::EaseIn,
            EasingFunction::Bounce => EasingFunction::EaseOut,
        }
    }

    pub fn all() -> &'static [EasingFunction] {
        &[
            EasingFunction::Linear,
            EasingFunction::EaseIn,
            EasingFunction::EaseOut,
            EasingFunction::Bounce,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct TweenConfig {
    pub start_frame: usize,
    pub end_frame: usize,
    pub num_frames: usize,
    pub easing: EasingFunction,
}

impl Default for TweenConfig {
    fn default() -> Self {
        Self {
            start_frame: 0,
            end_frame: 0,
            num_frames: 5,
            easing: EasingFunction::Linear,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TweenPreview {
    pub generated_frames: Vec<TimelineFrame>,
    pub config: TweenConfig,
    pub valid: bool,
    pub field_index: usize,
}

#[derive(Debug, Clone)]
pub struct AnimationTimeline {
    pub frame_thumb_width: u16,
    pub frame_thumb_height: u16,
    pub frame_gap: u16,
    pub visible_frames: usize,
    pub theme: TimelineTheme,
    pub onion_skinning: bool,
}

#[derive(Debug, Clone)]
pub struct TimelineState {
    pub frames: Vec<TimelineFrame>,
    pub current_frame: usize,
    pub scroll_offset: usize,
    pub playing: bool,
    pub fps: u8,
    pub keyframe_editor: KeyframeEditState,
    pub tween: Option<TweenPreview>,
}

impl Default for TimelineState {
    fn default() -> Self {
        Self {
            frames: Vec::new(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        }
    }
}

impl TimelineState {
    /// Add a frame to the end of the timeline.
    pub fn add_frame(&mut self, frame: TimelineFrame) {
        self.frames.push(frame);
    }

    /// Insert a frame at the given index.
    pub fn insert_frame(&mut self, index: usize, frame: TimelineFrame) {
        let idx = index.min(self.frames.len());
        self.frames.insert(idx, frame);
        if idx <= self.current_frame {
            self.current_frame += 1;
        }
    }

    /// Remove the frame at `index`. Fails if it would leave the timeline empty.
    pub fn remove_frame(&mut self, index: usize) -> Result<TimelineFrame, String> {
        if index >= self.frames.len() {
            return Err(format!(
                "Frame index {} out of bounds (len={})",
                index,
                self.frames.len()
            ));
        }
        if self.frames.len() <= 1 {
            return Err("Cannot remove the last remaining frame".into());
        }
        let removed = self.frames.remove(index);
        if self.current_frame >= self.frames.len() {
            self.current_frame = self.frames.len().saturating_sub(1);
        } else if index < self.current_frame {
            self.current_frame -= 1;
        }
        Ok(removed)
    }

    /// Duplicate the frame at `index`, inserting the copy after it.
    pub fn duplicate_frame(&mut self, index: usize) -> Result<(), String> {
        if index >= self.frames.len() {
            return Err(format!(
                "Frame index {} out of bounds (len={})",
                index,
                self.frames.len()
            ));
        }
        let mut dup = self.frames[index].clone();
        dup.label = format!("{} (copy)", dup.label);
        let insert_at = index + 1;
        self.frames.insert(insert_at, dup);
        if insert_at <= self.current_frame {
            self.current_frame += 1;
        }
        Ok(())
    }

    /// Move a frame from `from` to `to`.
    pub fn reorder_frame(&mut self, from: usize, to: usize) -> Result<(), String> {
        if from >= self.frames.len() {
            return Err(format!(
                "Source index {} out of bounds (len={})",
                from,
                self.frames.len()
            ));
        }
        if to >= self.frames.len() {
            return Err(format!(
                "Target index {} out of bounds (len={})",
                to,
                self.frames.len()
            ));
        }
        if from == to {
            return Ok(());
        }
        let frame = self.frames.remove(from);
        let insert_at = if to > from { to.saturating_sub(1) } else { to };
        self.frames.insert(insert_at, frame);

        let old = self.current_frame;
        if from == old {
            self.current_frame = to;
        } else if from < old && to >= old {
            self.current_frame = old.saturating_sub(1);
        } else if from > old && to <= old {
            self.current_frame = old.saturating_add(1);
        }
        Ok(())
    }

    // ── Keyframing ────────────────────────────────────────────────────

    pub fn set_keyframe(
        &mut self,
        frame_idx: usize,
        layer_idx: usize,
        props: LayerKeyframe,
    ) -> bool {
        let frame = match self.frames.get_mut(frame_idx) {
            Some(f) => f,
            None => return false,
        };
        while frame.layer_keyframes.len() <= layer_idx {
            frame.layer_keyframes.push(None);
        }
        frame.layer_keyframes[layer_idx] = Some(props);
        frame.has_keyframe = frame.layer_keyframes.iter().any(|k| k.is_some());
        true
    }

    pub fn remove_keyframe(&mut self, frame_idx: usize, layer_idx: usize) -> bool {
        let frame = match self.frames.get_mut(frame_idx) {
            Some(f) => f,
            None => return false,
        };
        if layer_idx < frame.layer_keyframes.len() {
            frame.layer_keyframes[layer_idx] = None;
            frame.has_keyframe = frame.layer_keyframes.iter().any(|k| k.is_some());
            true
        } else {
            false
        }
    }

    pub fn get_keyframe(&self, frame_idx: usize, layer_idx: usize) -> Option<LayerKeyframe> {
        self.frames
            .get(frame_idx)?
            .layer_keyframes
            .get(layer_idx)
            .copied()
            .flatten()
    }

    pub fn get_interpolated_properties(&self, frame_idx: usize, layer_idx: usize) -> LayerKeyframe {
        let prev = (0..=frame_idx).rev().find_map(|i| {
            let f = self.frames.get(i)?;
            let kf = (*f.layer_keyframes.get(layer_idx)?)?;
            Some((i, kf))
        });
        let next = (frame_idx..self.frames.len()).find_map(|i| {
            let f = self.frames.get(i)?;
            let kf = (*f.layer_keyframes.get(layer_idx)?)?;
            Some((i, kf))
        });
        match (prev, next) {
            (Some((pi, pk)), Some((_ni, _))) if pi == _ni => pk,
            (Some((pi, pk)), Some((ni, nk))) => {
                if frame_idx <= pi {
                    pk
                } else if frame_idx >= ni {
                    nk
                } else {
                    let range = ni - pi;
                    let offset = frame_idx.saturating_sub(pi);
                    let t = if range == 0 {
                        0.0
                    } else {
                        offset as f64 / range as f64
                    };
                    LayerKeyframe {
                        position_offset: (
                            lerp_i16(pk.position_offset.0, nk.position_offset.0, t),
                            lerp_i16(pk.position_offset.1, nk.position_offset.1, t),
                        ),
                        opacity: lerp_u8(pk.opacity, nk.opacity, t),
                        blend_mode: step_blend_mode(pk.blend_mode, nk.blend_mode, t),
                    }
                }
            }
            (Some((_, pk)), None) => pk,
            (None, Some((_, nk))) => nk,
            (None, None) => LayerKeyframe::default(),
        }
    }

    // ── Tweening ──────────────────────────────────────────────────────

    pub fn open_tween(&mut self) {
        let start = self.current_frame;
        let end = (start + 1).min(self.frames.len().saturating_sub(1));
        self.tween = Some(TweenPreview {
            config: TweenConfig {
                start_frame: start,
                end_frame: end,
                ..TweenConfig::default()
            },
            ..TweenPreview::default()
        });
    }

    pub fn compute_tween(&mut self) {
        let tween = match self.tween.as_mut() {
            Some(t) => t,
            None => return,
        };

        let config = &tween.config;
        let start = config.start_frame;
        let end = config.end_frame;
        let num_frames = config.num_frames.clamp(1, 120);
        let easing = config.easing;

        if start >= end || end >= self.frames.len() || start >= self.frames.len() {
            tween.generated_frames.clear();
            tween.valid = false;
            return;
        }

        let start_frame = &self.frames[start];
        let end_frame = &self.frames[end];

        let max_layers = start_frame
            .layer_keyframes
            .len()
            .max(end_frame.layer_keyframes.len());

        let mut generated = Vec::with_capacity(num_frames);
        let mut has_any_keyframe = false;

        for i in 0..num_frames {
            let t = (i + 1) as f64 / (num_frames + 1) as f64;
            let et = easing.apply(t);

            let mut frame_layers: Vec<Option<LayerKeyframe>> = Vec::new();
            for layer in 0..max_layers {
                let start_kf = start_frame.layer_keyframes.get(layer).copied().flatten();
                let end_kf = end_frame.layer_keyframes.get(layer).copied().flatten();

                match (start_kf, end_kf) {
                    (Some(skf), Some(ekf)) => {
                        has_any_keyframe = true;
                        frame_layers.push(Some(LayerKeyframe {
                            position_offset: (
                                lerp_i16(skf.position_offset.0, ekf.position_offset.0, et),
                                lerp_i16(skf.position_offset.1, ekf.position_offset.1, et),
                            ),
                            opacity: lerp_u8(skf.opacity, ekf.opacity, et),
                            blend_mode: step_blend_mode(skf.blend_mode, ekf.blend_mode, et),
                        }));
                    }
                    _ => {
                        frame_layers.push(None);
                    }
                }
            }

            let has_kf = frame_layers.iter().any(|k| k.is_some());
            generated.push(TimelineFrame {
                thumbnail: start_frame.thumbnail.clone(),
                has_keyframe: has_kf,
                label: format!("tween {}/{}", i + 1, num_frames),
                layer_state: None,
                layer_keyframes: frame_layers,
            });
        }

        tween.generated_frames = generated;
        tween.valid = has_any_keyframe;
    }

    pub fn commit_tween(&mut self) {
        let tween = match self.tween.take() {
            Some(t) if t.valid => t,
            _ => return,
        };

        let insert_at = tween.config.start_frame + 1;
        let num = tween.generated_frames.len();

        for (i, frame) in tween.generated_frames.into_iter().enumerate() {
            let idx = insert_at + i;
            if idx <= self.frames.len() {
                self.frames.insert(idx, frame);
            } else {
                self.frames.push(frame);
            }
        }

        if insert_at <= self.current_frame {
            self.current_frame += num;
        }
    }

    pub fn discard_tween(&mut self) {
        self.tween = None;
    }

    pub fn render_tween_panel(&self, frame: &mut Frame, area: Rect, theme: &TimelineTheme) {
        let tween = match self.tween.as_ref() {
            Some(t) => t,
            None => return,
        };

        if area.width < 20 || area.height < 6 {
            return;
        }

        let block = Block::default()
            .title(" Tween ")
            .borders(Borders::ALL)
            .style(Style::default().fg(theme.keyframe));
        let inner = block.inner(area);
        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        let mut lines: Vec<String> = Vec::new();
        let has_frames = !self.frames.is_empty();
        if !has_frames {
            lines.push(" No frames in timeline".to_string());
            let para = Paragraph::new(lines.join("\n")).style(Style::default());
            frame.render_widget(para, inner);
            return;
        }

        let cfg = &tween.config;
        let fields = [
            ("Start Frame", cfg.start_frame, 0),
            ("End Frame", cfg.end_frame, 1),
            ("Frames", cfg.num_frames, 2),
        ];

        for (label, value, idx) in fields {
            let prefix = if tween.field_index == idx { '>' } else { ' ' };
            lines.push(format!(" {} {}: {}", prefix, label, value));
        }

        let easing_prefix = if tween.field_index == 3 { '>' } else { ' ' };
        lines.push(format!(
            " {} Easing: {}",
            easing_prefix,
            cfg.easing.display_name()
        ));

        lines.push(String::new());

        if tween.valid && !tween.generated_frames.is_empty() {
            lines.push(format!(
                " Status: Generated ({} frames)",
                tween.generated_frames.len()
            ));
        } else {
            lines.push(" Status: Needs generate".to_string());
        }

        lines.push(String::new());
        lines.push(
            " \u{2191}\u{2193} field  \u{2190}\u{2192} value  Enter=Generate/Commit  C=Commit  Esc=Discard"
                .to_string(),
        );

        let para = Paragraph::new(lines.join("\n")).style(Style::default());
        frame.render_widget(para, inner);
    }

    pub fn handle_tween_key(&mut self, code: KeyCode) -> bool {
        let tween = match self.tween.as_mut() {
            Some(t) => t,
            None => return false,
        };

        match code {
            KeyCode::Up if tween.field_index > 0 => {
                tween.field_index -= 1;
            }
            KeyCode::Down if tween.field_index < 3 => {
                tween.field_index += 1;
            }
            KeyCode::Left => match tween.field_index {
                0 => {
                    if tween.config.start_frame > 0 {
                        tween.config.start_frame -= 1;
                    }
                    tween.valid = false;
                }
                1 => {
                    if tween.config.end_frame > tween.config.start_frame + 1 {
                        tween.config.end_frame -= 1;
                    }
                    tween.valid = false;
                }
                2 => {
                    if tween.config.num_frames > 1 {
                        tween.config.num_frames -= 1;
                    }
                    tween.valid = false;
                }
                3 => {
                    tween.config.easing = tween.config.easing.cycle_back();
                    tween.valid = false;
                }
                _ => {}
            },
            KeyCode::Right => match tween.field_index {
                0 => {
                    if tween.config.start_frame + 1 < tween.config.end_frame {
                        tween.config.start_frame += 1;
                    }
                    tween.valid = false;
                }
                1 => {
                    if tween.config.end_frame + 1 < self.frames.len() {
                        tween.config.end_frame += 1;
                    }
                    tween.valid = false;
                }
                2 => {
                    if tween.config.num_frames < 120 {
                        tween.config.num_frames += 1;
                    }
                    tween.valid = false;
                }
                3 => {
                    tween.config.easing = tween.config.easing.cycle();
                    tween.valid = false;
                }
                _ => {}
            },
            KeyCode::Enter => {
                if tween.valid && !tween.generated_frames.is_empty() {
                    self.commit_tween();
                } else {
                    self.compute_tween();
                }
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                if tween.valid && !tween.generated_frames.is_empty() {
                    self.commit_tween();
                }
            }
            KeyCode::Esc => {
                self.discard_tween();
            }
            _ => return false,
        }
        true
    }

    pub fn handle_keyframe_editor_key(&mut self, code: KeyCode) -> bool {
        if !self.keyframe_editor.open {
            return false;
        }
        if self.keyframe_editor.edit_mode {
            match code {
                KeyCode::Esc => {
                    self.keyframe_editor.edit_mode = false;
                    self.keyframe_editor.edit_buffer.clear();
                }
                KeyCode::Enter => {
                    let value_str = self.keyframe_editor.edit_buffer.trim().to_string();
                    self.keyframe_editor.edit_mode = false;
                    if !value_str.is_empty() {
                        if let Ok(value) = value_str.parse::<i16>() {
                            let current = self.get_interpolated_properties(
                                self.current_frame,
                                self.keyframe_editor.selected_layer,
                            );
                            let mut new_kf = current;
                            match self.keyframe_editor.selected_property {
                                0 => new_kf.position_offset.0 = value,
                                1 => new_kf.position_offset.1 = value,
                                2 => new_kf.opacity = value.clamp(0, 255) as u8,
                                _ => {}
                            }
                            self.set_keyframe(
                                self.current_frame,
                                self.keyframe_editor.selected_layer,
                                new_kf,
                            );
                        }
                    }
                    self.keyframe_editor.edit_buffer.clear();
                }
                KeyCode::Backspace => {
                    self.keyframe_editor.edit_buffer.pop();
                }
                KeyCode::Char(c) if c.is_ascii_digit() || c == '-' => {
                    self.keyframe_editor.edit_buffer.push(c);
                }
                _ => {}
            }
            return true;
        }
        match code {
            KeyCode::Up if self.keyframe_editor.selected_layer > 0 => {
                self.keyframe_editor.selected_layer -= 1;
            }
            KeyCode::Down => {
                self.keyframe_editor.selected_layer += 1;
            }
            KeyCode::Left if self.keyframe_editor.selected_property > 0 => {
                self.keyframe_editor.selected_property -= 1;
            }
            KeyCode::Right if self.keyframe_editor.selected_property < 3 => {
                self.keyframe_editor.selected_property += 1;
            }
            KeyCode::Enter => {
                if self.keyframe_editor.selected_property == 3 {
                    let current = self.get_interpolated_properties(
                        self.current_frame,
                        self.keyframe_editor.selected_layer,
                    );
                    let next_blend = match current.blend_mode {
                        BlendMode::Normal => BlendMode::Multiply,
                        BlendMode::Multiply => BlendMode::Overlay,
                        BlendMode::Overlay => BlendMode::Screen,
                        BlendMode::Screen => BlendMode::Add,
                        BlendMode::Add => BlendMode::Subtract,
                        BlendMode::Subtract => BlendMode::Normal,
                    };
                    self.set_keyframe(
                        self.current_frame,
                        self.keyframe_editor.selected_layer,
                        LayerKeyframe {
                            blend_mode: next_blend,
                            ..current
                        },
                    );
                } else {
                    self.keyframe_editor.edit_mode = true;
                    self.keyframe_editor.edit_buffer.clear();
                    let current = self.get_interpolated_properties(
                        self.current_frame,
                        self.keyframe_editor.selected_layer,
                    );
                    let val = match self.keyframe_editor.selected_property {
                        0 => current.position_offset.0,
                        1 => current.position_offset.1,
                        _ => current.opacity as i16,
                    };
                    self.keyframe_editor.edit_buffer = val.to_string();
                }
            }
            KeyCode::Esc => {
                self.keyframe_editor.open = false;
            }
            _ => {}
        }
        true
    }

    pub fn render_keyframe_editor(&self, frame: &mut Frame, area: Rect, theme: &TimelineTheme) {
        if !self.keyframe_editor.open || area.width < 20 || area.height < 6 {
            return;
        }
        let block = Block::default()
            .title(" Keyframe Editor ")
            .borders(Borders::ALL)
            .style(Style::default().fg(theme.keyframe));
        let inner = block.inner(area);
        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        let mut lines: Vec<String> = Vec::new();
        let has_frames = !self.frames.is_empty();
        if !has_frames {
            lines.push(" No frames in timeline".to_string());
            let para = Paragraph::new(lines.join("\n")).style(Style::default());
            frame.render_widget(para, inner);
            return;
        }

        let frame_idx = self.current_frame.min(self.frames.len().saturating_sub(1));
        lines.push(format!(
            " Frame: {}  Layer: {}",
            frame_idx, self.keyframe_editor.selected_layer
        ));
        lines.push(String::new());

        let max_layer = self
            .frames
            .first()
            .map(|f| {
                f.layer_keyframes
                    .len()
                    .max(self.keyframe_editor.selected_layer + 1)
            })
            .unwrap_or(4);

        for layer_idx in 0..max_layer.min(6) {
            let props = self.get_interpolated_properties(frame_idx, layer_idx);
            let is_selected = layer_idx == self.keyframe_editor.selected_layer;
            let prefix = if is_selected { '>' } else { ' ' };
            lines.push(format!(" {} Layer {}:", prefix, layer_idx));

            for prop_idx in 0..4 {
                let is_prop_selected =
                    is_selected && prop_idx == self.keyframe_editor.selected_property;
                let prop_prefix = if is_prop_selected { '>' } else { ' ' };
                let value_str = match prop_idx {
                    0 => {
                        if is_prop_selected && self.keyframe_editor.edit_mode {
                            format!("Pos X: {}", self.keyframe_editor.edit_buffer)
                        } else {
                            format!("Pos X: {:4}", props.position_offset.0)
                        }
                    }
                    1 => {
                        if is_prop_selected && self.keyframe_editor.edit_mode {
                            format!("Pos Y: {}", self.keyframe_editor.edit_buffer)
                        } else {
                            format!("Pos Y: {:4}", props.position_offset.1)
                        }
                    }
                    2 => {
                        if is_prop_selected && self.keyframe_editor.edit_mode {
                            format!("Opacity: {}", self.keyframe_editor.edit_buffer)
                        } else {
                            format!("Opacity: {:3}", props.opacity)
                        }
                    }
                    _ => format!("Blend: {}", props.blend_mode.display_name()),
                };
                lines.push(format!(" {}  {}", prop_prefix, value_str));
            }
        }

        lines.push(String::new());
        lines.push(
            " \u{2191}\u{2193} layer  \u{2190}\u{2192} prop  Enter edit  Esc close".to_string(),
        );

        let para = Paragraph::new(lines.join("\n")).style(Style::default());
        frame.render_widget(para, inner);
    }
}

fn lerp_i16(a: i16, b: i16, t: f64) -> i16 {
    (a as f64 + (b as f64 - a as f64) * t).round() as i16
}

fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t).round() as u8
}

fn step_blend_mode(a: BlendMode, b: BlendMode, t: f64) -> BlendMode {
    if t < 0.5 {
        a
    } else {
        b
    }
}

fn ease_bounce(t: f64) -> f64 {
    if t < 1.0 / 2.75 {
        7.5625 * t * t
    } else if t < 2.0 / 2.75 {
        let t2 = t - 1.5 / 2.75;
        7.5625 * t2 * t2 + 0.75
    } else if t < 2.5 / 2.75 {
        let t2 = t - 2.25 / 2.75;
        7.5625 * t2 * t2 + 0.9375
    } else {
        let t2 = t - 2.625 / 2.75;
        7.5625 * t2 * t2 + 0.984375
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

        let tween_preview = state
            .tween
            .as_ref()
            .filter(|t| t.valid && !t.generated_frames.is_empty());
        let tween_insert_offset = tween_preview.map(|t| t.config.start_frame + 1);

        let mut tween_rendered = false;
        let mut tween_frame_count = 0usize;

        for vis_i in 0..(end - start) {
            let frame_idx = start + vis_i;
            let x_start = area.x + (vis_i as u16) * slot_w;
            let frame = &state.frames[frame_idx];
            let is_active = frame_idx == state.current_frame;

            // Insert tween preview frames after start_frame
            if let (Some(insert_at), Some(tp)) = (tween_insert_offset, tween_preview) {
                if frame_idx == insert_at && !tween_rendered {
                    tween_rendered = true;
                    for (ti, tf) in tp.generated_frames.iter().enumerate() {
                        let tx = area.x + ((vis_i + tween_frame_count) as u16) * slot_w;
                        tween_frame_count += 1;
                        if tx + slot_w > area.x + area.width {
                            break;
                        }
                        // Ruler line
                        let ruler_y = area.y;
                        let label = format!("{}+{}", insert_at, ti + 1);
                        for (ci, ch) in label.chars().enumerate() {
                            let cx = tx + ci as u16;
                            if cx < area.x + area.width {
                                if let Some(cell) = buf.cell_mut((cx, ruler_y)) {
                                    cell.set_char(ch);
                                    cell.set_style(Style::default().fg(Color::Cyan));
                                }
                            }
                        }
                        // Ghost thumbnail
                        let thumb_y = area.y + 1;
                        for ty in 0..self.frame_thumb_height.min(area.height - 1) {
                            let cy = thumb_y + ty;
                            if cy >= area.y + area.height {
                                break;
                            }
                            for tx2 in 0..self.frame_thumb_width {
                                let cx = tx + tx2;
                                if cx >= area.x + area.width {
                                    break;
                                }
                                if let Some(cell) = buf.cell_mut((cx, cy)) {
                                    let ch = tf
                                        .thumbnail
                                        .get(ty as usize)
                                        .and_then(|row| row.get(tx2 as usize))
                                        .copied()
                                        .unwrap_or(' ');
                                    cell.set_char(ch);
                                    cell.set_style(
                                        Style::default()
                                            .fg(Color::Cyan)
                                            .add_modifier(ratatui::style::Modifier::DIM),
                                    );
                                }
                            }
                        }
                        // Marker
                        let marker_y = area.y + 1 + self.frame_thumb_height;
                        if marker_y < area.y + area.height {
                            if let Some(cell) = buf.cell_mut((tx, marker_y)) {
                                cell.set_char('◇');
                                cell.set_style(Style::default().fg(Color::Cyan));
                            }
                        }
                        // Label
                        let bottom_y = area.y + 1 + self.frame_thumb_height + 1;
                        if bottom_y < area.y + area.height {
                            for (ci, ch) in tf.label.chars().enumerate() {
                                let cx = tx + ci as u16;
                                if cx >= area.x + area.width {
                                    break;
                                }
                                if let Some(cell) = buf.cell_mut((cx, bottom_y)) {
                                    cell.set_char(ch);
                                    cell.set_style(Style::default().fg(Color::Cyan));
                                }
                            }
                        }
                    }
                }
            }

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
            // Onion skinning: render previous frame's thumbnail dimly if enabled
            if self.onion_skinning && is_active && frame_idx > 0 {
                if let Some(prev) = state.frames.get(frame_idx.saturating_sub(1)) {
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
                                let ch = prev
                                    .thumbnail
                                    .get(ty as usize)
                                    .and_then(|row| row.get(tx as usize))
                                    .copied()
                                    .unwrap_or(' ');
                                if ch != ' ' {
                                    cell.set_char(ch);
                                    cell.set_style(
                                        Style::default().fg(self.theme.thumbnail_border),
                                    );
                                }
                            }
                        }
                    }
                }
            }
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

    fn make_test_timeline_with_onion(
        thumb_w: u16,
        thumb_h: u16,
        gap: u16,
        visible: usize,
        onion: bool,
    ) -> AnimationTimeline {
        AnimationTimeline {
            frame_thumb_width: thumb_w,
            frame_thumb_height: thumb_h,
            frame_gap: gap,
            visible_frames: visible,
            theme: TimelineTheme::default(),
            onion_skinning: onion,
        }
    }

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
            onion_skinning: false,
        }
    }

    fn make_frame(thumb: Vec<Vec<char>>, has_kf: bool, label: &str) -> TimelineFrame {
        TimelineFrame {
            thumbnail: thumb,
            has_keyframe: has_kf,
            label: label.to_string(),
            layer_state: None,
            layer_keyframes: Vec::new(),
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
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
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
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
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
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
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
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
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
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
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
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };

        let slot_w = timeline.frame_thumb_width + timeline.frame_gap;
        let area = Rect::new(0, 0, slot_w, 1 + 3 + 1 + 1);
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

    #[test]
    fn test_onion_skinning_render() {
        let timeline = make_test_timeline_with_onion(3, 2, 1, 5, true);
        let thumb0 = vec![vec![' '; 3]; 2];
        let thumb1 = vec![vec!['O'; 3]; 2];
        let mut state = TimelineState {
            frames: vec![
                make_frame(thumb0, false, "0"),
                make_frame(thumb1, false, "1"),
            ],
            current_frame: 1,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        let slot_w = timeline.frame_thumb_width + timeline.frame_gap;
        let area = Rect::new(0, 0, 2 * slot_w, 1 + 2 + 1 + 1);
        let mut buf = Buffer::empty(area);
        StatefulWidget::render(&timeline, area, &mut buf, &mut state);
        // Frame 1's slot (active) should show 'O' from its own thumbnail on top
        let cell = buf.cell((slot_w, 1)).unwrap();
        assert_eq!(cell.symbol(), "O", "active frame shows its own content");
    }

    #[test]
    fn test_add_frame() {
        let mut state = TimelineState::default();
        assert_eq!(state.frames.len(), 0);
        state.add_frame(make_frame(vec![vec!['A'; 3]; 2], false, "new"));
        assert_eq!(state.frames.len(), 1);
        assert_eq!(state.frames[0].label, "new");
    }

    #[test]
    fn test_insert_frame_updates_current() {
        let mut state = TimelineState {
            frames: (0..3)
                .map(|i| make_frame(vec![vec!['X'; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 1,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        state.insert_frame(0, make_frame(vec![vec!['Y'; 3]; 2], false, "inserted"));
        assert_eq!(state.frames.len(), 4);
        assert_eq!(state.frames[0].label, "inserted");
        assert_eq!(state.current_frame, 2, "current_frame should shift right");
    }

    #[test]
    fn test_remove_frame_middle() {
        let mut state = TimelineState {
            frames: (0..3)
                .map(|i| make_frame(vec![vec!['X'; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 2,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        let removed = state.remove_frame(1).unwrap();
        assert_eq!(removed.label, "1");
        assert_eq!(state.frames.len(), 2);
        assert_eq!(state.current_frame, 1, "current_frame should clamp");
        assert_eq!(state.frames[1].label, "2");
    }

    #[test]
    fn test_remove_frame_last_remaining_fails() {
        let mut state = TimelineState {
            frames: vec![make_frame(vec![vec!['X'; 3]; 2], false, "only")],
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        assert!(state.remove_frame(0).is_err());
    }

    #[test]
    fn test_remove_frame_out_of_bounds_fails() {
        let mut state = TimelineState {
            frames: (0..2)
                .map(|i| make_frame(vec![vec!['X'; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        assert!(state.remove_frame(5).is_err());
    }

    #[test]
    fn test_duplicate_frame() {
        let mut state = TimelineState {
            frames: (0..3)
                .map(|i| make_frame(vec![vec!['X'; 3]; 2], i == 1, &format!("{}", i)))
                .collect(),
            current_frame: 1,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        state.duplicate_frame(1).unwrap();
        assert_eq!(state.frames.len(), 4);
        assert_eq!(state.frames[2].label, "1 (copy)");
        assert!(state.frames[2].has_keyframe);
        assert_eq!(
            state.current_frame, 1,
            "current_frame stays on original frame"
        );
    }

    #[test]
    fn test_duplicate_frame_out_of_bounds_fails() {
        let mut state = TimelineState::default();
        assert!(state.duplicate_frame(0).is_err());
    }

    #[test]
    fn test_reorder_forward() {
        let mut state = TimelineState {
            frames: (0..4)
                .map(|i| make_frame(vec![vec!['X'; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        state.reorder_frame(0, 2).unwrap();
        let labels: Vec<&str> = state.frames.iter().map(|f| f.label.as_str()).collect();
        assert_eq!(labels, vec!["1", "0", "2", "3"]);
    }

    #[test]
    fn test_reorder_backward() {
        let mut state = TimelineState {
            frames: (0..4)
                .map(|i| make_frame(vec![vec!['X'; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 3,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        state.reorder_frame(3, 0).unwrap();
        let labels: Vec<&str> = state.frames.iter().map(|f| f.label.as_str()).collect();
        assert_eq!(labels, vec!["3", "0", "1", "2"]);
    }

    #[test]
    fn test_reorder_out_of_bounds_fails() {
        let mut state = TimelineState::default();
        state.add_frame(make_frame(vec![vec!['A'; 3]; 2], false, "a"));
        assert!(state.reorder_frame(0, 1).is_err());
        assert!(state.reorder_frame(2, 0).is_err());
    }

    // ─── Keyframing tests ────────────────────────────────────────────

    fn make_keyframe(dx: i16, dy: i16, opacity: u8, blend: BlendMode) -> LayerKeyframe {
        LayerKeyframe {
            position_offset: (dx, dy),
            opacity,
            blend_mode: blend,
        }
    }

    #[test]
    fn test_set_keyframe_properties() {
        let mut state = TimelineState {
            frames: (0..5)
                .map(|i| make_frame(vec![vec!['X'; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        let kf = make_keyframe(5, -3, 128, BlendMode::Screen);
        assert!(state.set_keyframe(2, 0, kf));
        let got = state.get_keyframe(2, 0).unwrap();
        assert_eq!(got.position_offset, (5, -3));
        assert_eq!(got.opacity, 128);
        assert_eq!(got.blend_mode, BlendMode::Screen);
        assert!(state.frames[2].has_keyframe);
    }

    #[test]
    fn test_set_keyframe_out_of_bounds() {
        let mut state = TimelineState::default();
        let kf = make_keyframe(0, 0, 255, BlendMode::Normal);
        assert!(!state.set_keyframe(0, 0, kf));
    }

    #[test]
    fn test_interpolate_position_linear() {
        let mut state = TimelineState {
            frames: (0..11)
                .map(|i| make_frame(vec![vec![' '; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        state.set_keyframe(0, 0, make_keyframe(0, 0, 255, BlendMode::Normal));
        state.set_keyframe(10, 0, make_keyframe(100, 50, 255, BlendMode::Normal));
        let interp = state.get_interpolated_properties(5, 0);
        assert_eq!(interp.position_offset, (50, 25));
    }

    #[test]
    fn test_interpolate_opacity_linear() {
        let mut state = TimelineState {
            frames: (0..11)
                .map(|i| make_frame(vec![vec![' '; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        state.set_keyframe(0, 0, make_keyframe(0, 0, 255, BlendMode::Normal));
        state.set_keyframe(10, 0, make_keyframe(0, 0, 0, BlendMode::Normal));
        let at3 = state.get_interpolated_properties(3, 0);
        let at7 = state.get_interpolated_properties(7, 0);
        assert_eq!(at3.opacity, 179);
        assert_eq!(at7.opacity, 77);
    }

    #[test]
    fn test_interpolate_blend_mode_step() {
        let mut state = TimelineState {
            frames: (0..11)
                .map(|i| make_frame(vec![vec![' '; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        state.set_keyframe(0, 0, make_keyframe(0, 0, 255, BlendMode::Normal));
        state.set_keyframe(10, 0, make_keyframe(0, 0, 255, BlendMode::Multiply));
        for f in 0..5 {
            let props = state.get_interpolated_properties(f, 0);
            assert_eq!(
                props.blend_mode,
                BlendMode::Normal,
                "frame {f} should be Normal"
            );
        }
        for f in 5..10 {
            let props = state.get_interpolated_properties(f, 0);
            assert_eq!(
                props.blend_mode,
                BlendMode::Multiply,
                "frame {f} should be Multiply"
            );
        }
    }

    #[test]
    fn test_interpolate_before_first_keyframe() {
        let mut state = TimelineState {
            frames: (0..10)
                .map(|i| make_frame(vec![vec![' '; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        state.set_keyframe(5, 0, make_keyframe(42, 99, 128, BlendMode::Screen));
        let props = state.get_interpolated_properties(2, 0);
        assert_eq!(props.position_offset, (42, 99));
        assert_eq!(props.opacity, 128);
        assert_eq!(props.blend_mode, BlendMode::Screen);
    }

    #[test]
    fn test_interpolate_after_last_keyframe() {
        let mut state = TimelineState {
            frames: (0..10)
                .map(|i| make_frame(vec![vec![' '; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        state.set_keyframe(3, 0, make_keyframe(10, 20, 200, BlendMode::Add));
        let props = state.get_interpolated_properties(8, 0);
        assert_eq!(props.position_offset, (10, 20));
        assert_eq!(props.opacity, 200);
        assert_eq!(props.blend_mode, BlendMode::Add);
    }

    #[test]
    fn test_interpolate_single_keyframe() {
        let mut state = TimelineState {
            frames: (0..10)
                .map(|i| make_frame(vec![vec![' '; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        state.set_keyframe(5, 0, make_keyframe(7, 8, 100, BlendMode::Overlay));
        for f in 0..10 {
            let props = state.get_interpolated_properties(f, 0);
            assert_eq!(props.position_offset, (7, 8), "frame {f}");
            assert_eq!(props.opacity, 100, "frame {f}");
        }
    }

    #[test]
    fn test_interpolate_no_keyframes() {
        let state = TimelineState {
            frames: (0..5)
                .map(|i| make_frame(vec![vec![' '; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        let props = state.get_interpolated_properties(2, 0);
        assert_eq!(props.position_offset, (0, 0));
        assert_eq!(props.opacity, 255);
        assert_eq!(props.blend_mode, BlendMode::Normal);
    }

    #[test]
    fn test_remove_keyframe() {
        let mut state = TimelineState {
            frames: (0..10)
                .map(|i| make_frame(vec![vec![' '; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        state.set_keyframe(0, 0, make_keyframe(0, 0, 255, BlendMode::Normal));
        state.set_keyframe(5, 0, make_keyframe(50, 0, 128, BlendMode::Multiply));
        state.set_keyframe(9, 0, make_keyframe(100, 0, 0, BlendMode::Screen));
        assert!(state.remove_keyframe(5, 0));
        let props = state.get_interpolated_properties(5, 0);
        // After removal, interpolates between frame 0 (0,0) and frame 9 (100,0): lerp at t=5/9
        assert_eq!(props.position_offset, (56, 0));
    }

    #[test]
    fn test_keyframe_multi_layer() {
        let mut state = TimelineState {
            frames: (0..11)
                .map(|i| make_frame(vec![vec![' '; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        state.set_keyframe(0, 0, make_keyframe(0, 0, 100, BlendMode::Normal));
        state.set_keyframe(10, 0, make_keyframe(100, 0, 200, BlendMode::Normal));
        state.set_keyframe(0, 1, make_keyframe(0, 50, 50, BlendMode::Screen));
        state.set_keyframe(10, 1, make_keyframe(0, 100, 150, BlendMode::Add));
        let l0 = state.get_interpolated_properties(5, 0);
        let l1 = state.get_interpolated_properties(5, 1);
        assert_eq!(l0.position_offset, (50, 0));
        assert_eq!(l0.opacity, 150);
        assert_eq!(l1.position_offset, (0, 75));
        assert_eq!(l1.opacity, 100);
        assert_eq!(l1.blend_mode, BlendMode::Add);
    }

    #[test]
    fn test_keyframe_editor_open_close() {
        let mut state = TimelineState::default();
        assert!(!state.keyframe_editor.open);
        state.keyframe_editor.open = true;
        assert!(state.keyframe_editor.open);
        state.handle_keyframe_editor_key(KeyCode::Esc);
        assert!(!state.keyframe_editor.open);
    }

    #[test]
    fn test_keyframe_editor_navigation() {
        let mut state = TimelineState::default();
        state.keyframe_editor.open = true;
        state.keyframe_editor.selected_layer = 2;
        state.handle_keyframe_editor_key(KeyCode::Up);
        assert_eq!(state.keyframe_editor.selected_layer, 1);
        state.handle_keyframe_editor_key(KeyCode::Down);
        assert_eq!(state.keyframe_editor.selected_layer, 2);
        state.handle_keyframe_editor_key(KeyCode::Left);
        assert_eq!(state.keyframe_editor.selected_property, 0);
        state.keyframe_editor.selected_property = 2;
        state.handle_keyframe_editor_key(KeyCode::Right);
        assert_eq!(state.keyframe_editor.selected_property, 3);
        state.handle_keyframe_editor_key(KeyCode::Right);
        assert_eq!(state.keyframe_editor.selected_property, 3);
    }

    #[test]
    fn test_keyframe_editor_numeric_edit() {
        let mut state = TimelineState {
            frames: (0..5)
                .map(|i| make_frame(vec![vec![' '; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        state.keyframe_editor.open = true;
        state.set_keyframe(0, 0, make_keyframe(10, 20, 128, BlendMode::Normal));
        state.handle_keyframe_editor_key(KeyCode::Enter);
        assert!(state.keyframe_editor.edit_mode);
        state.handle_keyframe_editor_key(KeyCode::Backspace);
        state.handle_keyframe_editor_key(KeyCode::Backspace);
        state.handle_keyframe_editor_key(KeyCode::Char('3'));
        state.handle_keyframe_editor_key(KeyCode::Char('0'));
        state.handle_keyframe_editor_key(KeyCode::Enter);
        assert!(!state.keyframe_editor.edit_mode);
        let props = state.get_interpolated_properties(0, 0);
        assert_eq!(props.position_offset.0, 30);
    }

    #[test]
    fn test_keyframe_editor_blend_cycle() {
        let mut state = TimelineState {
            frames: (0..5)
                .map(|i| make_frame(vec![vec![' '; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        state.set_keyframe(0, 0, make_keyframe(0, 0, 255, BlendMode::Normal));
        state.keyframe_editor.open = true;
        state.keyframe_editor.selected_property = 3;
        // Blend mode: Normal -> Multiply
        state.handle_keyframe_editor_key(KeyCode::Enter);
        let props = state.get_interpolated_properties(0, 0);
        assert_eq!(props.blend_mode, BlendMode::Multiply);
    }

    #[test]
    fn test_keyframe_has_keyframe_derived() {
        let mut state = TimelineState {
            frames: (0..3)
                .map(|i| make_frame(vec![vec![' '; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        assert!(!state.frames[0].has_keyframe);
        state.set_keyframe(0, 0, make_keyframe(0, 0, 255, BlendMode::Normal));
        assert!(state.frames[0].has_keyframe);
        state.remove_keyframe(0, 0);
        assert!(!state.frames[0].has_keyframe);
    }

    #[test]
    fn test_get_keyframe_nonexistent() {
        let mut state = TimelineState::default();
        state.add_frame(make_frame(vec![vec![' '; 3]; 2], false, "0"));
        assert!(state.get_keyframe(0, 0).is_none());
        assert!(state.get_keyframe(5, 0).is_none());
    }

    #[test]
    fn test_interpolate_same_frame_keyframe() {
        let mut state = TimelineState {
            frames: (0..10)
                .map(|i| make_frame(vec![vec![' '; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        state.set_keyframe(5, 0, make_keyframe(10, 20, 100, BlendMode::Overlay));
        let props = state.get_interpolated_properties(5, 0);
        assert_eq!(props.position_offset, (10, 20));
        assert_eq!(props.opacity, 100);
        assert_eq!(props.blend_mode, BlendMode::Overlay);
    }

    // ─── Tweening tests ──────────────────────────────────────────────

    fn make_tween_state() -> TimelineState {
        let mut state = TimelineState {
            frames: (0..11)
                .map(|i| make_frame(vec![vec![' '; 3]; 2], false, &format!("{}", i)))
                .collect(),
            current_frame: 0,
            scroll_offset: 0,
            playing: false,
            fps: 12,
            keyframe_editor: KeyframeEditState::default(),
            tween: None,
        };
        state.set_keyframe(0, 0, make_keyframe(0, 0, 255, BlendMode::Normal));
        state.set_keyframe(10, 0, make_keyframe(100, 50, 0, BlendMode::Multiply));
        state
    }

    #[test]
    fn test_easing_linear() {
        let f = EasingFunction::Linear;
        assert!((f.apply(0.0) - 0.0).abs() < 1e-10);
        assert!((f.apply(0.5) - 0.5).abs() < 1e-10);
        assert!((f.apply(1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_easing_ease_in() {
        let f = EasingFunction::EaseIn;
        assert!((f.apply(0.0) - 0.0).abs() < 1e-10);
        assert!((f.apply(1.0) - 1.0).abs() < 1e-10);
        // EaseIn at t=0.5 is slower (cubic: 0.125) vs linear (0.5)
        assert!(f.apply(0.5) < 0.5);
        assert!((f.apply(0.5) - 0.125).abs() < 1e-10);
    }

    #[test]
    fn test_easing_ease_out() {
        let f = EasingFunction::EaseOut;
        assert!((f.apply(0.0) - 0.0).abs() < 1e-10);
        assert!((f.apply(1.0) - 1.0).abs() < 1e-10);
        // EaseOut at t=0.5 is faster (1 - 0.5^3 = 0.875) vs linear (0.5)
        assert!(f.apply(0.5) > 0.5);
    }

    #[test]
    fn test_easing_bounce() {
        let f = EasingFunction::Bounce;
        assert!((f.apply(0.0) - 0.0).abs() < 1e-10);
        assert!((f.apply(1.0) - 1.0).abs() < 1e-10);
        // Bounce overshoots 1.0 at some t < 1.0
        assert!(f.apply(0.5) > 0.5);
    }

    #[test]
    fn test_easing_display_names() {
        assert_eq!(EasingFunction::Linear.display_name(), "Linear");
        assert_eq!(EasingFunction::EaseIn.display_name(), "Ease In");
        assert_eq!(EasingFunction::EaseOut.display_name(), "Ease Out");
        assert_eq!(EasingFunction::Bounce.display_name(), "Bounce");
    }

    #[test]
    fn test_easing_cycle() {
        assert_eq!(EasingFunction::Linear.cycle(), EasingFunction::EaseIn);
        assert_eq!(EasingFunction::EaseIn.cycle(), EasingFunction::EaseOut);
        assert_eq!(EasingFunction::EaseOut.cycle(), EasingFunction::Bounce);
        assert_eq!(EasingFunction::Bounce.cycle(), EasingFunction::Linear);
    }

    #[test]
    fn test_tween_generates_correct_count() {
        let mut state = make_tween_state();
        state.open_tween();
        state.tween.as_mut().unwrap().config.start_frame = 0;
        state.tween.as_mut().unwrap().config.end_frame = 10;
        state.tween.as_mut().unwrap().config.num_frames = 5;
        state.compute_tween();
        let tp = state.tween.as_ref().unwrap();
        assert!(tp.valid);
        assert_eq!(tp.generated_frames.len(), 5);
    }

    #[test]
    fn test_tween_linear_midpoint() {
        let mut state = make_tween_state();
        state.open_tween();
        {
            let t = state.tween.as_mut().unwrap();
            t.config.start_frame = 0;
            t.config.end_frame = 10;
            t.config.num_frames = 1;
            t.config.easing = EasingFunction::Linear;
        }
        state.compute_tween();
        let tp = state.tween.as_ref().unwrap();
        assert!(tp.valid);
        assert_eq!(tp.generated_frames.len(), 1);
        let kf = tp.generated_frames[0].layer_keyframes[0].unwrap();
        // With 1 intermediate frame, t = 1/(1+1) = 0.5
        // Linear midpoint between (0,0) and (100,50) = (50, 25)
        assert_eq!(kf.position_offset, (50, 25));
    }

    #[test]
    fn test_tween_ease_in_midpoint() {
        let mut state = make_tween_state();
        state.open_tween();
        {
            let t = state.tween.as_mut().unwrap();
            t.config.start_frame = 0;
            t.config.end_frame = 10;
            t.config.num_frames = 1;
            t.config.easing = EasingFunction::EaseIn;
        }
        state.compute_tween();
        let tp = state.tween.as_ref().unwrap();
        assert!(tp.valid);
        let kf = tp.generated_frames[0].layer_keyframes[0].unwrap();
        // EaseIn at t=0.5: 0.125. Interpolate X: 0 + (100-0) * 0.125 = 12.5 ≈ 13
        assert!(kf.position_offset.0 < 50);
        assert_eq!(kf.position_offset.0, 13);
    }

    #[test]
    fn test_tween_ease_out_midpoint() {
        let mut state = make_tween_state();
        state.open_tween();
        {
            let t = state.tween.as_mut().unwrap();
            t.config.start_frame = 0;
            t.config.end_frame = 10;
            t.config.num_frames = 1;
            t.config.easing = EasingFunction::EaseOut;
        }
        state.compute_tween();
        let tp = state.tween.as_ref().unwrap();
        assert!(tp.valid);
        let kf = tp.generated_frames[0].layer_keyframes[0].unwrap();
        // EaseOut at t=0.5: 1 - (0.5)^3 = 0.875. Interpolate X: 0 + (100-0) * 0.875 = 87.5 ≈ 88
        assert!(kf.position_offset.0 > 50);
        assert_eq!(kf.position_offset.0, 88);
    }

    #[test]
    fn test_tween_bounce_midpoint() {
        let mut state = make_tween_state();
        state.open_tween();
        {
            let t = state.tween.as_mut().unwrap();
            t.config.start_frame = 0;
            t.config.end_frame = 10;
            t.config.num_frames = 1;
            t.config.easing = EasingFunction::Bounce;
        }
        state.compute_tween();
        let tp = state.tween.as_ref().unwrap();
        assert!(tp.valid);
        let kf = tp.generated_frames[0].layer_keyframes[0].unwrap();
        // Bounce at t=0.5 produces > 0.5, so position X > 50
        assert!(kf.position_offset.0 > 50);
    }

    #[test]
    fn test_tween_opacity_blend() {
        let mut state = make_tween_state();
        state.open_tween();
        {
            let t = state.tween.as_mut().unwrap();
            t.config.start_frame = 0;
            t.config.end_frame = 10;
            t.config.num_frames = 3;
        }
        state.compute_tween();
        let tp = state.tween.as_ref().unwrap();
        assert!(tp.valid);

        // Frame 0: t = 1/4 = 0.25, opacity: 255 + (0 - 255) * 0.25 = 191
        let kf0 = tp.generated_frames[0].layer_keyframes[0].unwrap();
        assert_eq!(kf0.opacity, 191);
        assert_eq!(kf0.blend_mode, BlendMode::Normal); // t < 0.5

        // Frame 1: t = 2/4 = 0.5, opacity: 255 + (0 - 255) * 0.5 = 128
        let kf1 = tp.generated_frames[1].layer_keyframes[0].unwrap();
        assert_eq!(kf1.opacity, 128);
        // blend_mode at t=0.5: step => end (Multiply) because t >= 0.5
        assert_eq!(kf1.blend_mode, BlendMode::Multiply);

        // Frame 2: t = 3/4 = 0.75, opacity: 255 + (0 - 255) * 0.75 = 64
        let kf2 = tp.generated_frames[2].layer_keyframes[0].unwrap();
        assert_eq!(kf2.opacity, 64);
        assert_eq!(kf2.blend_mode, BlendMode::Multiply); // t > 0.5
    }

    #[test]
    fn test_tween_commit_inserts_frames() {
        let mut state = make_tween_state();
        assert_eq!(state.frames.len(), 11);
        state.open_tween();
        {
            let t = state.tween.as_mut().unwrap();
            t.config.start_frame = 0;
            t.config.end_frame = 10;
            t.config.num_frames = 3;
        }
        state.compute_tween();
        assert_eq!(state.tween.as_ref().unwrap().generated_frames.len(), 3);
        state.commit_tween();
        assert_eq!(state.frames.len(), 14);
        assert!(state.tween.is_none());
        // Check frames 1, 2, 3 are tween frames
        assert!(state.frames[1].label.starts_with("tween"));
        assert!(state.frames[2].label.starts_with("tween"));
        assert!(state.frames[3].label.starts_with("tween"));
    }

    #[test]
    fn test_tween_commit_advances_current() {
        let mut state = make_tween_state();
        state.current_frame = 0;
        state.open_tween();
        {
            let t = state.tween.as_mut().unwrap();
            t.config.start_frame = 0;
            t.config.end_frame = 10;
            t.config.num_frames = 3;
        }
        state.compute_tween();
        state.commit_tween();
        // current_frame was 0, insert at 1, so current shifts by 3 → 3
        assert_eq!(state.current_frame, 3);
    }

    #[test]
    fn test_tween_discard_clears() {
        let mut state = make_tween_state();
        assert_eq!(state.frames.len(), 11);
        state.open_tween();
        state.discard_tween();
        assert!(state.tween.is_none());
        assert_eq!(state.frames.len(), 11);
    }

    #[test]
    fn test_tween_skip_unkeyframed_layer() {
        let mut state = make_tween_state();
        // Layer 1 has no keyframes
        state.open_tween();
        {
            let t = state.tween.as_mut().unwrap();
            t.config.start_frame = 0;
            t.config.end_frame = 10;
            t.config.num_frames = 2;
        }
        state.compute_tween();
        let tp = state.tween.as_ref().unwrap();
        assert!(tp.valid);
        // Layer 0 should have keyframes
        assert!(tp.generated_frames[0].layer_keyframes[0].is_some());
        // Layer 1 should not have keyframes (not keyframed in either boundary)
        assert!(
            tp.generated_frames[0].layer_keyframes.get(1).is_none()
                || tp.generated_frames[0].layer_keyframes[1].is_none()
        );
    }

    #[test]
    fn test_tween_start_equals_end() {
        let mut state = make_tween_state();
        state.open_tween();
        {
            let t = state.tween.as_mut().unwrap();
            t.config.start_frame = 0;
            t.config.end_frame = 0;
        }
        state.compute_tween();
        let tp = state.tween.as_ref().unwrap();
        assert!(!tp.valid);
        assert!(tp.generated_frames.is_empty());
    }

    #[test]
    fn test_tween_handle_key_navigation() {
        let mut state = make_tween_state();
        state.open_tween();
        assert!(state.tween.is_some());
        assert_eq!(state.tween.as_ref().unwrap().field_index, 0);

        state.handle_tween_key(KeyCode::Down);
        assert_eq!(state.tween.as_ref().unwrap().field_index, 1);

        state.handle_tween_key(KeyCode::Down);
        assert_eq!(state.tween.as_ref().unwrap().field_index, 2);

        state.handle_tween_key(KeyCode::Down);
        assert_eq!(state.tween.as_ref().unwrap().field_index, 3);

        // Cant go below 3
        state.handle_tween_key(KeyCode::Down);
        assert_eq!(state.tween.as_ref().unwrap().field_index, 3);

        state.handle_tween_key(KeyCode::Up);
        assert_eq!(state.tween.as_ref().unwrap().field_index, 2);
    }

    #[test]
    fn test_tween_handle_key_easing_cycle() {
        let mut state = make_tween_state();
        state.open_tween();
        {
            let t = state.tween.as_mut().unwrap();
            t.field_index = 3;
        }
        assert_eq!(
            state.tween.as_ref().unwrap().config.easing,
            EasingFunction::Linear
        );

        state.handle_tween_key(KeyCode::Right);
        assert_eq!(
            state.tween.as_ref().unwrap().config.easing,
            EasingFunction::EaseIn
        );

        state.handle_tween_key(KeyCode::Left);
        assert_eq!(
            state.tween.as_ref().unwrap().config.easing,
            EasingFunction::Linear
        );
    }

    #[test]
    fn test_tween_handle_key_esc_discards() {
        let mut state = make_tween_state();
        state.open_tween();
        assert!(state.tween.is_some());
        state.handle_tween_key(KeyCode::Esc);
        assert!(state.tween.is_none());
    }
}
