use super::buffer::Buffer;
use super::cursor::Cursor;
use super::mode::Mode;
use super::motions::{self, Motion};
use super::register::{RegisterContent, RegisterFile};
use super::text_objects::TextObject;

/// Operators that combine with motions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Delete,
    Change,
    Yank,
}

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

    // Page scrolling (handled by app — needs viewport height)
    ScrollHalfDown,   // Ctrl-d
    ScrollHalfUp,     // Ctrl-u
    ScrollFullDown,   // Ctrl-f
    ScrollFullUp,     // Ctrl-b

    // Mode changes
    EnterInsertMode,   // i
    EnterNormalMode,
    InsertAfter,       // a
    InsertAtStart,     // I
    InsertAtEnd,       // A
    OpenLineBelow,     // o
    OpenLineAbove,     // O

    // Simple editing
    DeleteChar,        // x
    DeleteCharBefore,  // X
    DeleteLine,        // dd
    DeleteToEnd,       // D
    YankLine,          // yy
    PasteAfter,        // p
    PasteBefore,       // P
    ChangeToEnd,       // C

    // Operator + motion (d{motion}, c{motion}, y{motion})
    // The usize is the motion repeat count (e.g., d3w = Delete, WordForward, 3)
    OperatorMotion(Operator, Motion, usize),

    // Operator + text object (diw, ca", yi(, etc.)
    OperatorTextObject(Operator, TextObject),

    // Visual mode
    EnterVisualMode,       // v
    EnterVisualLineMode,   // V

    // Replace
    ReplaceChar(char),   // r{char}
    EnterReplaceMode,    // R
    ReplaceOverwrite(char), // char typed in Replace mode

    // Command line
    EnterCmdLine,        // : — open command line (handled by app)

    // Search
    SearchForward,       // / — start search input (handled by app)
    SearchBackward,      // ? — start search input (handled by app)
    SearchNext,          // n — repeat last search
    SearchPrev,          // N — repeat last search in opposite direction
    SearchWordForward,   // * — search word under cursor forward
    SearchWordBackward,  // # — search word under cursor backward

    // Undo/Redo
    Undo,
    Redo,

    // Dot repeat
    DotRepeat,           // . — repeat last edit (handled by app)

    // Macros (handled by app)
    MacroRecord(char),   // q{reg} — start recording into register
    MacroStop,           // q — stop recording
    MacroPlay(char),     // @{reg} — play macro from register

    // Insert mode actions
    InsertChar(char),
    Backspace,

    None,
}

impl Action {
    /// Whether this action modifies the buffer (for undo tracking).
    pub fn is_edit(&self) -> bool {
        matches!(
            self,
            Action::DeleteChar
                | Action::DeleteCharBefore
                | Action::DeleteLine
                | Action::DeleteToEnd
                | Action::YankLine
                | Action::PasteAfter
                | Action::PasteBefore
                | Action::ChangeToEnd
                | Action::OperatorMotion(_, _, _)
                | Action::OperatorTextObject(_, _)
                | Action::ReplaceChar(_)
                | Action::ReplaceOverwrite(_)
                | Action::Undo
                | Action::Redo
                | Action::InsertChar(_)
                | Action::Backspace
                | Action::OpenLineBelow
                | Action::OpenLineAbove
        )
    }
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
    WaitingOperator(Operator),          // after d/c/y, waiting for motion or self-repeat (dd/cc/yy)
    WaitingOperatorFind(Operator, FindKind), // after d/c/y then f/t/F/T, waiting for char
    WaitingOperatorG(Operator),         // after d/c/y then g, waiting for g (dgg)
    WaitingOperatorTextObj(Operator, bool), // after d/c/y then i/a, waiting for object char (inner=true)
    WaitingReplace,                     // after 'r', waiting for replacement char
    WaitingMacroReg,                    // after 'q', waiting for register char
    WaitingMacroPlay,                   // after '@', waiting for register char
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
/// Handles count prefixes (5j), multi-key sequences (gg, fa, dd, yy), and repeat find (;/,).
pub struct CommandParser {
    state: ParseState,
    count: Option<usize>,
    last_find: Option<FindState>,
    /// Whether a macro is currently being recorded (for q toggle behavior).
    pub recording_macro: bool,
}

impl CommandParser {
    pub fn new() -> Self {
        Self {
            state: ParseState::Ready,
            count: None,
            last_find: None,
            recording_macro: false,
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
            ParseState::WaitingOperator(op) => self.feed_waiting_operator(ch, op),
            ParseState::WaitingOperatorFind(op, kind) => self.feed_waiting_operator_find(ch, op, kind),
            ParseState::WaitingOperatorG(op) => self.feed_waiting_operator_g(ch, op),
            ParseState::WaitingOperatorTextObj(op, inner) => self.feed_waiting_operator_textobj(ch, op, inner),
            ParseState::WaitingReplace => self.feed_waiting_replace(ch),
            ParseState::WaitingMacroReg => self.feed_waiting_macro_reg(ch),
            ParseState::WaitingMacroPlay => self.feed_waiting_macro_play(ch),
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

    fn feed_waiting_operator(&mut self, ch: char, op: Operator) -> ParseResult {
        // Self-repeat: dd, cc, yy — operate on current line(s)
        let self_char = match op {
            Operator::Delete => 'd',
            Operator::Change => 'c',
            Operator::Yank => 'y',
        };
        if ch == self_char {
            self.state = ParseState::Ready;
            return match op {
                Operator::Delete => self.action_with_count(Action::DeleteLine),
                Operator::Change => {
                    // cc: delete line content (keep newline), enter insert at first non-blank
                    // For simplicity, treat as delete-line then open
                    self.action_with_count(Action::DeleteLine) // TODO: proper cc
                }
                Operator::Yank => self.action_with_count(Action::YankLine),
            };
        }

        // Count after operator: d3w means motion_count=3
        if ch.is_ascii_digit() && !(ch == '0' && self.count.is_none()) {
            let digit = ch as usize - '0' as usize;
            self.count = Some(self.count.unwrap_or(0) * 10 + digit);
            return ParseResult::Pending;
        }

        // Try to parse as a motion
        self.state = ParseState::Ready;
        if let Some(motion) = self.char_to_motion(ch) {
            let count = self.take_count().unwrap_or(1);
            // Embed count in the action, return 1 to avoid app loop repeating
            return ParseResult::Action(Action::OperatorMotion(op, motion, count), 1);
        }

        // Multi-key motions: f/t/F/T need another char
        match ch {
            'f' => {
                self.state = ParseState::WaitingOperatorFind(op, FindKind::Forward);
                return ParseResult::Pending;
            }
            't' => {
                self.state = ParseState::WaitingOperatorFind(op, FindKind::TillForward);
                return ParseResult::Pending;
            }
            'F' => {
                self.state = ParseState::WaitingOperatorFind(op, FindKind::Backward);
                return ParseResult::Pending;
            }
            'T' => {
                self.state = ParseState::WaitingOperatorFind(op, FindKind::TillBackward);
                return ParseResult::Pending;
            }
            'g' => {
                self.state = ParseState::WaitingOperatorG(op);
                return ParseResult::Pending;
            }
            'i' => {
                self.state = ParseState::WaitingOperatorTextObj(op, true);
                return ParseResult::Pending;
            }
            'a' => {
                self.state = ParseState::WaitingOperatorTextObj(op, false);
                return ParseResult::Pending;
            }
            _ => {}
        }

        self.count = None;
        ParseResult::None
    }

    fn feed_waiting_operator_textobj(&mut self, ch: char, op: Operator, inner: bool) -> ParseResult {
        self.state = ParseState::Ready;
        if let Some(obj) = TextObject::from_char(ch, inner) {
            self.count = None;
            return ParseResult::Action(Action::OperatorTextObject(op, obj), 1);
        }
        self.count = None;
        ParseResult::None
    }

    fn feed_waiting_operator_find(&mut self, ch: char, op: Operator, kind: FindKind) -> ParseResult {
        self.state = ParseState::Ready;
        self.last_find = Some(FindState { kind, ch });
        let motion = match kind {
            FindKind::Forward => Motion::FindCharForward(ch),
            FindKind::Backward => Motion::FindCharBackward(ch),
            FindKind::TillForward => Motion::TillCharForward(ch),
            FindKind::TillBackward => Motion::TillCharBackward(ch),
        };
        let count = self.take_count().unwrap_or(1);
        ParseResult::Action(Action::OperatorMotion(op, motion, count), 1)
    }

    fn feed_waiting_operator_g(&mut self, ch: char, op: Operator) -> ParseResult {
        self.state = ParseState::Ready;
        match ch {
            'g' => {
                let motion = match self.take_count() {
                    Some(n) => Motion::GotoLine(n),
                    None => Motion::GotoFirstLine,
                };
                ParseResult::Action(Action::OperatorMotion(op, motion, 1), 1)
            }
            _ => {
                self.count = None;
                ParseResult::None
            }
        }
    }

    fn feed_waiting_macro_reg(&mut self, ch: char) -> ParseResult {
        self.state = ParseState::Ready;
        if ch.is_ascii_lowercase() {
            self.recording_macro = true;
            self.count = None;
            return ParseResult::Action(Action::MacroRecord(ch), 1);
        }
        self.count = None;
        ParseResult::None
    }

    fn feed_waiting_macro_play(&mut self, ch: char) -> ParseResult {
        self.state = ParseState::Ready;
        if ch.is_ascii_lowercase() || ch == '@' {
            // @@ replays last played macro — handled by app
            let reg = if ch == '@' { '\0' } else { ch };
            return self.action_with_count(Action::MacroPlay(reg));
        }
        self.count = None;
        ParseResult::None
    }

    fn feed_waiting_replace(&mut self, ch: char) -> ParseResult {
        self.state = ParseState::Ready;
        let count = self.take_count().unwrap_or(1);
        ParseResult::Action(Action::ReplaceChar(ch), count)
    }

    /// Map a single char to a Motion (for operator+motion combos).
    fn char_to_motion(&self, ch: char) -> Option<Motion> {
        match ch {
            'h' => Some(Motion::Left),
            'j' => Some(Motion::Down),
            'k' => Some(Motion::Up),
            'l' => Some(Motion::Right),
            'w' => Some(Motion::WordForward),
            'b' => Some(Motion::WordBackward),
            'e' => Some(Motion::WordEnd),
            'W' => Some(Motion::BigWordForward),
            'B' => Some(Motion::BigWordBackward),
            'E' => Some(Motion::BigWordEnd),
            '0' => Some(Motion::LineStart),
            '^' => Some(Motion::LineFirstChar),
            '$' => Some(Motion::LineEnd),
            'G' => Some(Motion::GotoLastLine),
            '{' => Some(Motion::ParagraphBackward),
            '}' => Some(Motion::ParagraphForward),
            '%' => Some(Motion::MatchBracket),
            _ => None,
        }
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

            // Mode changes — insert
            'i' => self.action_no_count(Action::EnterInsertMode),
            'a' => self.action_no_count(Action::InsertAfter),
            'I' => self.action_no_count(Action::InsertAtStart),
            'A' => self.action_no_count(Action::InsertAtEnd),
            'o' => self.action_no_count(Action::OpenLineBelow),
            'O' => self.action_no_count(Action::OpenLineAbove),

            // Simple editing
            'x' => self.action_with_count(Action::DeleteChar),
            'X' => self.action_with_count(Action::DeleteCharBefore),
            'D' => self.action_no_count(Action::DeleteToEnd),
            'C' => self.action_no_count(Action::ChangeToEnd),
            'p' => self.action_with_count(Action::PasteAfter),
            'P' => self.action_with_count(Action::PasteBefore),

            // Visual mode
            'v' => self.action_no_count(Action::EnterVisualMode),
            'V' => self.action_no_count(Action::EnterVisualLineMode),

            // Replace
            'r' => {
                self.state = ParseState::WaitingReplace;
                ParseResult::Pending
            }
            'R' => self.action_no_count(Action::EnterReplaceMode),

            // Command line
            ':' => self.action_no_count(Action::EnterCmdLine),

            // Dot repeat
            '.' => self.action_with_count(Action::DotRepeat),

            // Search
            '/' => self.action_no_count(Action::SearchForward),
            '?' => self.action_no_count(Action::SearchBackward),
            'n' => self.action_with_count(Action::SearchNext),
            'N' => self.action_with_count(Action::SearchPrev),
            '*' => self.action_no_count(Action::SearchWordForward),
            '#' => self.action_no_count(Action::SearchWordBackward),

            // Macros
            'q' => {
                if self.recording_macro {
                    self.recording_macro = false;
                    self.count = None;
                    ParseResult::Action(Action::MacroStop, 1)
                } else {
                    self.state = ParseState::WaitingMacroReg;
                    ParseResult::Pending
                }
            }
            '@' => {
                self.state = ParseState::WaitingMacroPlay;
                ParseResult::Pending
            }

            // Undo
            'u' => self.action_no_count(Action::Undo),

            // Operators (multi-key: d/c/y + motion)
            'd' => {
                self.state = ParseState::WaitingOperator(Operator::Delete);
                ParseResult::Pending
            }
            'c' => {
                self.state = ParseState::WaitingOperator(Operator::Change);
                ParseResult::Pending
            }
            'y' => {
                self.state = ParseState::WaitingOperator(Operator::Yank);
                ParseResult::Pending
            }

            _ => {
                self.count = None;
                ParseResult::None
            }
        }
    }
}

/// Execute an action against the buffer, cursor, mode, and registers.
pub fn execute(
    action: Action,
    buffer: &mut Buffer,
    cursor: &mut Cursor,
    mode: &mut Mode,
    registers: &mut RegisterFile,
) {
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

        // Visual mode entry (anchor set by app.rs)
        Action::EnterVisualMode => {
            *mode = Mode::Visual;
        }
        Action::EnterVisualLineMode => {
            *mode = Mode::VisualLine;
        }

        // Mode changes — basic insert
        Action::EnterInsertMode => {
            *mode = Mode::Insert;
        }
        Action::EnterNormalMode => {
            *mode = Mode::Normal;
            cursor.clamp(buffer, false);
        }

        // Insert mode entry variants
        Action::InsertAfter => {
            // 'a': move cursor right one, then insert mode
            let line_len = buffer.line_len(cursor.line);
            if line_len > 0 {
                cursor.col = (cursor.col + 1).min(line_len);
            }
            *mode = Mode::Insert;
        }
        Action::InsertAtStart => {
            // 'I': move to first non-blank, then insert mode
            *cursor = motions::line_first_char(cursor, buffer);
            *mode = Mode::Insert;
        }
        Action::InsertAtEnd => {
            // 'A': move to end of line, then insert mode
            cursor.col = buffer.line_len(cursor.line);
            *mode = Mode::Insert;
        }
        Action::OpenLineBelow => {
            // 'o': open new line below, enter insert mode
            let line_len = buffer.line_len(cursor.line);
            buffer.insert_char(cursor.line, line_len, '\n');
            cursor.line += 1;
            cursor.col = 0;
            *mode = Mode::Insert;
        }
        Action::OpenLineAbove => {
            // 'O': open new line above, enter insert mode
            buffer.insert_char(cursor.line, 0, '\n');
            cursor.col = 0;
            *mode = Mode::Insert;
        }

        // Simple editing
        Action::DeleteChar => {
            // 'x': delete char under cursor
            let line_len = buffer.line_len(cursor.line);
            if line_len > 0 && cursor.col < line_len {
                let ch = buffer.char_at(cursor.line, cursor.col).unwrap_or(' ');
                buffer.delete_chars(cursor.line, cursor.col, 1);
                registers.delete(None, RegisterContent::Charwise(ch.to_string()));
                // Clamp cursor if we deleted the last char
                let new_len = buffer.line_len(cursor.line);
                if new_len > 0 && cursor.col >= new_len {
                    cursor.col = new_len - 1;
                }
            }
        }
        Action::DeleteCharBefore => {
            // 'X': delete char before cursor
            if cursor.col > 0 {
                let ch = buffer.char_at(cursor.line, cursor.col - 1).unwrap_or(' ');
                buffer.delete_chars(cursor.line, cursor.col - 1, 1);
                cursor.col -= 1;
                registers.delete(None, RegisterContent::Charwise(ch.to_string()));
            }
        }
        Action::DeleteLine => {
            // 'dd': delete entire line
            let text = buffer.delete_lines(cursor.line, cursor.line);
            // Ensure text ends with newline for linewise register
            let reg_text = if text.ends_with('\n') {
                text
            } else {
                format!("{}\n", text)
            };
            registers.delete(None, RegisterContent::Linewise(reg_text));
            // Clamp cursor
            cursor.clamp(buffer, false);
        }
        Action::DeleteToEnd => {
            // 'D': delete from cursor to end of line
            let line_len = buffer.line_len(cursor.line);
            if cursor.col < line_len {
                let text = buffer.delete_range(cursor.line, cursor.col, cursor.line, line_len);
                registers.delete(None, RegisterContent::Charwise(text));
                cursor.clamp(buffer, false);
            }
        }
        Action::ChangeToEnd => {
            // 'C': delete from cursor to end of line, enter insert mode
            let line_len = buffer.line_len(cursor.line);
            if cursor.col < line_len {
                let text = buffer.delete_range(cursor.line, cursor.col, cursor.line, line_len);
                registers.delete(None, RegisterContent::Charwise(text));
            }
            *mode = Mode::Insert;
        }
        Action::YankLine => {
            // 'yy': yank entire line
            if let Some(line) = buffer.line(cursor.line) {
                registers.yank(None, RegisterContent::Linewise(format!("{}\n", line)));
            }
        }
        Action::PasteAfter => {
            // 'p': paste after cursor/line
            let content = registers.get(None).clone();
            match content {
                RegisterContent::Linewise(text) => {
                    // Paste as new line below
                    let line_len = buffer.line_len(cursor.line);
                    buffer.insert_char(cursor.line, line_len, '\n');
                    let paste_text = text.trim_end_matches('\n');
                    buffer.insert_str(cursor.line + 1, 0, paste_text);
                    cursor.line += 1;
                    cursor.col = 0;
                    // Move to first non-blank
                    *cursor = motions::line_first_char(cursor, buffer);
                }
                RegisterContent::Charwise(text) => {
                    if !text.is_empty() {
                        // Paste after cursor
                        let insert_col = (cursor.col + 1).min(buffer.line_len(cursor.line));
                        buffer.insert_str(cursor.line, insert_col, &text);
                        cursor.col = insert_col + text.len() - 1;
                    }
                }
            }
        }
        Action::PasteBefore => {
            // 'P': paste before cursor/line
            let content = registers.get(None).clone();
            match content {
                RegisterContent::Linewise(text) => {
                    // Paste as new line above
                    buffer.insert_char(cursor.line, 0, '\n');
                    let paste_text = text.trim_end_matches('\n');
                    buffer.insert_str(cursor.line, 0, paste_text);
                    cursor.col = 0;
                    *cursor = motions::line_first_char(cursor, buffer);
                }
                RegisterContent::Charwise(text) => {
                    if !text.is_empty() {
                        buffer.insert_str(cursor.line, cursor.col, &text);
                        cursor.col += text.len() - 1;
                    }
                }
            }
        }

        // Replace
        Action::ReplaceChar(ch) => {
            let line_len = buffer.line_len(cursor.line);
            if line_len > 0 && cursor.col < line_len {
                buffer.delete_chars(cursor.line, cursor.col, 1);
                buffer.insert_char(cursor.line, cursor.col, ch);
                // Cursor stays on the replaced char
            }
        }
        Action::EnterReplaceMode => {
            *mode = Mode::Replace;
        }
        Action::ReplaceOverwrite(ch) => {
            if mode.is_replace() {
                let line_len = buffer.line_len(cursor.line);
                if ch == '\n' {
                    // In replace mode, Enter still inserts a newline
                    buffer.insert_char(cursor.line, cursor.col, '\n');
                    cursor.line += 1;
                    cursor.col = 0;
                } else if cursor.col < line_len {
                    // Overwrite existing char
                    buffer.delete_chars(cursor.line, cursor.col, 1);
                    buffer.insert_char(cursor.line, cursor.col, ch);
                    cursor.col += 1;
                } else {
                    // Past end of line: insert like normal
                    buffer.insert_char(cursor.line, cursor.col, ch);
                    cursor.col += 1;
                }
            }
        }

        // Handled by app.rs (need viewport/undo/search access)
        Action::Undo | Action::Redo => {}
        Action::ScrollHalfDown | Action::ScrollHalfUp
        | Action::ScrollFullDown | Action::ScrollFullUp => {}
        Action::SearchForward | Action::SearchBackward
        | Action::SearchNext | Action::SearchPrev
        | Action::SearchWordForward | Action::SearchWordBackward => {}
        Action::EnterCmdLine => {} // handled by app.rs
        Action::DotRepeat => {} // handled by app.rs
        Action::MacroRecord(_) | Action::MacroStop | Action::MacroPlay(_) => {} // handled by app.rs

        // Operator + motion
        Action::OperatorMotion(op, motion, count) => {
            // Apply the motion `count` times to find the final position
            let mut motion_result = *cursor;
            for _ in 0..count {
                motion_result = motions::apply_motion(motion, &motion_result, buffer);
            }
            let linewise = motion.is_linewise();

            if linewise {
                // Linewise: operate on complete lines from cursor.line to motion_result.line
                let start_line = cursor.line.min(motion_result.line);
                let end_line = cursor.line.max(motion_result.line);

                match op {
                    Operator::Delete => {
                        let text = buffer.delete_lines(start_line, end_line);
                        let reg_text = if text.ends_with('\n') { text } else { format!("{}\n", text) };
                        registers.delete(None, RegisterContent::Linewise(reg_text));
                        cursor.line = start_line.min(buffer.line_count().saturating_sub(1));
                        cursor.clamp(buffer, false);
                    }
                    Operator::Change => {
                        let text = buffer.delete_lines(start_line, end_line);
                        let reg_text = if text.ends_with('\n') { text } else { format!("{}\n", text) };
                        registers.delete(None, RegisterContent::Linewise(reg_text));
                        // Insert a blank line and enter insert mode
                        if buffer.is_empty() || buffer.line_count() == 1 && buffer.line_len(0) == 0 {
                            cursor.line = 0;
                        } else if start_line >= buffer.line_count() {
                            let last = buffer.line_count().saturating_sub(1);
                            let ll = buffer.line_len(last);
                            buffer.insert_char(last, ll, '\n');
                            cursor.line = last + 1;
                        } else {
                            buffer.insert_char(start_line, 0, '\n');
                            cursor.line = start_line;
                        }
                        cursor.col = 0;
                        *mode = Mode::Insert;
                    }
                    Operator::Yank => {
                        // Yank lines without modifying buffer
                        let mut text = String::new();
                        for line_idx in start_line..=end_line {
                            if let Some(line) = buffer.line(line_idx) {
                                text.push_str(&line);
                                text.push('\n');
                            }
                        }
                        registers.yank(None, RegisterContent::Linewise(text));
                    }
                }
            } else {
                // Charwise: operate from cursor to motion result
                let (start, end) = if (cursor.line, cursor.col) <= (motion_result.line, motion_result.col) {
                    (*cursor, motion_result)
                } else {
                    (motion_result, *cursor)
                };

                // For forward-exclusive motions (w, W, e, E), the end is inclusive of the char
                // For $ the end is end of line (exclusive already works)
                // Determine the exclusive end position
                let end_exclusive = match motion {
                    // Inclusive motions: include the end character
                    Motion::WordEnd | Motion::BigWordEnd
                    | Motion::FindCharForward(_) | Motion::FindCharBackward(_)
                    | Motion::TillCharForward(_) | Motion::TillCharBackward(_)
                    | Motion::LineEnd | Motion::MatchBracket => {
                        // One past the end character
                        if end.col + 1 <= buffer.line_len(end.line) {
                            Cursor::new(end.line, end.col + 1)
                        } else if end.line + 1 < buffer.line_count() {
                            Cursor::new(end.line + 1, 0)
                        } else {
                            Cursor::new(end.line, buffer.line_len(end.line))
                        }
                    }
                    // Exclusive motions: end position is already exclusive
                    _ => end,
                };

                let text = buffer.text_range(
                    start.line,
                    start.col,
                    end_exclusive.line,
                    end_exclusive.col,
                );

                match op {
                    Operator::Delete => {
                        buffer.delete_range(
                            start.line,
                            start.col,
                            end_exclusive.line,
                            end_exclusive.col,
                        );
                        registers.delete(None, RegisterContent::Charwise(text));
                        *cursor = start;
                        cursor.clamp(buffer, false);
                    }
                    Operator::Change => {
                        buffer.delete_range(
                            start.line,
                            start.col,
                            end_exclusive.line,
                            end_exclusive.col,
                        );
                        registers.delete(None, RegisterContent::Charwise(text));
                        *cursor = start;
                        *mode = Mode::Insert;
                    }
                    Operator::Yank => {
                        registers.yank(None, RegisterContent::Charwise(text));
                        // Cursor stays at start of yanked region
                        *cursor = start;
                    }
                }
            }
        }

        // Operator + text object
        Action::OperatorTextObject(op, obj) => {
            if let Some(range) = obj.resolve(cursor, buffer) {
                let text = buffer.text_range(
                    range.start_line,
                    range.start_col,
                    range.end_line,
                    range.end_col,
                );

                match op {
                    Operator::Delete => {
                        buffer.delete_range(
                            range.start_line,
                            range.start_col,
                            range.end_line,
                            range.end_col,
                        );
                        registers.delete(None, RegisterContent::Charwise(text));
                        cursor.line = range.start_line;
                        cursor.col = range.start_col;
                        cursor.clamp(buffer, false);
                    }
                    Operator::Change => {
                        buffer.delete_range(
                            range.start_line,
                            range.start_col,
                            range.end_line,
                            range.end_col,
                        );
                        registers.delete(None, RegisterContent::Charwise(text));
                        cursor.line = range.start_line;
                        cursor.col = range.start_col;
                        *mode = Mode::Insert;
                    }
                    Operator::Yank => {
                        registers.yank(None, RegisterContent::Charwise(text));
                        cursor.line = range.start_line;
                        cursor.col = range.start_col;
                    }
                }
            }
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
        assert_eq!(parse_sequence("10j"), ParseResult::Action(Action::MoveDown, 10));
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
        parser.feed('a');
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
        parser.feed('5');
        assert!(parser.is_pending());
        parser.cancel();
        assert!(!parser.is_pending());
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

    // -- Parser: simple edit keys --

    #[test]
    fn test_parser_simple_edits() {
        assert_eq!(parse_sequence("x"), ParseResult::Action(Action::DeleteChar, 1));
        assert_eq!(parse_sequence("X"), ParseResult::Action(Action::DeleteCharBefore, 1));
        assert_eq!(parse_sequence("D"), ParseResult::Action(Action::DeleteToEnd, 1));
        assert_eq!(parse_sequence("C"), ParseResult::Action(Action::ChangeToEnd, 1));
        assert_eq!(parse_sequence("p"), ParseResult::Action(Action::PasteAfter, 1));
        assert_eq!(parse_sequence("P"), ParseResult::Action(Action::PasteBefore, 1));
    }

    #[test]
    fn test_parser_dd() {
        assert_eq!(parse_sequence("d"), ParseResult::Pending);
        assert_eq!(parse_sequence("dd"), ParseResult::Action(Action::DeleteLine, 1));
        assert_eq!(parse_sequence("3dd"), ParseResult::Action(Action::DeleteLine, 3));
    }

    #[test]
    fn test_parser_yy() {
        assert_eq!(parse_sequence("y"), ParseResult::Pending);
        assert_eq!(parse_sequence("yy"), ParseResult::Action(Action::YankLine, 1));
    }

    #[test]
    fn test_parser_insert_modes() {
        assert_eq!(parse_sequence("a"), ParseResult::Action(Action::InsertAfter, 1));
        assert_eq!(parse_sequence("A"), ParseResult::Action(Action::InsertAtEnd, 1));
        assert_eq!(parse_sequence("I"), ParseResult::Action(Action::InsertAtStart, 1));
        assert_eq!(parse_sequence("o"), ParseResult::Action(Action::OpenLineBelow, 1));
        assert_eq!(parse_sequence("O"), ParseResult::Action(Action::OpenLineAbove, 1));
    }

    #[test]
    fn test_parser_x_with_count() {
        assert_eq!(parse_sequence("3x"), ParseResult::Action(Action::DeleteChar, 3));
    }

    // -- Execute tests --

    fn exec(action: Action, buf: &mut Buffer, cur: &mut Cursor, mode: &mut Mode) {
        let mut regs = RegisterFile::new();
        execute(action, buf, cur, mode, &mut regs);
    }

    fn exec_with_regs(
        action: Action,
        buf: &mut Buffer,
        cur: &mut Cursor,
        mode: &mut Mode,
        regs: &mut RegisterFile,
    ) {
        execute(action, buf, cur, mode, regs);
    }

    #[test]
    fn test_execute_move() {
        let mut buf = Buffer::from_str("hello\nworld");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        exec(Action::MoveRight, &mut buf, &mut cur, &mut mode);
        assert_eq!(cur, Cursor::new(0, 1));

        exec(Action::MoveDown, &mut buf, &mut cur, &mut mode);
        assert_eq!(cur, Cursor::new(1, 1));
    }

    #[test]
    fn test_execute_word_forward() {
        let mut buf = Buffer::from_str("hello world");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        exec(Action::WordForward, &mut buf, &mut cur, &mut mode);
        assert_eq!(cur, Cursor::new(0, 6));
    }

    #[test]
    fn test_execute_goto_line() {
        let mut buf = Buffer::from_str("aaa\nbbb\nccc");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        exec(Action::GotoLine(2), &mut buf, &mut cur, &mut mode);
        assert_eq!(cur.line, 1);
    }

    #[test]
    fn test_execute_enter_insert_and_type() {
        let mut buf = Buffer::from_str("hllo");
        let mut cur = Cursor::new(0, 1);
        let mut mode = Mode::Normal;

        exec(Action::EnterInsertMode, &mut buf, &mut cur, &mut mode);
        assert_eq!(mode, Mode::Insert);

        exec(Action::InsertChar('e'), &mut buf, &mut cur, &mut mode);
        assert_eq!(buf.line(0), Some("hello".to_string()));
        assert_eq!(cur.col, 2);
    }

    #[test]
    fn test_execute_escape_clamps_cursor() {
        let mut buf = Buffer::from_str("hello");
        let mut cur = Cursor::new(0, 5);
        let mut mode = Mode::Insert;

        exec(Action::EnterNormalMode, &mut buf, &mut cur, &mut mode);
        assert_eq!(mode, Mode::Normal);
        assert_eq!(cur.col, 4);
    }

    #[test]
    fn test_insert_ignored_in_normal_mode() {
        let mut buf = Buffer::from_str("hello");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        exec(Action::InsertChar('x'), &mut buf, &mut cur, &mut mode);
        assert_eq!(buf.line(0), Some("hello".to_string()));
    }

    #[test]
    fn test_execute_find_char() {
        let mut buf = Buffer::from_str("hello world");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        exec(Action::FindCharForward('w'), &mut buf, &mut cur, &mut mode);
        assert_eq!(cur, Cursor::new(0, 6));
    }

    #[test]
    fn test_execute_match_bracket() {
        let mut buf = Buffer::from_str("(abc)");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        exec(Action::MatchBracket, &mut buf, &mut cur, &mut mode);
        assert_eq!(cur, Cursor::new(0, 4));
    }

    // -- Execute: simple edit tests --

    #[test]
    fn test_execute_x_deletes_char() {
        let mut buf = Buffer::from_str("hello");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(Action::DeleteChar, &mut buf, &mut cur, &mut mode, &mut regs);
        assert_eq!(buf.line(0), Some("ello".to_string()));
        assert_eq!(regs.get(None).text(), "h");
    }

    #[test]
    fn test_execute_x_at_end_of_line() {
        let mut buf = Buffer::from_str("ab");
        let mut cur = Cursor::new(0, 1);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(Action::DeleteChar, &mut buf, &mut cur, &mut mode, &mut regs);
        assert_eq!(buf.line(0), Some("a".to_string()));
        assert_eq!(cur.col, 0); // clamp back
    }

    #[test]
    fn test_execute_big_x_deletes_before() {
        let mut buf = Buffer::from_str("hello");
        let mut cur = Cursor::new(0, 2);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(Action::DeleteCharBefore, &mut buf, &mut cur, &mut mode, &mut regs);
        assert_eq!(buf.line(0), Some("hllo".to_string()));
        assert_eq!(cur.col, 1);
        assert_eq!(regs.get(None).text(), "e");
    }

    #[test]
    fn test_execute_big_x_at_col_zero() {
        let mut buf = Buffer::from_str("hello");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        exec(Action::DeleteCharBefore, &mut buf, &mut cur, &mut mode);
        assert_eq!(buf.line(0), Some("hello".to_string())); // no change
    }

    #[test]
    fn test_execute_dd() {
        let mut buf = Buffer::from_str("aaa\nbbb\nccc");
        let mut cur = Cursor::new(1, 0);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(Action::DeleteLine, &mut buf, &mut cur, &mut mode, &mut regs);
        assert_eq!(buf.line_count(), 2);
        assert_eq!(buf.line(0), Some("aaa".to_string()));
        assert_eq!(buf.line(1), Some("ccc".to_string()));
        assert!(regs.get(None).is_linewise());
        assert_eq!(regs.get(None).text(), "bbb\n");
    }

    #[test]
    fn test_execute_delete_to_end() {
        let mut buf = Buffer::from_str("hello world");
        let mut cur = Cursor::new(0, 5);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(Action::DeleteToEnd, &mut buf, &mut cur, &mut mode, &mut regs);
        assert_eq!(buf.line(0), Some("hello".to_string()));
        assert_eq!(regs.get(None).text(), " world");
    }

    #[test]
    fn test_execute_change_to_end() {
        let mut buf = Buffer::from_str("hello world");
        let mut cur = Cursor::new(0, 5);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(Action::ChangeToEnd, &mut buf, &mut cur, &mut mode, &mut regs);
        assert_eq!(buf.line(0), Some("hello".to_string()));
        assert_eq!(mode, Mode::Insert);
        assert_eq!(regs.get(None).text(), " world");
    }

    #[test]
    fn test_execute_yy() {
        let mut buf = Buffer::from_str("hello\nworld");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(Action::YankLine, &mut buf, &mut cur, &mut mode, &mut regs);
        assert_eq!(regs.get(None).text(), "hello\n");
        assert!(regs.get(None).is_linewise());
        // Buffer unchanged
        assert_eq!(buf.line(0), Some("hello".to_string()));
    }

    #[test]
    fn test_execute_paste_after_linewise() {
        let mut buf = Buffer::from_str("aaa\nccc");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        // Yank line
        regs.yank(None, RegisterContent::Linewise("bbb\n".into()));

        exec_with_regs(Action::PasteAfter, &mut buf, &mut cur, &mut mode, &mut regs);
        assert_eq!(buf.line(0), Some("aaa".to_string()));
        assert_eq!(buf.line(1), Some("bbb".to_string()));
        assert_eq!(buf.line(2), Some("ccc".to_string()));
        assert_eq!(cur.line, 1);
    }

    #[test]
    fn test_execute_paste_before_linewise() {
        let mut buf = Buffer::from_str("aaa\nccc");
        let mut cur = Cursor::new(1, 0);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        regs.yank(None, RegisterContent::Linewise("bbb\n".into()));

        exec_with_regs(Action::PasteBefore, &mut buf, &mut cur, &mut mode, &mut regs);
        assert_eq!(buf.line(0), Some("aaa".to_string()));
        assert_eq!(buf.line(1), Some("bbb".to_string()));
        assert_eq!(buf.line(2), Some("ccc".to_string()));
        assert_eq!(cur.line, 1);
    }

    #[test]
    fn test_execute_paste_after_charwise() {
        let mut buf = Buffer::from_str("hllo");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        regs.delete(None, RegisterContent::Charwise("e".into()));

        exec_with_regs(Action::PasteAfter, &mut buf, &mut cur, &mut mode, &mut regs);
        assert_eq!(buf.line(0), Some("hello".to_string()));
        assert_eq!(cur.col, 1);
    }

    #[test]
    fn test_execute_paste_before_charwise() {
        let mut buf = Buffer::from_str("hllo");
        let mut cur = Cursor::new(0, 1);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        regs.delete(None, RegisterContent::Charwise("e".into()));

        exec_with_regs(Action::PasteBefore, &mut buf, &mut cur, &mut mode, &mut regs);
        assert_eq!(buf.line(0), Some("hello".to_string()));
    }

    #[test]
    fn test_execute_dd_then_p() {
        let mut buf = Buffer::from_str("aaa\nbbb\nccc");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        // dd on first line
        exec_with_regs(Action::DeleteLine, &mut buf, &mut cur, &mut mode, &mut regs);
        assert_eq!(buf.line(0), Some("bbb".to_string()));

        // p to paste below
        exec_with_regs(Action::PasteAfter, &mut buf, &mut cur, &mut mode, &mut regs);
        assert_eq!(buf.line(0), Some("bbb".to_string()));
        assert_eq!(buf.line(1), Some("aaa".to_string()));
        assert_eq!(buf.line(2), Some("ccc".to_string()));
    }

    // -- Insert mode variants --

    #[test]
    fn test_execute_insert_after() {
        let mut buf = Buffer::from_str("hello");
        let mut cur = Cursor::new(0, 2);
        let mut mode = Mode::Normal;

        exec(Action::InsertAfter, &mut buf, &mut cur, &mut mode);
        assert_eq!(mode, Mode::Insert);
        assert_eq!(cur.col, 3); // one past 'l'
    }

    #[test]
    fn test_execute_insert_at_start() {
        let mut buf = Buffer::from_str("    hello");
        let mut cur = Cursor::new(0, 7);
        let mut mode = Mode::Normal;

        exec(Action::InsertAtStart, &mut buf, &mut cur, &mut mode);
        assert_eq!(mode, Mode::Insert);
        assert_eq!(cur.col, 4); // first non-blank
    }

    #[test]
    fn test_execute_insert_at_end() {
        let mut buf = Buffer::from_str("hello");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        exec(Action::InsertAtEnd, &mut buf, &mut cur, &mut mode);
        assert_eq!(mode, Mode::Insert);
        assert_eq!(cur.col, 5); // past last char
    }

    #[test]
    fn test_execute_open_line_below() {
        let mut buf = Buffer::from_str("hello\nworld");
        let mut cur = Cursor::new(0, 3);
        let mut mode = Mode::Normal;

        exec(Action::OpenLineBelow, &mut buf, &mut cur, &mut mode);
        assert_eq!(mode, Mode::Insert);
        assert_eq!(cur.line, 1);
        assert_eq!(cur.col, 0);
        assert_eq!(buf.line_count(), 3);
        assert_eq!(buf.line(0), Some("hello".to_string()));
        assert_eq!(buf.line(1), Some("".to_string()));
        assert_eq!(buf.line(2), Some("world".to_string()));
    }

    #[test]
    fn test_execute_open_line_above() {
        let mut buf = Buffer::from_str("hello\nworld");
        let mut cur = Cursor::new(1, 0);
        let mut mode = Mode::Normal;

        exec(Action::OpenLineAbove, &mut buf, &mut cur, &mut mode);
        assert_eq!(mode, Mode::Insert);
        assert_eq!(cur.line, 1);
        assert_eq!(cur.col, 0);
        assert_eq!(buf.line_count(), 3);
        assert_eq!(buf.line(0), Some("hello".to_string()));
        assert_eq!(buf.line(1), Some("".to_string()));
        assert_eq!(buf.line(2), Some("world".to_string()));
    }

    // -- is_edit --

    #[test]
    fn test_is_edit() {
        assert!(Action::DeleteChar.is_edit());
        assert!(Action::DeleteLine.is_edit());
        assert!(Action::InsertChar('a').is_edit());
        assert!(Action::OperatorMotion(Operator::Delete, Motion::WordForward, 1).is_edit());
        assert!(!Action::MoveDown.is_edit());
        assert!(!Action::EnterInsertMode.is_edit());
    }

    // -- Parser: operator+motion --

    #[test]
    fn test_parser_dw() {
        assert_eq!(
            parse_sequence("dw"),
            ParseResult::Action(Action::OperatorMotion(Operator::Delete, Motion::WordForward, 1), 1)
        );
    }

    #[test]
    fn test_parser_d_dollar() {
        assert_eq!(
            parse_sequence("d$"),
            ParseResult::Action(Action::OperatorMotion(Operator::Delete, Motion::LineEnd, 1), 1)
        );
    }

    #[test]
    fn test_parser_d0() {
        assert_eq!(
            parse_sequence("d0"),
            ParseResult::Action(Action::OperatorMotion(Operator::Delete, Motion::LineStart, 1), 1)
        );
    }

    #[test]
    fn test_parser_cw() {
        assert_eq!(
            parse_sequence("cw"),
            ParseResult::Action(Action::OperatorMotion(Operator::Change, Motion::WordForward, 1), 1)
        );
    }

    #[test]
    fn test_parser_yw() {
        assert_eq!(
            parse_sequence("yw"),
            ParseResult::Action(Action::OperatorMotion(Operator::Yank, Motion::WordForward, 1), 1)
        );
    }

    #[test]
    fn test_parser_d3w() {
        assert_eq!(
            parse_sequence("d3w"),
            ParseResult::Action(Action::OperatorMotion(Operator::Delete, Motion::WordForward, 3), 1)
        );
    }

    #[test]
    fn test_parser_3dw() {
        // 3dw: count=3 before d, then w has count=1. Total motion count = 3*1 = 3
        // But our parser: count=3 is consumed by 'dd' path? No — 3d goes to WaitingOperator(Delete)
        // with count=3 still set. Then 'w' motion takes that count.
        assert_eq!(
            parse_sequence("3dw"),
            ParseResult::Action(Action::OperatorMotion(Operator::Delete, Motion::WordForward, 3), 1)
        );
    }

    #[test]
    fn test_parser_dfa() {
        assert_eq!(
            parse_sequence("dfa"),
            ParseResult::Action(
                Action::OperatorMotion(Operator::Delete, Motion::FindCharForward('a'), 1),
                1
            )
        );
    }

    #[test]
    fn test_parser_dgg() {
        assert_eq!(
            parse_sequence("dgg"),
            ParseResult::Action(
                Action::OperatorMotion(Operator::Delete, Motion::GotoFirstLine, 1),
                1
            )
        );
    }

    #[test]
    fn test_parser_dj() {
        assert_eq!(
            parse_sequence("dj"),
            ParseResult::Action(
                Action::OperatorMotion(Operator::Delete, Motion::Down, 1),
                1
            )
        );
    }

    #[test]
    fn test_parser_replace_char() {
        assert_eq!(parse_sequence("r"), ParseResult::Pending);
        assert_eq!(parse_sequence("rx"), ParseResult::Action(Action::ReplaceChar('x'), 1));
        assert_eq!(parse_sequence("3rx"), ParseResult::Action(Action::ReplaceChar('x'), 3));
    }

    #[test]
    fn test_parser_replace_mode() {
        assert_eq!(parse_sequence("R"), ParseResult::Action(Action::EnterReplaceMode, 1));
    }

    #[test]
    fn test_parser_undo() {
        assert_eq!(parse_sequence("u"), ParseResult::Action(Action::Undo, 1));
    }

    #[test]
    fn test_parser_cc() {
        assert_eq!(parse_sequence("cc"), ParseResult::Action(Action::DeleteLine, 1));
    }

    // -- Execute: operator+motion --

    #[test]
    fn test_execute_dw() {
        let mut buf = Buffer::from_str("hello world");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(
            Action::OperatorMotion(Operator::Delete, Motion::WordForward, 1),
            &mut buf, &mut cur, &mut mode, &mut regs,
        );
        assert_eq!(buf.line(0), Some("world".to_string()));
        assert_eq!(regs.get(None).text(), "hello ");
        assert_eq!(cur.col, 0);
    }

    #[test]
    fn test_execute_d_dollar() {
        let mut buf = Buffer::from_str("hello world");
        let mut cur = Cursor::new(0, 5);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(
            Action::OperatorMotion(Operator::Delete, Motion::LineEnd, 1),
            &mut buf, &mut cur, &mut mode, &mut regs,
        );
        assert_eq!(buf.line(0), Some("hello".to_string()));
    }

    #[test]
    fn test_execute_d0() {
        let mut buf = Buffer::from_str("hello world");
        let mut cur = Cursor::new(0, 6);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(
            Action::OperatorMotion(Operator::Delete, Motion::LineStart, 1),
            &mut buf, &mut cur, &mut mode, &mut regs,
        );
        assert_eq!(buf.line(0), Some("world".to_string()));
        assert_eq!(cur.col, 0);
    }

    #[test]
    fn test_execute_db() {
        let mut buf = Buffer::from_str("hello world");
        let mut cur = Cursor::new(0, 6);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(
            Action::OperatorMotion(Operator::Delete, Motion::WordBackward, 1),
            &mut buf, &mut cur, &mut mode, &mut regs,
        );
        // db from 'w' of "world" deletes backward to col 0: removes "hello "
        assert_eq!(buf.line(0), Some("world".to_string()));
        assert_eq!(cur.col, 0);
    }

    #[test]
    fn test_execute_dj_linewise() {
        let mut buf = Buffer::from_str("aaa\nbbb\nccc");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(
            Action::OperatorMotion(Operator::Delete, Motion::Down, 1),
            &mut buf, &mut cur, &mut mode, &mut regs,
        );
        assert_eq!(buf.line_count(), 1);
        assert_eq!(buf.line(0), Some("ccc".to_string()));
        assert!(regs.get(None).is_linewise());
    }

    #[test]
    fn test_execute_cw() {
        let mut buf = Buffer::from_str("hello world");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(
            Action::OperatorMotion(Operator::Change, Motion::WordForward, 1),
            &mut buf, &mut cur, &mut mode, &mut regs,
        );
        assert_eq!(buf.line(0), Some("world".to_string()));
        assert_eq!(mode, Mode::Insert);
        assert_eq!(cur.col, 0);
    }

    #[test]
    fn test_execute_yw() {
        let mut buf = Buffer::from_str("hello world");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(
            Action::OperatorMotion(Operator::Yank, Motion::WordForward, 1),
            &mut buf, &mut cur, &mut mode, &mut regs,
        );
        // Buffer unchanged
        assert_eq!(buf.line(0), Some("hello world".to_string()));
        assert_eq!(regs.get(None).text(), "hello ");
    }

    #[test]
    fn test_execute_d3w() {
        let mut buf = Buffer::from_str("one two three four");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(
            Action::OperatorMotion(Operator::Delete, Motion::WordForward, 3),
            &mut buf, &mut cur, &mut mode, &mut regs,
        );
        assert_eq!(buf.line(0), Some("four".to_string()));
        assert_eq!(regs.get(None).text(), "one two three ");
    }

    #[test]
    fn test_execute_yj_linewise() {
        let mut buf = Buffer::from_str("aaa\nbbb\nccc");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(
            Action::OperatorMotion(Operator::Yank, Motion::Down, 1),
            &mut buf, &mut cur, &mut mode, &mut regs,
        );
        // Buffer unchanged
        assert_eq!(buf.line_count(), 3);
        assert!(regs.get(None).is_linewise());
        assert_eq!(regs.get(None).text(), "aaa\nbbb\n");
    }

    // -- Execute: replace --

    #[test]
    fn test_execute_replace_char() {
        let mut buf = Buffer::from_str("hello");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        exec(Action::ReplaceChar('H'), &mut buf, &mut cur, &mut mode);
        assert_eq!(buf.line(0), Some("Hello".to_string()));
        assert_eq!(cur.col, 0); // stays on replaced char
        assert_eq!(mode, Mode::Normal); // stays in normal mode
    }

    #[test]
    fn test_execute_enter_replace_mode() {
        let mut buf = Buffer::from_str("hello");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        exec(Action::EnterReplaceMode, &mut buf, &mut cur, &mut mode);
        assert_eq!(mode, Mode::Replace);
    }

    #[test]
    fn test_execute_replace_overwrite() {
        let mut buf = Buffer::from_str("hello");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Replace;

        exec(Action::ReplaceOverwrite('H'), &mut buf, &mut cur, &mut mode);
        assert_eq!(buf.line(0), Some("Hello".to_string()));
        assert_eq!(cur.col, 1); // advances cursor

        exec(Action::ReplaceOverwrite('E'), &mut buf, &mut cur, &mut mode);
        assert_eq!(buf.line(0), Some("HEllo".to_string()));
        assert_eq!(cur.col, 2);
    }

    // -- Text object parser tests --

    #[test]
    fn test_parser_diw() {

        assert_eq!(parse_sequence("d"), ParseResult::Pending);
        let mut parser = CommandParser::new();
        assert_eq!(parser.feed('d'), ParseResult::Pending);
        assert_eq!(parser.feed('i'), ParseResult::Pending);
        assert_eq!(
            parser.feed('w'),
            ParseResult::Action(Action::OperatorTextObject(Operator::Delete, TextObject::InnerWord), 1)
        );
    }

    #[test]
    fn test_parser_ci_double_quote() {

        let mut parser = CommandParser::new();
        assert_eq!(parser.feed('c'), ParseResult::Pending);
        assert_eq!(parser.feed('i'), ParseResult::Pending);
        assert_eq!(
            parser.feed('"'),
            ParseResult::Action(Action::OperatorTextObject(Operator::Change, TextObject::InnerQuote('"')), 1)
        );
    }

    #[test]
    fn test_parser_ya_paren() {

        let mut parser = CommandParser::new();
        assert_eq!(parser.feed('y'), ParseResult::Pending);
        assert_eq!(parser.feed('a'), ParseResult::Pending);
        assert_eq!(
            parser.feed('('),
            ParseResult::Action(Action::OperatorTextObject(Operator::Yank, TextObject::AParen), 1)
        );
    }

    // -- Text object execution tests --

    #[test]
    fn test_execute_diw() {
        let mut buf = Buffer::from_str("hello world");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(
            Action::OperatorTextObject(Operator::Delete, TextObject::InnerWord),
            &mut buf, &mut cur, &mut mode, &mut regs,
        );
        assert_eq!(buf.line(0), Some(" world".to_string()));
        assert_eq!(regs.get(None).text(), "hello");
    }

    #[test]
    fn test_execute_daw() {
        let mut buf = Buffer::from_str("hello world");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(
            Action::OperatorTextObject(Operator::Delete, TextObject::AWord),
            &mut buf, &mut cur, &mut mode, &mut regs,
        );
        assert_eq!(buf.line(0), Some("world".to_string()));
        assert_eq!(regs.get(None).text(), "hello ");
    }

    #[test]
    fn test_execute_ci_quote() {
        let mut buf = Buffer::from_str(r#"let x = "hello";"#);
        let mut cur = Cursor::new(0, 10);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(
            Action::OperatorTextObject(Operator::Change, TextObject::InnerQuote('"')),
            &mut buf, &mut cur, &mut mode, &mut regs,
        );
        assert_eq!(buf.line(0), Some(r#"let x = "";"#.to_string()));
        assert_eq!(mode, Mode::Insert);
        assert_eq!(regs.get(None).text(), "hello");
    }

    #[test]
    fn test_execute_yi_paren() {
        let mut buf = Buffer::from_str("fn foo(x, y)");
        let mut cur = Cursor::new(0, 8);
        let mut mode = Mode::Normal;
        let mut regs = RegisterFile::new();

        exec_with_regs(
            Action::OperatorTextObject(Operator::Yank, TextObject::InnerParen),
            &mut buf, &mut cur, &mut mode, &mut regs,
        );
        // Buffer unchanged
        assert_eq!(buf.line(0), Some("fn foo(x, y)".to_string()));
        assert_eq!(regs.get(None).text(), "x, y");
    }

}
