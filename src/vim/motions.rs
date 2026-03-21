use super::buffer::Buffer;
use super::cursor::Cursor;

/// Move cursor left by one character. Stops at column 0.
pub fn move_left(cursor: &Cursor, _buffer: &Buffer) -> Cursor {
    Cursor::new(cursor.line, cursor.col.saturating_sub(1))
}

/// Move cursor down by one line. Clamps column to the new line's length.
pub fn move_down(cursor: &Cursor, buffer: &Buffer) -> Cursor {
    let max_line = buffer.line_count().saturating_sub(1);
    let new_line = (cursor.line + 1).min(max_line);
    let new_col = cursor.col.min(buffer.line_len(new_line).saturating_sub(1));
    Cursor::new(new_line, new_col)
}

/// Move cursor up by one line. Clamps column to the new line's length.
pub fn move_up(cursor: &Cursor, buffer: &Buffer) -> Cursor {
    let new_line = cursor.line.saturating_sub(1);
    let new_col = cursor.col.min(buffer.line_len(new_line).saturating_sub(1));
    Cursor::new(new_line, new_col)
}

/// Move cursor right by one character. Stops at last character on the line.
pub fn move_right(cursor: &Cursor, buffer: &Buffer) -> Cursor {
    let max_col = buffer.line_len(cursor.line).saturating_sub(1);
    let new_col = (cursor.col + 1).min(max_col);
    Cursor::new(cursor.line, new_col)
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

    // -- move_left --

    #[test]
    fn test_move_left_basic() {
        let buf = test_buffer();
        let cur = Cursor::new(0, 3);
        let result = move_left(&cur, &buf);
        assert_eq!(result, Cursor::new(0, 2));
    }

    #[test]
    fn test_move_left_at_col_zero() {
        let buf = test_buffer();
        let cur = Cursor::new(1, 0);
        let result = move_left(&cur, &buf);
        assert_eq!(result, Cursor::new(1, 0)); // stays at 0
    }

    // -- move_right --

    #[test]
    fn test_move_right_basic() {
        let buf = test_buffer();
        let cur = Cursor::new(0, 2);
        let result = move_right(&cur, &buf);
        assert_eq!(result, Cursor::new(0, 3));
    }

    #[test]
    fn test_move_right_at_end_of_line() {
        let buf = test_buffer();
        let cur = Cursor::new(0, 4); // 'o' in "hello"
        let result = move_right(&cur, &buf);
        assert_eq!(result, Cursor::new(0, 4)); // stays at last char
    }

    // -- move_down --

    #[test]
    fn test_move_down_basic() {
        let buf = test_buffer();
        let cur = Cursor::new(0, 2);
        let result = move_down(&cur, &buf);
        assert_eq!(result, Cursor::new(1, 2));
    }

    #[test]
    fn test_move_down_clamps_col() {
        let buf = test_buffer();
        let cur = Cursor::new(2, 6); // col 6 in "foo bar"
        let result = move_down(&cur, &buf);
        // line 3 is empty, col clamps to 0
        assert_eq!(result, Cursor::new(3, 0));
    }

    #[test]
    fn test_move_down_at_last_line() {
        let buf = test_buffer();
        let cur = Cursor::new(4, 1);
        let result = move_down(&cur, &buf);
        assert_eq!(result, Cursor::new(4, 1)); // stays on last line
    }

    // -- move_up --

    #[test]
    fn test_move_up_basic() {
        let buf = test_buffer();
        let cur = Cursor::new(2, 3);
        let result = move_up(&cur, &buf);
        assert_eq!(result, Cursor::new(1, 3));
    }

    #[test]
    fn test_move_up_clamps_col() {
        let buf = test_buffer();
        let cur = Cursor::new(2, 6); // col 6 in "foo bar"
        let result = move_up(&cur, &buf);
        // line 1 "world" has max col 4
        assert_eq!(result, Cursor::new(1, 4));
    }

    #[test]
    fn test_move_up_at_first_line() {
        let buf = test_buffer();
        let cur = Cursor::new(0, 2);
        let result = move_up(&cur, &buf);
        assert_eq!(result, Cursor::new(0, 2)); // stays on line 0
    }

    // -- empty line behavior --

    #[test]
    fn test_move_through_empty_line() {
        let buf = test_buffer();
        // Move down from line 2 onto empty line 3
        let cur = Cursor::new(2, 4);
        let result = move_down(&cur, &buf);
        assert_eq!(result, Cursor::new(3, 0));
        // Move down again to line 4 — col stays 0 since empty line had col 0
        let result = move_down(&result, &buf);
        assert_eq!(result, Cursor::new(4, 0));
    }
}
