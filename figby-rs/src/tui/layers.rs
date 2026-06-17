use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

use super::canvas::{CanvasBuffer, CanvasCell};
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
pub struct LayerMask {
    pub buffer: CanvasBuffer,
    pub enabled: bool,
}

impl LayerMask {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            buffer: CanvasBuffer::new(width, height),
            enabled: true,
        }
    }

    pub fn buffer(&self) -> &CanvasBuffer {
        &self.buffer
    }

    pub fn buffer_mut(&mut self) -> &mut CanvasBuffer {
        &mut self.buffer
    }
}

#[derive(Debug, Clone)]
pub struct LayerGroup {
    pub name: String,
    pub collapsed: bool,
}

impl LayerGroup {
    pub fn new(name: String) -> Self {
        Self {
            name,
            collapsed: false,
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
    pub mask: Option<LayerMask>,
    pub group: Option<usize>,
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
            mask: None,
            group: None,
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
    pub groups: Vec<LayerGroup>,
}

impl LayerStack {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            layers: vec![Layer::new(width, height, "Background".to_string())],
            active: 0,
            groups: Vec::new(),
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
        Self {
            layers,
            active: 0,
            groups: Vec::new(),
        }
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

    pub fn create_group(&mut self, indices: &[usize], name: String) -> Option<usize> {
        if indices.is_empty() {
            return None;
        }
        for &idx in indices {
            if idx >= self.layers.len() {
                return None;
            }
        }
        let group_idx = self.groups.len();
        self.groups.push(LayerGroup::new(name));
        for &idx in indices {
            self.layers[idx].group = Some(group_idx);
        }
        Some(group_idx)
    }

    pub fn remove_group(&mut self, group_idx: usize) -> bool {
        if group_idx >= self.groups.len() {
            return false;
        }
        for layer in &mut self.layers {
            if layer.group == Some(group_idx) {
                layer.group = None;
            }
        }
        self.groups.remove(group_idx);
        for layer in &mut self.layers {
            if let Some(g) = layer.group {
                if g > group_idx {
                    layer.group = Some(g - 1);
                }
            }
        }
        true
    }

    pub fn toggle_group_collapsed(&mut self, group_idx: usize) -> bool {
        if let Some(group) = self.groups.get_mut(group_idx) {
            group.collapsed = !group.collapsed;
            true
        } else {
            false
        }
    }

    pub fn rename_group(&mut self, group_idx: usize, name: String) -> bool {
        if let Some(group) = self.groups.get_mut(group_idx) {
            group.name = name;
            true
        } else {
            false
        }
    }

    pub fn group_of_layer(&self, layer_idx: usize) -> Option<usize> {
        self.layers.get(layer_idx).and_then(|l| l.group)
    }

    pub fn layers_in_group(&self, group_idx: usize) -> Vec<usize> {
        self.layers
            .iter()
            .enumerate()
            .filter(|(_, l)| l.group == Some(group_idx))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn create_mask(&mut self, layer_idx: usize) -> bool {
        if let Some(layer) = self.layers.get_mut(layer_idx) {
            if layer.mask.is_some() {
                return false;
            }
            let w = layer.buffer.width();
            let h = layer.buffer.height();
            layer.mask = Some(LayerMask::new(w, h));
            true
        } else {
            false
        }
    }

    pub fn remove_mask(&mut self, layer_idx: usize) -> bool {
        if let Some(layer) = self.layers.get_mut(layer_idx) {
            if layer.mask.is_some() {
                layer.mask = None;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn toggle_mask(&mut self, layer_idx: usize) -> bool {
        let layer = match self.layers.get_mut(layer_idx) {
            Some(l) => l,
            None => return false,
        };
        if layer.mask.is_some() {
            layer.mask = None;
        } else {
            let w = layer.buffer.width();
            let h = layer.buffer.height();
            layer.mask = Some(LayerMask::new(w, h));
        }
        true
    }

    pub fn toggle_mask_enabled(&mut self, layer_idx: usize) -> bool {
        if let Some(layer) = self.layers.get_mut(layer_idx) {
            if let Some(ref mut mask) = layer.mask {
                mask.enabled = !mask.enabled;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn set_mask_pixel(&mut self, layer_idx: usize, x: usize, y: usize, cell: CanvasCell) {
        if let Some(layer) = self.layers.get_mut(layer_idx) {
            if let Some(ref mut mask) = layer.mask {
                if x < mask.buffer.width() && y < mask.buffer.height() {
                    mask.buffer.set(x, y, cell);
                }
            }
        }
    }

    pub fn get_mask_pixel(&self, layer_idx: usize, x: usize, y: usize) -> Option<CanvasCell> {
        self.layers
            .get(layer_idx)
            .and_then(|l| l.mask.as_ref())
            .and_then(|m| m.buffer.get(x, y))
            .copied()
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
                        if let Some(ref mask) = layer.mask {
                            if mask.enabled {
                                if let Some(mask_cell) = mask.buffer.get(x, y) {
                                    if mask_cell.ch == ' ' {
                                        continue;
                                    }
                                }
                            }
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

pub(crate) fn blend_colors(
    top: Option<Color>,
    bottom: Option<Color>,
    opacity: u8,
) -> Option<Color> {
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

pub(crate) fn blend_mode_color(
    top: Option<Color>,
    bottom: Option<Color>,
    mode: BlendMode,
) -> Option<Color> {
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

    pub fn handle_key(&mut self, key: KeyEvent, stack: &mut LayerStack) -> bool {
        let code = key.code;
        let modifiers = key.modifiers;

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
            KeyCode::Char('g') if modifiers == KeyModifiers::CONTROL => {
                let indices = [stack.active];
                let group_name = format!("Group {}", stack.groups.len() + 1);
                stack.create_group(&indices, group_name);
                true
            }
            KeyCode::Char('g') if modifiers == KeyModifiers::CONTROL | KeyModifiers::SHIFT => {
                if let Some(g) = stack.group_of_layer(stack.active) {
                    stack.remove_group(g);
                }
                true
            }
            KeyCode::Right => {
                if let Some(g) = stack.group_of_layer(stack.active) {
                    if let Some(grp) = stack.groups.get_mut(g) {
                        grp.collapsed = false;
                    }
                }
                true
            }
            KeyCode::Left => {
                if let Some(g) = stack.group_of_layer(stack.active) {
                    if let Some(grp) = stack.groups.get_mut(g) {
                        grp.collapsed = true;
                    }
                }
                true
            }
            KeyCode::Char('M') => {
                stack.toggle_mask(stack.active);
                true
            }
            KeyCode::Char('m') => {
                if stack
                    .layers
                    .get(stack.active)
                    .and_then(|l| l.mask.as_ref())
                    .is_some()
                {
                    stack.toggle_mask_enabled(stack.active);
                } else {
                    stack.merge_down(stack.active);
                }
                true
            }
            KeyCode::Tab => {
                if stack.groups.is_empty() {
                    return false;
                }
                if let Some(g) = stack.group_of_layer(stack.active) {
                    let members = stack.layers_in_group(g);
                    if let Some(&last) = members.last() {
                        if stack.active == last {
                            let next = last + 1;
                            if next < stack.layers.len() {
                                stack.active = next;
                            }
                        } else if let Some(&next) = members.iter().find(|&&i| i > stack.active) {
                            stack.active = next;
                        }
                    }
                } else {
                    let mut found = false;
                    for i in (0..stack.active).rev() {
                        if stack.layers[i].group.is_some() {
                            stack.active = i;
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        for i in stack.active + 1..stack.layers.len() {
                            if stack.layers[i].group.is_some() {
                                stack.active = i;
                                break;
                            }
                        }
                    }
                }
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
        let help_lines = [
            "↑↓ sel ↵vis Llock ±opa Bbld",
            "Nnew Ddup Xdel Mmask(Grp",
            ",↑ .↓ reorder ←→col",
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
        let mut display_idx: usize = 0;
        let mut emitted_group_header = vec![false; stack.groups.len()];

        for rev_idx in 0..stack.layers.len() {
            let real_idx = stack.layers.len() - 1 - rev_idx;
            let layer = &stack.layers[real_idx];

            if let Some(g) = layer.group {
                if g < emitted_group_header.len() && !emitted_group_header[g] {
                    emitted_group_header[g] = true;
                    let y_pos = offset + display_idx;
                    if y_pos < inner_h {
                        if let Some(group) = stack.groups.get(g) {
                            let disclosure = if group.collapsed { "▶" } else { "▼" };
                            let count = stack.layers.iter().filter(|l| l.group == Some(g)).count();
                            let group_label = format!(" {} {} ({})", disclosure, group.name, count);
                            let row_style = Style::default()
                                .bg(self.theme.menu.dropdown_bg)
                                .fg(self.theme.general.secondary)
                                .add_modifier(Modifier::BOLD);
                            let para = ratatui::widgets::Paragraph::new(Line::from(Span::styled(
                                group_label,
                                row_style,
                            )));
                            frame.render_widget(
                                para,
                                Rect {
                                    x: inner_x,
                                    y: inner_y + y_pos as u16,
                                    width: inner.width,
                                    height: 1,
                                },
                            );
                        }
                    }
                    display_idx += 1;

                    if let Some(group) = stack.groups.get(g) {
                        if group.collapsed {
                            continue;
                        }
                    }
                }
            }

            let y_pos = offset + display_idx;
            if y_pos >= inner_h {
                break;
            }

            let is_active = real_idx == stack.active;
            let row_bg = if is_active {
                self.theme.menu.highlight
            } else {
                self.theme.menu.dropdown_bg
            };

            let indent = if layer.group.is_some() { "  " } else { "" };
            let vis_ch = if layer.visible { vis_icon } else { " " };
            let lock_ch = if layer.locked { "L" } else { " " };
            let blend_icon = self
                .icons
                .get(layer.blend_mode.icon_key())
                .map(|s| s.as_str())
                .unwrap_or("");

            let name_max = (inner.width as usize).saturating_sub(16).max(4);

            let label = format!(
                "{}{} {} {} {} {:3}% {}{}",
                indent,
                vis_ch,
                lock_ch,
                if is_active { ">" } else { " " },
                blend_icon,
                (layer.opacity as f32 / 255.0 * 100.0).round() as u8,
                truncate_str(&layer.name, name_max),
                self.render_mask_thumbnail(layer),
            );
            let row_style = Style::default().bg(row_bg).fg(self.theme.menu.fg);

            let row_para =
                ratatui::widgets::Paragraph::new(Line::from(Span::styled(label, row_style)));
            frame.render_widget(
                row_para,
                Rect {
                    x: inner_x,
                    y: inner_y + y_pos as u16,
                    width: inner.width,
                    height: 1,
                },
            );
            display_idx += 1;
        }
    }

    fn render_mask_thumbnail(&self, layer: &Layer) -> String {
        if let Some(ref mask) = layer.mask {
            let mut s = String::with_capacity(4);
            s.push(' ');
            for i in 0..3 {
                if let Some(cell) = mask.buffer.get(i, 0) {
                    if cell.ch == ' ' {
                        s.push('░');
                    } else {
                        s.push('▓');
                    }
                } else {
                    s.push(' ');
                }
            }
            s
        } else {
            String::new()
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

    // --- Layer group tests ---

    #[test]
    fn test_create_group() {
        let mut stack = make_stack(10, 10);
        stack.add(10, 10);
        stack.add(10, 10);
        let group_idx = stack.create_group(&[0, 1], "My Group".to_string());
        assert!(group_idx.is_some());
        let g = group_idx.unwrap();
        assert_eq!(stack.groups.len(), 1);
        assert_eq!(stack.groups[g].name, "My Group");
        assert!(!stack.groups[g].collapsed);
        assert_eq!(stack.layers[0].group, Some(g));
        assert_eq!(stack.layers[1].group, Some(g));
        assert!(stack.layers[2].group.is_none());
    }

    #[test]
    fn test_remove_group() {
        let mut stack = make_stack(10, 10);
        stack.add(10, 10);
        let g = stack.create_group(&[0, 1], "G".to_string()).unwrap();
        assert!(stack.remove_group(g));
        assert!(stack.layers[0].group.is_none());
        assert!(stack.layers[1].group.is_none());
        assert_eq!(stack.groups.len(), 0);
    }

    #[test]
    fn test_toggle_group_collapsed() {
        let mut stack = make_stack(10, 10);
        let g = stack.create_group(&[0], "G".to_string()).unwrap();
        assert!(!stack.groups[g].collapsed);
        assert!(stack.toggle_group_collapsed(g));
        assert!(stack.groups[g].collapsed);
        assert!(stack.toggle_group_collapsed(g));
        assert!(!stack.groups[g].collapsed);
    }

    #[test]
    fn test_rename_group() {
        let mut stack = make_stack(10, 10);
        let g = stack.create_group(&[0], "Old".to_string()).unwrap();
        assert!(stack.rename_group(g, "New".to_string()));
        assert_eq!(stack.groups[g].name, "New");
    }

    #[test]
    fn test_group_of_layer() {
        let mut stack = make_stack(10, 10);
        stack.add(10, 10);
        let g = stack.create_group(&[1], "G".to_string()).unwrap();
        assert_eq!(stack.group_of_layer(1), Some(g));
        assert_eq!(stack.group_of_layer(0), None);
    }

    #[test]
    fn test_layers_in_group() {
        let mut stack = make_stack(10, 10);
        stack.add(10, 10);
        stack.add(10, 10);
        let g = stack.create_group(&[0, 2], "G".to_string()).unwrap();
        let members = stack.layers_in_group(g);
        assert_eq!(members.len(), 2);
        assert!(members.contains(&0));
        assert!(members.contains(&2));
    }

    #[test]
    fn test_create_group_empty_indices() {
        let mut stack = make_stack(10, 10);
        assert!(stack.create_group(&[], "Empty".to_string()).is_none());
        assert_eq!(stack.groups.len(), 0);
    }

    #[test]
    fn test_create_group_invalid_index() {
        let mut stack = make_stack(10, 10);
        assert!(stack.create_group(&[0, 5], "Bad".to_string()).is_none());
        assert_eq!(stack.groups.len(), 0);
    }

    #[test]
    fn test_remove_group_nonexistent() {
        let mut stack = make_stack(10, 10);
        assert!(!stack.remove_group(0));
    }

    #[test]
    fn test_toggle_group_collapsed_nonexistent() {
        let mut stack = make_stack(10, 10);
        assert!(!stack.toggle_group_collapsed(0));
    }

    #[test]
    fn test_rename_group_nonexistent() {
        let mut stack = make_stack(10, 10);
        assert!(!stack.rename_group(0, "X".to_string()));
    }

    #[test]
    fn test_group_preserved_after_delete() {
        let mut stack = make_stack(10, 10);
        stack.add(10, 10);
        stack.add(10, 10);
        let g = stack.create_group(&[0, 1], "G".to_string()).unwrap();
        stack.delete(0);
        assert_eq!(stack.layers[0].group, Some(g));
        assert_eq!(stack.groups.len(), 1);
    }

    #[test]
    fn test_group_index_shifted_after_remove() {
        let mut stack = make_stack(10, 10);
        stack.add(10, 10);
        stack.add(10, 10);
        let g0 = stack.create_group(&[0], "G0".to_string()).unwrap();
        let g1 = stack.create_group(&[1, 2], "G1".to_string()).unwrap();
        assert_eq!(g0, 0);
        assert_eq!(g1, 1);
        stack.remove_group(0);
        assert_eq!(stack.layers[0].group, None);
        assert_eq!(stack.layers[1].group, Some(0));
        assert_eq!(stack.layers[2].group, Some(0));
    }

    // --- Layer mask tests ---

    #[test]
    fn test_create_mask() {
        let mut stack = make_stack(10, 10);
        assert!(stack.layers[0].mask.is_none());
        assert!(stack.create_mask(0));
        assert!(stack.layers[0].mask.is_some());
        let mask = stack.layers[0].mask.as_ref().unwrap();
        assert!(mask.enabled);
        assert_eq!(mask.buffer.width(), 10);
        assert_eq!(mask.buffer.height(), 10);
    }

    #[test]
    fn test_create_mask_on_nonexistent_layer() {
        let mut stack = make_stack(10, 10);
        assert!(!stack.create_mask(5));
    }

    #[test]
    fn test_create_mask_twice_noop() {
        let mut stack = make_stack(10, 10);
        assert!(stack.create_mask(0));
        assert!(!stack.create_mask(0));
    }

    #[test]
    fn test_remove_mask() {
        let mut stack = make_stack(10, 10);
        stack.create_mask(0);
        assert!(stack.remove_mask(0));
        assert!(stack.layers[0].mask.is_none());
    }

    #[test]
    fn test_remove_mask_nonexistent() {
        let mut stack = make_stack(10, 10);
        assert!(!stack.remove_mask(0));
    }

    #[test]
    fn test_toggle_mask() {
        let mut stack = make_stack(10, 10);
        assert!(stack.toggle_mask(0));
        assert!(stack.layers[0].mask.is_some());
        assert!(stack.toggle_mask(0));
        assert!(stack.layers[0].mask.is_none());
    }

    #[test]
    fn test_layer_mask_toggle_enabled() {
        let mut stack = make_stack(10, 10);
        stack.create_mask(0);
        assert!(stack.layers[0].mask.as_ref().unwrap().enabled);
        assert!(stack.toggle_mask_enabled(0));
        assert!(!stack.layers[0].mask.as_ref().unwrap().enabled);
        assert!(stack.toggle_mask_enabled(0));
        assert!(stack.layers[0].mask.as_ref().unwrap().enabled);
    }

    #[test]
    fn test_mask_paint() {
        let mut stack = make_stack(10, 10);
        stack.create_mask(0);
        let cell = make_cell('X');
        stack.set_mask_pixel(0, 3, 4, cell);
        let got = stack.get_mask_pixel(0, 3, 4);
        assert!(got.is_some());
        assert_eq!(got.unwrap().ch, 'X');
    }

    #[test]
    fn test_set_mask_pixel_out_of_bounds() {
        let mut stack = make_stack(5, 5);
        stack.create_mask(0);
        stack.set_mask_pixel(0, 100, 100, make_cell('X'));
    }

    #[test]
    fn test_set_mask_pixel_no_mask() {
        let mut stack = make_stack(5, 5);
        stack.set_mask_pixel(0, 0, 0, make_cell('X'));
    }

    #[test]
    fn test_composite_with_mask() {
        let mut stack = make_stack(3, 3);
        stack.layers[0].buffer.set(0, 0, make_cell('A'));
        stack.create_mask(0);
        // Mask is all spaces by default -> layer is fully hidden
        let comp = stack.composite();
        assert_eq!(comp.get(0, 0).unwrap().ch, ' ');
    }

    #[test]
    fn test_composite_with_mask_painted() {
        let mut stack = make_stack(3, 3);
        stack.layers[0].buffer.set(0, 0, make_cell('A'));
        stack.create_mask(0);
        // Paint mask pixel at (0,0) to reveal it
        stack.set_mask_pixel(0, 0, 0, make_cell('▓'));
        let comp = stack.composite();
        assert_eq!(comp.get(0, 0).unwrap().ch, 'A');
    }

    #[test]
    fn test_composite_with_mask_disabled() {
        let mut stack = make_stack(3, 3);
        stack.layers[0].buffer.set(0, 0, make_cell('A'));
        stack.create_mask(0);
        stack.layers[0].mask.as_mut().unwrap().enabled = false;
        let comp = stack.composite();
        assert_eq!(comp.get(0, 0).unwrap().ch, 'A');
    }

    #[test]
    fn test_mask_thumbnail() {
        let panel = LayerPanel::new();
        let layer = Layer::new(5, 5, "Test".to_string());
        let thumb = panel.render_mask_thumbnail(&layer);
        assert_eq!(thumb, ""); // No mask => empty string
    }

    #[test]
    fn test_mask_thumbnail_with_mask() {
        let panel = LayerPanel::new();
        let mut layer = Layer::new(5, 5, "Test".to_string());
        let mut mask = LayerMask::new(5, 5);
        // Paint first two pixels
        mask.buffer.set(0, 0, make_cell('▓'));
        mask.buffer.set(1, 0, make_cell('▓'));
        layer.mask = Some(mask);
        let thumb = panel.render_mask_thumbnail(&layer);
        assert_eq!(thumb, " ▓▓░");
    }

    #[test]
    fn test_composite_two_layers_with_mask() {
        let mut stack = make_stack(3, 3);
        stack.add(3, 3);
        // Bottom layer has content at (0,0) and (1,1)
        stack.layers[0].buffer.set(0, 0, make_cell('A'));
        stack.layers[0].buffer.set(1, 1, make_cell('B'));
        // Top layer has content at (0,0) with mask revealing only part
        stack.layers[1].buffer.set(0, 0, make_cell('C'));
        stack.create_mask(1);
        stack.set_mask_pixel(1, 0, 0, make_cell('▓'));
        let comp = stack.composite();
        // (0,0): top layer visible through mask => 'C'
        assert_eq!(comp.get(0, 0).unwrap().ch, 'C');
        // (1,1): only bottom layer => 'B'
        assert_eq!(comp.get(1, 1).unwrap().ch, 'B');
    }
}
