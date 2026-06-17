use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

use super::canvas::CanvasBuffer;
use super::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Overlay,
    Screen,
    Add,
    Subtract,
}

impl BlendMode {
    pub fn next(&self) -> Self {
        match self {
            BlendMode::Normal => BlendMode::Multiply,
            BlendMode::Multiply => BlendMode::Overlay,
            BlendMode::Overlay => BlendMode::Screen,
            BlendMode::Screen => BlendMode::Add,
            BlendMode::Add => BlendMode::Subtract,
            BlendMode::Subtract => BlendMode::Normal,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            BlendMode::Normal => BlendMode::Subtract,
            BlendMode::Multiply => BlendMode::Normal,
            BlendMode::Overlay => BlendMode::Multiply,
            BlendMode::Screen => BlendMode::Overlay,
            BlendMode::Add => BlendMode::Screen,
            BlendMode::Subtract => BlendMode::Add,
        }
    }

    pub fn icon_key(&self) -> &str {
        match self {
            BlendMode::Normal => "blend_normal",
            BlendMode::Multiply => "blend_multiply",
            BlendMode::Overlay => "blend_overlay",
            BlendMode::Screen => "blend_screen",
            BlendMode::Add => "blend_add",
            BlendMode::Subtract => "blend_subtract",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            BlendMode::Normal => "Normal",
            BlendMode::Multiply => "Multiply",
            BlendMode::Overlay => "Overlay",
            BlendMode::Screen => "Screen",
            BlendMode::Add => "Add",
            BlendMode::Subtract => "Subtract",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Layer {
    pub buffer: CanvasBuffer,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub opacity: u8,
    pub blend_mode: BlendMode,
}

impl Layer {
    pub fn new(width: usize, height: usize, name: String) -> Self {
        Self {
            buffer: CanvasBuffer::new(width, height),
            name,
            visible: true,
            locked: false,
            opacity: 255,
            blend_mode: BlendMode::Normal,
        }
    }

    pub fn buffer_mut(&mut self) -> &mut CanvasBuffer {
        &mut self.buffer
    }

    pub fn buffer(&self) -> &CanvasBuffer {
        &self.buffer
    }
}

#[derive(Debug, Clone)]
pub struct LayerStack {
    pub layers: Vec<Layer>,
    pub active: usize,
}

impl LayerStack {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            layers: vec![Layer::new(width, height, "Background".to_string())],
            active: 0,
        }
    }

    pub fn with_capacity(width: usize, height: usize, initial_layers: usize) -> Self {
        let mut layers = Vec::with_capacity(initial_layers.max(1));
        for i in 0..initial_layers.max(1) {
            let name = if i == 0 {
                "Background".to_string()
            } else {
                format!("Layer {}", i + 1)
            };
            layers.push(Layer::new(width, height, name));
        }
        Self { layers, active: 0 }
    }

    pub fn len(&self) -> usize {
        self.layers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    pub fn active_layer(&self) -> &Layer {
        &self.layers[self.active.min(self.layers.len().saturating_sub(1))]
    }

    pub fn active_layer_mut(&mut self) -> &mut Layer {
        let idx = self.active.min(self.layers.len().saturating_sub(1));
        &mut self.layers[idx]
    }

    pub fn add(&mut self, width: usize, height: usize) -> usize {
        let idx = self.layers.len();
        self.layers
            .push(Layer::new(width, height, format!("Layer {}", idx + 1)));
        self.active = idx;
        idx
    }

    pub fn delete(&mut self, index: usize) -> bool {
        if index >= self.layers.len() || self.layers.len() <= 1 {
            return false;
        }
        self.layers.remove(index);
        if self.active >= self.layers.len() {
            self.active = self.layers.len() - 1;
        } else if self.active > 0 && index <= self.active {
            self.active = self.active.saturating_sub(1);
        }
        true
    }

    pub fn duplicate(&mut self, index: usize) -> bool {
        if index >= self.layers.len() {
            return false;
        }
        let original = self.layers[index].clone();
        let idx = self.layers.len();
        self.layers.push(original);
        self.active = idx;
        true
    }

    pub fn merge_down(&mut self, index: usize) -> bool {
        if index == 0 || index >= self.layers.len() {
            return false;
        }
        if self.layers[index].locked {
            return false;
        }
        let above = self.layers.remove(index);
        let below = &mut self.layers[index - 1];
        for y in 0..below.buffer.height().min(above.buffer.height()) {
            for x in 0..below.buffer.width().min(above.buffer.width()) {
                if let Some(cell) = above.buffer.get(x, y) {
                    if cell.ch != ' ' {
                        below.buffer.set(x, y, *cell);
                    }
                }
            }
        }
        self.active = (index - 1).min(self.layers.len().saturating_sub(1));
        true
    }

    pub fn move_up(&mut self, index: usize) -> bool {
        if index + 1 >= self.layers.len() {
            return false;
        }
        self.layers.swap(index, index + 1);
        if self.active == index {
            self.active = index + 1;
        } else if self.active == index + 1 {
            self.active = index;
        }
        true
    }

    pub fn move_down(&mut self, index: usize) -> bool {
        if index == 0 || index >= self.layers.len() {
            return false;
        }
        self.layers.swap(index, index - 1);
        if self.active == index {
            self.active = index - 1;
        } else if self.active == index - 1 {
            self.active = index;
        }
        true
    }

    pub fn reorder(&mut self, from: usize, to: usize) -> bool {
        if from >= self.layers.len() || to >= self.layers.len() || from == to {
            return false;
        }
        let layer = self.layers.remove(from);
        self.layers.insert(to, layer);
        if self.active == from {
            self.active = to;
        } else {
            if from < self.active && to >= self.active {
                self.active = self.active.saturating_sub(1);
            } else if from > self.active && to <= self.active {
                self.active = self.active.saturating_add(1);
            }
        }
        true
    }

    pub fn set_active(&mut self, index: usize) {
        if index < self.layers.len() {
            self.active = index;
        }
    }

    pub fn resize_all(&mut self, width: usize, height: usize) {
        for layer in &mut self.layers {
            if layer.buffer.width() != width || layer.buffer.height() != height {
                let mut new_buf = CanvasBuffer::new(width, height);
                for y in 0..layer.buffer.height().min(height) {
                    for x in 0..layer.buffer.width().min(width) {
                        if let Some(cell) = layer.buffer.get(x, y) {
                            new_buf.set(x, y, *cell);
                        }
                    }
                }
                layer.buffer = new_buf;
            }
        }
    }

    pub fn composite(&self) -> CanvasBuffer {
        if self.layers.is_empty() {
            return CanvasBuffer::new(1, 1);
        }
        let w = self.layers[0].buffer.width();
        let h = self.layers[0].buffer.height();
        let mut result = CanvasBuffer::new(w, h);
        for layer in &self.layers {
            if !layer.visible {
                continue;
            }
            let opacity = layer.opacity;
            let blend_mode = layer.blend_mode;
            for y in 0..h.min(layer.buffer.height()) {
                for x in 0..w.min(layer.buffer.width()) {
                    if let Some(top) = layer.buffer.get(x, y) {
                        if top.ch == ' ' && top.fg.is_none() && top.bg.is_none() {
                            continue;
                        }
                        if opacity == 255 && blend_mode == BlendMode::Normal {
                            result.set(x, y, *top);
                        } else if opacity > 0 {
                            let bottom = result.get(x, y).copied().unwrap_or_default();
                            let blended_fg = blend_mode_color(top.fg, bottom.fg, blend_mode);
                            let blended_bg = blend_mode_color(top.bg, bottom.bg, blend_mode);
                            let final_fg = blend_colors(blended_fg, bottom.fg, opacity);
                            let final_bg = blend_colors(blended_bg, bottom.bg, opacity);
                            result.set(
                                x,
                                y,
                                super::canvas::CanvasCell {
                                    ch: top.ch,
                                    fg: final_fg,
                                    bg: final_bg,
                                },
                            );
                        }
                    }
                }
            }
        }
        result
    }

    pub fn toggle_visibility(&mut self, index: usize) {
        if let Some(layer) = self.layers.get_mut(index) {
            layer.visible = !layer.visible;
        }
    }

    pub fn toggle_lock(&mut self, index: usize) {
        if let Some(layer) = self.layers.get_mut(index) {
            layer.locked = !layer.locked;
        }
    }

    pub fn set_opacity(&mut self, index: usize, opacity: u8) {
        if let Some(layer) = self.layers.get_mut(index) {
            layer.opacity = opacity;
        }
    }

    pub fn set_blend_mode(&mut self, index: usize, mode: BlendMode) {
        if let Some(layer) = self.layers.get_mut(index) {
            layer.blend_mode = mode;
        }
    }
}

fn blend_colors(top: Option<Color>, bottom: Option<Color>, opacity: u8) -> Option<Color> {
    match (top, bottom) {
        (Some(t), Some(b)) => {
            let f = opacity as f32 / 255.0;
            match (t, b) {
                (Color::Rgb(tr, tg, tb), Color::Rgb(br, bg, bb)) => {
                    let r = (tr as f32 * f + br as f32 * (1.0 - f)).round() as u8;
                    let g = (tg as f32 * f + bg as f32 * (1.0 - f)).round() as u8;
                    let b = (tb as f32 * f + bb as f32 * (1.0 - f)).round() as u8;
                    Some(Color::Rgb(r, g, b))
                }
                _ => Some(t),
            }
        }
        (Some(t), None) => Some(t),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn blend_channel(top: u8, bottom: u8, mode: BlendMode) -> u8 {
    match mode {
        BlendMode::Normal => top,
        BlendMode::Multiply => ((top as u32) * (bottom as u32) / 255) as u8,
        BlendMode::Overlay => {
            if bottom < 128 {
                ((2u32 * top as u32 * bottom as u32) / 255) as u8
            } else {
                (255u32 - (2u32 * (255u32 - top as u32) * (255u32 - bottom as u32)) / 255) as u8
            }
        }
        BlendMode::Screen => {
            (255u32 - ((255u32 - top as u32) * (255u32 - bottom as u32)) / 255) as u8
        }
        BlendMode::Add => top.saturating_add(bottom),
        BlendMode::Subtract => bottom.saturating_sub(top),
    }
}

fn blend_mode_color(top: Option<Color>, bottom: Option<Color>, mode: BlendMode) -> Option<Color> {
    if mode == BlendMode::Normal {
        return top;
    }
    match (top, bottom) {
        (Some(t), Some(b)) => match (t, b) {
            (Color::Rgb(tr, tg, tb), Color::Rgb(br, bg, bb)) => {
                let r = blend_channel(tr, br, mode);
                let g = blend_channel(tg, bg, mode);
                let b = blend_channel(tb, bb, mode);
                Some(Color::Rgb(r, g, b))
            }
            _ => Some(t),
        },
        (Some(t), None) => Some(t),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

pub struct LayerPanel {
    pub scroll: u16,
    pub theme: Theme,
    pub icons: std::collections::BTreeMap<String, String>,
}

impl LayerPanel {
    pub fn new() -> Self {
        Self {
            scroll: 0,
            theme: Theme::default(),
            icons: std::collections::BTreeMap::new(),
        }
    }

    pub fn handle_key(&mut self, code: KeyCode, stack: &mut LayerStack) -> bool {
        match code {
            KeyCode::Up => {
                if stack.active > 0 {
                    stack.active -= 1;
                }
                true
            }
            KeyCode::Down => {
                if stack.active + 1 < stack.layers.len() {
                    stack.active += 1;
                }
                true
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                stack.toggle_visibility(stack.active);
                true
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                stack.toggle_lock(stack.active);
                true
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                let o = stack.layers[stack.active].opacity;
                stack.layers[stack.active].opacity = o.saturating_add(25);
                true
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                let o = stack.layers[stack.active].opacity;
                stack.layers[stack.active].opacity = o.saturating_sub(25);
                true
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                let w = stack.layers[0].buffer.width();
                let h = stack.layers[0].buffer.height();
                stack.add(w, h);
                true
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                stack.duplicate(stack.active);
                true
            }
            KeyCode::Delete | KeyCode::Char('x') | KeyCode::Char('X') => {
                stack.delete(stack.active);
                true
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                stack.merge_down(stack.active);
                true
            }
            KeyCode::Char(',') => {
                stack.move_up(stack.active);
                true
            }
            KeyCode::Char('.') => {
                stack.move_down(stack.active);
                true
            }
            KeyCode::Char('b') => {
                let mode = stack.layers[stack.active].blend_mode;
                stack.layers[stack.active].blend_mode = mode.next();
                true
            }
            KeyCode::Char('B') => {
                let mode = stack.layers[stack.active].blend_mode;
                stack.layers[stack.active].blend_mode = mode.prev();
                true
            }
            _ => false,
        }
    }
}

impl Default for LayerPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl LayerPanel {
    pub fn render_with_stack(&self, frame: &mut Frame, area: Rect, stack: &LayerStack) {
        let block = Block::default()
            .title(" Layers ")
            .borders(Borders::ALL)
            .style(
                Style::default()
                    .bg(self.theme.menu.dropdown_bg)
                    .fg(self.theme.menu.fg),
            );
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let inner_y = inner.y;
        let inner_x = inner.x;
        let inner_h = inner.height as usize;
        let name_max = (inner.width as usize).saturating_sub(11).max(4);
        let help_lines = [
            "↑↓ sel ↵vis Llock ±opa Bbld",
            "Nnew Ddup Xdel Mmerge",
            ",↑ .↓ reorder",
        ];

        for (y, help) in help_lines.iter().enumerate() {
            if y >= inner_h {
                return;
            }
            let para = ratatui::widgets::Paragraph::new(Line::from(Span::styled(
                *help,
                Style::default()
                    .fg(self.theme.general.secondary)
                    .add_modifier(Modifier::DIM),
            )));
            frame.render_widget(
                para,
                Rect {
                    x: inner_x,
                    y: inner_y + y as u16,
                    width: inner.width,
                    height: 1,
                },
            );
        }

        let offset = help_lines.len() + 1;

        let vis_icon = "v";

        for (rev_idx, layer) in stack.layers.iter().enumerate().rev() {
            let y = offset + rev_idx;
            if y >= inner_h {
                break;
            }

            let real_idx = stack.layers.len() - 1 - rev_idx;
            let is_active = real_idx == stack.active;
            let row_bg = if is_active {
                self.theme.menu.highlight
            } else {
                self.theme.menu.dropdown_bg
            };

            let vis_ch = if layer.visible { vis_icon } else { " " };
            let lock_ch = if layer.locked { "L" } else { " " };
            let blend_icon = self
                .icons
                .get(layer.blend_mode.icon_key())
                .map(|s| s.as_str())
                .unwrap_or("");

            let label = format!(
                " {} {} {} {} {:3}% {}",
                vis_ch,
                lock_ch,
                if is_active { ">" } else { " " },
                blend_icon,
                (layer.opacity as f32 / 255.0 * 100.0).round() as u8,
                truncate_str(&layer.name, name_max),
            );
            let row_style = Style::default().bg(row_bg).fg(self.theme.menu.fg);

            let row_para =
                ratatui::widgets::Paragraph::new(Line::from(Span::styled(label, row_style)));
            frame.render_widget(
                row_para,
                Rect {
                    x: inner_x,
                    y: inner_y + y as u16,
                    width: inner.width,
                    height: 1,
                },
            );
        }
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!(
            "{}…",
            s.chars()
                .take(max_len.saturating_sub(1))
                .collect::<String>()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::canvas::CanvasCell;
    use super::*;

    fn make_cell(ch: char) -> CanvasCell {
        CanvasCell {
            ch,
            fg: None,
            bg: None,
        }
    }

    fn make_stack(w: usize, h: usize) -> LayerStack {
        LayerStack::new(w, h)
    }

    #[test]
    fn test_new_layer() {
        let mut stack = make_stack(10, 10);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.active, 0);
        stack.add(10, 10);
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.active, 1);
    }

    #[test]
    fn test_delete_layer() {
        let mut stack = make_stack(10, 10);
        stack.add(10, 10);
        stack.add(10, 10);
        assert_eq!(stack.len(), 3);
        stack.active = 1;
        assert!(stack.delete(1));
        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn test_delete_cannot_remove_last() {
        let mut stack = make_stack(10, 10);
        assert!(!stack.delete(0));
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_duplicate_layer() {
        let mut stack = make_stack(10, 10);
        stack.layers[0].buffer.set(0, 0, make_cell('X'));
        assert!(stack.duplicate(0));
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.layers[1].buffer.get(0, 0).unwrap().ch, 'X');
        stack.layers[1].buffer.set(0, 0, make_cell('Y'));
        assert_eq!(stack.layers[0].buffer.get(0, 0).unwrap().ch, 'X');
    }

    #[test]
    fn test_reorder_layers() {
        let mut stack = make_stack(10, 10);
        stack.add(10, 10);
        stack.add(10, 10);
        stack.layers[0].name = "A".to_string();
        stack.layers[1].name = "B".to_string();
        stack.layers[2].name = "C".to_string();
        assert!(stack.move_up(0));
        assert_eq!(stack.layers[0].name, "B");
        assert_eq!(stack.layers[1].name, "A");
        assert!(stack.move_down(1));
        assert_eq!(stack.layers[0].name, "A");
        assert_eq!(stack.layers[1].name, "B");
    }

    #[test]
    fn test_merge_down() {
        let mut stack = make_stack(5, 5);
        stack.add(5, 5);
        stack.layers[0].buffer.set(0, 0, make_cell('A'));
        stack.layers[1].buffer.set(1, 1, make_cell('B'));
        assert!(stack.merge_down(1));
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.layers[0].buffer.get(0, 0).unwrap().ch, 'A');
        assert_eq!(stack.layers[0].buffer.get(1, 1).unwrap().ch, 'B');
    }

    #[test]
    fn test_merge_down_locked_top_noop() {
        let mut stack = make_stack(5, 5);
        stack.add(5, 5);
        stack.layers[1].locked = true;
        assert!(!stack.merge_down(1));
        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn test_composite_visibility() {
        let mut stack = make_stack(3, 3);
        stack.add(3, 3);
        stack.layers[0].buffer.set(0, 0, make_cell('A'));
        stack.layers[1].buffer.set(1, 1, make_cell('B'));
        stack.layers[1].visible = false;
        let comp = stack.composite();
        assert_eq!(comp.get(0, 0).unwrap().ch, 'A');
        assert_eq!(comp.get(1, 1).unwrap().ch, ' ');
    }

    #[test]
    fn test_composite_order() {
        let mut stack = make_stack(3, 3);
        stack.add(3, 3);
        stack.layers[0].buffer.set(0, 0, make_cell('A'));
        stack.layers[1].buffer.set(0, 0, make_cell('B'));
        let comp = stack.composite();
        assert_eq!(comp.get(0, 0).unwrap().ch, 'B');
    }

    #[test]
    fn test_composite_opacity() {
        let mut stack = make_stack(3, 3);
        stack.add(3, 3);
        stack.layers[0].buffer.set(0, 0, make_cell('A'));
        stack.layers[1].buffer.set(0, 0, make_cell('B'));
        stack.layers[1].opacity = 128;
        let comp = stack.composite();
        assert_eq!(comp.get(0, 0).unwrap().ch, 'B');
    }

    #[test]
    fn test_active_mut_independence() {
        let mut stack = make_stack(5, 5);
        stack.add(5, 5);
        stack.active = 1;
        stack.active_layer_mut().buffer.set(0, 0, make_cell('X'));
        assert_eq!(stack.layers[1].buffer.get(0, 0).unwrap().ch, 'X');
        assert_eq!(stack.layers[0].buffer.get(0, 0).unwrap().ch, ' ');
    }

    #[test]
    fn test_resize_all() {
        let mut stack = make_stack(5, 5);
        stack.add(5, 5);
        stack.resize_all(10, 10);
        assert_eq!(stack.layers[0].buffer.width(), 10);
        assert_eq!(stack.layers[0].buffer.height(), 10);
        assert_eq!(stack.layers[1].buffer.width(), 10);
    }

    #[test]
    fn test_composite_empty_stack() {
        let mut stack = make_stack(1, 1);
        stack.layers.clear();
        let comp = stack.composite();
        assert_eq!(comp.width(), 1);
        assert_eq!(comp.height(), 1);
    }

    #[test]
    fn test_toggle_visibility() {
        let mut stack = make_stack(5, 5);
        assert!(stack.layers[0].visible);
        stack.toggle_visibility(0);
        assert!(!stack.layers[0].visible);
        stack.toggle_visibility(0);
        assert!(stack.layers[0].visible);
    }

    #[test]
    fn test_toggle_lock() {
        let mut stack = make_stack(5, 5);
        assert!(!stack.layers[0].locked);
        stack.toggle_lock(0);
        assert!(stack.layers[0].locked);
        stack.toggle_lock(0);
        assert!(!stack.layers[0].locked);
    }

    #[test]
    fn test_set_opacity() {
        let mut stack = make_stack(5, 5);
        stack.set_opacity(0, 128);
        assert_eq!(stack.layers[0].opacity, 128);
        stack.set_opacity(0, 255);
        assert_eq!(stack.layers[0].opacity, 255);
    }

    #[test]
    fn test_move_up_edge() {
        let mut stack = make_stack(5, 5);
        assert!(!stack.move_up(0));
    }

    #[test]
    fn test_move_down_edge() {
        let mut stack = make_stack(5, 5);
        assert!(!stack.move_down(0));
    }

    #[test]
    fn test_reorder_invalid() {
        let mut stack = make_stack(5, 5);
        stack.add(5, 5);
        assert!(!stack.reorder(0, 0));
    }

    #[test]
    fn test_with_capacity() {
        let stack = LayerStack::with_capacity(10, 10, 3);
        assert_eq!(stack.len(), 3);
        assert_eq!(stack.layers[0].name, "Background");
        assert_eq!(stack.layers[1].name, "Layer 2");
        assert_eq!(stack.layers[2].name, "Layer 3");
    }

    // --- Blend mode tests ---

    fn rgb_cell(ch: char, r: u8, g: u8, b: u8) -> CanvasCell {
        CanvasCell {
            ch,
            fg: Some(Color::Rgb(r, g, b)),
            bg: None,
        }
    }

    #[test]
    fn test_blend_multiply() {
        let top = rgb_cell('X', 200, 100, 50);
        let bottom = rgb_cell('Y', 100, 200, 50);
        let result = blend_mode_color(top.fg, bottom.fg, BlendMode::Multiply);
        match result {
            Some(Color::Rgb(r, g, b)) => {
                assert_eq!(r, 78);
                assert_eq!(g, 78);
                assert_eq!(b, 9);
            }
            _ => panic!("expected Some(Color::Rgb)"),
        }
    }

    #[test]
    fn test_blend_overlay_dark() {
        let top = rgb_cell('X', 200, 50, 50);
        let bottom = rgb_cell('Y', 50, 100, 150);
        let result = blend_mode_color(top.fg, bottom.fg, BlendMode::Overlay);
        match result {
            Some(Color::Rgb(r, g, b)) => {
                assert_eq!(r, 78);
                assert_eq!(g, 39);
                assert_eq!(b, 87);
            }
            _ => panic!("expected Some(Color::Rgb)"),
        }
    }

    #[test]
    fn test_blend_overlay_light() {
        let top = rgb_cell('X', 200, 50, 50);
        let bottom = rgb_cell('Y', 200, 100, 50);
        let result = blend_mode_color(top.fg, bottom.fg, BlendMode::Overlay);
        match result {
            Some(Color::Rgb(r, g, b)) => {
                assert_eq!(r, 232);
                assert_eq!(g, 39);
                assert_eq!(b, 19);
            }
            _ => panic!("expected Some(Color::Rgb)"),
        }
    }

    #[test]
    fn test_blend_screen() {
        let top = rgb_cell('X', 200, 100, 50);
        let bottom = rgb_cell('Y', 100, 200, 50);
        let result = blend_mode_color(top.fg, bottom.fg, BlendMode::Screen);
        match result {
            Some(Color::Rgb(r, g, b)) => {
                assert_eq!(r, 222);
                assert_eq!(g, 222);
                assert_eq!(b, 91);
            }
            _ => panic!("expected Some(Color::Rgb)"),
        }
    }

    #[test]
    fn test_blend_add() {
        let top = rgb_cell('X', 200, 100, 200);
        let bottom = rgb_cell('Y', 100, 200, 50);
        let result = blend_mode_color(top.fg, bottom.fg, BlendMode::Add);
        match result {
            Some(Color::Rgb(r, g, b)) => {
                assert_eq!(r, 255);
                assert_eq!(g, 255);
                assert_eq!(b, 250);
            }
            _ => panic!("expected Some(Color::Rgb)"),
        }
    }

    #[test]
    fn test_blend_subtract() {
        let top = rgb_cell('X', 200, 100, 50);
        let bottom = rgb_cell('Y', 100, 200, 50);
        let result = blend_mode_color(top.fg, bottom.fg, BlendMode::Subtract);
        match result {
            Some(Color::Rgb(r, g, b)) => {
                assert_eq!(r, 0);
                assert_eq!(g, 100);
                assert_eq!(b, 0);
            }
            _ => panic!("expected Some(Color::Rgb)"),
        }
    }

    #[test]
    fn test_composite_blend_mode() {
        let mut stack = make_stack(3, 3);
        stack.add(3, 3);
        stack.layers[0]
            .buffer
            .set(0, 0, rgb_cell('A', 200, 100, 50));
        stack.layers[1]
            .buffer
            .set(0, 0, rgb_cell('B', 100, 200, 50));
        stack.layers[1].blend_mode = BlendMode::Multiply;
        let comp = stack.composite();
        let cell = comp.get(0, 0).unwrap();
        assert_eq!(cell.ch, 'B');
        match cell.fg {
            Some(Color::Rgb(r, g, b)) => {
                assert_eq!(r, 78);
                assert_eq!(g, 78);
                assert_eq!(b, 9);
            }
            _ => panic!("expected Some(Color::Rgb)"),
        }
    }

    #[test]
    fn test_blend_mode_cycle() {
        let mut mode = BlendMode::Normal;
        mode = mode.next();
        assert_eq!(mode, BlendMode::Multiply);
        mode = mode.next();
        assert_eq!(mode, BlendMode::Overlay);
        mode = mode.next();
        assert_eq!(mode, BlendMode::Screen);
        mode = mode.next();
        assert_eq!(mode, BlendMode::Add);
        mode = mode.next();
        assert_eq!(mode, BlendMode::Subtract);
        mode = mode.next();
        assert_eq!(mode, BlendMode::Normal);
        // prev cycle
        mode = mode.prev();
        assert_eq!(mode, BlendMode::Subtract);
        mode = mode.prev();
        assert_eq!(mode, BlendMode::Add);
        mode = mode.prev();
        assert_eq!(mode, BlendMode::Screen);
        mode = mode.prev();
        assert_eq!(mode, BlendMode::Overlay);
        mode = mode.prev();
        assert_eq!(mode, BlendMode::Multiply);
        mode = mode.prev();
        assert_eq!(mode, BlendMode::Normal);
    }

    #[test]
    fn test_set_blend_mode() {
        let mut stack = make_stack(5, 5);
        assert_eq!(stack.layers[0].blend_mode, BlendMode::Normal);
        stack.set_blend_mode(0, BlendMode::Multiply);
        assert_eq!(stack.layers[0].blend_mode, BlendMode::Multiply);
        stack.set_blend_mode(0, BlendMode::Overlay);
        assert_eq!(stack.layers[0].blend_mode, BlendMode::Overlay);
    }

    #[test]
    fn test_blend_mode_icon_key() {
        assert_eq!(BlendMode::Normal.icon_key(), "blend_normal");
        assert_eq!(BlendMode::Multiply.icon_key(), "blend_multiply");
        assert_eq!(BlendMode::Overlay.icon_key(), "blend_overlay");
        assert_eq!(BlendMode::Screen.icon_key(), "blend_screen");
        assert_eq!(BlendMode::Add.icon_key(), "blend_add");
        assert_eq!(BlendMode::Subtract.icon_key(), "blend_subtract");
    }

    #[test]
    fn test_blend_mode_display_name() {
        assert_eq!(BlendMode::Normal.display_name(), "Normal");
        assert_eq!(BlendMode::Multiply.display_name(), "Multiply");
        assert_eq!(BlendMode::Overlay.display_name(), "Overlay");
        assert_eq!(BlendMode::Screen.display_name(), "Screen");
        assert_eq!(BlendMode::Add.display_name(), "Add");
        assert_eq!(BlendMode::Subtract.display_name(), "Subtract");
    }

    #[test]
    fn test_blend_mode_normal_returns_top() {
        let top = rgb_cell('X', 100, 150, 200);
        let bottom = rgb_cell('Y', 50, 100, 150);
        let result = blend_mode_color(top.fg, bottom.fg, BlendMode::Normal);
        assert_eq!(result, top.fg);
    }

    #[test]
    fn test_composite_blend_mode_with_opacity() {
        let mut stack = make_stack(3, 3);
        stack.add(3, 3);
        stack.layers[0]
            .buffer
            .set(0, 0, rgb_cell('A', 200, 100, 50));
        stack.layers[1]
            .buffer
            .set(0, 0, rgb_cell('B', 100, 200, 50));
        stack.layers[1].blend_mode = BlendMode::Multiply;
        stack.layers[1].opacity = 128;
        let comp = stack.composite();
        let cell = comp.get(0, 0).unwrap();
        assert_eq!(cell.ch, 'B');
        // With opacity=128, result is lerp between bottom and multiply blend
        match cell.fg {
            Some(Color::Rgb(r, g, b)) => {
                assert_eq!(r, 139);
                assert_eq!(g, 89);
                assert_eq!(b, 29);
            }
            _ => panic!("expected Some(Color::Rgb)"),
        }
    }
}
