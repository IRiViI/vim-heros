use std::time::{Duration, Instant};

/// Current state of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Playing,
    GameOver,
}

/// Drives scroll ticking, scoring, and game state.
pub struct Engine {
    pub state: GameState,
    pub score: i64,
    pub scroll_speed: Duration,
    last_scroll: Instant,
    start_time: Instant,
}

impl Engine {
    pub fn new(scroll_speed_ms: u64) -> Self {
        let now = Instant::now();
        Self {
            state: GameState::Playing,
            score: 0,
            scroll_speed: Duration::from_millis(scroll_speed_ms),
            last_scroll: now,
            start_time: now,
        }
    }

    /// Time remaining until the next scroll tick.
    pub fn time_until_next_scroll(&self) -> Duration {
        let elapsed = self.last_scroll.elapsed();
        self.scroll_speed.saturating_sub(elapsed)
    }

    /// Returns true if a scroll tick is due.
    pub fn should_scroll(&self) -> bool {
        self.last_scroll.elapsed() >= self.scroll_speed
    }

    /// Mark that a scroll tick occurred.
    pub fn record_scroll(&mut self) {
        self.last_scroll = Instant::now();
    }

    /// Add survival points (called each scroll tick).
    pub fn award_survival_points(&mut self) {
        self.score += 10;
    }

    /// Deduct keystroke penalty.
    pub fn penalize_keystroke(&mut self) {
        self.score -= 2;
    }

    /// Seconds elapsed since game start.
    pub fn elapsed_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Reset the engine for a new game.
    pub fn reset(&mut self) {
        let now = Instant::now();
        self.state = GameState::Playing;
        self.score = 0;
        self.last_scroll = now;
        self.start_time = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_new_engine() {
        let engine = Engine::new(2000);
        assert_eq!(engine.state, GameState::Playing);
        assert_eq!(engine.score, 0);
        assert!(!engine.should_scroll());
    }

    #[test]
    fn test_scoring() {
        let mut engine = Engine::new(2000);
        engine.award_survival_points();
        assert_eq!(engine.score, 10);
        engine.penalize_keystroke();
        assert_eq!(engine.score, 8);
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
        assert_eq!(engine.state, GameState::Playing);
        assert_eq!(engine.score, 0);
    }
}
