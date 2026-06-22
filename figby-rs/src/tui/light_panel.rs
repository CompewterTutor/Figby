use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};
use super::lighting::{Light, Scene};

#[derive(Clone)]
pub struct LightPanel {
    pub selected_index: usize,
}

impl LightPanel {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn set_selected(&mut self, idx: usize, scene: &Scene) {
        if scene.lights.is_empty() {
            self.selected_index = 0;
        } else {
            self.selected_index = idx.min(scene.lights.len() - 1);
        }
    }

    pub fn adjust_intensity(scene: &mut Scene, idx: usize, delta: f32) {
        if idx >= scene.lights.len() {
            return;
        }
        match &mut scene.lights[idx] {
            Light::Ambient { intensity, .. }
            | Light::Directional { intensity, .. }
            | Light::Point { intensity, .. } => {
                *intensity = (*intensity + delta).clamp(0.0, 1.0);
            }
        }
    }

    pub fn light_type_str(scene: &Scene, idx: usize) -> Option<&'static str> {
        scene.lights.get(idx).map(|l| match l {
            Light::Ambient { .. } => "Ambient",
            Light::Directional { .. } => "Directional",
            Light::Point { .. } => "Point",
        })
    }

    pub fn light_intensity(scene: &Scene, idx: usize) -> Option<f32> {
        scene.lights.get(idx).map(|l| match l {
            Light::Ambient { intensity, .. }
            | Light::Directional { intensity, .. }
            | Light::Point { intensity, .. } => *intensity,
        })
    }

    pub fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        scene: &Option<Scene>,
        theme: &super::theme::Theme,
    ) {
        let block = Block::default()
            .title(" Lights ")
            .borders(super::layout::toolbox_list_borders())
            .style(Style::default().fg(theme.general.secondary));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let scene = match scene {
            Some(s) => s,
            None => return,
        };

        let mut lines: Vec<Line> = Vec::new();
        for (i, light) in scene.lights.iter().enumerate() {
            let prefix = if i == self.selected_index {
                " \u{25b6} "
            } else {
                "   "
            };
            let label = match light {
                Light::Ambient { intensity, .. } => format!("Amb  {:.2}", intensity),
                Light::Directional { intensity, .. } => format!("Dir  {:.2}", intensity),
                Light::Point { intensity, position, .. } => format!(
                    "Pnt  {:.2} ({},{})",
                    intensity, position.0 as u16, position.1 as u16
                ),
            };
            let style = if i == self.selected_index {
                Style::default()
                    .fg(theme.general.primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.general.secondary)
            };
            lines.push(Line::from(Span::styled(format!("{}{}", prefix, label), style)));
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                " (no lights) ",
                Style::default().fg(theme.general.secondary),
            )));
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }
}

impl Default for LightPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::lighting::{Attenuation, Rgb};

    #[test]
    fn test_light_panel_empty() {
        let panel = LightPanel::new();
        let scene = Scene::new();
        assert_eq!(panel.selected_index(), 0);
        assert!(scene.lights.is_empty());
    }

    #[test]
    fn test_light_panel_single() {
        let mut scene = Scene::new();
        scene.add_light(Light::Ambient {
            intensity: 0.7,
            color: Rgb(255, 255, 255),
        });
        assert_eq!(LightPanel::light_type_str(&scene, 0), Some("Ambient"));
        assert!((LightPanel::light_intensity(&scene, 0).unwrap() - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_light_panel_selection() {
        let mut scene = Scene::new();
        scene.add_light(Light::Ambient {
            intensity: 0.5,
            color: Rgb(255, 255, 255),
        });
        scene.add_light(Light::Directional {
            direction: (0.0, 0.0, 1.0),
            intensity: 0.8,
            color: Rgb(255, 255, 255),
        });

        let mut panel = LightPanel::new();
        assert_eq!(panel.selected_index(), 0);
        panel.set_selected(1, &scene);
        assert_eq!(panel.selected_index(), 1);
        panel.set_selected(5, &scene);
        assert_eq!(panel.selected_index(), 1);
    }

    #[test]
    fn test_light_panel_multiple() {
        let mut scene = Scene::new();
        scene.add_light(Light::Ambient {
            intensity: 0.5,
            color: Rgb(255, 255, 255),
        });
        scene.add_light(Light::Directional {
            direction: (0.0, 0.0, 1.0),
            intensity: 0.8,
            color: Rgb(255, 255, 255),
        });
        scene.add_light(Light::Point {
            position: (5.0, 3.0, 5.0),
            intensity: 0.9,
            color: Rgb(255, 255, 255),
            attenuation: Attenuation::default(),
        });

        assert_eq!(scene.lights.len(), 3);
        assert_eq!(LightPanel::light_type_str(&scene, 0), Some("Ambient"));
        assert_eq!(LightPanel::light_type_str(&scene, 1), Some("Directional"));
        assert_eq!(LightPanel::light_type_str(&scene, 2), Some("Point"));
    }

    #[test]
    fn test_light_panel_adjust_intensity() {
        let mut scene = Scene::new();
        scene.add_light(Light::Ambient {
            intensity: 0.5,
            color: Rgb(255, 255, 255),
        });

        LightPanel::adjust_intensity(&mut scene, 0, 0.2);
        assert!((LightPanel::light_intensity(&scene, 0).unwrap() - 0.7).abs() < 0.001);

        LightPanel::adjust_intensity(&mut scene, 0, 0.5);
        assert!((LightPanel::light_intensity(&scene, 0).unwrap() - 1.0).abs() < 0.001);

        LightPanel::adjust_intensity(&mut scene, 0, -0.8);
        assert!((LightPanel::light_intensity(&scene, 0).unwrap() - 0.2).abs() < 0.001);
    }
}
