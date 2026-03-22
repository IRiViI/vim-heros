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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    MoveTo,
    // Future: DeleteLine, ChangeWord, etc.
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
}
