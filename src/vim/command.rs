use super::buffer::Buffer;
use super::cursor::Cursor;
use super::mode::Mode;
use super::motions;

/// Actions that the Vim engine can execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    // Basic movement
    MoveLeft,
    MoveDown,
    MoveUp,
    MoveRight,

    // Word motions
    WordForward,
    WordBackward,
    WordEnd,
    BigWordForward,
    BigWordBackward,
    BigWordEnd,

    // Line position
    LineStart,
    LineFirstChar,
    LineEnd,

    // Line jumping
    GotoFirstLine,
    GotoLastLine,
    GotoLine(usize),

    // Find character
    FindCharForward(char),
    FindCharBackward(char),
    TillCharForward(char),
    TillCharBackward(char),

    // Paragraph
    ParagraphForward,
    ParagraphBackward,

    // Bracket matching
    MatchBracket,

    // Mode changes
    EnterInsertMode,
    EnterNormalMode,

    // Insert mode actions
    InsertChar(char),
    Backspace,

    None,
}

// ===== Command Parser =====

#[derive(Debug, Clone, Copy, PartialEq)]
enum FindKind {
    Forward,
    Backward,
    TillForward,
    TillBackward,
}

impl FindKind {
    fn reverse(self) -> FindKind {
        match self {
            FindKind::Forward => FindKind::Backward,
            FindKind::Backward => FindKind::Forward,
            FindKind::TillForward => FindKind::TillBackward,
            FindKind::TillBackward => FindKind::TillForward,
        }
    }

    fn to_action(self, ch: char) -> Action {
        match self {
            FindKind::Forward => Action::FindCharForward(ch),
            FindKind::Backward => Action::FindCharBackward(ch),
            FindKind::TillForward => Action::TillCharForward(ch),
            FindKind::TillBackward => Action::TillCharBackward(ch),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FindState {
    kind: FindKind,
    ch: char,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ParseState {
    Ready,
    WaitingG,
    WaitingFind(FindKind),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParseResult {
    /// A complete action with a repeat count.
    Action(Action, usize),
    /// Waiting for more input (multi-key sequence).
    Pending,
    /// Unrecognized input.
    None,
}

/// Stateful parser for Normal mode keystrokes.
/// Handles count prefixes (5j), multi-key sequences (gg, fa), and repeat find (;/,).
pub struct CommandParser {
    state: ParseState,
    count: Option<usize>,
    last_find: Option<FindState>,
}

impl CommandParser {
    pub fn new() -> Self {
        Self {
            state: ParseState::Ready,
            count: None,
            last_find: None,
        }
    }

    /// Whether the parser is waiting for more input.
    pub fn is_pending(&self) -> bool {
        !matches!(self.state, ParseState::Ready) || self.count.is_some()
    }

    /// Cancel any pending multi-key sequence or count prefix.
    pub fn cancel(&mut self) {
        self.state = ParseState::Ready;
        self.count = None;
    }

    fn take_count(&mut self) -> Option<usize> {
        self.count.take()
    }

    fn action_with_count(&mut self, action: Action) -> ParseResult {
        let count = self.take_count().unwrap_or(1);
        ParseResult::Action(action, count)
    }

    fn action_no_count(&mut self, action: Action) -> ParseResult {
        self.count = None;
        ParseResult::Action(action, 1)
    }

    /// Feed a character from Normal mode. Returns the parse result.
    pub fn feed(&mut self, ch: char) -> ParseResult {
        match self.state {
            ParseState::WaitingG => self.feed_waiting_g(ch),
            ParseState::WaitingFind(kind) => self.feed_waiting_find(ch, kind),
            ParseState::Ready => self.feed_ready(ch),
        }
    }

    fn feed_waiting_g(&mut self, ch: char) -> ParseResult {
        self.state = ParseState::Ready;
        match ch {
            'g' => {
                let action = match self.take_count() {
                    Some(n) => Action::GotoLine(n),
                    None => Action::GotoFirstLine,
                };
                ParseResult::Action(action, 1)
            }
            _ => {
                self.count = None;
                ParseResult::None
            }
        }
    }

    fn feed_waiting_find(&mut self, ch: char, kind: FindKind) -> ParseResult {
        self.state = ParseState::Ready;
        self.last_find = Some(FindState { kind, ch });
        let count = self.take_count().unwrap_or(1);
        ParseResult::Action(kind.to_action(ch), count)
    }

    fn feed_ready(&mut self, ch: char) -> ParseResult {
        // Count prefix: digits accumulate, but '0' without a preceding count is LineStart
        if ch.is_ascii_digit() {
            if ch == '0' && self.count.is_none() {
                return ParseResult::Action(Action::LineStart, 1);
            }
            let digit = ch as usize - '0' as usize;
            self.count = Some(self.count.unwrap_or(0) * 10 + digit);
            return ParseResult::Pending;
        }

        match ch {
            // Basic movement
            'h' => self.action_with_count(Action::MoveLeft),
            'j' => self.action_with_count(Action::MoveDown),
            'k' => self.action_with_count(Action::MoveUp),
            'l' => self.action_with_count(Action::MoveRight),

            // Word motions
            'w' => self.action_with_count(Action::WordForward),
            'b' => self.action_with_count(Action::WordBackward),
            'e' => self.action_with_count(Action::WordEnd),
            'W' => self.action_with_count(Action::BigWordForward),
            'B' => self.action_with_count(Action::BigWordBackward),
            'E' => self.action_with_count(Action::BigWordEnd),

            // Line position
            '^' => self.action_no_count(Action::LineFirstChar),
            '$' => self.action_with_count(Action::LineEnd),

            // Line jumping
            'G' => {
                let action = match self.take_count() {
                    Some(n) => Action::GotoLine(n),
                    None => Action::GotoLastLine,
                };
                ParseResult::Action(action, 1)
            }
            'g' => {
                self.state = ParseState::WaitingG;
                ParseResult::Pending
            }

            // Find character
            'f' => {
                self.state = ParseState::WaitingFind(FindKind::Forward);
                ParseResult::Pending
            }
            't' => {
                self.state = ParseState::WaitingFind(FindKind::TillForward);
                ParseResult::Pending
            }
            'F' => {
                self.state = ParseState::WaitingFind(FindKind::Backward);
                ParseResult::Pending
            }
            'T' => {
                self.state = ParseState::WaitingFind(FindKind::TillBackward);
                ParseResult::Pending
            }

            // Repeat find
            ';' => {
                if let Some(find) = self.last_find {
                    self.action_with_count(find.kind.to_action(find.ch))
                } else {
                    self.count = None;
                    ParseResult::None
                }
            }
            ',' => {
                if let Some(find) = self.last_find {
                    self.action_with_count(find.kind.reverse().to_action(find.ch))
                } else {
                    self.count = None;
                    ParseResult::None
                }
            }

            // Paragraph
            '{' => self.action_with_count(Action::ParagraphBackward),
            '}' => self.action_with_count(Action::ParagraphForward),

            // Bracket matching
            '%' => self.action_no_count(Action::MatchBracket),

            // Mode changes
            'i' => self.action_no_count(Action::EnterInsertMode),

            _ => {
                self.count = None;
                ParseResult::None
            }
        }
    }
}

/// Execute an action against the buffer, cursor, and mode.
pub fn execute(action: Action, buffer: &mut Buffer, cursor: &mut Cursor, mode: &mut Mode) {
    match action {
        // Basic movement
        Action::MoveLeft => *cursor = motions::move_left(cursor, buffer),
        Action::MoveDown => *cursor = motions::move_down(cursor, buffer),
        Action::MoveUp => *cursor = motions::move_up(cursor, buffer),
        Action::MoveRight => *cursor = motions::move_right(cursor, buffer),

        // Word motions
        Action::WordForward => *cursor = motions::word_forward(cursor, buffer),
        Action::WordBackward => *cursor = motions::word_backward(cursor, buffer),
        Action::WordEnd => *cursor = motions::word_end(cursor, buffer),
        Action::BigWordForward => *cursor = motions::big_word_forward(cursor, buffer),
        Action::BigWordBackward => *cursor = motions::big_word_backward(cursor, buffer),
        Action::BigWordEnd => *cursor = motions::big_word_end(cursor, buffer),

        // Line position
        Action::LineStart => *cursor = motions::line_start(cursor, buffer),
        Action::LineFirstChar => *cursor = motions::line_first_char(cursor, buffer),
        Action::LineEnd => *cursor = motions::line_end(cursor, buffer),

        // Line jumping
        Action::GotoFirstLine => *cursor = motions::goto_first_line(cursor, buffer),
        Action::GotoLastLine => *cursor = motions::goto_last_line(cursor, buffer),
        Action::GotoLine(n) => *cursor = motions::goto_line(cursor, buffer, n),

        // Find character
        Action::FindCharForward(ch) => {
            *cursor = motions::find_char_forward(cursor, buffer, ch)
        }
        Action::FindCharBackward(ch) => {
            *cursor = motions::find_char_backward(cursor, buffer, ch)
        }
        Action::TillCharForward(ch) => {
            *cursor = motions::till_char_forward(cursor, buffer, ch)
        }
        Action::TillCharBackward(ch) => {
            *cursor = motions::till_char_backward(cursor, buffer, ch)
        }

        // Paragraph
        Action::ParagraphForward => *cursor = motions::paragraph_forward(cursor, buffer),
        Action::ParagraphBackward => *cursor = motions::paragraph_backward(cursor, buffer),

        // Bracket matching
        Action::MatchBracket => *cursor = motions::match_bracket(cursor, buffer),

        // Mode changes
        Action::EnterInsertMode => {
            *mode = Mode::Insert;
        }
        Action::EnterNormalMode => {
            *mode = Mode::Normal;
            cursor.clamp(buffer, false);
        }

        // Insert mode actions
        Action::InsertChar(ch) => {
            if mode.is_insert() {
                if ch == '\n' {
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
                    let prev_len = buffer.line_len(cursor.line - 1);
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

    // -- CommandParser tests --

    fn parse_sequence(input: &str) -> ParseResult {
        let mut parser = CommandParser::new();
        let mut result = ParseResult::None;
        for ch in input.chars() {
            result = parser.feed(ch);
        }
        result
    }

    #[test]
    fn test_parser_single_keys() {
        assert_eq!(parse_sequence("h"), ParseResult::Action(Action::MoveLeft, 1));
        assert_eq!(parse_sequence("j"), ParseResult::Action(Action::MoveDown, 1));
        assert_eq!(parse_sequence("k"), ParseResult::Action(Action::MoveUp, 1));
        assert_eq!(parse_sequence("l"), ParseResult::Action(Action::MoveRight, 1));
        assert_eq!(parse_sequence("w"), ParseResult::Action(Action::WordForward, 1));
        assert_eq!(parse_sequence("b"), ParseResult::Action(Action::WordBackward, 1));
        assert_eq!(parse_sequence("e"), ParseResult::Action(Action::WordEnd, 1));
        assert_eq!(parse_sequence("$"), ParseResult::Action(Action::LineEnd, 1));
        assert_eq!(parse_sequence("^"), ParseResult::Action(Action::LineFirstChar, 1));
        assert_eq!(parse_sequence("0"), ParseResult::Action(Action::LineStart, 1));
        assert_eq!(parse_sequence("%"), ParseResult::Action(Action::MatchBracket, 1));
        assert_eq!(parse_sequence("{"), ParseResult::Action(Action::ParagraphBackward, 1));
        assert_eq!(parse_sequence("}"), ParseResult::Action(Action::ParagraphForward, 1));
        assert_eq!(parse_sequence("i"), ParseResult::Action(Action::EnterInsertMode, 1));
    }

    #[test]
    fn test_parser_count_prefix() {
        assert_eq!(parse_sequence("5j"), ParseResult::Action(Action::MoveDown, 5));
        assert_eq!(parse_sequence("12w"), ParseResult::Action(Action::WordForward, 12));
        assert_eq!(parse_sequence("3$"), ParseResult::Action(Action::LineEnd, 3));
    }

    #[test]
    fn test_parser_zero_is_line_start() {
        assert_eq!(parse_sequence("0"), ParseResult::Action(Action::LineStart, 1));
    }

    #[test]
    fn test_parser_count_with_zero() {
        // 10j = move down 10 times
        assert_eq!(parse_sequence("10j"), ParseResult::Action(Action::MoveDown, 10));
        // 20w = word forward 20 times
        assert_eq!(parse_sequence("20w"), ParseResult::Action(Action::WordForward, 20));
    }

    #[test]
    fn test_parser_gg() {
        assert_eq!(parse_sequence("gg"), ParseResult::Action(Action::GotoFirstLine, 1));
    }

    #[test]
    fn test_parser_num_gg() {
        assert_eq!(parse_sequence("5gg"), ParseResult::Action(Action::GotoLine(5), 1));
    }

    #[test]
    fn test_parser_big_g() {
        assert_eq!(parse_sequence("G"), ParseResult::Action(Action::GotoLastLine, 1));
    }

    #[test]
    fn test_parser_num_g() {
        assert_eq!(parse_sequence("5G"), ParseResult::Action(Action::GotoLine(5), 1));
    }

    #[test]
    fn test_parser_find_char() {
        assert_eq!(
            parse_sequence("fa"),
            ParseResult::Action(Action::FindCharForward('a'), 1)
        );
        assert_eq!(
            parse_sequence("Fa"),
            ParseResult::Action(Action::FindCharBackward('a'), 1)
        );
        assert_eq!(
            parse_sequence("ta"),
            ParseResult::Action(Action::TillCharForward('a'), 1)
        );
        assert_eq!(
            parse_sequence("Ta"),
            ParseResult::Action(Action::TillCharBackward('a'), 1)
        );
    }

    #[test]
    fn test_parser_find_with_count() {
        assert_eq!(
            parse_sequence("3fa"),
            ParseResult::Action(Action::FindCharForward('a'), 3)
        );
    }

    #[test]
    fn test_parser_repeat_find() {
        let mut parser = CommandParser::new();
        parser.feed('f');
        parser.feed('a'); // sets last_find
        let result = parser.feed(';');
        assert_eq!(result, ParseResult::Action(Action::FindCharForward('a'), 1));
    }

    #[test]
    fn test_parser_repeat_find_reverse() {
        let mut parser = CommandParser::new();
        parser.feed('f');
        parser.feed('a');
        let result = parser.feed(',');
        assert_eq!(
            result,
            ParseResult::Action(Action::FindCharBackward('a'), 1)
        );
    }

    #[test]
    fn test_parser_semicolon_without_find() {
        assert_eq!(parse_sequence(";"), ParseResult::None);
    }

    #[test]
    fn test_parser_pending_state() {
        let mut parser = CommandParser::new();
        assert!(!parser.is_pending());
        assert_eq!(parser.feed('f'), ParseResult::Pending);
        assert!(parser.is_pending());
        assert_eq!(
            parser.feed('a'),
            ParseResult::Action(Action::FindCharForward('a'), 1)
        );
        assert!(!parser.is_pending());
    }

    #[test]
    fn test_parser_cancel() {
        let mut parser = CommandParser::new();
        parser.feed('5'); // start count
        assert!(parser.is_pending());
        parser.cancel();
        assert!(!parser.is_pending());
        // Next input starts fresh
        assert_eq!(parser.feed('j'), ParseResult::Action(Action::MoveDown, 1));
    }

    #[test]
    fn test_parser_unknown_key() {
        assert_eq!(parse_sequence("z"), ParseResult::None);
    }

    #[test]
    fn test_parser_g_then_unknown() {
        assert_eq!(parse_sequence("gz"), ParseResult::None);
    }

    #[test]
    fn test_parser_big_word_motions() {
        assert_eq!(parse_sequence("W"), ParseResult::Action(Action::BigWordForward, 1));
        assert_eq!(parse_sequence("B"), ParseResult::Action(Action::BigWordBackward, 1));
        assert_eq!(parse_sequence("E"), ParseResult::Action(Action::BigWordEnd, 1));
    }

    // -- Execute tests --

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
    fn test_execute_word_forward() {
        let mut buf = Buffer::from_str("hello world");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        execute(Action::WordForward, &mut buf, &mut cur, &mut mode);
        assert_eq!(cur, Cursor::new(0, 6));
    }

    #[test]
    fn test_execute_goto_line() {
        let mut buf = Buffer::from_str("aaa\nbbb\nccc");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        execute(Action::GotoLine(2), &mut buf, &mut cur, &mut mode);
        assert_eq!(cur.line, 1);
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
        let mut cur = Cursor::new(0, 5);
        let mut mode = Mode::Insert;

        execute(Action::EnterNormalMode, &mut buf, &mut cur, &mut mode);
        assert_eq!(mode, Mode::Normal);
        assert_eq!(cur.col, 4);
    }

    #[test]
    fn test_insert_ignored_in_normal_mode() {
        let mut buf = Buffer::from_str("hello");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        execute(Action::InsertChar('x'), &mut buf, &mut cur, &mut mode);
        assert_eq!(buf.line(0), Some("hello".to_string()));
    }

    #[test]
    fn test_execute_find_char() {
        let mut buf = Buffer::from_str("hello world");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        execute(Action::FindCharForward('w'), &mut buf, &mut cur, &mut mode);
        assert_eq!(cur, Cursor::new(0, 6));
    }

    #[test]
    fn test_execute_match_bracket() {
        let mut buf = Buffer::from_str("(abc)");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        execute(Action::MatchBracket, &mut buf, &mut cur, &mut mode);
        assert_eq!(cur, Cursor::new(0, 4));
    }
}
