use super::buffer::Buffer;
use super::cursor::Cursor;
use super::mode::Mode;
use super::motions;

/// Actions that the Vim engine can execute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    MoveLeft,
    MoveDown,
    MoveUp,
    MoveRight,
    EnterInsertMode,
    EnterNormalMode,
    InsertChar(char),
    /// Delete the character before the cursor in insert mode.
    Backspace,
    /// No valid action for this input.
    None,
}

/// Parse a keystroke into an action, given the current mode.
pub fn parse_keystroke(ch: char, mode: Mode) -> Action {
    match mode {
        Mode::Normal => parse_normal(ch),
        Mode::Insert => parse_insert(ch),
    }
}

fn parse_normal(ch: char) -> Action {
    match ch {
        'h' => Action::MoveLeft,
        'j' => Action::MoveDown,
        'k' => Action::MoveUp,
        'l' => Action::MoveRight,
        'i' => Action::EnterInsertMode,
        _ => Action::None,
    }
}

fn parse_insert(ch: char) -> Action {
    match ch {
        '\x1b' => Action::EnterNormalMode, // Escape
        _ => Action::InsertChar(ch),
    }
}

/// Execute an action against the buffer, cursor, and mode. Returns the new state.
pub fn execute(action: Action, buffer: &mut Buffer, cursor: &mut Cursor, mode: &mut Mode) {
    match action {
        Action::MoveLeft => {
            *cursor = motions::move_left(cursor, buffer);
        }
        Action::MoveDown => {
            *cursor = motions::move_down(cursor, buffer);
        }
        Action::MoveUp => {
            *cursor = motions::move_up(cursor, buffer);
        }
        Action::MoveRight => {
            *cursor = motions::move_right(cursor, buffer);
        }
        Action::EnterInsertMode => {
            *mode = Mode::Insert;
        }
        Action::EnterNormalMode => {
            *mode = Mode::Normal;
            // When returning to normal mode, clamp cursor (can't be past last char)
            cursor.clamp(buffer, false);
        }
        Action::InsertChar(ch) => {
            if mode.is_insert() {
                if ch == '\n' {
                    // Split line at cursor position
                    buffer.insert_char(cursor.line, cursor.col, '\n');
                    cursor.line += 1;
                    cursor.col = 0;
                } else {
                    buffer.insert_char(cursor.line, cursor.col, ch);
                    cursor.col += 1;
                }
            }
        }
        Action::Backspace => {
            if mode.is_insert() {
                if cursor.col > 0 {
                    buffer.delete_chars(cursor.line, cursor.col - 1, 1);
                    cursor.col -= 1;
                } else if cursor.line > 0 {
                    // Join with previous line
                    let prev_len = buffer.line_len(cursor.line - 1);
                    // Delete the newline at end of previous line
                    buffer.delete_chars(cursor.line - 1, prev_len, 1);
                    cursor.line -= 1;
                    cursor.col = prev_len;
                }
            }
        }
        Action::None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_normal_hjkl() {
        assert_eq!(parse_keystroke('h', Mode::Normal), Action::MoveLeft);
        assert_eq!(parse_keystroke('j', Mode::Normal), Action::MoveDown);
        assert_eq!(parse_keystroke('k', Mode::Normal), Action::MoveUp);
        assert_eq!(parse_keystroke('l', Mode::Normal), Action::MoveRight);
    }

    #[test]
    fn test_parse_normal_unknown() {
        assert_eq!(parse_keystroke('z', Mode::Normal), Action::None);
    }

    #[test]
    fn test_parse_insert_char() {
        assert_eq!(parse_keystroke('a', Mode::Insert), Action::InsertChar('a'));
    }

    #[test]
    fn test_parse_insert_escape() {
        assert_eq!(parse_keystroke('\x1b', Mode::Insert), Action::EnterNormalMode);
    }

    #[test]
    fn test_execute_move() {
        let mut buf = Buffer::from_str("hello\nworld");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        execute(Action::MoveRight, &mut buf, &mut cur, &mut mode);
        assert_eq!(cur, Cursor::new(0, 1));

        execute(Action::MoveDown, &mut buf, &mut cur, &mut mode);
        assert_eq!(cur, Cursor::new(1, 1));
    }

    #[test]
    fn test_execute_enter_insert_and_type() {
        let mut buf = Buffer::from_str("hllo");
        let mut cur = Cursor::new(0, 1);
        let mut mode = Mode::Normal;

        execute(Action::EnterInsertMode, &mut buf, &mut cur, &mut mode);
        assert_eq!(mode, Mode::Insert);

        execute(Action::InsertChar('e'), &mut buf, &mut cur, &mut mode);
        assert_eq!(buf.line(0), Some("hello".to_string()));
        assert_eq!(cur.col, 2);
    }

    #[test]
    fn test_execute_escape_clamps_cursor() {
        let mut buf = Buffer::from_str("hello");
        let mut cur = Cursor::new(0, 5); // one past end (valid in insert mode)
        let mut mode = Mode::Insert;

        execute(Action::EnterNormalMode, &mut buf, &mut cur, &mut mode);
        assert_eq!(mode, Mode::Normal);
        assert_eq!(cur.col, 4); // clamped to last char
    }

    #[test]
    fn test_insert_ignored_in_normal_mode() {
        let mut buf = Buffer::from_str("hello");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        execute(Action::InsertChar('x'), &mut buf, &mut cur, &mut mode);
        assert_eq!(buf.line(0), Some("hello".to_string())); // unchanged
    }
}
