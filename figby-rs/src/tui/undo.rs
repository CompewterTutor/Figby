use super::canvas::CanvasBuffer;

#[derive(Debug, Clone)]
pub struct UndoEntry {
    pub buffer: CanvasBuffer,
    pub label: String,
}

pub struct UndoSystem {
    undo: Vec<UndoEntry>,
    redo: Vec<CanvasBuffer>,
    limit: usize,
    in_batch: bool,
    batch_has_snapshot: bool,
}

impl UndoSystem {
    pub fn new(limit: usize) -> Self {
        Self {
            undo: Vec::with_capacity(limit),
            redo: Vec::new(),
            limit,
            in_batch: false,
            batch_has_snapshot: false,
        }
    }

    pub fn push_snapshot(&mut self, buffer: CanvasBuffer, label: String) {
        if self.in_batch {
            if self.batch_has_snapshot {
                return;
            }
            self.batch_has_snapshot = true;
        }
        self.undo.push(UndoEntry { buffer, label });
        self.redo.clear();
        while self.undo.len() > self.limit {
            self.undo.remove(0);
        }
    }

    pub fn begin_batch(&mut self) {
        self.in_batch = true;
        self.batch_has_snapshot = false;
    }

    pub fn end_batch(&mut self) {
        self.in_batch = false;
        self.batch_has_snapshot = false;
    }

    pub fn undo(&mut self, current_buffer: CanvasBuffer) -> Option<(CanvasBuffer, String)> {
        let entry = self.undo.pop()?;
        self.redo.push(current_buffer);
        Some((entry.buffer, entry.label))
    }

    pub fn redo(&mut self, current_buffer: CanvasBuffer) -> Option<(CanvasBuffer, String)> {
        let buf = self.redo.pop()?;
        self.undo.push(UndoEntry {
            buffer: current_buffer,
            label: "Redo".to_string(),
        });
        Some((buf, "Redo".to_string()))
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.in_batch = false;
        self.batch_has_snapshot = false;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn history_len(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    pub fn history_entries(&self) -> &[UndoEntry] {
        &self.undo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_buffer(w: usize, h: usize) -> CanvasBuffer {
        CanvasBuffer::new(w, h)
    }

    #[test]
    fn test_new_empty() {
        let us = UndoSystem::new(50);
        assert!(!us.can_undo());
        assert!(!us.can_redo());
        assert_eq!(us.history_len(), 0);
        assert_eq!(us.redo_len(), 0);
    }

    #[test]
    fn test_push_undo() {
        let mut us = UndoSystem::new(50);
        let buf = make_buffer(5, 5);
        us.push_snapshot(buf, "Brush".to_string());
        assert!(us.can_undo());
        assert!(!us.can_redo());
        assert_eq!(us.history_len(), 1);
    }

    #[test]
    fn test_undo_restores_buffer() {
        let mut us = UndoSystem::new(50);
        let before = make_buffer(5, 5);
        us.push_snapshot(before.clone(), "Brush".to_string());
        let after = make_buffer(5, 5);
        let result = us.undo(after);
        assert!(result.is_some());
        let (restored, _) = result.unwrap();
        assert_eq!(restored.width(), before.width());
        assert_eq!(restored.height(), before.height());
        assert!(us.can_redo());
        assert!(!us.can_undo());
    }

    #[test]
    fn test_undo_redo_cycle() {
        let mut us = UndoSystem::new(50);
        let buf_a = make_buffer(3, 3);
        us.push_snapshot(buf_a, "Action 1".to_string());
        let buf_b = make_buffer(3, 3);
        let (restored, _) = us.undo(buf_b).unwrap();
        assert_eq!(restored.width(), 3);
        assert!(us.can_redo());
        let buf_c = make_buffer(5, 5);
        let (redone, _) = us.redo(buf_c).unwrap();
        assert_eq!(redone.width(), 3); // buf_b was 3x3
        assert!(!us.can_redo());
    }

    #[test]
    fn test_undo_multiple_actions() {
        let mut us = UndoSystem::new(50);
        us.push_snapshot(make_buffer(1, 1), "1".to_string());
        us.push_snapshot(make_buffer(2, 2), "2".to_string());
        us.push_snapshot(make_buffer(3, 3), "3".to_string());
        assert_eq!(us.history_len(), 3);
        let cur = make_buffer(4, 4);
        let (buf3, _) = us.undo(cur).unwrap();
        assert_eq!(buf3.width(), 3);
        assert_eq!(us.history_len(), 2);
        let cur2 = make_buffer(5, 5);
        let (buf2, _) = us.undo(cur2).unwrap();
        assert_eq!(buf2.width(), 2);
        assert_eq!(us.history_len(), 1);
    }

    #[test]
    fn test_undo_limit_enforcement() {
        let mut us = UndoSystem::new(5);
        for i in 0..10 {
            us.push_snapshot(make_buffer((i + 1) as usize, 1), i.to_string());
        }
        assert_eq!(us.history_len(), 5);
        let cur = make_buffer(1, 1);
        let (buf, _) = us.undo(cur).unwrap();
        assert_eq!(buf.width(), 10);
    }

    #[test]
    fn test_undo_clears_redo() {
        let mut us = UndoSystem::new(50);
        us.push_snapshot(make_buffer(1, 1), "1".to_string());
        let cur = make_buffer(2, 2);
        us.undo(cur);
        assert!(us.can_redo());
        us.push_snapshot(make_buffer(3, 3), "2".to_string());
        assert!(!us.can_redo());
    }

    #[test]
    fn test_clear() {
        let mut us = UndoSystem::new(50);
        us.push_snapshot(make_buffer(1, 1), "1".to_string());
        us.push_snapshot(make_buffer(2, 2), "2".to_string());
        us.clear();
        assert!(!us.can_undo());
        assert!(!us.can_redo());
        assert_eq!(us.history_len(), 0);
    }

    #[test]
    fn test_batch_first_pushes_rest_discarded() {
        let mut us = UndoSystem::new(50);
        us.begin_batch();
        us.push_snapshot(make_buffer(1, 1), "first".to_string());
        us.push_snapshot(make_buffer(2, 2), "second".to_string());
        us.push_snapshot(make_buffer(3, 3), "third".to_string());
        us.end_batch();
        assert_eq!(us.history_len(), 1);
    }

    #[test]
    fn test_batch_no_snapshot_ok() {
        let mut us = UndoSystem::new(50);
        us.begin_batch();
        us.end_batch();
        assert_eq!(us.history_len(), 0);
        us.push_snapshot(make_buffer(1, 1), "after".to_string());
        assert_eq!(us.history_len(), 1);
    }

    #[test]
    fn test_undo_on_empty_returns_none() {
        let mut us = UndoSystem::new(50);
        assert!(us.undo(make_buffer(1, 1)).is_none());
    }

    #[test]
    fn test_redo_on_empty_returns_none() {
        let mut us = UndoSystem::new(50);
        assert!(us.redo(make_buffer(1, 1)).is_none());
    }

    #[test]
    fn test_history_entries_order() {
        let mut us = UndoSystem::new(50);
        us.push_snapshot(make_buffer(1, 1), "A".to_string());
        us.push_snapshot(make_buffer(2, 2), "B".to_string());
        us.push_snapshot(make_buffer(3, 3), "C".to_string());
        let entries = us.history_entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].label, "A");
        assert_eq!(entries[1].label, "B");
        assert_eq!(entries[2].label, "C");
    }

    #[test]
    fn test_batch_preserves_two_separate_batches() {
        let mut us = UndoSystem::new(50);
        us.begin_batch();
        us.push_snapshot(make_buffer(1, 1), "batch1".to_string());
        us.end_batch();
        us.begin_batch();
        us.push_snapshot(make_buffer(2, 2), "batch2".to_string());
        us.end_batch();
        assert_eq!(us.history_len(), 2);
    }

    #[test]
    fn test_redo_label() {
        let mut us = UndoSystem::new(50);
        us.push_snapshot(make_buffer(1, 1), "draw".to_string());
        us.undo(make_buffer(2, 2));
        let (buf, label) = us.redo(make_buffer(3, 3)).unwrap();
        assert_eq!(label, "Redo");
        assert_eq!(buf.width(), 2);
    }
}
