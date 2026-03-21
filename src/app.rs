use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use crate::vim::buffer::Buffer;
use crate::vim::command::{self, Action};
use crate::vim::cursor::Cursor;
use crate::vim::mode::Mode;

const SAMPLE_CODE: &str = r#"fn main() {
    let greeting = "Hello, Vim Heroes!";
    println!("{}", greeting);

    let numbers = vec![1, 2, 3, 4, 5];
    let total: i32 = numbers.iter().sum();
    println!("Total: {}", total);

    for i in 0..10 {
        if i % 2 == 0 {
            println!("{} is even", i);
        } else {
            println!("{} is odd", i);
        }
    }

    let message = format!(
        "The sum of 1..5 is {}",
        total
    );
    println!("{}", message);
}"#;

pub struct App {
    pub buffer: Buffer,
    pub cursor: Cursor,
    pub mode: Mode,
    pub running: bool,
    pub keystroke_count: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            buffer: Buffer::from_str(SAMPLE_CODE),
            cursor: Cursor::new(0, 0),
            mode: Mode::Normal,
            running: true,
            keystroke_count: 0,
        }
    }

    /// Poll for input and process it. Returns true if a frame should be rendered.
    pub fn handle_input(&mut self) -> bool {
        // Poll with a short timeout so the UI stays responsive
        if !event::poll(Duration::from_millis(50)).unwrap_or(false) {
            return false;
        }

        let event = match event::read() {
            Ok(ev) => ev,
            Err(_) => return false,
        };

        match event {
            Event::Key(key_event) => self.handle_key(key_event),
            _ => false,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Ctrl-c always quits
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
