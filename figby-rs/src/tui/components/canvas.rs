use crate::tui::canvas::{CanvasBuffer, CanvasCell};
use crate::tui::layers::LayerStack;
use crate::tui::lighting::{self, LightingLut, Scene};

pub fn shade_composited(
    composited: &CanvasBuffer,
    layer_stack: &LayerStack,
    scene: &Scene,
    lut: &LightingLut,
    max_shadow_distance: u16,
    height_scale: f32,
) -> CanvasBuffer {
    let w = composited.width();
    let h = composited.height();
    if w == 0 || h == 0 {
        return composited.clone();
    }

    let mut shadow_mask = vec![vec![false; w]; h];
    let mut lighting_mask = vec![vec![false; w]; h];

    for layer in &layer_stack.layers {
        if !layer.visible {
            continue;
        }
        for y in 0..h.min(layer.buffer.height()) {
            for x in 0..w.min(layer.buffer.width()) {
                if let Some(cell) = layer.buffer.get(x, y) {
                    let has_content = cell.ch != ' ' || cell.fg.is_some() || cell.bg.is_some();
                    if has_content {
                        if layer.casts_shadow {
                            shadow_mask[y][x] = true;
                        }
                        if layer.accepts_lighting {
                            lighting_mask[y][x] = true;
                        }
                    }
                }
            }
        }
    }

    let heightfield: Vec<Vec<f32>> = (0..h)
        .map(|y| {
            (0..w)
                .map(|x| {
                    composited
                        .get(x, y)
                        .map_or(0.0, |c| c.height.unwrap_or(0) as f32 / 255.0)
                })
                .collect()
        })
        .collect();

    let normal_map = lighting::compute_normal_map_figfont(&heightfield, height_scale);

    let shadow_check = |x: u16, y: u16| -> bool {
        let (ux, uy) = (x as usize, y as usize);
        ux < w && uy < h && shadow_mask[uy][ux]
    };
    let luminance = lighting::shade_canvas(scene, &normal_map, shadow_check, max_shadow_distance);

    let mut result = CanvasBuffer::new(w, h);
    for y in 0..h {
        for x in 0..w {
            if lighting_mask[y][x] {
                let entry = lut.get(luminance[y][x]);
                result.set(
                    x,
                    y,
                    CanvasCell {
                        ch: entry.ch,
                        fg: Some(ratatui::style::Color::Rgb(
                            entry.fg_color.0,
                            entry.fg_color.1,
                            entry.fg_color.2,
                        )),
                        bg: entry
                            .bg_color
                            .map(|(r, gg, b)| ratatui::style::Color::Rgb(r, gg, b)),
                        height: None,
                    },
                );
            } else if let Some(cell) = composited.get(x, y) {
                result.set(x, y, *cell);
            }
        }
    }

    result
}
