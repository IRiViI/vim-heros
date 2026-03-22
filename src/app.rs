use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
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

/// The default (and currently only) level.
fn default_level() -> LevelInfo {
    LevelInfo {
        world: 1,
        level: 1,
        name: "First Steps".to_string(),
        zone: "starter".to_string(),
        language: "python".to_string(),
        scroll_speed_ms: DEFAULT_SCROLL_SPEED_MS,
    }
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
            GameState::Playing => self.tick_playing(),
            GameState::GameOver | GameState::LevelComplete => self.tick_game_over(),
        }
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
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.running = false;
            return true;
        }

        match self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::Insert => self.handle_insert_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.parser.cancel();
                true
            }
            KeyCode::Char(ch) => {
                // 'q' quits the game, but only when not in a pending sequence
                if ch == 'q' && !self.parser.is_pending() {
                    self.running = false;
                    return true;
                }

                                self.scoring.penalize_keystroke();

                match self.parser.feed(ch) {
                    ParseResult::Action(action, count) => {
                        for _ in 0..count {
                            command::execute(
                                action,
                                &mut self.buffer,
                                &mut self.cursor,
                                &mut self.mode,
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
            KeyCode::Esc => Action::EnterNormalMode,
            KeyCode::Char(ch) => Action::InsertChar(ch),
            KeyCode::Enter => Action::InsertChar('\n'),
            KeyCode::Backspace => Action::Backspace,
            _ => return false,
        };

                self.scoring.penalize_keystroke();
        command::execute(action, &mut self.buffer, &mut self.cursor, &mut self.mode);
        self.check_task_completion();
        true
    }

    fn check_task_completion(&mut self) {
        for task in &mut self.tasks {
            if !task.is_completable() {
                continue;
            }
            let completed = match task.kind {
                TaskKind::MoveTo => {
                    self.cursor.line == task.target_line
                        && self.cursor.col == task.target_col
                }
            };
            if completed {
                task.mark_completed();
                self.scoring.complete_task(task.points);
            }
        }
    }
}
