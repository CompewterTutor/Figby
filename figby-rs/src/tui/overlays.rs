use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::{file_ops, keymap, timeline, AppMode, TuiApp};

impl TuiApp {
    /// Render all floating overlays (dialogs, keybindings, undo panel).
    pub(super) fn render_overlays(&mut self, frame: &mut Frame<'_>) {
        // Export dialog overlay
        if self.dialogs.export_dialog.active {
            let overlay = super::centered_overlay(frame.area());
            frame.render_widget(Clear, overlay);
            self.dialogs.export_dialog.render(frame, overlay);
            self.dialogs.export_dialog.preview_tick();
        }

        // File ops overlay
        if self.dialogs.file_ops.mode != file_ops::FileOpsMode::Idle {
            let overlay = super::centered_overlay(frame.area());
            frame.render_widget(Clear, overlay);
            self.dialogs.file_ops.render(frame, overlay);
        }

        // Keybindings overlay
        if self.show_keybindings {
            let area = frame.area();
            let overlay = Rect {
                x: area.width / 8,
                y: area.height / 8,
                width: area.width * 3 / 4,
                height: area.height * 3 / 4,
            };
            frame.render_widget(Clear, overlay);
            let block = Block::default()
                .title(" Keybindings (Esc to close) ")
                .borders(Borders::ALL)
                .style(
                    Style::default()
                        .bg(self.theme.menu.dropdown_bg)
                        .fg(self.theme.menu.fg),
                );
            let inner = block.inner(overlay);
            frame.render_widget(block, overlay);

            let mut lines: Vec<Line> = Vec::new();
            let mut last_scope: Option<keymap::Scope> = None;
            for binding in keymap::KEYMAP {
                if last_scope != Some(binding.scope) {
                    if last_scope.is_some() {
                        lines.push(Line::from(""));
                    }
                    lines.push(Line::from(Span::styled(
                        format!(" {}", binding.scope.label()),
                        Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    )));
                    last_scope = Some(binding.scope);
                }
                lines.push(Line::from(format!(
                    "  {:<22} {}",
                    binding.keys, binding.description
                )));
            }
            frame.render_widget(Paragraph::new(lines), inner);
        }

        // Undo history panel overlay
        if self.dialogs.undo_panel.open {
            frame.render_widget(Clear, frame.area());
            self.dialogs
                .undo_panel
                .render(frame, frame.area(), self.editor.undo.history_entries());
        }

        // Keyframe editor panel
        if self.animation.timeline_state.keyframe_editor.open {
            let area = frame.area();
            let panel_w = area.width.clamp(30, 42);
            let panel_x = area.x + area.width.saturating_sub(panel_w);
            let panel_h = (area.height / 2).max(10).min(area.height - 3);
            let panel_y = area.y + area.height.saturating_sub(panel_h + 3);
            let panel_rect = Rect {
                x: panel_x,
                y: panel_y,
                width: panel_w,
                height: panel_h,
            };
            frame.render_widget(Clear, panel_rect);
            self.animation.timeline_state.render_keyframe_editor(
                frame,
                panel_rect,
                &timeline::TimelineTheme::default(),
            );
        }

        // Tween panel
        if self.animation.timeline_state.tween.is_some() {
            let area = frame.area();
            let panel_w = area.width.clamp(30, 42);
            let panel_x = area.x + area.width.saturating_sub(panel_w);
            let panel_h = (area.height / 2).max(10).min(area.height - 3);
            let panel_y = area.y + 1;
            let panel_rect = Rect {
                x: panel_x,
                y: panel_y,
                width: panel_w,
                height: panel_h,
            };
            frame.render_widget(Clear, panel_rect);
            self.animation.timeline_state.render_tween_panel(
                frame,
                panel_rect,
                &timeline::TimelineTheme::default(),
            );
        }

        // Rascii import dialog
        if self.dialogs.rascii_import.active {
            let overlay = super::centered_overlay(frame.area());
            frame.render_widget(Clear, overlay);
            self.dialogs.rascii_import.render(frame, overlay);
        }

        // Emitter config panel overlay
        if self.animation.emitter_panel.open {
            let area = frame.area();
            let panel_w = area.width.clamp(30, 36);
            let panel_x = area.x + area.width.saturating_sub(panel_w);
            let panel_h = (area.height / 2).max(14).min(area.height - 3);
            let panel_y = area.y + 1;
            let panel_rect = Rect {
                x: panel_x,
                y: panel_y,
                width: panel_w,
                height: panel_h,
            };
            frame.render_widget(Clear, panel_rect);
            self.animation
                .emitter_panel
                .render_config_panel(frame, panel_rect, &self.animation.particle_system.config);
        }

        // Palette editor overlay
        if self.palette_editor.open {
            let was_lighting = self.palette_editor.lighting_pickers_visible;
            self.palette_editor.lighting_pickers_visible =
                self.mode == AppMode::Lighting || self.lighting.scene.is_some();
            if was_lighting != self.palette_editor.lighting_pickers_visible {
                self.dirty = true;
            }
            self.palette_editor.render(frame, frame.area(), &self.theme);
        }
    }
}
