use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::time::Duration;

use crate::content::assembler;
use crate::content::loader;
use crate::game::engine::{Engine, GameState};
use crate::game::scoring::Scoring;
use crate::game::task::{self, Task, TaskKind, TaskState};
use crate::game::viewport::Viewport;
use crate::vim::buffer::Buffer;
use crate::vim::command::{self, Action, CommandParser, ParseResult};
use crate::vim::cursor::Cursor;
use crate::vim::mode::Mode;
use crate::vim::register::RegisterFile;
use crate::vim::search::{self, SearchDirection, SearchState};
use crate::vim::undo::UndoHistory;

const SAMPLE_CODE: &str = r#"use std::collections::HashMap;

fn main() {
    let greeting = "Hello, Vim Heroes!";
    println!("{}", greeting);

    // Calculate some numbers
    let numbers = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let total: i32 = numbers.iter().sum();
    let average = total as f64 / numbers.len() as f64;
    println!("Total: {}, Average: {:.1}", total, average);

    // Fizzbuzz
    for i in 1..=20 {
        let result = match (i % 3, i % 5) {
            (0, 0) => "FizzBuzz".to_string(),
            (0, _) => "Fizz".to_string(),
            (_, 0) => "Buzz".to_string(),
            _ => i.to_string(),
        };
        println!("{}: {}", i, result);
    }

    // Build a frequency map
    let words = vec!["hello", "world", "hello", "rust", "world", "hello"];
    let mut freq: HashMap<&str, usize> = HashMap::new();
    for word in &words {
        *freq.entry(word).or_insert(0) += 1;
    }
    println!("Word frequencies: {:?}", freq);

    // Fibonacci sequence
    let fibs = fibonacci(10);
    println!("Fibonacci: {:?}", fibs);

    // String manipulation
    let sentence = "the quick brown fox jumps over the lazy dog";
    let title_case: String = sentence
        .split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    println!("Title case: {}", title_case);

    // Pattern matching with enums
    let shapes = vec![
        Shape::Circle(5.0),
        Shape::Rectangle(4.0, 6.0),
        Shape::Triangle(3.0, 4.0, 5.0),
    ];
    for shape in &shapes {
        println!("Area: {:.2}", shape.area());
    }
}

fn fibonacci(n: usize) -> Vec<u64> {
    let mut fibs = vec![0, 1];
    for i in 2..n {
        let next = fibs[i - 1] + fibs[i - 2];
        fibs.push(next);
    }
    fibs
}

enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
    Triangle(f64, f64, f64),
}

impl Shape {
    fn area(&self) -> f64 {
        match self {
            Shape::Circle(r) => std::f64::consts::PI * r * r,
            Shape::Rectangle(w, h) => w * h,
            Shape::Triangle(a, b, c) => {
                let s = (a + b + c) / 2.0;
                (s * (s - a) * (s - b) * (s - c)).sqrt()
            }
        }
    }
}"#;

const DEFAULT_SCROLL_SPEED_MS: u64 = 2000;
const SEGMENTS_PER_LEVEL: usize = 4;

/// Level metadata.
pub struct LevelInfo {
    pub world: usize,
    pub level: usize,
    pub name: String,
    pub zone: String,
    pub language: String,
    pub scroll_speed_ms: u64,
}

impl LevelInfo {
    pub fn display_id(&self) -> String {
        format!("{}-{}", self.world, self.level)
    }
}

/// All available levels.
fn level_list() -> Vec<LevelInfo> {
    vec![
        LevelInfo { world: 1, level: 1, name: "First Steps".into(), zone: "starter".into(), language: "python".into(), scroll_speed_ms: 2500 },
        LevelInfo { world: 1, level: 2, name: "Word Jumps".into(), zone: "starter".into(), language: "python".into(), scroll_speed_ms: 2500 },
        LevelInfo { world: 1, level: 3, name: "Line Moves".into(), zone: "starter".into(), language: "typescript".into(), scroll_speed_ms: 2500 },
        LevelInfo { world: 2, level: 1, name: "First Edits".into(), zone: "starter".into(), language: "python".into(), scroll_speed_ms: 2200 },
        LevelInfo { world: 2, level: 2, name: "Cut & Paste".into(), zone: "starter".into(), language: "typescript".into(), scroll_speed_ms: 2200 },
        LevelInfo { world: 3, level: 1, name: "Precision".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 2000 },
        LevelInfo { world: 3, level: 2, name: "Operator Combos".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 2000 },
        LevelInfo { world: 3, level: 3, name: "Find & Delete".into(), zone: "junior".into(), language: "typescript".into(), scroll_speed_ms: 2000 },
        LevelInfo { world: 4, level: 1, name: "Speed Run".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1800 },
        LevelInfo { world: 4, level: 2, name: "TS Speed".into(), zone: "junior".into(), language: "typescript".into(), scroll_speed_ms: 1800 },
    ]
}

fn default_level() -> LevelInfo {
    level_list().into_iter().next().unwrap()
}

/// A repeatable edit for dot (.) repeat.
#[derive(Debug, Clone)]
struct RepeatableEdit {
    /// The initial action that triggered the edit.
    action: Action,
    /// How many times to repeat the initial action.
    count: usize,
    /// Characters typed during insert mode (for change/insert actions).
    insert_text: Vec<char>,
}

pub struct App {
    pub buffer: Buffer,
    pub cursor: Cursor,
    pub mode: Mode,
    pub running: bool,
    pub viewport: Viewport,
    pub engine: Engine,
    pub scoring: Scoring,
    pub tasks: Vec<Task>,
    pub level: LevelInfo,
    parser: CommandParser,
    recently_seen: Vec<String>,
    registers: RegisterFile,
    undo: UndoHistory,
    pub search: SearchState,
    /// Anchor cursor for visual mode selection (where v/V was pressed).
    pub visual_anchor: Cursor,
    /// Last repeatable edit for dot (.) repeat.
    last_edit: Option<RepeatableEdit>,
    /// Buffer for collecting insert-mode keystrokes (for dot repeat).
    insert_chars: Vec<char>,
    /// Macro register storage: maps register char -> recorded keystrokes.
    macro_regs: HashMap<char, Vec<KeyEvent>>,
    /// Current macro recording buffer (None = not recording).
    macro_recording: Option<(char, Vec<KeyEvent>)>,
    /// Last played macro register (for @@ repeat).
    last_macro_reg: Option<char>,
    /// Command-line input buffer (for : commands). None = not active.
    pub cmdline: Option<String>,
    level_index: usize,
}

impl App {
    pub fn new(viewport_height: usize) -> Self {
        let level = default_level();
        let (buffer, tasks, seen) = Self::load_level(&level, &[]);
        let tasks_total = tasks.len();
        Self {
            buffer,
            cursor: Cursor::new(0, 0),
            mode: Mode::Normal,
            running: true,
            viewport: Viewport::new(viewport_height),
            engine: Engine::new(level.scroll_speed_ms),
            scoring: Scoring::new(tasks_total),
            tasks,
            level,
            parser: CommandParser::new(),
            recently_seen: seen,
            registers: RegisterFile::new(),
            undo: UndoHistory::new(),
            search: SearchState::new(),
            visual_anchor: Cursor::new(0, 0),
            last_edit: None,
            insert_chars: Vec::new(),
            macro_regs: HashMap::new(),
            macro_recording: None,
            last_macro_reg: None,
            cmdline: None,
            level_index: 0,
        }
    }

    /// Load a level from content segments. Returns (buffer, tasks, segment IDs used).
    fn load_level(
        level: &LevelInfo,
        recently_seen: &[String],
    ) -> (Buffer, Vec<Task>, Vec<String>) {
        let pool = loader::load_segments(&level.language, &level.zone);

        if pool.is_empty() {
            // Fallback to hardcoded SAMPLE_CODE
            let buffer = Buffer::from_str(SAMPLE_CODE);
            let tasks = task::hardcoded_tasks(&buffer);
            return (buffer, tasks, Vec::new());
        }

        let selected = assembler::select_segments(&pool, SEGMENTS_PER_LEVEL, recently_seen);
        let ids: Vec<String> = selected.iter().map(|s| s.meta.id.clone()).collect();
        let assembled = assembler::assemble(&selected);
        (assembled.buffer, assembled.tasks, ids)
    }

    /// Update viewport height when terminal is resized.
    pub fn update_viewport_height(&mut self, terminal_height: usize) {
        // terminal_height minus 2 (borders) minus 1 (HUD) minus 1 (status bar)
        self.viewport.height = terminal_height.saturating_sub(4);
    }

    /// Main tick: poll for input, handle scroll, check game over.
    /// Returns true if a frame should be rendered.
    pub fn tick(&mut self) -> bool {
        match self.engine.state {
            GameState::Countdown => self.tick_countdown(),
            GameState::Playing => self.tick_playing(),
            GameState::GameOver | GameState::LevelComplete => self.tick_game_over(),
        }
    }

    fn tick_countdown(&mut self) -> bool {
        // Check if countdown is done
        if self.engine.check_countdown() {
            return true;
        }

        // Consume input during countdown (allow quit, but don't penalize)
        if event::poll(Duration::from_millis(50)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && key.code == KeyCode::Char('c')
                {
                    self.running = false;
                    return true;
                }
                if key.code == KeyCode::Char('q') {
                    self.running = false;
                    return true;
                }
            }
        }

        // Re-render each tick so the countdown number updates
        true
    }

    fn tick_playing(&mut self) -> bool {
        let timeout = self
            .engine
            .time_until_next_scroll()
            .min(Duration::from_millis(50));
        let mut needs_render = false;

        // Poll for input
        if event::poll(timeout).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                needs_render = self.handle_key(key);
            }
        }

        // Scroll boost: if cursor moved past viewport bottom, snap viewport forward
        if self.cursor.line > self.viewport.bottom_line() {
            let overshoot = self.cursor.line - self.viewport.bottom_line();
            let max_scroll = self
                .buffer
                .line_count()
                .saturating_sub(self.viewport.height);
            for _ in 0..overshoot {
                if self.viewport.top_line < max_scroll {
                    self.viewport.scroll_down();
                    self.scoring.award_survival();
                }
            }
            // Reset scroll timer so the player isn't immediately punished
            self.engine.record_scroll();
            needs_render = true;
        }

        // Check scroll tick
        if self.engine.should_scroll() {
            let max_scroll = self
                .buffer
                .line_count()
                .saturating_sub(self.viewport.height);
            if self.viewport.top_line < max_scroll {
                self.viewport.scroll_down();
                self.scoring.award_survival();
            }
            self.engine.record_scroll();
            needs_render = true;

            // Game over: cursor scrolled above viewport
            if self.cursor.line < self.viewport.top_line {
                self.engine.state = GameState::GameOver;
            }

            // Check for missed tasks (scrolled above viewport)
            for task in &mut self.tasks {
                if task.is_completable() && task.target_line < self.viewport.top_line {
                    task.mark_missed();
                    self.scoring.miss_task();
                }
            }

            // Activate tasks that are within or near the viewport
            let activation_bottom = self.viewport.bottom_line() + 5;
            for task in &mut self.tasks {
                if task.state == TaskState::Pending
                    && task.target_line >= self.viewport.top_line
                    && task.target_line <= activation_bottom
                {
                    task.mark_active();
                }
            }
        }

        // Check level complete: all tasks resolved (completed or missed)
        // and viewport has scrolled past the buffer
        let all_resolved = self.tasks.iter().all(|t| !t.is_completable());
        let buffer_done = self.viewport.top_line + self.viewport.height
            >= self.buffer.line_count();
        if all_resolved && buffer_done {
            self.engine.state = GameState::LevelComplete;
            needs_render = true;
        }

        needs_render
    }

    fn tick_game_over(&mut self) -> bool {
        if !event::poll(Duration::from_millis(50)).unwrap_or(false) {
            return false;
        }

        let event = match event::read() {
            Ok(ev) => ev,
            Err(_) => return false,
        };

        match event {
            Event::Key(key) => {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && key.code == KeyCode::Char('c')
                {
                    self.running = false;
                    return true;
                }
                match key.code {
                    KeyCode::Char('q') => {
                        self.running = false;
                        true
                    }
                    KeyCode::Char('r') => {
                        self.restart();
                        true
                    }
                    KeyCode::Char('n') => {
                        self.next_level();
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn restart(&mut self) {
        let (buffer, tasks, seen) = Self::load_level(&self.level, &self.recently_seen);
        self.buffer = buffer;
        self.tasks = tasks;
        self.recently_seen = seen;
        self.cursor = Cursor::new(0, 0);
        self.mode = Mode::Normal;
        self.viewport = Viewport::new(self.viewport.height);
        self.engine.reset();
        self.scoring.reset(self.tasks.len());
        self.parser = CommandParser::new();
        self.registers = RegisterFile::new();
        self.undo = UndoHistory::new();
        self.search = SearchState::new();
        self.visual_anchor = Cursor::new(0, 0);
        self.last_edit = None;
        self.insert_chars.clear();
        self.macro_regs.clear();
        self.macro_recording = None;
        self.last_macro_reg = None;
        self.cmdline = None;
    }

    /// Move cursor (and viewport) by `lines` in a direction.
    fn scroll_cursor(&mut self, lines: usize, down: bool) {
        let max_line = self.buffer.line_count().saturating_sub(1);
        if down {
            self.cursor.line = (self.cursor.line + lines).min(max_line);
        } else {
            self.cursor.line = self.cursor.line.saturating_sub(lines);
        }
        self.cursor.clamp(&self.buffer, false);
        self.check_task_completion();
    }

    fn next_level(&mut self) {
        let levels = level_list();
        self.level_index = (self.level_index + 1) % levels.len();
        self.level = levels.into_iter().nth(self.level_index).unwrap();
        self.recently_seen.clear();
        self.restart();
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.running = false;
            return true;
        }

        // Command-line mode intercepts all keys
        if self.cmdline.is_some() {
            return self.handle_cmdline_input(key);
        }

        // Search input mode intercepts all keys
        if self.search.active {
            return self.handle_search_input(key);
        }

        // Record keystrokes for macro (except the q that stops recording)
        let is_macro_stop = key.code == KeyCode::Char('q')
            && self.mode.is_normal()
            && self.macro_recording.is_some();
        if self.macro_recording.is_some() && !is_macro_stop {
            if let Some((_, ref mut keys)) = self.macro_recording {
                keys.push(key);
            }
        }

        match self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::Insert => self.handle_insert_key(key),
            Mode::Replace => self.handle_replace_key(key),
            Mode::Visual | Mode::VisualLine => self.handle_visual_key(key),
        }
    }

    fn handle_search_input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Enter => {
                if self.search.commit_input() {
                    // Execute the search
                    self.execute_search(self.search.direction);
                }
                true
            }
            KeyCode::Esc => {
                self.search.cancel_input();
                true
            }
            KeyCode::Backspace => {
                if self.search.input_buf.is_empty() {
                    self.search.cancel_input();
                } else {
                    self.search.pop_char();
                }
                true
            }
            KeyCode::Char(ch) => {
                self.search.push_char(ch);
                true
            }
            _ => false,
        }
    }

    fn handle_cmdline_input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Enter => {
                let cmd = self.cmdline.take().unwrap_or_default();
                self.execute_cmdline(&cmd);
                true
            }
            KeyCode::Esc => {
                self.cmdline = None;
                true
            }
            KeyCode::Backspace => {
                if let Some(ref mut buf) = self.cmdline {
                    if buf.is_empty() {
                        self.cmdline = None;
                    } else {
                        buf.pop();
                    }
                }
                true
            }
            KeyCode::Char(ch) => {
                if let Some(ref mut buf) = self.cmdline {
                    buf.push(ch);
                }
                true
            }
            _ => false,
        }
    }

    fn execute_cmdline(&mut self, cmd: &str) {
        let cmd = cmd.trim();
        match cmd {
            "q" => {
                // Quit current level — for now, quit the game
                self.running = false;
            }
            "q!" => {
                // Force quit
                self.running = false;
            }
            "r" | "restart" => {
                self.restart();
            }
            "n" | "next" => {
                self.next_level();
            }
            _ => {
                // Unknown command — just dismiss
            }
        }
    }

    fn execute_dot_repeat(&mut self, repeat_count: usize) {
        let edit = match &self.last_edit {
            Some(e) => e.clone(),
            None => return,
        };

        for _ in 0..repeat_count {
            self.undo.push(self.buffer.rope(), self.cursor);

            // Execute the initial action
            for _ in 0..edit.count {
                command::execute(
                    edit.action,
                    &mut self.buffer,
                    &mut self.cursor,
                    &mut self.mode,
                    &mut self.registers,
                );
            }

            // If the action entered insert mode, replay the insert text
            if !edit.insert_text.is_empty() && self.mode.is_insert() {
                for &ch in &edit.insert_text {
                    command::execute(
                        Action::InsertChar(ch),
                        &mut self.buffer,
                        &mut self.cursor,
                        &mut self.mode,
                        &mut self.registers,
                    );
                }
                // Return to normal mode
                command::execute(
                    Action::EnterNormalMode,
                    &mut self.buffer,
                    &mut self.cursor,
                    &mut self.mode,
                    &mut self.registers,
                );
            }
        }
        self.check_task_completion();
    }

    fn execute_macro(&mut self, reg: char, count: usize) {
        let keys = match self.macro_regs.get(&reg) {
            Some(k) => k.clone(),
            None => return,
        };
        // Temporarily stop recording to avoid nested recording
        let was_recording = self.macro_recording.take();
        for _ in 0..count {
            for key in &keys {
                self.handle_key(*key);
                if !self.running {
                    break;
                }
            }
        }
        self.macro_recording = was_recording;
    }

    fn execute_search(&mut self, direction: SearchDirection) {
        if !self.search.has_pattern() {
            return;
        }
        let result = search::search_next(
            &self.cursor,
            &self.buffer,
            &self.search.pattern,
            direction,
        );
        if let Some(new_cursor) = result {
            self.cursor = new_cursor;
            self.check_task_completion();
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> bool {
        // Ctrl key combos
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('r') => {
                    self.scoring.penalize_keystroke();
                    if let Some((rope, cursor)) = self.undo.redo() {
                        self.buffer.set_rope(rope);
                        self.cursor = cursor;
                    }
                    return true;
                }
                KeyCode::Char('d') => {
                    self.scoring.penalize_keystroke();
                    self.scroll_cursor(self.viewport.height / 2, true);
                    return true;
                }
                KeyCode::Char('u') => {
                    self.scoring.penalize_keystroke();
                    self.scroll_cursor(self.viewport.height / 2, false);
                    return true;
                }
                KeyCode::Char('f') => {
                    self.scoring.penalize_keystroke();
                    self.scroll_cursor(self.viewport.height.saturating_sub(2), true);
                    return true;
                }
                KeyCode::Char('b') => {
                    self.scoring.penalize_keystroke();
                    self.scroll_cursor(self.viewport.height.saturating_sub(2), false);
                    return true;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Esc => {
                self.parser.cancel();
                true
            }
            KeyCode::Char(ch) => {
                self.scoring.penalize_keystroke();

                match self.parser.feed(ch) {
                    ParseResult::Action(action, count) => {
                        // Handle undo specially
                        if matches!(action, Action::Undo) {
                            if let Some((rope, cursor)) = self.undo.undo(self.buffer.rope(), self.cursor) {
                                self.buffer.set_rope(rope);
                                self.cursor = cursor;
                            }
                            return true;
                        }

                        // Handle visual mode entry — set anchor
                        if matches!(action, Action::EnterVisualMode | Action::EnterVisualLineMode) {
                            self.visual_anchor = self.cursor;
                        }

                        // Handle search/cmdline actions specially
                        match action {
                            Action::EnterCmdLine => {
                                self.cmdline = Some(String::new());
                                return true;
                            }
                            Action::SearchForward => {
                                self.search.start_input(SearchDirection::Forward);
                                return true;
                            }
                            Action::SearchBackward => {
                                self.search.start_input(SearchDirection::Backward);
                                return true;
                            }
                            Action::SearchNext => {
                                for _ in 0..count {
                                    self.execute_search(self.search.direction);
                                }
                                return true;
                            }
                            Action::SearchPrev => {
                                let reverse = match self.search.direction {
                                    SearchDirection::Forward => SearchDirection::Backward,
                                    SearchDirection::Backward => SearchDirection::Forward,
                                };
                                for _ in 0..count {
                                    self.execute_search(reverse);
                                }
                                return true;
                            }
                            Action::SearchWordForward => {
                                if let Some(word) = search::word_under_cursor(&self.cursor, &self.buffer) {
                                    self.search.pattern = word;
                                    self.search.direction = SearchDirection::Forward;
                                    self.execute_search(SearchDirection::Forward);
                                }
                                return true;
                            }
                            Action::SearchWordBackward => {
                                if let Some(word) = search::word_under_cursor(&self.cursor, &self.buffer) {
                                    self.search.pattern = word;
                                    self.search.direction = SearchDirection::Backward;
                                    self.execute_search(SearchDirection::Backward);
                                }
                                return true;
                            }
                            Action::DotRepeat => {
                                self.execute_dot_repeat(count);
                                return true;
                            }
                            Action::MacroRecord(reg) => {
                                self.macro_recording = Some((reg, Vec::new()));
                                return true;
                            }
                            Action::MacroStop => {
                                if let Some((reg, keys)) = self.macro_recording.take() {
                                    self.macro_regs.insert(reg, keys);
                                }
                                return true;
                            }
                            Action::MacroPlay(reg) => {
                                let actual_reg = if reg == '\0' {
                                    // @@ — replay last
                                    match self.last_macro_reg {
                                        Some(r) => r,
                                        None => return true,
                                    }
                                } else {
                                    reg
                                };
                                self.last_macro_reg = Some(actual_reg);
                                self.execute_macro(actual_reg, count);
                                return true;
                            }
                            _ => {}
                        }

                        // Push undo snapshot before editing actions
                        if action.is_edit() {
                            self.undo.push(self.buffer.rope(), self.cursor);
                        }

                        // Record repeatable edits for dot repeat
                        if action.is_edit() && !matches!(action, Action::InsertChar(_) | Action::Backspace) {
                            self.last_edit = Some(RepeatableEdit {
                                action,
                                count,
                                insert_text: Vec::new(),
                            });
                            self.insert_chars.clear();
                        }

                        for _ in 0..count {
                            command::execute(
                                action,
                                &mut self.buffer,
                                &mut self.cursor,
                                &mut self.mode,
                                &mut self.registers,
                            );
                        }
                        self.check_task_completion();
                    }
                    ParseResult::Pending => {}
                    ParseResult::None => {}
                }
                true
            }
            _ => false,
        }
    }

    fn handle_insert_key(&mut self, key: KeyEvent) -> bool {
        let action = match key.code {
            KeyCode::Esc => {
                // Finalize the repeatable edit with collected insert chars
                if let Some(ref mut edit) = self.last_edit {
                    edit.insert_text = self.insert_chars.clone();
                }
                self.insert_chars.clear();
                Action::EnterNormalMode
            }
            KeyCode::Char(ch) => {
                self.insert_chars.push(ch);
                Action::InsertChar(ch)
            }
            KeyCode::Enter => {
                self.insert_chars.push('\n');
                Action::InsertChar('\n')
            }
            KeyCode::Backspace => Action::Backspace,
            _ => return false,
        };

        self.scoring.penalize_keystroke();
        if action.is_edit() {
            self.undo.push(self.buffer.rope(), self.cursor);
        }
        command::execute(action, &mut self.buffer, &mut self.cursor, &mut self.mode, &mut self.registers);
        self.check_task_completion();
        true
    }

    fn handle_replace_key(&mut self, key: KeyEvent) -> bool {
        let action = match key.code {
            KeyCode::Esc => Action::EnterNormalMode,
            KeyCode::Char(ch) => Action::ReplaceOverwrite(ch),
            KeyCode::Enter => Action::ReplaceOverwrite('\n'),
            KeyCode::Backspace => Action::Backspace,
            _ => return false,
        };

        self.scoring.penalize_keystroke();
        if action.is_edit() {
            self.undo.push(self.buffer.rope(), self.cursor);
        }
        command::execute(action, &mut self.buffer, &mut self.cursor, &mut self.mode, &mut self.registers);
        self.check_task_completion();
        true
    }

    fn handle_visual_key(&mut self, key: KeyEvent) -> bool {
        // Ctrl combos
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('d') => {
                    self.scoring.penalize_keystroke();
                    self.scroll_cursor(self.viewport.height / 2, true);
                    return true;
                }
                KeyCode::Char('u') => {
                    self.scoring.penalize_keystroke();
                    self.scroll_cursor(self.viewport.height / 2, false);
                    return true;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.parser.cancel();
                true
            }
            KeyCode::Char(ch) => {
                self.scoring.penalize_keystroke();

                // Operators act on the visual selection
                match ch {
                    'd' | 'x' => {
                        self.visual_operator(command::Operator::Delete);
                        return true;
                    }
                    'c' | 's' => {
                        self.visual_operator(command::Operator::Change);
                        return true;
                    }
                    'y' => {
                        self.visual_operator(command::Operator::Yank);
                        return true;
                    }
                    // Toggle between visual and visual-line
                    'v' => {
                        if self.mode == Mode::Visual {
                            self.mode = Mode::Normal;
                        } else {
                            self.mode = Mode::Visual;
                            self.visual_anchor = self.cursor;
                        }
                        return true;
                    }
                    'V' => {
                        if self.mode == Mode::VisualLine {
                            self.mode = Mode::Normal;
                        } else {
                            self.mode = Mode::VisualLine;
                            self.visual_anchor = self.cursor;
                        }
                        return true;
                    }
                    _ => {}
                }

                // Motions: use the parser to interpret, then apply as cursor movement
                match self.parser.feed(ch) {
                    ParseResult::Action(action, count) => {
                        // Apply motion to move the cursor (extending selection)
                        for _ in 0..count {
                            command::execute(
                                action,
                                &mut self.buffer,
                                &mut self.cursor,
                                &mut self.mode,
                                &mut self.registers,
                            );
                        }
                        // Stay in visual mode (execute may have changed it for insert actions)
                        // Only if execution didn't change mode to something else
                    }
                    ParseResult::Pending => {}
                    ParseResult::None => {}
                }
                true
            }
            _ => false,
        }
    }

    /// Execute an operator on the visual selection.
    fn visual_operator(&mut self, op: command::Operator) {
        let anchor = self.visual_anchor;
        let cursor = self.cursor;
        let is_linewise = self.mode == Mode::VisualLine;

        self.undo.push(self.buffer.rope(), self.cursor);

        if is_linewise {
            let start_line = anchor.line.min(cursor.line);
            let end_line = anchor.line.max(cursor.line);

            match op {
                command::Operator::Delete => {
                    let text = self.buffer.delete_lines(start_line, end_line);
                    let reg_text = if text.ends_with('\n') { text } else { format!("{}\n", text) };
                    self.registers.delete(None, super::vim::register::RegisterContent::Linewise(reg_text));
                    self.cursor.line = start_line.min(self.buffer.line_count().saturating_sub(1));
                    self.cursor.clamp(&self.buffer, false);
                }
                command::Operator::Change => {
                    let text = self.buffer.delete_lines(start_line, end_line);
                    let reg_text = if text.ends_with('\n') { text } else { format!("{}\n", text) };
                    self.registers.delete(None, super::vim::register::RegisterContent::Linewise(reg_text));
                    if start_line < self.buffer.line_count() {
                        self.buffer.insert_char(start_line, 0, '\n');
                        self.cursor.line = start_line;
                    }
                    self.cursor.col = 0;
                    self.mode = Mode::Insert;
                    self.check_task_completion();
                    return;
                }
                command::Operator::Yank => {
                    let mut text = String::new();
                    for line_idx in start_line..=end_line {
                        if let Some(line) = self.buffer.line(line_idx) {
                            text.push_str(&line);
                            text.push('\n');
                        }
                    }
                    self.registers.yank(None, super::vim::register::RegisterContent::Linewise(text));
                    self.cursor.line = start_line;
                    self.cursor.clamp(&self.buffer, false);
                }
            }
        } else {
            // Charwise visual
            let (start, end) = if (anchor.line, anchor.col) <= (cursor.line, cursor.col) {
                (anchor, cursor)
            } else {
                (cursor, anchor)
            };

            // End is inclusive in charwise visual — make it exclusive for the range
            let end_exclusive = if end.col + 1 <= self.buffer.line_len(end.line) {
                Cursor::new(end.line, end.col + 1)
            } else if end.line + 1 < self.buffer.line_count() {
                Cursor::new(end.line + 1, 0)
            } else {
                Cursor::new(end.line, self.buffer.line_len(end.line))
            };

            let text = self.buffer.text_range(
                start.line,
                start.col,
                end_exclusive.line,
                end_exclusive.col,
            );

            match op {
                command::Operator::Delete => {
                    self.buffer.delete_range(
                        start.line, start.col,
                        end_exclusive.line, end_exclusive.col,
                    );
                    self.registers.delete(None, super::vim::register::RegisterContent::Charwise(text));
                    self.cursor = start;
                    self.cursor.clamp(&self.buffer, false);
                }
                command::Operator::Change => {
                    self.buffer.delete_range(
                        start.line, start.col,
                        end_exclusive.line, end_exclusive.col,
                    );
                    self.registers.delete(None, super::vim::register::RegisterContent::Charwise(text));
                    self.cursor = start;
                    self.mode = Mode::Insert;
                    self.check_task_completion();
                    return;
                }
                command::Operator::Yank => {
                    self.registers.yank(None, super::vim::register::RegisterContent::Charwise(text));
                    self.cursor = start;
                }
            }
        }

        self.mode = Mode::Normal;
        self.check_task_completion();
    }

    fn check_task_completion(&mut self) {
        for task in &mut self.tasks {
            if !task.is_completable() {
                continue;
            }
            let completed = match &task.kind {
                TaskKind::MoveTo => {
                    self.cursor.line == task.target_line
                        && self.cursor.col == task.target_col
                }
                TaskKind::DeleteLine { original_content } => {
                    // Line is deleted if the content at that line no longer matches
                    match self.buffer.line(task.target_line) {
                        Some(line) => line.trim() != original_content.trim(),
                        None => true, // line doesn't exist anymore = deleted
                    }
                }
                TaskKind::DeleteWord { word } => {
                    // Word is deleted if the line no longer contains it
                    match self.buffer.line(task.target_line) {
                        Some(line) => !line.contains(word.as_str()),
                        None => true,
                    }
                }
                TaskKind::ChangeWord { new_text, .. } => {
                    // Completed when the line contains the new_text
                    match self.buffer.line(task.target_line) {
                        Some(line) => line.contains(new_text.as_str()),
                        None => false,
                    }
                }
                TaskKind::ReplaceChar { expected } => {
                    // Completed when the char at position matches expected
                    self.buffer
                        .char_at(task.target_line, task.target_col)
                        .map(|ch| ch == *expected)
                        .unwrap_or(false)
                }
                TaskKind::ChangeInside { new_text, .. } => {
                    // Completed when the line contains the new_text
                    match self.buffer.line(task.target_line) {
                        Some(line) => line.contains(new_text.as_str()),
                        None => false,
                    }
                }
                TaskKind::YankPaste { expected_text } => {
                    // Completed when the target line contains the expected text
                    match self.buffer.line(task.target_line) {
                        Some(line) => line.contains(expected_text.as_str()),
                        None => false,
                    }
                }
                TaskKind::DeleteBlock { original_lines } => {
                    // Completed when none of the original lines exist at their positions
                    original_lines.iter().enumerate().all(|(i, orig)| {
                        let line_idx = task.target_line + i;
                        match self.buffer.line(line_idx) {
                            Some(line) => line.trim() != orig.trim(),
                            None => true,
                        }
                    })
                }
                TaskKind::Indent { expected_indent } => {
                    // Completed when the line starts with the expected indentation
                    match self.buffer.line(task.target_line) {
                        Some(line) => line.starts_with(expected_indent.as_str()),
                        None => false,
                    }
                }
            };
            if completed {
                task.mark_completed();
                self.scoring.complete_task(task.points);
            }
        }
    }
}
