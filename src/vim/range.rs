use super::cursor::Cursor;

/// A text range defined by two cursor positions.
/// Used by operators to know what region of the buffer to act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRange {
    /// Start position (inclusive).
    pub start: Cursor,
    /// End position (exclusive for charwise, inclusive line for linewise).
    pub end: Cursor,
    /// Whether this range covers whole lines.
    pub linewise: bool,
}

impl TextRange {
    /// Create a range from two cursor positions, automatically ordering them.
    pub fn new(a: Cursor, b: Cursor, linewise: bool) -> Self {
        let (start, end) = if (a.line, a.col) <= (b.line, b.col) {
            (a, b)
        } else {
            (b, a)
        };
        Self {
            start,
            end,
            linewise,
        }
    }

    /// Create a charwise range.
    pub fn charwise(a: Cursor, b: Cursor) -> Self {
        Self::new(a, b, false)
    }

    /// Create a linewise range spanning from line a to line b.
    pub fn linewise(a: Cursor, b: Cursor) -> Self {
        Self::new(a, b, true)
    }

    /// Whether the range spans multiple lines.
    pub fn is_multiline(&self) -> bool {
        self.start.line != self.end.line
    }

    /// Number of lines this range spans (inclusive).
    pub fn line_span(&self) -> usize {
        self.end.line - self.start.line + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_orders_positions() {
        let r = TextRange::charwise(Cursor::new(2, 5), Cursor::new(0, 3));
        assert_eq!(r.start, Cursor::new(0, 3));
        assert_eq!(r.end, Cursor::new(2, 5));
    }

    #[test]
    fn test_range_already_ordered() {
        let r = TextRange::charwise(Cursor::new(0, 0), Cursor::new(1, 4));
        assert_eq!(r.start, Cursor::new(0, 0));
        assert_eq!(r.end, Cursor::new(1, 4));
    }

    #[test]
    fn test_linewise_flag() {
        let r = TextRange::linewise(Cursor::new(0, 0), Cursor::new(2, 0));
        assert!(r.linewise);
        let r2 = TextRange::charwise(Cursor::new(0, 0), Cursor::new(2, 0));
        assert!(!r2.linewise);
    }

    #[test]
    fn test_is_multiline() {
        let single = TextRange::charwise(Cursor::new(0, 2), Cursor::new(0, 8));
        assert!(!single.is_multiline());
        let multi = TextRange::charwise(Cursor::new(0, 2), Cursor::new(1, 3));
        assert!(multi.is_multiline());
    }

    #[test]
    fn test_line_span() {
        let r = TextRange::linewise(Cursor::new(1, 0), Cursor::new(3, 0));
        assert_eq!(r.line_span(), 3);
        let single = TextRange::charwise(Cursor::new(5, 0), Cursor::new(5, 10));
        assert_eq!(single.line_span(), 1);
    }

    #[test]
    fn test_same_position_range() {
        let r = TextRange::charwise(Cursor::new(0, 0), Cursor::new(0, 0));
        assert_eq!(r.start, r.end);
        assert!(!r.is_multiline());
        assert_eq!(r.line_span(), 1);
    }
}
