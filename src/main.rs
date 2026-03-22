mod app;
mod content;
mod game;
mod ui;
mod vim;

use std::io;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;

fn main() -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let result = run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {err}");
    }

    Ok(())
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let size = terminal.size()?;
    let viewport_height = size.height.saturating_sub(3) as usize; // borders + status bar
    let mut app = App::new(viewport_height);

    // Initial render
    terminal.draw(|frame| ui::game_view::render(frame, &app))?;

    while app.running {
        if app.tick() {
            // Update viewport height in case terminal was resized
            let size = terminal.size()?;
            app.update_viewport_height(size.height as usize);
            terminal.draw(|frame| ui::game_view::render(frame, &app))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use crate::vim::buffer::Buffer;
    use crate::vim::command::{self, Action, CommandParser, ParseResult};
    use crate::vim::cursor::Cursor;
    use crate::vim::mode::Mode;

    /// Helper: feed a sequence of keystrokes through the engine.
    fn feed_keys(
        keys: &str,
        buffer: &mut Buffer,
        cursor: &mut Cursor,
        mode: &mut Mode,
    ) -> usize {
        let mut keystroke_count = 0;
        let mut parser = CommandParser::new();
        for ch in keys.chars() {
            keystroke_count += 1;
            if mode.is_insert() {
                let action = match ch {
                    '\x1b' => Action::EnterNormalMode,
                    _ => Action::InsertChar(ch),
                };
                command::execute(action, buffer, cursor, mode);
            } else {
                if let ParseResult::Action(action, count) = parser.feed(ch) {
                    for _ in 0..count {
                        command::execute(action, buffer, cursor, mode);
                    }
                }
            }
        }
        keystroke_count
    }

    #[test]
    fn test_navigate_to_word() {
        let mut buf = Buffer::from_str("hello world");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        feed_keys("llllll", &mut buf, &mut cur, &mut mode);
        assert_eq!(cur, Cursor::new(0, 6));
    }

    #[test]
    fn test_navigate_down_and_across() {
        let mut buf = Buffer::from_str("line one\nline two\nline three");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        feed_keys("jjlllll", &mut buf, &mut cur, &mut mode);
        assert_eq!(cur, Cursor::new(2, 5));
    }

    #[test]
    fn test_insert_mode_round_trip() {
        let mut buf = Buffer::from_str("hllo world");
        let mut cur = Cursor::new(0, 1);
        let mut mode = Mode::Normal;

        feed_keys("ie\x1b", &mut buf, &mut cur, &mut mode);

        assert_eq!(mode, Mode::Normal);
        assert_eq!(buf.line(0), Some("hello world".to_string()));
        assert_eq!(cur.col, 2);
    }

    #[test]
    fn test_cursor_clamps_on_short_lines() {
        let mut buf = Buffer::from_str("long line here\nhi\nback to long");
        let mut cur = Cursor::new(0, 10);
        let mut mode = Mode::Normal;

        feed_keys("j", &mut buf, &mut cur, &mut mode);
        assert_eq!(cur, Cursor::new(1, 1));

        feed_keys("j", &mut buf, &mut cur, &mut mode);
        assert_eq!(cur, Cursor::new(2, 1));
    }

    #[test]
    fn test_cannot_move_past_buffer_bounds() {
        let mut buf = Buffer::from_str("only line");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        feed_keys("kkk", &mut buf, &mut cur, &mut mode);
        assert_eq!(cur, Cursor::new(0, 0));

        feed_keys("jjj", &mut buf, &mut cur, &mut mode);
        assert_eq!(cur, Cursor::new(0, 0));

        feed_keys("hhh", &mut buf, &mut cur, &mut mode);
        assert_eq!(cur, Cursor::new(0, 0));

        feed_keys("lllllllllllll", &mut buf, &mut cur, &mut mode);
        assert_eq!(cur, Cursor::new(0, 8));
    }

    #[test]
    fn test_insert_multiple_chars() {
        let mut buf = Buffer::from_str("fn()");
        let mut cur = Cursor::new(0, 3);
        let mut mode = Mode::Normal;

        feed_keys("ix, y\x1b", &mut buf, &mut cur, &mut mode);

        assert_eq!(buf.line(0), Some("fn(x, y)".to_string()));
        assert_eq!(mode, Mode::Normal);
    }

    #[test]
    fn test_keystroke_counting() {
        let mut buf = Buffer::from_str("hello\nworld");
        let mut cur = Cursor::new(0, 0);
        let mut mode = Mode::Normal;

        let count = feed_keys("jjlll", &mut buf, &mut cur, &mut mode);
        assert_eq!(count, 5);
    }
}
