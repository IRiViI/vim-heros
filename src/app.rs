use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use crate::game::engine::{Engine, GameState};
use crate::game::viewport::Viewport;
use crate::vim::buffer::Buffer;
use crate::vim::command::{self, Action};
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

pub struct App {
    pub buffer: Buffer,
    pub cursor: Cursor,
    pub mode: Mode,
    pub running: bool,
    pub keystroke_count: usize,
    pub viewport: Viewport,
    pub engine: Engine,
}

impl App {
    pub fn new(viewport_height: usize) -> Self {
        Self {
            buffer: Buffer::from_str(SAMPLE_CODE),
            cursor: Cursor::new(0, 0),
            mode: Mode::Normal,
            running: true,
            keystroke_count: 0,
            viewport: Viewport::new(viewport_height),
            engine: Engine::new(DEFAULT_SCROLL_SPEED_MS),
        }
    }

    /// Update viewport height when terminal is resized.
    pub fn update_viewport_height(&mut self, terminal_height: usize) {
        // terminal_height minus 2 (borders) minus 1 (status bar)
        self.viewport.height = terminal_height.saturating_sub(3);
    }

    /// Main tick: poll for input, handle scroll, check game over.
    /// Returns true if a frame should be rendered.
    pub fn tick(&mut self) -> bool {
        match self.engine.state {
            GameState::Playing => self.tick_playing(),
            GameState::GameOver => self.tick_game_over(),
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
                    self.engine.award_survival_points();
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
                self.engine.award_survival_points();
            }
            self.engine.record_scroll();
            needs_render = true;

            // Game over: cursor scrolled above viewport
            if self.cursor.line < self.viewport.top_line {
                self.engine.state = GameState::GameOver;
            }
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
        self.buffer = Buffer::from_str(SAMPLE_CODE);
        self.cursor = Cursor::new(0, 0);
        self.mode = Mode::Normal;
        self.keystroke_count = 0;
        self.viewport = Viewport::new(self.viewport.height);
        self.engine.reset();
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.running = false;
            return true;
        }

        let action = match self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::Insert => self.handle_insert_key(key),
        };

        if action != Action::None {
            self.keystroke_count += 1;
            self.engine.penalize_keystroke();
            command::execute(action, &mut self.buffer, &mut self.cursor, &mut self.mode);
        }

        true
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') => {
                self.running = false;
                Action::None
            }
            KeyCode::Char(ch) => command::parse_keystroke(ch, Mode::Normal),
            KeyCode::Esc => Action::None,
            _ => Action::None,
        }
    }

    fn handle_insert_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => Action::EnterNormalMode,
            KeyCode::Char(ch) => Action::InsertChar(ch),
            KeyCode::Enter => Action::InsertChar('\n'),
            KeyCode::Backspace => Action::Backspace,
            _ => Action::None,
        }
    }
}
