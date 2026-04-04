use std::time::{Duration, Instant};

const COUNTDOWN_SECS: u64 = 3;

/// Current state of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    /// Classic countdown (3, 2, 1, GO!)
    Countdown,
    /// World 1 mode: waiting for the player's first keystroke to start.
    WaitingForInput,
    Playing,
    GameOver,
    LevelComplete,
}

/// Drives scroll ticking, scoring, and game state.
pub struct Engine {
    pub state: GameState,
    pub score: i64,
    pub scroll_speed: Duration,
    /// When true, scroll speed is 4x faster (target not in view).
    pub catching_up: bool,
    last_scroll: Instant,
    start_time: Instant,
    countdown_start: Instant,
}

impl Engine {
    pub fn new(scroll_speed_ms: u64) -> Self {
        let now = Instant::now();
        Self {
            state: GameState::Countdown,
            score: 0,
            scroll_speed: Duration::from_millis(scroll_speed_ms),
            catching_up: false,
            last_scroll: now,
            start_time: now,
            countdown_start: now,
        }
    }

    /// Create an engine in WaitingForInput mode (World 1: start on first keystroke).
    pub fn new_waiting(scroll_speed_ms: u64) -> Self {
        let now = Instant::now();
        Self {
            state: GameState::WaitingForInput,
            score: 0,
            scroll_speed: Duration::from_millis(scroll_speed_ms),
            catching_up: false,
            last_scroll: now,
            start_time: now,
            countdown_start: now,
        }
    }

    /// Seconds remaining in the countdown (0 if not in countdown).
    pub fn countdown_remaining(&self) -> u64 {
        if self.state != GameState::Countdown {
            return 0;
        }
        let elapsed = self.countdown_start.elapsed().as_secs();
        COUNTDOWN_SECS.saturating_sub(elapsed)
    }

    /// Check if countdown is finished; if so, transition to Playing.
    /// Returns true if state changed.
    pub fn check_countdown(&mut self) -> bool {
        if self.state == GameState::Countdown
            && self.countdown_start.elapsed() >= Duration::from_secs(COUNTDOWN_SECS)
        {
            self.state = GameState::Playing;
            let now = Instant::now();
            self.last_scroll = now;
            self.start_time = now;
            true
        } else {
            false
        }
    }

    /// Transition from WaitingForInput to Playing (on first keystroke).
    pub fn start_on_input(&mut self) {
        if self.state == GameState::WaitingForInput {
            self.state = GameState::Playing;
            let now = Instant::now();
            self.last_scroll = now;
            self.start_time = now;
        }
    }

    /// Current effective scroll speed, accounting for catch-up mode (4x).
    fn effective_scroll_speed(&self) -> Duration {
        if self.catching_up {
            self.scroll_speed / 4
        } else {
            self.scroll_speed
        }
    }

    /// Time remaining until the next scroll tick.
    pub fn time_until_next_scroll(&self) -> Duration {
        let elapsed = self.last_scroll.elapsed();
        self.effective_scroll_speed().saturating_sub(elapsed)
    }

    /// Returns true if a scroll tick is due.
    pub fn should_scroll(&self) -> bool {
        self.last_scroll.elapsed() >= self.effective_scroll_speed()
    }

    /// Mark that a scroll tick occurred.
    pub fn record_scroll(&mut self) {
        self.last_scroll = Instant::now();
    }

    /// Seconds elapsed since game start.
    pub fn elapsed_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Reset the engine for a new game (classic countdown mode).
    pub fn reset(&mut self) {
        let now = Instant::now();
        self.state = GameState::Countdown;
        self.score = 0;
        self.catching_up = false;
        self.last_scroll = now;
        self.start_time = now;
        self.countdown_start = now;
    }

    /// Reset the engine in WaitingForInput mode (World 1).
    pub fn reset_waiting(&mut self) {
        let now = Instant::now();
        self.state = GameState::WaitingForInput;
        self.score = 0;
        self.catching_up = false;
        self.last_scroll = now;
        self.start_time = now;
        self.countdown_start = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_new_engine() {
        let engine = Engine::new(2000);
        assert_eq!(engine.state, GameState::Countdown);
        assert_eq!(engine.score, 0);
    }

    #[test]
    fn test_new_waiting() {
        let engine = Engine::new_waiting(2000);
        assert_eq!(engine.state, GameState::WaitingForInput);
    }

    #[test]
    fn test_start_on_input() {
        let mut engine = Engine::new_waiting(2000);
        engine.start_on_input();
        assert_eq!(engine.state, GameState::Playing);
    }

    #[test]
    fn test_should_scroll_after_delay() {
        let mut engine = Engine::new(10); // 10ms
        thread::sleep(Duration::from_millis(15));
        assert!(engine.should_scroll());
        engine.record_scroll();
        assert!(!engine.should_scroll());
    }

    #[test]
    fn test_reset() {
        let mut engine = Engine::new(2000);
        engine.score = 500;
        engine.state = GameState::GameOver;
        engine.reset();
        assert_eq!(engine.state, GameState::Countdown);
        assert_eq!(engine.score, 0);
    }

    #[test]
    fn test_reset_waiting() {
        let mut engine = Engine::new(2000);
        engine.state = GameState::GameOver;
        engine.reset_waiting();
        assert_eq!(engine.state, GameState::WaitingForInput);
    }
}
