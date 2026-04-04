use crate::vim::buffer::Buffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Upcoming — not yet near the viewport.
    Pending,
    /// Within or near the viewport — player should act.
    Active,
    /// Player completed the task.
    Completed,
    /// Scrolled past without completion.
    Missed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskKind {
    /// Move cursor to target position.
    MoveTo,
    /// Delete the entire line at target_line. Completed when that line's content is gone.
    DeleteLine {
        /// The original content of the line (to detect it was deleted).
        original_content: String,
    },
    /// Delete a word at the anchor. Completed when the word is gone from that line.
    DeleteWord {
        /// The word that should be deleted.
        word: String,
    },
    /// Change text at the anchor to new_text. Completed when the line contains new_text.
    ChangeWord {
        /// The original text to be changed.
        original: String,
        /// What it should become.
        new_text: String,
    },
    /// Replace a single character. Completed when char at position matches expected.
    ReplaceChar {
        /// The character it should become.
        expected: char,
    },
    /// Change text inside a delimiter (e.g., ci" task).
    /// Completed when the content between delimiters matches new_text.
    ChangeInside {
        /// The delimiter character (e.g., '"', '(', '{').
        delimiter: char,
        /// What the inside should become.
        new_text: String,
    },
    /// Yank text and paste it at a target location.
    /// Completed when the target line contains the expected text.
    YankPaste {
        /// Text that should appear at the target location.
        expected_text: String,
    },
    /// Delete a block of lines (e.g., delete from line N to line M).
    /// Completed when all specified lines are gone.
    DeleteBlock {
        /// Original content of the lines that should be deleted.
        original_lines: Vec<String>,
    },
    /// Indent or dedent lines. Completed when the line starts with expected indentation.
    Indent {
        /// Expected leading whitespace.
        expected_indent: String,
    },
}

/// How well a task was completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionQuality {
    /// Completed, but not within any optimal budget.
    Done,
    /// Completed within the world-constrained optimal budget.
    Great,
    /// Completed within the absolute optimal budget.
    Perfect,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub kind: TaskKind,
    pub state: TaskState,
    pub target_line: usize,
    pub target_col: usize,
    pub description: String,
    pub points: i64,
    pub gutter_text: String,
    /// Optimal keystrokes for this world's available commands (0 = no tracking).
    pub good_keys: usize,
    /// Absolute optimal keystrokes with any vim command (0 = no tracking).
    pub perfect_keys: usize,
    /// How well this task was completed.
    pub quality: CompletionQuality,
    /// Precomputed hint for practice mode (world-appropriate command suggestion).
    pub hint_command: String,
    /// Whether this task targets the end of a word (for `e` motion hints).
    pub at_end: bool,
    /// World 1 Level 4: restricted zone for this task.
    /// "hl" = h/l only, "wb" = w/b only, "ft" = f/t only, "line_edge" = $/0 only.
    /// None = no restriction.
    pub zone: Option<String>,
}

impl Task {
    pub fn move_to(
        target_line: usize,
        target_col: usize,
        description: impl Into<String>,
        gutter_text: impl Into<String>,
        points: i64,
    ) -> Self {
        Self {
            kind: TaskKind::MoveTo,
            state: TaskState::Pending,
            target_line,
            target_col,
            description: description.into(),
            points,
            gutter_text: gutter_text.into(),
            good_keys: 0,
            perfect_keys: 0,
            quality: CompletionQuality::Done,
            hint_command: String::new(),
            at_end: false,
            zone: None,
        }
    }

    pub fn delete_line(
        target_line: usize,
        original_content: impl Into<String>,
        description: impl Into<String>,
        gutter_text: impl Into<String>,
        points: i64,
    ) -> Self {
        Self {
            kind: TaskKind::DeleteLine {
                original_content: original_content.into(),
            },
            state: TaskState::Pending,
            target_line,
            target_col: 0,
            description: description.into(),
            points,
            gutter_text: gutter_text.into(),
            good_keys: 0,
            perfect_keys: 0,
            quality: CompletionQuality::Done,
            hint_command: String::new(),
            at_end: false,
            zone: None,
        }
    }

    pub fn delete_word(
        target_line: usize,
        target_col: usize,
        word: impl Into<String>,
        description: impl Into<String>,
        gutter_text: impl Into<String>,
        points: i64,
    ) -> Self {
        Self {
            kind: TaskKind::DeleteWord {
                word: word.into(),
            },
            state: TaskState::Pending,
            target_line,
            target_col,
            description: description.into(),
            points,
            gutter_text: gutter_text.into(),
            good_keys: 0,
            perfect_keys: 0,
            quality: CompletionQuality::Done,
            hint_command: String::new(),
            at_end: false,
            zone: None,
        }
    }

    pub fn change_word(
        target_line: usize,
        target_col: usize,
        original: impl Into<String>,
        new_text: impl Into<String>,
        description: impl Into<String>,
        gutter_text: impl Into<String>,
        points: i64,
    ) -> Self {
        let new_text = new_text.into();
        Self {
            kind: TaskKind::ChangeWord {
                original: original.into(),
                new_text: new_text.clone(),
            },
            state: TaskState::Pending,
            target_line,
            target_col,
            description: description.into(),
            points,
            gutter_text: gutter_text.into(),
            good_keys: 0,
            perfect_keys: 0,
            quality: CompletionQuality::Done,
            hint_command: String::new(),
            at_end: false,
            zone: None,
        }
    }

    pub fn replace_char(
        target_line: usize,
        target_col: usize,
        expected: char,
        description: impl Into<String>,
        gutter_text: impl Into<String>,
        points: i64,
    ) -> Self {
        Self {
            kind: TaskKind::ReplaceChar { expected },
            state: TaskState::Pending,
            target_line,
            target_col,
            description: description.into(),
            points,
            gutter_text: gutter_text.into(),
            good_keys: 0,
            perfect_keys: 0,
            quality: CompletionQuality::Done,
            hint_command: String::new(),
            at_end: false,
            zone: None,
        }
    }

    pub fn change_inside(
        target_line: usize,
        target_col: usize,
        delimiter: char,
        new_text: impl Into<String>,
        description: impl Into<String>,
        gutter_text: impl Into<String>,
        points: i64,
    ) -> Self {
        Self {
            kind: TaskKind::ChangeInside {
                delimiter,
                new_text: new_text.into(),
            },
            state: TaskState::Pending,
            target_line,
            target_col,
            description: description.into(),
            points,
            gutter_text: gutter_text.into(),
            good_keys: 0,
            perfect_keys: 0,
            quality: CompletionQuality::Done,
            hint_command: String::new(),
            at_end: false,
            zone: None,
        }
    }

    pub fn yank_paste(
        target_line: usize,
        target_col: usize,
        expected_text: impl Into<String>,
        description: impl Into<String>,
        gutter_text: impl Into<String>,
        points: i64,
    ) -> Self {
        Self {
            kind: TaskKind::YankPaste {
                expected_text: expected_text.into(),
            },
            state: TaskState::Pending,
            target_line,
            target_col,
            description: description.into(),
            points,
            gutter_text: gutter_text.into(),
            good_keys: 0,
            perfect_keys: 0,
            quality: CompletionQuality::Done,
            hint_command: String::new(),
            at_end: false,
            zone: None,
        }
    }

    pub fn delete_block(
        target_line: usize,
        original_lines: Vec<String>,
        description: impl Into<String>,
        gutter_text: impl Into<String>,
        points: i64,
    ) -> Self {
        Self {
            kind: TaskKind::DeleteBlock { original_lines },
            state: TaskState::Pending,
            target_line,
            target_col: 0,
            description: description.into(),
            points,
            gutter_text: gutter_text.into(),
            good_keys: 0,
            perfect_keys: 0,
            quality: CompletionQuality::Done,
            hint_command: String::new(),
            at_end: false,
            zone: None,
        }
    }

    pub fn indent(
        target_line: usize,
        expected_indent: impl Into<String>,
        description: impl Into<String>,
        gutter_text: impl Into<String>,
        points: i64,
    ) -> Self {
        Self {
            kind: TaskKind::Indent {
                expected_indent: expected_indent.into(),
            },
            state: TaskState::Pending,
            target_line,
            target_col: 0,
            description: description.into(),
            points,
            gutter_text: gutter_text.into(),
            good_keys: 0,
            perfect_keys: 0,
            quality: CompletionQuality::Done,
            hint_command: String::new(),
            at_end: false,
            zone: None,
        }
    }

    pub fn is_completable(&self) -> bool {
        matches!(self.state, TaskState::Pending | TaskState::Active)
    }

    pub fn mark_active(&mut self) {
        if self.state == TaskState::Pending {
            self.state = TaskState::Active;
        }
    }

    pub fn mark_completed(&mut self) {
        if self.is_completable() {
            self.state = TaskState::Completed;
        }
    }

    pub fn mark_missed(&mut self) {
        if self.is_completable() {
            self.state = TaskState::Missed;
        }
    }

    /// Derive the expected vim command from the task kind.
    /// Returns the precomputed hint if set (by the assembler), otherwise falls back
    /// to a generic derivation from the TaskKind.
    pub fn expected_command(&self) -> String {
        if !self.hint_command.is_empty() {
            return self.hint_command.clone();
        }
        match &self.kind {
            TaskKind::MoveTo => {
                // Fallback for tasks not processed by assembler (e.g., hardcoded tasks).
                let pattern = self.description.as_str()
                    .trim_start_matches("Move to '")
                    .trim_start_matches("Navigate to '")
                    .trim_end_matches('\'');
                format!("/{}", pattern)
            }
            TaskKind::DeleteLine { .. } => "dd".to_string(),
            TaskKind::DeleteWord { .. } => "dw".to_string(),
            TaskKind::ChangeWord { new_text, .. } => {
                format!("cw{}<Esc>", new_text)
            }
            TaskKind::ReplaceChar { expected } => {
                format!("r{}", expected)
            }
            TaskKind::ChangeInside { delimiter, new_text } => {
                format!("ci{}{}<Esc>", delimiter, new_text)
            }
            TaskKind::YankPaste { .. } => "yy p".to_string(),
            TaskKind::DeleteBlock { original_lines } => {
                let count = original_lines.len();
                if count > 1 {
                    format!("{}dd", count)
                } else {
                    "dd".to_string()
                }
            }
            TaskKind::Indent { .. } => ">> or <<".to_string(),
        }
    }
}

/// Search the buffer for the Nth occurrence of `pattern` (1-indexed).
/// Returns the (line, col) of the first character of the match.
pub fn resolve_pattern(buffer: &Buffer, pattern: &str, occurrence: usize) -> Option<(usize, usize)> {
    if pattern.is_empty() || occurrence == 0 {
        return None;
    }

    let mut found = 0;
    for line_idx in 0..buffer.line_count() {
        let line_content = buffer.line(line_idx).unwrap_or_default();
        let mut search_start = 0;
        while let Some(pos) = line_content[search_start..].find(pattern) {
            found += 1;
            let col = search_start + pos;
            if found == occurrence {
                return Some((line_idx, col));
            }
            search_start = col + 1;
        }
    }
    None
}

/// Create hardcoded tasks for the SAMPLE_CODE buffer.
/// Tasks are anchored by pattern so they survive minor buffer changes.
pub fn hardcoded_tasks(buffer: &Buffer) -> Vec<Task> {
    let mut tasks = Vec::new();

    // Task 1: Move to "Fizzbuzz" comment (early in the file)
    if let Some((line, col)) = resolve_pattern(buffer, "Fizzbuzz", 1) {
        tasks.push(Task::move_to(
            line,
            col,
            "Move to 'Fizzbuzz'",
            "MOVE",
            50,
        ));
    }

    // Task 2: Move to "fibonacci" function call
    if let Some((line, col)) = resolve_pattern(buffer, "fibonacci", 1) {
        tasks.push(Task::move_to(
            line,
            col,
            "Move to 'fibonacci'",
            "MOVE",
            75,
        ));
    }

    // Task 3: Move to "title_case" variable
    if let Some((line, col)) = resolve_pattern(buffer, "title_case", 1) {
        tasks.push(Task::move_to(
            line,
            col,
            "Move to 'title_case'",
            "MOVE",
            75,
        ));
    }

    // Task 4: Move to "enum Shape" declaration
    if let Some((line, col)) = resolve_pattern(buffer, "enum Shape", 1) {
        tasks.push(Task::move_to(
            line,
            col,
            "Move to 'enum Shape'",
            "MOVE",
            100,
        ));
    }

    // Task 5: Move to "sqrt" at the end
    if let Some((line, col)) = resolve_pattern(buffer, "sqrt", 1) {
        tasks.push(Task::move_to(
            line,
            col,
            "Move to 'sqrt'",
            "MOVE",
            100,
        ));
    }

    tasks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_buffer() -> Buffer {
        Buffer::from_str("hello world\nfoo bar baz\n\nfoo again")
    }

    // -- resolve_pattern --

    #[test]
    fn test_resolve_pattern_basic() {
        let buf = sample_buffer();
        assert_eq!(resolve_pattern(&buf, "world", 1), Some((0, 6)));
    }

    #[test]
    fn test_resolve_pattern_second_occurrence() {
        let buf = sample_buffer();
        // "foo" appears on line 1 and line 3
        assert_eq!(resolve_pattern(&buf, "foo", 1), Some((1, 0)));
        assert_eq!(resolve_pattern(&buf, "foo", 2), Some((3, 0)));
    }

    #[test]
    fn test_resolve_pattern_not_found() {
        let buf = sample_buffer();
        assert_eq!(resolve_pattern(&buf, "xyz", 1), None);
    }

    #[test]
    fn test_resolve_pattern_empty() {
        let buf = sample_buffer();
        assert_eq!(resolve_pattern(&buf, "", 1), None);
    }

    #[test]
    fn test_resolve_pattern_zero_occurrence() {
        let buf = sample_buffer();
        assert_eq!(resolve_pattern(&buf, "hello", 0), None);
    }

    #[test]
    fn test_resolve_pattern_multiple_on_same_line() {
        let buf = Buffer::from_str("aXbXcX");
        assert_eq!(resolve_pattern(&buf, "X", 1), Some((0, 1)));
        assert_eq!(resolve_pattern(&buf, "X", 2), Some((0, 3)));
        assert_eq!(resolve_pattern(&buf, "X", 3), Some((0, 5)));
        assert_eq!(resolve_pattern(&buf, "X", 4), None);
    }

    // -- Task state transitions --

    #[test]
    fn test_task_state_transitions() {
        let mut task = Task::move_to(0, 0, "test", "MOVE", 50);
        assert_eq!(task.state, TaskState::Pending);
        assert!(task.is_completable());

        task.mark_active();
        assert_eq!(task.state, TaskState::Active);
        assert!(task.is_completable());

        task.mark_completed();
        assert_eq!(task.state, TaskState::Completed);
        assert!(!task.is_completable());
    }

    #[test]
    fn test_task_missed() {
        let mut task = Task::move_to(0, 0, "test", "MOVE", 50);
        task.mark_active();
        task.mark_missed();
        assert_eq!(task.state, TaskState::Missed);
        assert!(!task.is_completable());
    }

    #[test]
    fn test_cannot_complete_missed_task() {
        let mut task = Task::move_to(0, 0, "test", "MOVE", 50);
        task.mark_missed();
        task.mark_completed(); // should be no-op
        assert_eq!(task.state, TaskState::Missed);
    }

    #[test]
    fn test_cannot_miss_completed_task() {
        let mut task = Task::move_to(0, 0, "test", "MOVE", 50);
        task.mark_completed();
        task.mark_missed(); // should be no-op
        assert_eq!(task.state, TaskState::Completed);
    }

    // -- hardcoded_tasks --

    #[test]
    fn test_hardcoded_tasks_with_sample_code() {
        let sample = include_str!("../../src/app.rs");
        // Extract just SAMPLE_CODE content - we can't easily, so test with a known buffer
        let buf = Buffer::from_str(
            "// Fizzbuzz\nlet fibs = fibonacci(10);\nlet title_case = foo;\nenum Shape {\n(s).sqrt()",
        );
        let tasks = hardcoded_tasks(&buf);
        assert!(tasks.len() >= 4);
        for task in &tasks {
            assert_eq!(task.state, TaskState::Pending);
            assert!(task.points > 0);
        }
    }

    // -- new task types --

    #[test]
    fn test_change_inside_task() {
        let task = Task::change_inside(0, 5, '"', "world", "Change inside quotes", "CHG", 75);
        assert_eq!(task.state, TaskState::Pending);
        assert!(task.is_completable());
        if let TaskKind::ChangeInside { delimiter, new_text } = &task.kind {
            assert_eq!(*delimiter, '"');
            assert_eq!(new_text, "world");
        } else {
            panic!("wrong task kind");
        }
    }

    #[test]
    fn test_yank_paste_task() {
        let task = Task::yank_paste(5, 0, "expected", "Yank and paste", "Y+P", 100);
        if let TaskKind::YankPaste { expected_text } = &task.kind {
            assert_eq!(expected_text, "expected");
        } else {
            panic!("wrong task kind");
        }
    }

    #[test]
    fn test_delete_block_task() {
        let lines = vec!["line 1".to_string(), "line 2".to_string()];
        let task = Task::delete_block(3, lines.clone(), "Delete block", "DEL BLK", 120);
        if let TaskKind::DeleteBlock { original_lines } = &task.kind {
            assert_eq!(original_lines, &lines);
        } else {
            panic!("wrong task kind");
        }
    }

    #[test]
    fn test_indent_task() {
        let task = Task::indent(2, "    ", "Fix indentation", "INDENT", 60);
        if let TaskKind::Indent { expected_indent } = &task.kind {
            assert_eq!(expected_indent, "    ");
        } else {
            panic!("wrong task kind");
        }
    }
}
