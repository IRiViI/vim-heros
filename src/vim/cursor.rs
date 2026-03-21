use super::buffer::Buffer;

/// Cursor position in the buffer (0-indexed line and column).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub line: usize,
    pub col: usize,
}

impl Cursor {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }

    /// Clamp the cursor to valid buffer bounds.
    /// In Normal mode, the cursor can't go past the last character (len - 1).
    /// In Insert mode, it can go one past the last character (len).
    pub fn clamp(&mut self, buffer: &Buffer, insert_mode: bool) {
        let max_line = buffer.line_count().saturating_sub(1);
        self.line = self.line.min(max_line);

        let line_len = buffer.line_len(self.line);
        if insert_mode {
            // In insert mode, cursor can be at position line_len (after last char)
            self.col = self.col.min(line_len);
        } else {
            // In normal mode, cursor sits ON a character, so max is len - 1
            self.col = self.col.min(line_len.saturating_sub(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_normal_mode() {
        let buf = Buffer::from_str("hello\nhi");
        let mut cur = Cursor::new(0, 10);
        cur.clamp(&buf, false);
        assert_eq!(cur.col, 4); // 'o' is last char at index 4

        let mut cur = Cursor::new(5, 0);
        cur.clamp(&buf, false);
        assert_eq!(cur.line, 1); // clamped to last line
    }

    #[test]
    fn test_clamp_insert_mode() {
        let buf = Buffer::from_str("hello");
        let mut cur = Cursor::new(0, 10);
        cur.clamp(&buf, true);
        assert_eq!(cur.col, 5); // can go one past last char in insert mode
    }

    #[test]
    fn test_clamp_empty_line() {
        let buf = Buffer::from_str("hello\n\nworld");
        let mut cur = Cursor::new(1, 5);
        cur.clamp(&buf, false);
        assert_eq!(cur.col, 0); // empty line, col clamped to 0
    }
}
