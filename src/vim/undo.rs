use ropey::Rope;

use super::cursor::Cursor;

/// A snapshot of the editor state at a point in time.
#[derive(Clone)]
struct Snapshot {
    rope: Rope,
    cursor: Cursor,
}

/// Snapshot-based undo/redo history.
///
/// Before each editing action, `push` saves the current buffer+cursor.
/// `undo` restores the previous state; `redo` moves forward again.
/// Making a new edit after undo discards the redo future.
pub struct UndoHistory {
    snapshots: Vec<Snapshot>,
    /// Points to the "current" snapshot index. Everything above is redo future.
    current: usize,
    max_size: usize,
}

impl UndoHistory {
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
            current: 0,
            max_size: 200,
        }
    }

    /// Save the current state before an edit.
    pub fn push(&mut self, rope: &Rope, cursor: Cursor) {
        // Discard any redo future
        self.snapshots.truncate(self.current);

        self.snapshots.push(Snapshot {
            rope: rope.clone(),
            cursor,
        });
        self.current = self.snapshots.len();

        // Cap history size
        if self.snapshots.len() > self.max_size {
            let excess = self.snapshots.len() - self.max_size;
            self.snapshots.drain(0..excess);
            self.current = self.snapshots.len();
        }
    }

    /// Undo: returns the previous (rope, cursor) if available.
    pub fn undo(&mut self, current_rope: &Rope, current_cursor: Cursor) -> Option<(Rope, Cursor)> {
        if self.current == 0 {
            return None;
        }

        // If we're at the tip (no redo future), save current state as the redo target
        if self.current == self.snapshots.len() {
            self.snapshots.push(Snapshot {
                rope: current_rope.clone(),
                cursor: current_cursor,
            });
        }

        self.current -= 1;
        let snap = &self.snapshots[self.current];
        Some((snap.rope.clone(), snap.cursor))
    }

    /// Redo: returns the next (rope, cursor) if available.
    pub fn redo(&mut self) -> Option<(Rope, Cursor)> {
        if self.current + 1 >= self.snapshots.len() {
            return None;
        }

        self.current += 1;
        let snap = &self.snapshots[self.current];
        Some((snap.rope.clone(), snap.cursor))
    }

    /// Whether undo is available.
    pub fn can_undo(&self) -> bool {
        self.current > 0
    }

    /// Whether redo is available.
    pub fn can_redo(&self) -> bool {
        self.current + 1 < self.snapshots.len()
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.snapshots.clear();
        self.current = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rope(s: &str) -> Rope {
        Rope::from_str(s)
    }

    #[test]
    fn test_undo_single_edit() {
        let mut hist = UndoHistory::new();
        let r0 = rope("hello");
        let c0 = Cursor::new(0, 0);

        hist.push(&r0, c0);

        let r1 = rope("hello world");
        let c1 = Cursor::new(0, 11);

        let (restored_rope, restored_cursor) = hist.undo(&r1, c1).unwrap();
        assert_eq!(restored_rope.to_string(), "hello");
        assert_eq!(restored_cursor, c0);
    }

    #[test]
    fn test_undo_empty_history() {
        let mut hist = UndoHistory::new();
        let r = rope("hello");
        let c = Cursor::new(0, 0);
        assert!(hist.undo(&r, c).is_none());
    }

    #[test]
    fn test_undo_redo_cycle() {
        let mut hist = UndoHistory::new();

        let r0 = rope("aaa");
        let c0 = Cursor::new(0, 0);
        hist.push(&r0, c0);

        let r1 = rope("bbb");
        let c1 = Cursor::new(0, 2);
        hist.push(&r1, c1);

        let r2 = rope("ccc");
        let c2 = Cursor::new(0, 1);

        // Undo from "ccc" -> "bbb"
        let (ur, uc) = hist.undo(&r2, c2).unwrap();
        assert_eq!(ur.to_string(), "bbb");
        assert_eq!(uc, c1);

        // Undo from "bbb" -> "aaa"
        let (ur2, uc2) = hist.undo(&ur, uc).unwrap();
        assert_eq!(ur2.to_string(), "aaa");
        assert_eq!(uc2, c0);

        // Redo -> "bbb"
        let (rr, rc) = hist.redo().unwrap();
        assert_eq!(rr.to_string(), "bbb");
        assert_eq!(rc, c1);

        // Redo -> "ccc"
        let (rr2, rc2) = hist.redo().unwrap();
        assert_eq!(rr2.to_string(), "ccc");
        assert_eq!(rc2, c2);

        // No more redo
        assert!(hist.redo().is_none());
    }

    #[test]
    fn test_new_edit_clears_redo() {
        let mut hist = UndoHistory::new();

        let r0 = rope("aaa");
        hist.push(&r0, Cursor::new(0, 0));

        let r1 = rope("bbb");
        hist.push(&r1, Cursor::new(0, 0));

        let r2 = rope("ccc");
        let c2 = Cursor::new(0, 0);

        // Undo twice
        hist.undo(&r2, c2);
        hist.undo(&r1, Cursor::new(0, 0));

        // New edit at this point
        let r_new = rope("aaa");
        hist.push(&r_new, Cursor::new(0, 0));

        // Redo should not work (future was discarded)
        assert!(!hist.can_redo());
    }

    #[test]
    fn test_can_undo_redo() {
        let mut hist = UndoHistory::new();
        assert!(!hist.can_undo());
        assert!(!hist.can_redo());

        let r0 = rope("x");
        hist.push(&r0, Cursor::new(0, 0));
        assert!(hist.can_undo());
        assert!(!hist.can_redo());
    }

    #[test]
    fn test_max_size_cap() {
        let mut hist = UndoHistory::new();
        hist.max_size = 5;

        for i in 0..10 {
            let r = rope(&format!("state{}", i));
            hist.push(&r, Cursor::new(0, 0));
        }

        assert_eq!(hist.snapshots.len(), 5);
        // Oldest snapshots are dropped, newest remain
        assert_eq!(hist.snapshots[0].rope.to_string(), "state5");
    }

    #[test]
    fn test_clear() {
        let mut hist = UndoHistory::new();
        hist.push(&rope("a"), Cursor::new(0, 0));
        hist.push(&rope("b"), Cursor::new(0, 0));
        hist.clear();
        assert!(!hist.can_undo());
        assert!(!hist.can_redo());
    }
}
