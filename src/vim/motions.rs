use super::buffer::Buffer;
use super::cursor::Cursor;

// ===== Character classification =====

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharClass {
    Word,
    Punct,
    Blank,
}

/// Classify for small-word motions (w, b, e).
fn classify(ch: char) -> CharClass {
    if ch.is_alphanumeric() || ch == '_' {
        CharClass::Word
    } else if ch.is_whitespace() {
        CharClass::Blank
    } else {
        CharClass::Punct
    }
}

/// Classify for big-WORD motions (W, B, E): only blank vs non-blank.
fn classify_big(ch: char) -> CharClass {
    if ch.is_whitespace() {
        CharClass::Blank
    } else {
        CharClass::Word
    }
}

/// Get a line's characters as a Vec.
fn line_chars(buffer: &Buffer, line: usize) -> Vec<char> {
    buffer.line(line).unwrap_or_default().chars().collect()
}

/// Move to the previous character position, crossing line boundaries.
fn prev_pos(line: usize, col: usize, buffer: &Buffer) -> Option<(usize, usize)> {
    if col > 0 {
        Some((line, col - 1))
    } else if line > 0 {
        let prev_len = buffer.line_len(line - 1);
        Some((line - 1, if prev_len > 0 { prev_len - 1 } else { 0 }))
    } else {
        None
    }
}

/// Find column of first non-blank character on a line.
fn first_non_blank(buffer: &Buffer, line: usize) -> Cursor {
    let chars = line_chars(buffer, line);
    for (i, &ch) in chars.iter().enumerate() {
        if !ch.is_whitespace() {
            return Cursor::new(line, i);
        }
    }
    Cursor::new(line, 0)
}

fn is_line_empty(buffer: &Buffer, line: usize) -> bool {
    buffer.line_len(line) == 0
}

// ===== Basic movement (h/j/k/l) =====

pub fn move_left(cursor: &Cursor, _buffer: &Buffer) -> Cursor {
    Cursor::new(cursor.line, cursor.col.saturating_sub(1))
}

pub fn move_down(cursor: &Cursor, buffer: &Buffer) -> Cursor {
    let max_line = buffer.line_count().saturating_sub(1);
    let new_line = (cursor.line + 1).min(max_line);
    let new_col = cursor.col.min(buffer.line_len(new_line).saturating_sub(1));
    Cursor::new(new_line, new_col)
}

pub fn move_up(cursor: &Cursor, buffer: &Buffer) -> Cursor {
    let new_line = cursor.line.saturating_sub(1);
    let new_col = cursor.col.min(buffer.line_len(new_line).saturating_sub(1));
    Cursor::new(new_line, new_col)
}

pub fn move_right(cursor: &Cursor, buffer: &Buffer) -> Cursor {
    let max_col = buffer.line_len(cursor.line).saturating_sub(1);
    let new_col = (cursor.col + 1).min(max_col);
    Cursor::new(cursor.line, new_col)
}

// ===== Word motions =====

pub fn word_forward(cursor: &Cursor, buffer: &Buffer) -> Cursor {
    word_forward_impl(cursor, buffer, classify)
}

pub fn big_word_forward(cursor: &Cursor, buffer: &Buffer) -> Cursor {
    word_forward_impl(cursor, buffer, classify_big)
}

fn word_forward_impl(cursor: &Cursor, buffer: &Buffer, cls: fn(char) -> CharClass) -> Cursor {
    let mut line = cursor.line;
    let mut col = cursor.col;
    let max_line = buffer.line_count().saturating_sub(1);
    let chars = line_chars(buffer, line);

    if chars.is_empty() {
        if line < max_line {
            return Cursor::new(line + 1, 0);
        }
        return *cursor;
    }

    if col >= chars.len() {
        col = chars.len().saturating_sub(1);
    }

    let class = cls(chars[col]);

    // Phase 1: skip current class (if non-blank)
    if class != CharClass::Blank {
        while col < chars.len() && cls(chars[col]) == class {
            col += 1;
        }
    }

    // Phase 2: skip blanks on current line
    while col < chars.len() && cls(chars[col]) == CharClass::Blank {
        col += 1;
    }

    // Found non-blank on current line
    if col < chars.len() {
        return Cursor::new(line, col);
    }

    // Phase 3: cross to subsequent lines
    loop {
        line += 1;
        if line > max_line {
            return Cursor::new(max_line, buffer.line_len(max_line).saturating_sub(1));
        }

        let next_chars = line_chars(buffer, line);

        // Empty line is a word boundary
        if next_chars.is_empty() {
            return Cursor::new(line, 0);
        }

        // Find first non-blank
        for (i, &ch) in next_chars.iter().enumerate() {
            if cls(ch) != CharClass::Blank {
                return Cursor::new(line, i);
            }
        }
        // Entire line is blanks, continue
    }
}

pub fn word_backward(cursor: &Cursor, buffer: &Buffer) -> Cursor {
    word_backward_impl(cursor, buffer, classify)
}

pub fn big_word_backward(cursor: &Cursor, buffer: &Buffer) -> Cursor {
    word_backward_impl(cursor, buffer, classify_big)
}

fn word_backward_impl(cursor: &Cursor, buffer: &Buffer, cls: fn(char) -> CharClass) -> Cursor {
    let mut line = cursor.line;
    let mut col = cursor.col;

    // Step 1: move back at least one position
    match prev_pos(line, col, buffer) {
        Some((l, c)) => {
            line = l;
            col = c;
        }
        None => return Cursor::new(0, 0),
    }

    // Step 2: skip blanks backward (crossing lines)
    loop {
        let chars = line_chars(buffer, line);
        if chars.is_empty() {
            // Empty line = word boundary, stop here
            return Cursor::new(line, 0);
        }
        if col < chars.len() && cls(chars[col]) != CharClass::Blank {
            break;
        }
        match prev_pos(line, col, buffer) {
            Some((l, c)) => {
                line = l;
                col = c;
            }
            None => return Cursor::new(0, 0),
        }
    }

    // Step 3: find start of current word
    let chars = line_chars(buffer, line);
    let class = cls(chars[col]);
    while col > 0 && cls(chars[col - 1]) == class {
        col -= 1;
    }

    Cursor::new(line, col)
}

pub fn word_end(cursor: &Cursor, buffer: &Buffer) -> Cursor {
    word_end_impl(cursor, buffer, classify)
}

pub fn big_word_end(cursor: &Cursor, buffer: &Buffer) -> Cursor {
    word_end_impl(cursor, buffer, classify_big)
}

fn word_end_impl(cursor: &Cursor, buffer: &Buffer, cls: fn(char) -> CharClass) -> Cursor {
    let mut line = cursor.line;
    let mut col = cursor.col;
    let max_line = buffer.line_count().saturating_sub(1);

    // Step 1: move forward one position
    let chars = line_chars(buffer, line);
    col += 1;
    if col >= chars.len() || chars.is_empty() {
        if line >= max_line {
            return *cursor;
        }
        line += 1;
        col = 0;
    }

    // Step 2: skip blanks/empty lines
    loop {
        let chars = line_chars(buffer, line);
        if chars.is_empty() {
            if line >= max_line {
                return Cursor::new(line, 0);
            }
            line += 1;
            col = 0;
            continue;
        }

        while col < chars.len() && cls(chars[col]) == CharClass::Blank {
            col += 1;
        }
        if col < chars.len() {
            break;
        }

        if line >= max_line {
            return Cursor::new(line, chars.len().saturating_sub(1));
        }
        line += 1;
        col = 0;
    }

    // Step 3: skip to end of current word
    let chars = line_chars(buffer, line);
    let class = cls(chars[col]);
    while col + 1 < chars.len() && cls(chars[col + 1]) == class {
        col += 1;
    }

    Cursor::new(line, col)
}

// ===== Line position motions =====

/// `0` — go to column 0.
pub fn line_start(cursor: &Cursor, _buffer: &Buffer) -> Cursor {
    Cursor::new(cursor.line, 0)
}

/// `^` — go to first non-whitespace character.
pub fn line_first_char(cursor: &Cursor, buffer: &Buffer) -> Cursor {
    first_non_blank(buffer, cursor.line)
}

/// `$` — go to last character on the line.
pub fn line_end(cursor: &Cursor, buffer: &Buffer) -> Cursor {
    let len = buffer.line_len(cursor.line);
    Cursor::new(cursor.line, len.saturating_sub(1))
}

// ===== Line jumping =====

/// `gg` — go to first line, first non-blank.
pub fn goto_first_line(_cursor: &Cursor, buffer: &Buffer) -> Cursor {
    first_non_blank(buffer, 0)
}

/// `G` — go to last line, first non-blank.
pub fn goto_last_line(_cursor: &Cursor, buffer: &Buffer) -> Cursor {
    let last = buffer.line_count().saturating_sub(1);
    first_non_blank(buffer, last)
}

/// `{num}G` / `{num}gg` — go to line (1-indexed), first non-blank.
pub fn goto_line(_cursor: &Cursor, buffer: &Buffer, line_num: usize) -> Cursor {
    let line = line_num.saturating_sub(1).min(buffer.line_count().saturating_sub(1));
    first_non_blank(buffer, line)
}

// ===== Find character motions (current line only) =====

/// `f{char}` — find next occurrence of char on line.
pub fn find_char_forward(cursor: &Cursor, buffer: &Buffer, target: char) -> Cursor {
    let chars = line_chars(buffer, cursor.line);
    for i in (cursor.col + 1)..chars.len() {
        if chars[i] == target {
            return Cursor::new(cursor.line, i);
        }
    }
    *cursor
}

/// `F{char}` — find previous occurrence of char on line.
pub fn find_char_backward(cursor: &Cursor, buffer: &Buffer, target: char) -> Cursor {
    let chars = line_chars(buffer, cursor.line);
    for i in (0..cursor.col).rev() {
        if chars[i] == target {
            return Cursor::new(cursor.line, i);
        }
    }
    *cursor
}

/// `t{char}` — move to just before next occurrence of char on line.
pub fn till_char_forward(cursor: &Cursor, buffer: &Buffer, target: char) -> Cursor {
    let chars = line_chars(buffer, cursor.line);
    for i in (cursor.col + 1)..chars.len() {
        if chars[i] == target {
            return Cursor::new(cursor.line, i - 1);
        }
    }
    *cursor
}

/// `T{char}` — move to just after previous occurrence of char on line.
pub fn till_char_backward(cursor: &Cursor, buffer: &Buffer, target: char) -> Cursor {
    let chars = line_chars(buffer, cursor.line);
    for i in (0..cursor.col).rev() {
        if chars[i] == target {
            return Cursor::new(cursor.line, i + 1);
        }
    }
    *cursor
}

// ===== Paragraph motions =====

/// `}` — move to next paragraph boundary (next empty line).
pub fn paragraph_forward(cursor: &Cursor, buffer: &Buffer) -> Cursor {
    let line_count = buffer.line_count();
    let mut line = cursor.line + 1;

    if line >= line_count {
        return Cursor::new(line_count.saturating_sub(1), 0);
    }

    let starting_empty = is_line_empty(buffer, cursor.line);

    if starting_empty {
        // Skip consecutive blank lines, then skip paragraph, stop at next blank
        while line < line_count && is_line_empty(buffer, line) {
            line += 1;
        }
        while line < line_count && !is_line_empty(buffer, line) {
            line += 1;
        }
    } else {
        // Find next blank line
        while line < line_count && !is_line_empty(buffer, line) {
            line += 1;
        }
    }

    Cursor::new(line.min(line_count.saturating_sub(1)), 0)
}

/// `{` — move to previous paragraph boundary (previous empty line).
pub fn paragraph_backward(cursor: &Cursor, buffer: &Buffer) -> Cursor {
    if cursor.line == 0 {
        return Cursor::new(0, 0);
    }

    let mut line = cursor.line - 1;
    let starting_empty = is_line_empty(buffer, cursor.line);

    if starting_empty {
        while line > 0 && is_line_empty(buffer, line) {
            line -= 1;
        }
        while line > 0 && !is_line_empty(buffer, line) {
            line -= 1;
        }
    } else {
        while line > 0 && !is_line_empty(buffer, line) {
            line -= 1;
        }
    }

    Cursor::new(line, 0)
}

// ===== Bracket matching =====

/// `%` — jump to matching bracket. If not on a bracket, scan forward on line.
pub fn match_bracket(cursor: &Cursor, buffer: &Buffer) -> Cursor {
    let chars = line_chars(buffer, cursor.line);
    if chars.is_empty() {
        return *cursor;
    }

    let bracket_chars = "(){}[]";

    // Find bracket at or after cursor on current line
    let start_col = if cursor.col < chars.len() && bracket_chars.contains(chars[cursor.col]) {
        cursor.col
    } else {
        let mut found = None;
        for i in (cursor.col + 1)..chars.len() {
            if bracket_chars.contains(chars[i]) {
                found = Some(i);
                break;
            }
        }
        match found {
            Some(c) => c,
            None => return *cursor,
        }
    };

    let bracket = chars[start_col];
    let (target, forward) = match bracket {
        '(' => (')', true),
        ')' => ('(', false),
        '{' => ('}', true),
        '}' => ('{', false),
        '[' => (']', true),
        ']' => ('[', false),
        _ => return *cursor,
    };

    let mut depth: i32 = 1;
    let mut line = cursor.line;
    let mut col = start_col;
    let max_line = buffer.line_count().saturating_sub(1);

    if forward {
        col += 1;
        loop {
            let lc = line_chars(buffer, line);
            while col < lc.len() {
                if lc[col] == bracket {
                    depth += 1;
                } else if lc[col] == target {
                    depth -= 1;
                    if depth == 0 {
                        return Cursor::new(line, col);
                    }
                }
                col += 1;
            }
            line += 1;
            col = 0;
            if line > max_line {
                return *cursor;
            }
        }
    } else {
        // Scan backward
        match prev_pos(line, col, buffer) {
            Some((l, c)) => {
                line = l;
                col = c;
            }
            None => return *cursor,
        }
        loop {
            let lc = line_chars(buffer, line);
            if !lc.is_empty() {
                loop {
                    if lc[col] == bracket {
                        depth += 1;
                    } else if lc[col] == target {
                        depth -= 1;
                        if depth == 0 {
                            return Cursor::new(line, col);
                        }
                    }
                    if col == 0 {
                        break;
                    }
                    col -= 1;
                }
            }
            if line == 0 {
                return *cursor;
            }
            line -= 1;
            let len = buffer.line_len(line);
            col = if len > 0 { len - 1 } else { 0 };
        }
    }
}

// ===== Motion enum for operator+motion combos =====

/// Motions that can be combined with operators (d, c, y).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Left,
    Down,
    Up,
    Right,
    WordForward,
    WordBackward,
    WordEnd,
    BigWordForward,
    BigWordBackward,
    BigWordEnd,
    LineStart,
    LineFirstChar,
    LineEnd,
    GotoFirstLine,
    GotoLastLine,
    GotoLine(usize),
    FindCharForward(char),
    FindCharBackward(char),
    TillCharForward(char),
    TillCharBackward(char),
    ParagraphForward,
    ParagraphBackward,
    MatchBracket,
}

impl Motion {
    /// Whether this motion produces a linewise range when used with an operator.
    pub fn is_linewise(&self) -> bool {
        matches!(
            self,
            Motion::Down
                | Motion::Up
                | Motion::GotoFirstLine
                | Motion::GotoLastLine
                | Motion::GotoLine(_)
                | Motion::ParagraphForward
                | Motion::ParagraphBackward
        )
    }
}

/// Apply a motion to a cursor, returning the new cursor position.
pub fn apply_motion(motion: Motion, cursor: &Cursor, buffer: &Buffer) -> Cursor {
    match motion {
        Motion::Left => move_left(cursor, buffer),
        Motion::Down => move_down(cursor, buffer),
        Motion::Up => move_up(cursor, buffer),
        Motion::Right => move_right(cursor, buffer),
        Motion::WordForward => word_forward(cursor, buffer),
        Motion::WordBackward => word_backward(cursor, buffer),
        Motion::WordEnd => word_end(cursor, buffer),
        Motion::BigWordForward => big_word_forward(cursor, buffer),
        Motion::BigWordBackward => big_word_backward(cursor, buffer),
        Motion::BigWordEnd => big_word_end(cursor, buffer),
        Motion::LineStart => line_start(cursor, buffer),
        Motion::LineFirstChar => line_first_char(cursor, buffer),
        Motion::LineEnd => line_end(cursor, buffer),
        Motion::GotoFirstLine => goto_first_line(cursor, buffer),
        Motion::GotoLastLine => goto_last_line(cursor, buffer),
        Motion::GotoLine(n) => goto_line(cursor, buffer, n),
        Motion::FindCharForward(ch) => find_char_forward(cursor, buffer, ch),
        Motion::FindCharBackward(ch) => find_char_backward(cursor, buffer, ch),
        Motion::TillCharForward(ch) => till_char_forward(cursor, buffer, ch),
        Motion::TillCharBackward(ch) => till_char_backward(cursor, buffer, ch),
        Motion::ParagraphForward => paragraph_forward(cursor, buffer),
        Motion::ParagraphBackward => paragraph_backward(cursor, buffer),
        Motion::MatchBracket => match_bracket(cursor, buffer),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_buffer() -> Buffer {
        Buffer::from_str("hello\nworld\nfoo bar\n\nend")
        // line 0: "hello"   (len 5)
        // line 1: "world"   (len 5)
        // line 2: "foo bar" (len 7)
        // line 3: ""        (len 0)
        // line 4: "end"     (len 3)
    }

    // -- Basic movement (h/j/k/l) --

    #[test]
    fn test_move_left_basic() {
        let buf = test_buffer();
        let cur = Cursor::new(0, 3);
        assert_eq!(move_left(&cur, &buf), Cursor::new(0, 2));
    }

    #[test]
    fn test_move_left_at_col_zero() {
        let buf = test_buffer();
        let cur = Cursor::new(1, 0);
        assert_eq!(move_left(&cur, &buf), Cursor::new(1, 0));
    }

    #[test]
    fn test_move_right_basic() {
        let buf = test_buffer();
        let cur = Cursor::new(0, 2);
        assert_eq!(move_right(&cur, &buf), Cursor::new(0, 3));
    }

    #[test]
    fn test_move_right_at_end() {
        let buf = test_buffer();
        let cur = Cursor::new(0, 4);
        assert_eq!(move_right(&cur, &buf), Cursor::new(0, 4));
    }

    #[test]
    fn test_move_down_basic() {
        let buf = test_buffer();
        let cur = Cursor::new(0, 2);
        assert_eq!(move_down(&cur, &buf), Cursor::new(1, 2));
    }

    #[test]
    fn test_move_down_clamps_col() {
        let buf = test_buffer();
        let cur = Cursor::new(2, 6);
        assert_eq!(move_down(&cur, &buf), Cursor::new(3, 0));
    }

    #[test]
    fn test_move_down_at_last_line() {
        let buf = test_buffer();
        let cur = Cursor::new(4, 1);
        assert_eq!(move_down(&cur, &buf), Cursor::new(4, 1));
    }

    #[test]
    fn test_move_up_basic() {
        let buf = test_buffer();
        let cur = Cursor::new(2, 3);
        assert_eq!(move_up(&cur, &buf), Cursor::new(1, 3));
    }

    #[test]
    fn test_move_up_clamps_col() {
        let buf = test_buffer();
        let cur = Cursor::new(2, 6);
        assert_eq!(move_up(&cur, &buf), Cursor::new(1, 4));
    }

    #[test]
    fn test_move_up_at_first_line() {
        let buf = test_buffer();
        let cur = Cursor::new(0, 2);
        assert_eq!(move_up(&cur, &buf), Cursor::new(0, 2));
    }

    #[test]
    fn test_move_through_empty_line() {
        let buf = test_buffer();
        let cur = Cursor::new(2, 4);
        let result = move_down(&cur, &buf);
        assert_eq!(result, Cursor::new(3, 0));
        let result = move_down(&result, &buf);
        assert_eq!(result, Cursor::new(4, 0));
    }

    // -- word_forward (w) --

    #[test]
    fn test_w_within_line() {
        let buf = Buffer::from_str("hello world foo");
        let cur = Cursor::new(0, 0);
        assert_eq!(word_forward(&cur, &buf), Cursor::new(0, 6));
    }

    #[test]
    fn test_w_punct_boundary() {
        let buf = Buffer::from_str("foo.bar baz");
        let cur = Cursor::new(0, 0);
        // "foo" is a word, "." is punct — w goes to "."
        assert_eq!(word_forward(&cur, &buf), Cursor::new(0, 3));
        // from ".", w goes to "bar"
        let cur2 = Cursor::new(0, 3);
        assert_eq!(word_forward(&cur2, &buf), Cursor::new(0, 4));
    }

    #[test]
    fn test_w_across_lines() {
        let buf = Buffer::from_str("hello\nworld");
        let cur = Cursor::new(0, 0);
        assert_eq!(word_forward(&cur, &buf), Cursor::new(1, 0));
    }

    #[test]
    fn test_w_stops_at_empty_line() {
        let buf = Buffer::from_str("hello\n\nworld");
        let cur = Cursor::new(0, 0);
        assert_eq!(word_forward(&cur, &buf), Cursor::new(1, 0));
    }

    #[test]
    fn test_w_from_empty_line() {
        let buf = Buffer::from_str("hello\n\nworld");
        let cur = Cursor::new(1, 0);
        assert_eq!(word_forward(&cur, &buf), Cursor::new(2, 0));
    }

    #[test]
    fn test_w_at_end_of_buffer() {
        let buf = Buffer::from_str("end");
        let cur = Cursor::new(0, 0);
        assert_eq!(word_forward(&cur, &buf), Cursor::new(0, 2));
    }

    // -- word_backward (b) --

    #[test]
    fn test_b_within_line() {
        let buf = Buffer::from_str("hello world foo");
        let cur = Cursor::new(0, 12);
        assert_eq!(word_backward(&cur, &buf), Cursor::new(0, 6));
    }

    #[test]
    fn test_b_across_lines() {
        let buf = Buffer::from_str("hello\nworld");
        let cur = Cursor::new(1, 0);
        assert_eq!(word_backward(&cur, &buf), Cursor::new(0, 0));
    }

    #[test]
    fn test_b_stops_at_empty_line() {
        let buf = Buffer::from_str("hello\n\nworld");
        let cur = Cursor::new(2, 0);
        assert_eq!(word_backward(&cur, &buf), Cursor::new(1, 0));
    }

    #[test]
    fn test_b_at_start_of_buffer() {
        let buf = Buffer::from_str("hello");
        let cur = Cursor::new(0, 0);
        assert_eq!(word_backward(&cur, &buf), Cursor::new(0, 0));
    }

    #[test]
    fn test_b_punct_boundary() {
        let buf = Buffer::from_str("foo.bar");
        let cur = Cursor::new(0, 4);
        // from 'b' in "bar", b goes to start of "bar" (col 4)... wait, col 4 IS 'b'
        // from col 4, move back to col 3 which is '.', then find start of '.' = col 3
        assert_eq!(word_backward(&cur, &buf), Cursor::new(0, 3));
    }

    // -- word_end (e) --

    #[test]
    fn test_e_within_line() {
        let buf = Buffer::from_str("hello world");
        let cur = Cursor::new(0, 0);
        assert_eq!(word_end(&cur, &buf), Cursor::new(0, 4));
    }

    #[test]
    fn test_e_at_end_of_word() {
        let buf = Buffer::from_str("hello world");
        let cur = Cursor::new(0, 4);
        assert_eq!(word_end(&cur, &buf), Cursor::new(0, 10));
    }

    #[test]
    fn test_e_across_lines() {
        let buf = Buffer::from_str("hi\nworld");
        let cur = Cursor::new(0, 1);
        assert_eq!(word_end(&cur, &buf), Cursor::new(1, 4));
    }

    // -- big word motions (W/B/E) --

    #[test]
    fn test_big_w_skips_punct() {
        let buf = Buffer::from_str("foo.bar baz");
        let cur = Cursor::new(0, 0);
        // W treats "foo.bar" as one WORD
        assert_eq!(big_word_forward(&cur, &buf), Cursor::new(0, 8));
    }

    #[test]
    fn test_big_b_skips_punct() {
        let buf = Buffer::from_str("foo.bar baz");
        let cur = Cursor::new(0, 8);
        assert_eq!(big_word_backward(&cur, &buf), Cursor::new(0, 0));
    }

    #[test]
    fn test_big_e_skips_punct() {
        let buf = Buffer::from_str("foo.bar baz");
        let cur = Cursor::new(0, 0);
        assert_eq!(big_word_end(&cur, &buf), Cursor::new(0, 6));
    }

    // -- line_start (0) --

    #[test]
    fn test_line_start() {
        let buf = Buffer::from_str("  hello");
        let cur = Cursor::new(0, 4);
        assert_eq!(line_start(&cur, &buf), Cursor::new(0, 0));
    }

    // -- line_first_char (^) --

    #[test]
    fn test_line_first_char() {
        let buf = Buffer::from_str("  hello");
        let cur = Cursor::new(0, 5);
        assert_eq!(line_first_char(&cur, &buf), Cursor::new(0, 2));
    }

    #[test]
    fn test_line_first_char_no_indent() {
        let buf = Buffer::from_str("hello");
        let cur = Cursor::new(0, 3);
        assert_eq!(line_first_char(&cur, &buf), Cursor::new(0, 0));
    }

    // -- line_end ($) --

    #[test]
    fn test_line_end() {
        let buf = Buffer::from_str("hello");
        let cur = Cursor::new(0, 0);
        assert_eq!(line_end(&cur, &buf), Cursor::new(0, 4));
    }

    #[test]
    fn test_line_end_empty() {
        let buf = Buffer::from_str("hello\n\nworld");
        let cur = Cursor::new(1, 0);
        assert_eq!(line_end(&cur, &buf), Cursor::new(1, 0));
    }

    // -- goto_first_line (gg) --

    #[test]
    fn test_goto_first_line() {
        let buf = Buffer::from_str("  hello\nworld");
        let cur = Cursor::new(1, 3);
        assert_eq!(goto_first_line(&cur, &buf), Cursor::new(0, 2));
    }

    // -- goto_last_line (G) --

    #[test]
    fn test_goto_last_line() {
        let buf = Buffer::from_str("hello\n  world");
        let cur = Cursor::new(0, 0);
        assert_eq!(goto_last_line(&cur, &buf), Cursor::new(1, 2));
    }

    // -- goto_line ({num}G) --

    #[test]
    fn test_goto_line() {
        let buf = Buffer::from_str("aaa\n  bbb\nccc");
        let cur = Cursor::new(0, 0);
        assert_eq!(goto_line(&cur, &buf, 2), Cursor::new(1, 2));
    }

    #[test]
    fn test_goto_line_clamped() {
        let buf = Buffer::from_str("aaa\nbbb");
        let cur = Cursor::new(0, 0);
        assert_eq!(goto_line(&cur, &buf, 100), Cursor::new(1, 0));
    }

    // -- find_char_forward (f) --

    #[test]
    fn test_find_char_forward_found() {
        let buf = Buffer::from_str("hello world");
        let cur = Cursor::new(0, 0);
        assert_eq!(find_char_forward(&cur, &buf, 'o'), Cursor::new(0, 4));
    }

    #[test]
    fn test_find_char_forward_not_found() {
        let buf = Buffer::from_str("hello");
        let cur = Cursor::new(0, 0);
        assert_eq!(find_char_forward(&cur, &buf, 'z'), Cursor::new(0, 0));
    }

    #[test]
    fn test_find_char_forward_second_occurrence() {
        let buf = Buffer::from_str("aabaa");
        let cur = Cursor::new(0, 0);
        // First f'a' from col 0 finds col 1
        let r1 = find_char_forward(&cur, &buf, 'a');
        assert_eq!(r1, Cursor::new(0, 1));
        // Next f'a' from col 1 finds col 3
        let r2 = find_char_forward(&r1, &buf, 'a');
        assert_eq!(r2, Cursor::new(0, 3));
    }

    // -- find_char_backward (F) --

    #[test]
    fn test_find_char_backward_found() {
        let buf = Buffer::from_str("hello world");
        let cur = Cursor::new(0, 10);
        assert_eq!(find_char_backward(&cur, &buf, 'o'), Cursor::new(0, 7));
    }

    #[test]
    fn test_find_char_backward_not_found() {
        let buf = Buffer::from_str("hello");
        let cur = Cursor::new(0, 4);
        assert_eq!(find_char_backward(&cur, &buf, 'z'), Cursor::new(0, 4));
    }

    // -- till_char_forward (t) --

    #[test]
    fn test_till_char_forward() {
        let buf = Buffer::from_str("hello world");
        let cur = Cursor::new(0, 0);
        assert_eq!(till_char_forward(&cur, &buf, 'o'), Cursor::new(0, 3));
    }

    #[test]
    fn test_till_char_forward_not_found() {
        let buf = Buffer::from_str("hello");
        let cur = Cursor::new(0, 0);
        assert_eq!(till_char_forward(&cur, &buf, 'z'), Cursor::new(0, 0));
    }

    // -- till_char_backward (T) --

    #[test]
    fn test_till_char_backward() {
        let buf = Buffer::from_str("hello world");
        let cur = Cursor::new(0, 10);
        assert_eq!(till_char_backward(&cur, &buf, 'o'), Cursor::new(0, 8));
    }

    // -- paragraph_forward (}) --

    #[test]
    fn test_paragraph_forward() {
        let buf = Buffer::from_str("foo\nbar\n\nbaz");
        let cur = Cursor::new(0, 0);
        assert_eq!(paragraph_forward(&cur, &buf), Cursor::new(2, 0));
    }

    #[test]
    fn test_paragraph_forward_from_blank() {
        let buf = Buffer::from_str("foo\n\nbaz\n\nqux");
        let cur = Cursor::new(1, 0);
        assert_eq!(paragraph_forward(&cur, &buf), Cursor::new(3, 0));
    }

    #[test]
    fn test_paragraph_forward_at_end() {
        let buf = Buffer::from_str("foo\nbar");
        let cur = Cursor::new(0, 0);
        assert_eq!(paragraph_forward(&cur, &buf), Cursor::new(1, 0));
    }

    // -- paragraph_backward ({) --

    #[test]
    fn test_paragraph_backward() {
        let buf = Buffer::from_str("foo\n\nbar\nbaz");
        let cur = Cursor::new(3, 0);
        assert_eq!(paragraph_backward(&cur, &buf), Cursor::new(1, 0));
    }

    #[test]
    fn test_paragraph_backward_from_blank() {
        let buf = Buffer::from_str("foo\n\n\nbar");
        let cur = Cursor::new(2, 0);
        assert_eq!(paragraph_backward(&cur, &buf), Cursor::new(0, 0));
    }

    #[test]
    fn test_paragraph_backward_at_start() {
        let buf = Buffer::from_str("hello");
        let cur = Cursor::new(0, 0);
        assert_eq!(paragraph_backward(&cur, &buf), Cursor::new(0, 0));
    }

    // -- match_bracket (%) --

    #[test]
    fn test_match_bracket_forward() {
        let buf = Buffer::from_str("(hello)");
        let cur = Cursor::new(0, 0);
        assert_eq!(match_bracket(&cur, &buf), Cursor::new(0, 6));
    }

    #[test]
    fn test_match_bracket_backward() {
        let buf = Buffer::from_str("(hello)");
        let cur = Cursor::new(0, 6);
        assert_eq!(match_bracket(&cur, &buf), Cursor::new(0, 0));
    }

    #[test]
    fn test_match_bracket_nested() {
        let buf = Buffer::from_str("((a)(b))");
        let cur = Cursor::new(0, 0);
        assert_eq!(match_bracket(&cur, &buf), Cursor::new(0, 7));
    }

    #[test]
    fn test_match_bracket_across_lines() {
        let buf = Buffer::from_str("if (x) {\n  y();\n}");
        let cur = Cursor::new(0, 7);
        assert_eq!(match_bracket(&cur, &buf), Cursor::new(2, 0));
    }

    #[test]
    fn test_match_bracket_scan_forward() {
        // Cursor not on bracket — scans forward to find first bracket
        let buf = Buffer::from_str("let x = (a + b);");
        let cur = Cursor::new(0, 0);
        assert_eq!(match_bracket(&cur, &buf), Cursor::new(0, 14));
    }

    #[test]
    fn test_match_bracket_no_bracket() {
        let buf = Buffer::from_str("hello");
        let cur = Cursor::new(0, 0);
        assert_eq!(match_bracket(&cur, &buf), Cursor::new(0, 0));
    }

    #[test]
    fn test_match_bracket_curly() {
        let buf = Buffer::from_str("{a{b}c}");
        let cur = Cursor::new(0, 0);
        assert_eq!(match_bracket(&cur, &buf), Cursor::new(0, 6));
        let cur2 = Cursor::new(0, 2);
        assert_eq!(match_bracket(&cur2, &buf), Cursor::new(0, 4));
    }

    #[test]
    fn test_match_bracket_square() {
        let buf = Buffer::from_str("[1, [2, 3]]");
        let cur = Cursor::new(0, 0);
        assert_eq!(match_bracket(&cur, &buf), Cursor::new(0, 10));
    }
}
