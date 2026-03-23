use super::buffer::Buffer;
use super::cursor::Cursor;

/// A text object defines a region of text (start..end) that an operator acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextObject {
    InnerWord,
    AWord,
    InnerQuote(char),   // i", i', i`
    AQuote(char),       // a", a', a`
    InnerParen,         // i( / i)
    AParen,             // a( / a)
    InnerBrace,         // i{ / i}
    ABrace,             // a{ / a}
    InnerBracket,       // i[ / i]
    ABracket,           // a[ / a]
    InnerAngle,         // i< / i>
    AAngle,             // a< / a>
}

/// The resolved range of a text object. All positions are (line, col).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextObjectRange {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize, // exclusive
    pub linewise: bool,
}

impl TextObject {
    /// Whether this text object operates linewise (for `{` / `}` blocks spanning multiple lines).
    pub fn is_linewise(&self) -> bool {
        match self {
            TextObject::InnerBrace | TextObject::ABrace => false, // charwise by default, unless multiline
            _ => false,
        }
    }

    /// Resolve the text object to a range in the buffer.
    pub fn resolve(&self, cursor: &Cursor, buffer: &Buffer) -> Option<TextObjectRange> {
        match self {
            TextObject::InnerWord => inner_word(cursor, buffer),
            TextObject::AWord => a_word(cursor, buffer),
            TextObject::InnerQuote(q) => inner_quote(cursor, buffer, *q),
            TextObject::AQuote(q) => a_quote(cursor, buffer, *q),
            TextObject::InnerParen => inner_delimited(cursor, buffer, '(', ')'),
            TextObject::AParen => a_delimited(cursor, buffer, '(', ')'),
            TextObject::InnerBrace => inner_delimited(cursor, buffer, '{', '}'),
            TextObject::ABrace => a_delimited(cursor, buffer, '{', '}'),
            TextObject::InnerBracket => inner_delimited(cursor, buffer, '[', ']'),
            TextObject::ABracket => a_delimited(cursor, buffer, '[', ']'),
            TextObject::InnerAngle => inner_delimited(cursor, buffer, '<', '>'),
            TextObject::AAngle => a_delimited(cursor, buffer, '<', '>'),
        }
    }

    /// Parse the character after `i` or `a` into a TextObject.
    /// `inner` is true for `i`, false for `a`.
    pub fn from_char(ch: char, inner: bool) -> Option<TextObject> {
        match (ch, inner) {
            ('w', true) => Some(TextObject::InnerWord),
            ('w', false) => Some(TextObject::AWord),
            ('"', true) => Some(TextObject::InnerQuote('"')),
            ('"', false) => Some(TextObject::AQuote('"')),
            ('\'', true) => Some(TextObject::InnerQuote('\'')),
            ('\'', false) => Some(TextObject::AQuote('\'')),
            ('`', true) => Some(TextObject::InnerQuote('`')),
            ('`', false) => Some(TextObject::AQuote('`')),
            ('(' | ')', true) => Some(TextObject::InnerParen),
            ('(' | ')', false) => Some(TextObject::AParen),
            ('b', true) => Some(TextObject::InnerParen),  // b = () alias
            ('b', false) => Some(TextObject::AParen),
            ('{' | '}', true) => Some(TextObject::InnerBrace),
            ('{' | '}', false) => Some(TextObject::ABrace),
            ('B', true) => Some(TextObject::InnerBrace),   // B = {} alias
            ('B', false) => Some(TextObject::ABrace),
            ('[' | ']', true) => Some(TextObject::InnerBracket),
            ('[' | ']', false) => Some(TextObject::ABracket),
            ('<' | '>', true) => Some(TextObject::InnerAngle),
            ('<' | '>', false) => Some(TextObject::AAngle),
            _ => None,
        }
    }
}

// ===== Word text objects =====

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn inner_word(cursor: &Cursor, buffer: &Buffer) -> Option<TextObjectRange> {
    let line = buffer.line(cursor.line)?;
    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return None;
    }
    let col = cursor.col.min(chars.len().saturating_sub(1));
    let ch = chars[col];

    // Determine what "class" we're in: word, whitespace, or punctuation
    let is_same_class: Box<dyn Fn(char) -> bool> = if is_word_char(ch) {
        Box::new(|c: char| is_word_char(c))
    } else if ch.is_whitespace() {
        Box::new(|c: char| c.is_whitespace())
    } else {
        // punctuation
        Box::new(|c: char| !is_word_char(c) && !c.is_whitespace())
    };

    // Scan left to find start
    let mut start = col;
    while start > 0 && is_same_class(chars[start - 1]) {
        start -= 1;
    }

    // Scan right to find end (exclusive)
    let mut end = col + 1;
    while end < chars.len() && is_same_class(chars[end]) {
        end += 1;
    }

    Some(TextObjectRange {
        start_line: cursor.line,
        start_col: start,
        end_line: cursor.line,
        end_col: end,
        linewise: false,
    })
}

fn a_word(cursor: &Cursor, buffer: &Buffer) -> Option<TextObjectRange> {
    let mut range = inner_word(cursor, buffer)?;
    let line = buffer.line(cursor.line)?;
    let chars: Vec<char> = line.chars().collect();

    // "a word" includes trailing whitespace, or leading if at end
    if range.end_col < chars.len() && chars[range.end_col].is_whitespace() {
        // Include trailing whitespace
        while range.end_col < chars.len() && chars[range.end_col].is_whitespace() {
            range.end_col += 1;
        }
    } else if range.start_col > 0 && chars[range.start_col - 1].is_whitespace() {
        // Include leading whitespace
        while range.start_col > 0 && chars[range.start_col - 1].is_whitespace() {
            range.start_col -= 1;
        }
    }

    Some(range)
}

// ===== Quote text objects =====

fn inner_quote(cursor: &Cursor, buffer: &Buffer, quote: char) -> Option<TextObjectRange> {
    let line = buffer.line(cursor.line)?;
    let chars: Vec<char> = line.chars().collect();
    let col = cursor.col;

    // Find the pair of quotes surrounding (or at) the cursor
    // Strategy: find all quote positions, pair them up, find the pair containing cursor
    let positions: Vec<usize> = chars
        .iter()
        .enumerate()
        .filter(|&(_, c)| *c == quote)
        .map(|(i, _)| i)
        .collect();

    // Try pairs: (0,1), (2,3), etc. — also check if cursor is between any pair
    for pair in positions.chunks(2) {
        if pair.len() == 2 {
            let open = pair[0];
            let close = pair[1];
            if col >= open && col <= close {
                return Some(TextObjectRange {
                    start_line: cursor.line,
                    start_col: open + 1,
                    end_line: cursor.line,
                    end_col: close,
                    linewise: false,
                });
            }
        }
    }

    None
}

fn a_quote(cursor: &Cursor, buffer: &Buffer, quote: char) -> Option<TextObjectRange> {
    let line = buffer.line(cursor.line)?;
    let chars: Vec<char> = line.chars().collect();
    let col = cursor.col;

    let positions: Vec<usize> = chars
        .iter()
        .enumerate()
        .filter(|&(_, c)| *c == quote)
        .map(|(i, _)| i)
        .collect();

    for pair in positions.chunks(2) {
        if pair.len() == 2 {
            let open = pair[0];
            let close = pair[1];
            if col >= open && col <= close {
                return Some(TextObjectRange {
                    start_line: cursor.line,
                    start_col: open,
                    end_line: cursor.line,
                    end_col: close + 1,
                    linewise: false,
                });
            }
        }
    }

    None
}

// ===== Delimited text objects (parens, braces, brackets, angles) =====

/// Collect all text from the buffer into a flat char array with position mapping.
fn buffer_chars(buffer: &Buffer) -> (Vec<char>, Vec<(usize, usize)>) {
    let mut chars = Vec::new();
    let mut positions = Vec::new();
    for line_idx in 0..buffer.line_count() {
        if let Some(line) = buffer.line(line_idx) {
            for (col, ch) in line.chars().enumerate() {
                chars.push(ch);
                positions.push((line_idx, col));
            }
        }
        // Add newline between lines (not after last)
        if line_idx + 1 < buffer.line_count() {
            chars.push('\n');
            positions.push((line_idx, buffer.line_len(line_idx)));
        }
    }
    (chars, positions)
}

fn cursor_to_flat_index(cursor: &Cursor, positions: &[(usize, usize)]) -> Option<usize> {
    positions
        .iter()
        .position(|&(l, c)| l == cursor.line && c == cursor.col)
}

fn inner_delimited(
    cursor: &Cursor,
    buffer: &Buffer,
    open: char,
    close: char,
) -> Option<TextObjectRange> {
    let (chars, positions) = buffer_chars(buffer);
    let flat_pos = cursor_to_flat_index(cursor, &positions)?;

    // Find matching open bracket going backward
    let open_idx = find_matching_open(&chars, flat_pos, open, close)?;
    // Find matching close bracket going forward
    let close_idx = find_matching_close(&chars, flat_pos, open, close)?;

    if open_idx >= close_idx {
        return None;
    }

    // Inner: between the delimiters (exclusive of delimiters)
    let start_idx = open_idx + 1;
    let end_idx = close_idx;

    if start_idx >= positions.len() || end_idx > positions.len() || start_idx >= end_idx {
        return None;
    }

    let (start_line, start_col) = positions[start_idx];
    // end is exclusive — use the position of the close delimiter
    let (end_line, end_col) = positions[end_idx.min(positions.len() - 1)];

    Some(TextObjectRange {
        start_line,
        start_col,
        end_line,
        end_col,
        linewise: false,
    })
}

fn a_delimited(
    cursor: &Cursor,
    buffer: &Buffer,
    open: char,
    close: char,
) -> Option<TextObjectRange> {
    let (chars, positions) = buffer_chars(buffer);
    let flat_pos = cursor_to_flat_index(cursor, &positions)?;

    let open_idx = find_matching_open(&chars, flat_pos, open, close)?;
    let close_idx = find_matching_close(&chars, flat_pos, open, close)?;

    if open_idx >= close_idx {
        return None;
    }

    // "A" delimiter: include the delimiters themselves
    let end_idx = close_idx + 1;

    if open_idx >= positions.len() || end_idx > positions.len() {
        return None;
    }

    let (start_line, start_col) = positions[open_idx];
    let (end_line, end_col) = if end_idx < positions.len() {
        positions[end_idx]
    } else {
        // Past end of buffer
        let last = positions[positions.len() - 1];
        (last.0, last.1 + 1)
    };

    Some(TextObjectRange {
        start_line,
        start_col,
        end_line,
        end_col,
        linewise: false,
    })
}

/// Search backward from `pos` for the matching open delimiter, respecting nesting.
fn find_matching_open(chars: &[char], pos: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0i32;
    let mut i = pos as i64;

    // If we're on the close delimiter, start by counting it
    if chars.get(pos as usize) == Some(&close) {
        depth += 1;
    }

    // If we're on the open delimiter, that's our answer
    if chars.get(pos as usize) == Some(&open) && depth == 0 {
        return Some(pos);
    }

    i -= 1;
    while i >= 0 {
        let idx = i as usize;
        if chars[idx] == close {
            depth += 1;
        } else if chars[idx] == open {
            if depth == 0 {
                return Some(idx);
            }
            depth -= 1;
        }
        i -= 1;
    }
    None
}

/// Search forward from `pos` for the matching close delimiter, respecting nesting.
fn find_matching_close(chars: &[char], pos: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0i32;
    let mut i = pos;

    // If we're on the open delimiter, start by counting it
    if chars.get(i) == Some(&open) {
        depth += 1;
    }

    // If we're on the close delimiter, that's our answer
    if chars.get(i) == Some(&close) && depth == 0 {
        return Some(i);
    }

    i += 1;
    while i < chars.len() {
        if chars[i] == open {
            depth += 1;
        } else if chars[i] == close {
            if depth == 0 {
                return Some(i);
            }
            depth -= 1;
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== inner word =====

    #[test]
    fn test_inner_word_simple() {
        let buf = Buffer::from_str("hello world");
        let cur = Cursor::new(0, 0);
        let range = TextObject::InnerWord.resolve(&cur, &buf).unwrap();
        assert_eq!(range.start_col, 0);
        assert_eq!(range.end_col, 5); // "hello"
    }

    #[test]
    fn test_inner_word_middle() {
        let buf = Buffer::from_str("hello world");
        let cur = Cursor::new(0, 2); // on 'l' in hello
        let range = TextObject::InnerWord.resolve(&cur, &buf).unwrap();
        assert_eq!(range.start_col, 0);
        assert_eq!(range.end_col, 5);
    }

    #[test]
    fn test_inner_word_second_word() {
        let buf = Buffer::from_str("hello world");
        let cur = Cursor::new(0, 6); // on 'w' in world
        let range = TextObject::InnerWord.resolve(&cur, &buf).unwrap();
        assert_eq!(range.start_col, 6);
        assert_eq!(range.end_col, 11);
    }

    #[test]
    fn test_inner_word_on_whitespace() {
        let buf = Buffer::from_str("hello   world");
        let cur = Cursor::new(0, 6); // on whitespace
        let range = TextObject::InnerWord.resolve(&cur, &buf).unwrap();
        assert_eq!(range.start_col, 5);
        assert_eq!(range.end_col, 8); // the whitespace block
    }

    #[test]
    fn test_inner_word_punctuation() {
        let buf = Buffer::from_str("foo.bar");
        let cur = Cursor::new(0, 3); // on '.'
        let range = TextObject::InnerWord.resolve(&cur, &buf).unwrap();
        assert_eq!(range.start_col, 3);
        assert_eq!(range.end_col, 4); // just the dot
    }

    // ===== a word =====

    #[test]
    fn test_a_word_trailing_space() {
        let buf = Buffer::from_str("hello world");
        let cur = Cursor::new(0, 0);
        let range = TextObject::AWord.resolve(&cur, &buf).unwrap();
        assert_eq!(range.start_col, 0);
        assert_eq!(range.end_col, 6); // "hello " including trailing space
    }

    #[test]
    fn test_a_word_last_word_leading_space() {
        let buf = Buffer::from_str("hello world");
        let cur = Cursor::new(0, 6);
        let range = TextObject::AWord.resolve(&cur, &buf).unwrap();
        // Last word: no trailing space, so includes leading space
        assert_eq!(range.start_col, 5); // includes the space before "world"
        assert_eq!(range.end_col, 11);
    }

    // ===== inner quote =====

    #[test]
    fn test_inner_double_quote() {
        let buf = Buffer::from_str(r#"let x = "hello";"#);
        let cur = Cursor::new(0, 10); // inside quotes
        let range = TextObject::InnerQuote('"').resolve(&cur, &buf).unwrap();
        assert_eq!(range.start_col, 9);
        assert_eq!(range.end_col, 14); // "hello" without quotes
    }

    #[test]
    fn test_inner_quote_on_quote() {
        let buf = Buffer::from_str(r#"let x = "hello";"#);
        let cur = Cursor::new(0, 8); // on opening quote
        let range = TextObject::InnerQuote('"').resolve(&cur, &buf).unwrap();
        assert_eq!(range.start_col, 9);
        assert_eq!(range.end_col, 14);
    }

    #[test]
    fn test_a_double_quote() {
        let buf = Buffer::from_str(r#"let x = "hello";"#);
        let cur = Cursor::new(0, 10);
        let range = TextObject::AQuote('"').resolve(&cur, &buf).unwrap();
        assert_eq!(range.start_col, 8);
        assert_eq!(range.end_col, 15); // includes both quotes
    }

    // ===== inner paren =====

    #[test]
    fn test_inner_paren() {
        let buf = Buffer::from_str("fn foo(x, y)");
        let cur = Cursor::new(0, 8); // on 'x'
        let range = TextObject::InnerParen.resolve(&cur, &buf).unwrap();
        assert_eq!(range.start_col, 7);
        assert_eq!(range.end_col, 11); // "x, y"
    }

    #[test]
    fn test_a_paren() {
        let buf = Buffer::from_str("fn foo(x, y)");
        let cur = Cursor::new(0, 8);
        let range = TextObject::AParen.resolve(&cur, &buf).unwrap();
        assert_eq!(range.start_col, 6);
        assert_eq!(range.end_col, 12); // "(x, y)"
    }

    // ===== inner brace (multiline) =====

    #[test]
    fn test_inner_brace_multiline() {
        let buf = Buffer::from_str("fn main() {\n    hello\n}");
        let cur = Cursor::new(1, 4); // inside braces
        let range = TextObject::InnerBrace.resolve(&cur, &buf).unwrap();
        // { is at (0, 11), the char after it is \n which maps to (0, 11) in flat pos
        // The inner content starts after the { — which is the \n between lines
        assert_eq!(range.start_line, 0);
        assert_eq!(range.start_col, 11); // the newline char position (end of line 0)
        assert_eq!(range.end_line, 2);
        assert_eq!(range.end_col, 0); // before }
    }

    #[test]
    fn test_inner_bracket() {
        let buf = Buffer::from_str("arr[i + 1]");
        let cur = Cursor::new(0, 5); // on 'i'
        let range = TextObject::InnerBracket.resolve(&cur, &buf).unwrap();
        assert_eq!(range.start_col, 4);
        assert_eq!(range.end_col, 9); // "i + 1"
    }

    // ===== nested delimiters =====

    #[test]
    fn test_nested_parens() {
        let buf = Buffer::from_str("foo(bar(x))");
        let cur = Cursor::new(0, 8); // on 'x', inside inner parens
        let range = TextObject::InnerParen.resolve(&cur, &buf).unwrap();
        assert_eq!(range.start_col, 8); // just "x"
        assert_eq!(range.end_col, 9);
    }

    #[test]
    fn test_nested_parens_outer() {
        let buf = Buffer::from_str("foo(bar(x))");
        let cur = Cursor::new(0, 4); // on 'b', inside outer parens
        let range = TextObject::InnerParen.resolve(&cur, &buf).unwrap();
        assert_eq!(range.start_col, 4); // "bar(x)"
        assert_eq!(range.end_col, 10);
    }

    // ===== edge cases =====

    #[test]
    fn test_no_matching_paren() {
        let buf = Buffer::from_str("hello world");
        let cur = Cursor::new(0, 3);
        assert!(TextObject::InnerParen.resolve(&cur, &buf).is_none());
    }

    #[test]
    fn test_empty_line() {
        let buf = Buffer::from_str("");
        let cur = Cursor::new(0, 0);
        assert!(TextObject::InnerWord.resolve(&cur, &buf).is_none());
    }

    #[test]
    fn test_from_char() {
        assert_eq!(TextObject::from_char('w', true), Some(TextObject::InnerWord));
        assert_eq!(TextObject::from_char('w', false), Some(TextObject::AWord));
        assert_eq!(TextObject::from_char('"', true), Some(TextObject::InnerQuote('"')));
        assert_eq!(TextObject::from_char('(', true), Some(TextObject::InnerParen));
        assert_eq!(TextObject::from_char(')', false), Some(TextObject::AParen));
        assert_eq!(TextObject::from_char('{', true), Some(TextObject::InnerBrace));
        assert_eq!(TextObject::from_char('B', false), Some(TextObject::ABrace));
        assert_eq!(TextObject::from_char('b', true), Some(TextObject::InnerParen));
        assert_eq!(TextObject::from_char('z', true), None);
    }
}
