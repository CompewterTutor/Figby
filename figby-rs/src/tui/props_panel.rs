use crossterm::event::KeyCode;
use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropAction {
    SizeUp,
    SizeDown,
    DensityUp,
    DensityDown,
    CycleShape,
    CycleSubMode,
    CycleJust,
    ScaleUp,
    ScaleDown,
    FontPrev,
    FontNext,
    BeginEditChar,
    BeginEditField,
    CommitChar(char),
    CancelEdit,
    FillThresholdUp,
    FillThresholdDown,
    MoveStrideUp,
    MoveStrideDown,
    MoveSnapToggle,
    MoveWrapToggle,
    RotateStepUp,
    RotateStepDown,
    RotateDirToggle,
    RotatePivotCycle,
    SelectFeatherUp,
    SelectFeatherDown,
    SelectAdditiveToggle,
    SelectSubtractiveToggle,
    SelectMoveToggle,
    LineWidthUp,
    LineWidthDown,
    LineArrowCycle,
    LineCurveToggle,
}

#[derive(Debug, Clone)]
pub struct PropsWidgetRect {
    pub rect: Rect,
    pub action: PropAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropsPanelMode {
    Idle,
    EditingChar,
}

pub struct PropsPanel {
    pub mode: PropsPanelMode,
    pub rects: Vec<PropsWidgetRect>,
    char_buffer: String,
}

impl PropsPanel {
    pub fn new() -> Self {
        Self {
            mode: PropsPanelMode::Idle,
            rects: Vec::new(),
            char_buffer: String::new(),
        }
    }

    pub fn clear_rects(&mut self) {
        self.rects.clear();
    }

    pub fn handle_click(&self, col: u16, row: u16) -> Option<PropAction> {
        for wr in &self.rects {
            let r = wr.rect;
            if col >= r.x && col < r.x + r.width && row >= r.y && row < r.y + r.height {
                return Some(wr.action);
            }
        }
        None
    }

    pub fn handle_key(&mut self, code: KeyCode) -> Option<PropAction> {
        match self.mode {
            PropsPanelMode::Idle => None,
            PropsPanelMode::EditingChar => match code {
                KeyCode::Char(c) => {
                    self.char_buffer.push(c);
                    if !self.char_buffer.is_empty() {
                        let ch = self.char_buffer.chars().next().unwrap_or('\u{2588}');
                        self.char_buffer.clear();
                        self.mode = PropsPanelMode::Idle;
                        Some(PropAction::CommitChar(ch))
                    } else {
                        None
                    }
                }
                KeyCode::Enter => {
                    let ch = self.char_buffer.chars().next().unwrap_or('\u{2588}');
                    self.char_buffer.clear();
                    self.mode = PropsPanelMode::Idle;
                    Some(PropAction::CommitChar(ch))
                }
                KeyCode::Esc => {
                    self.char_buffer.clear();
                    self.mode = PropsPanelMode::Idle;
                    Some(PropAction::CancelEdit)
                }
                _ => None,
            },
        }
    }

    pub fn start_char_edit(&mut self) {
        self.mode = PropsPanelMode::EditingChar;
        self.char_buffer.clear();
    }
}

impl Default for PropsPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_click_returns_action_for_matching_rect() {
        let rect = Rect::new(10, 5, 3, 1);
        let mut p = PropsPanel::new();
        p.rects.push(PropsWidgetRect {
            rect,
            action: PropAction::SizeUp,
        });
        assert_eq!(p.handle_click(10, 5), Some(PropAction::SizeUp));
        assert_eq!(p.handle_click(12, 5), Some(PropAction::SizeUp));
        assert_eq!(p.handle_click(9, 5), None);
        assert_eq!(p.handle_click(13, 5), None);
    }

    #[test]
    fn test_click_multiple_rects() {
        let mut p = PropsPanel::new();
        p.rects.push(PropsWidgetRect {
            rect: Rect::new(2, 1, 3, 1),
            action: PropAction::SizeDown,
        });
        p.rects.push(PropsWidgetRect {
            rect: Rect::new(15, 1, 3, 1),
            action: PropAction::SizeUp,
        });
        assert_eq!(p.handle_click(2, 1), Some(PropAction::SizeDown));
        assert_eq!(p.handle_click(3, 1), Some(PropAction::SizeDown));
        assert_eq!(p.handle_click(15, 1), Some(PropAction::SizeUp));
        assert_eq!(p.handle_click(17, 1), Some(PropAction::SizeUp));
        // gap between rects
        assert_eq!(p.handle_click(6, 1), None);
    }

    #[test]
    fn test_click_outside_any_rect_returns_none() {
        let mut p = PropsPanel::new();
        p.rects.push(PropsWidgetRect {
            rect: Rect::new(5, 5, 3, 1),
            action: PropAction::CycleShape,
        });
        assert_eq!(p.handle_click(0, 0), None);
        assert_eq!(p.handle_click(5, 6), None);
        assert_eq!(p.handle_click(8, 5), None);
    }

    #[test]
    fn test_clear_rects_empties_vector() {
        let mut p = PropsPanel::new();
        p.rects.push(PropsWidgetRect {
            rect: Rect::new(0, 0, 3, 1),
            action: PropAction::SizeUp,
        });
        assert_eq!(p.rects.len(), 1);
        p.clear_rects();
        assert!(p.rects.is_empty());
    }

    #[test]
    fn test_char_edit_mode_commit_on_typing() {
        let mut p = PropsPanel::new();
        p.start_char_edit();
        assert_eq!(p.mode, PropsPanelMode::EditingChar);
        // Typing a char should auto-commit (single char mode)
        let action = p.handle_key(KeyCode::Char('X'));
        assert_eq!(action, Some(PropAction::CommitChar('X')));
        assert_eq!(p.mode, PropsPanelMode::Idle);
    }

    #[test]
    fn test_char_edit_mode_enter_commit() {
        let mut p = PropsPanel::new();
        p.start_char_edit();
        // Press Enter with empty buffer — defaults to full block
        let action = p.handle_key(KeyCode::Enter);
        assert_eq!(action, Some(PropAction::CommitChar('\u{2588}')));
        assert_eq!(p.mode, PropsPanelMode::Idle);
    }

    #[test]
    fn test_char_edit_mode_esc_cancels() {
        let mut p = PropsPanel::new();
        p.start_char_edit();
        let action = p.handle_key(KeyCode::Esc);
        assert_eq!(action, Some(PropAction::CancelEdit));
        assert_eq!(p.mode, PropsPanelMode::Idle);
    }
}
