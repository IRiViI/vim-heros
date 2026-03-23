use super::buffer::Buffer;
use super::cursor::Cursor;

/// Direction of search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

/// Persistent search state for `/`, `?`, `n`, `N`, `*`, `#`.
#[derive(Debug, Clone)]
pub struct SearchState {
    pub pattern: String,
    pub direction: SearchDirection,
    /// Buffer for the search input line (while typing).
    pub input_buf: String,
    /// Whether a search input is currently active.
    pub active: bool,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            pattern: String::new(),
            direction: SearchDirection::Forward,
            input_buf: String::new(),
            active: false,
        }
    }

    /// Start a new search input session.
    pub fn start_input(&mut self, direction: SearchDirection) {
        self.direction = direction;
        self.input_buf.clear();
        self.active = true;
    }

    /// Commit the current input buffer as the search pattern.
    /// Returns true if the pattern is non-empty.
    pub fn commit_input(&mut self) -> bool {
        self.active = false;
        if !self.input_buf.is_empty() {
            self.pattern = self.input_buf.clone();
            self.input_buf.clear();
            true
        } else {
            false
        }
    }

    /// Cancel the search input.
    pub fn cancel_input(&mut self) {
        self.active = false;
        self.input_buf.clear();
    }

    /// Add a character to the search input.
    pub fn push_char(&mut self, ch: char) {
        self.input_buf.push(ch);
    }

    /// Remove the last character from the search input.
    pub fn pop_char(&mut self) {
        self.input_buf.pop();
    }

    /// Whether we have a pattern to search for.
    pub fn has_pattern(&self) -> bool {
        !self.pattern.is_empty()
    }

    /// Get the prompt character (/ or ?).
    pub fn prompt_char(&self) -> char {
        match self.direction {
            SearchDirection::Forward => '/',
            SearchDirection::Backward => '?',
        }
    }
}

/// Find the next occurrence of `pattern` in the buffer starting after `cursor`.
pub fn search_forward(cursor: &Cursor, buffer: &Buffer, pattern: &str) -> Option<Cursor> {
    if pattern.is_empty() {
        return None;
    }

    let line_count = buffer.line_count();
    if line_count == 0 {
        return None;
    }

    // Search from current position forward, wrapping around
    for offset in 0..line_count {
        let line_idx = (cursor.line + offset) % line_count;
        let line_content = buffer.line(line_idx).unwrap_or_default();

        // Starting column: if on the current line, start after cursor; otherwise from 0
        let start_col = if offset == 0 { cursor.col + 1 } else { 0 };

        if start_col < line_content.len() {
            if let Some(pos) = line_content[start_col..].find(pattern) {
                return Some(Cursor::new(line_idx, start_col + pos));
            }
        }
    }

    None
}

/// Find the previous occurrence of `pattern` in the buffer starting before `cursor`.
pub fn search_backward(cursor: &Cursor, buffer: &Buffer, pattern: &str) -> Option<Cursor> {
    if pattern.is_empty() {
        return None;
    }

    let line_count = buffer.line_count();
    if line_count == 0 {
        return None;
    }

    // Search from current position backward, wrapping around
    for offset in 0..line_count {
        let line_idx = (cursor.line + line_count - offset) % line_count;
        let line_content = buffer.line(line_idx).unwrap_or_default();

        // Ending column: if on the current line, search before cursor; otherwise full line
        let search_end = if offset == 0 {
            cursor.col
        } else {
            line_content.len()
        };

        // Find the last occurrence before search_end
        if search_end > 0 {
            if let Some(pos) = line_content[..search_end].rfind(pattern) {
                return Some(Cursor::new(line_idx, pos));
            }
        }
    }

    None
}

/// Search in the given direction.
pub fn search_next(
    cursor: &Cursor,
    buffer: &Buffer,
    pattern: &str,
    direction: SearchDirection,
) -> Option<Cursor> {
    match direction {
        SearchDirection::Forward => search_forward(cursor, buffer, pattern),
        SearchDirection::Backward => search_backward(cursor, buffer, pattern),
    }
}

/// Get the word under the cursor (for `*` and `#`).
pub fn word_under_cursor(cursor: &Cursor, buffer: &Buffer) -> Option<String> {
    let line = buffer.line(cursor.line)?;
    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() || cursor.col >= chars.len() {
        return None;
    }

    let ch = chars[cursor.col];
    if !ch.is_alphanumeric() && ch != '_' {
        return None;
    }

    // Scan left
    let mut start = cursor.col;
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }

    // Scan right
    let mut end = cursor.col + 1;
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }

    Some(chars[start..end].iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_buffer() -> Buffer {
        Buffer::from_str("hello world\nfoo bar baz\nhello again\nfoo end")
    }

    #[test]
    fn test_search_forward_basic() {
        let buf = test_buffer();
        let cur = Cursor::new(0, 0);
        let result = search_forward(&cur, &buf, "world").unwrap();
        assert_eq!(result, Cursor::new(0, 6));
    }

    #[test]
    fn test_search_forward_next_line() {
        let buf = test_buffer();
        let cur = Cursor::new(0, 0);
        let result = search_forward(&cur, &buf, "foo").unwrap();
        assert_eq!(result, Cursor::new(1, 0));
    }

    #[test]
    fn test_search_forward_wraps() {
        let buf = test_buffer();
        let cur = Cursor::new(3, 0); // on last line
        let result = search_forward(&cur, &buf, "hello").unwrap();
        assert_eq!(result, Cursor::new(0, 0)); // wraps to first line
    }

    #[test]
    fn test_search_backward_basic() {
        let buf = test_buffer();
        let cur = Cursor::new(2, 6); // on "again"
        let result = search_backward(&cur, &buf, "hello").unwrap();
        assert_eq!(result, Cursor::new(2, 0));
    }

    #[test]
    fn test_search_backward_prev_line() {
        let buf = test_buffer();
        let cur = Cursor::new(2, 0);
        let result = search_backward(&cur, &buf, "bar").unwrap();
        assert_eq!(result, Cursor::new(1, 4));
    }

    #[test]
    fn test_search_backward_wraps() {
        let buf = test_buffer();
        let cur = Cursor::new(0, 2);
        let result = search_backward(&cur, &buf, "end").unwrap();
        assert_eq!(result, Cursor::new(3, 4)); // wraps to last line
    }

    #[test]
    fn test_search_not_found() {
        let buf = test_buffer();
        let cur = Cursor::new(0, 0);
        assert!(search_forward(&cur, &buf, "xyz").is_none());
        assert!(search_backward(&cur, &buf, "xyz").is_none());
    }

    #[test]
    fn test_word_under_cursor() {
        let buf = test_buffer();
        let cur = Cursor::new(0, 0);
        assert_eq!(word_under_cursor(&cur, &buf), Some("hello".to_string()));
    }

    #[test]
    fn test_word_under_cursor_middle() {
        let buf = test_buffer();
        let cur = Cursor::new(0, 2); // on 'l' in "hello"
        assert_eq!(word_under_cursor(&cur, &buf), Some("hello".to_string()));
    }

    #[test]
    fn test_word_under_cursor_on_space() {
        let buf = test_buffer();
        let cur = Cursor::new(0, 5); // on space
        assert!(word_under_cursor(&cur, &buf).is_none());
    }

    #[test]
    fn test_search_state_lifecycle() {
        let mut state = SearchState::new();
        assert!(!state.active);
        assert!(!state.has_pattern());

        state.start_input(SearchDirection::Forward);
        assert!(state.active);

        state.push_char('h');
        state.push_char('i');
        assert_eq!(state.input_buf, "hi");

        state.pop_char();
        assert_eq!(state.input_buf, "h");

        state.push_char('e');
        state.push_char('l');
        assert!(state.commit_input());
        assert!(!state.active);
        assert_eq!(state.pattern, "hel");
        assert!(state.has_pattern());
    }

    #[test]
    fn test_search_state_cancel() {
        let mut state = SearchState::new();
        state.start_input(SearchDirection::Forward);
        state.push_char('x');
        state.cancel_input();
        assert!(!state.active);
        assert_eq!(state.pattern, ""); // pattern not set
    }
}
