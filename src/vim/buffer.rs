use ropey::Rope;

/// A text buffer backed by a rope data structure.
/// Provides the core text storage and manipulation for the Vim engine.
pub struct Buffer {
    rope: Rope,
}

impl Buffer {
    /// Create a buffer from a string.
    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
        }
    }

    /// Total number of lines in the buffer.
    pub fn line_count(&self) -> usize {
        let count = self.rope.len_lines();
        // Ropey counts a trailing newline as an extra empty line.
        // We treat a trailing newline as NOT adding an extra line,
        // matching Vim's behavior.
        if count > 0 && self.rope.len_chars() > 0 {
            let last_char = self.rope.char(self.rope.len_chars() - 1);
            if last_char == '\n' {
                return count - 1;
            }
        }
        count
    }

    /// Get the content of a line (0-indexed), without the trailing newline.
    pub fn line(&self, line_idx: usize) -> Option<String> {
        if line_idx >= self.line_count() {
            return None;
        }
        let line = self.rope.line(line_idx);
        let mut s = line.to_string();
        // Strip trailing newline
        if s.ends_with('\n') {
            s.pop();
            if s.ends_with('\r') {
                s.pop();
            }
        }
        Some(s)
    }

    /// Length of a line in characters (0-indexed), excluding newline.
    pub fn line_len(&self, line_idx: usize) -> usize {
        self.line(line_idx).map(|l| l.len()).unwrap_or(0)
    }

    /// Total character count in the buffer.
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    /// Insert a character at a line/column position.
    pub fn insert_char(&mut self, line: usize, col: usize, ch: char) {
        let char_idx = self.line_col_to_char_idx(line, col);
        self.rope.insert_char(char_idx, ch);
    }

    /// Insert a string at a line/column position.
    pub fn insert_str(&mut self, line: usize, col: usize, text: &str) {
        let char_idx = self.line_col_to_char_idx(line, col);
        self.rope.insert(char_idx, text);
    }

    /// Delete a range of characters: from (line, col) for `count` chars.
    pub fn delete_chars(&mut self, line: usize, col: usize, count: usize) {
        let start = self.line_col_to_char_idx(line, col);
        let end = (start + count).min(self.rope.len_chars());
        if start < end {
            self.rope.remove(start..end);
        }
    }

    /// Delete an entire line (0-indexed), including its newline.
    pub fn delete_line(&mut self, line_idx: usize) {
        if line_idx >= self.line_count() {
            return;
        }
        let start = self.rope.line_to_char(line_idx);
        let end = if line_idx + 1 < self.rope.len_lines() {
            self.rope.line_to_char(line_idx + 1)
        } else {
            // Last line: delete from start of line to end of buffer.
            // Also delete the preceding newline if there is one.
            let buf_end = self.rope.len_chars();
            if start > 0 {
                // Remove the newline before this last line too
                return self.rope.remove(start - 1..buf_end);
            }
            buf_end
        };
        self.rope.remove(start..end);
    }

    /// Get the full buffer content as a string.
    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }

    /// Convert a (line, col) position to an absolute char index in the rope.
    fn line_col_to_char_idx(&self, line: usize, col: usize) -> usize {
        let line_start = self.rope.line_to_char(line.min(self.rope.len_lines().saturating_sub(1)));
        let line_len = self.line_len(line);
        line_start + col.min(line_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_and_line_count() {
        let buf = Buffer::from_str("hello\nworld\n");
        assert_eq!(buf.line_count(), 2);

        let buf = Buffer::from_str("hello\nworld");
        assert_eq!(buf.line_count(), 2);

        let buf = Buffer::from_str("single line");
        assert_eq!(buf.line_count(), 1);

        let buf = Buffer::from_str("");
        assert_eq!(buf.line_count(), 1); // empty buffer still has one "line"
    }

    #[test]
    fn test_get_line() {
        let buf = Buffer::from_str("hello\nworld\nfoo");
        assert_eq!(buf.line(0), Some("hello".to_string()));
        assert_eq!(buf.line(1), Some("world".to_string()));
        assert_eq!(buf.line(2), Some("foo".to_string()));
        assert_eq!(buf.line(3), None);
    }

    #[test]
    fn test_line_len() {
        let buf = Buffer::from_str("hello\nhi\n");
        assert_eq!(buf.line_len(0), 5);
        assert_eq!(buf.line_len(1), 2);
    }

    #[test]
    fn test_insert_char() {
        let mut buf = Buffer::from_str("hllo");
        buf.insert_char(0, 1, 'e');
        assert_eq!(buf.line(0), Some("hello".to_string()));
    }

    #[test]
    fn test_insert_str() {
        let mut buf = Buffer::from_str("hd");
        buf.insert_str(0, 1, "ello worl");
        assert_eq!(buf.line(0), Some("hello world".to_string()));
    }

    #[test]
    fn test_delete_chars() {
        let mut buf = Buffer::from_str("hello world");
        buf.delete_chars(0, 5, 6); // delete " world"
        assert_eq!(buf.line(0), Some("hello".to_string()));
    }

    #[test]
    fn test_delete_line() {
        let mut buf = Buffer::from_str("aaa\nbbb\nccc");
        buf.delete_line(1);
        assert_eq!(buf.line_count(), 2);
        assert_eq!(buf.line(0), Some("aaa".to_string()));
        assert_eq!(buf.line(1), Some("ccc".to_string()));
    }

    #[test]
    fn test_delete_last_line() {
        let mut buf = Buffer::from_str("aaa\nbbb");
        buf.delete_line(1);
        assert_eq!(buf.line_count(), 1);
        assert_eq!(buf.line(0), Some("aaa".to_string()));
    }

    #[test]
    fn test_delete_first_line() {
        let mut buf = Buffer::from_str("aaa\nbbb\nccc");
        buf.delete_line(0);
        assert_eq!(buf.line_count(), 2);
        assert_eq!(buf.line(0), Some("bbb".to_string()));
        assert_eq!(buf.line(1), Some("ccc".to_string()));
    }

    #[test]
    fn test_empty_buffer() {
        let buf = Buffer::from_str("");
        assert_eq!(buf.line_len(0), 0);
        assert!(buf.line(0).is_some()); // empty line exists
    }
}
