use std::time::{Duration, Instant};

/// Default time limit per task (seconds).
const DEFAULT_TIMER_SECS: f64 = 20.0;

/// A countdown timer — the core survival mechanic.
///
/// The timer counts down in real time. Completing a task resets it.
/// If the timer reaches 0 the game is over.
pub struct Energy {
    /// Maximum seconds on the clock (reset value).
    pub max_seconds: f64,
    /// When the timer was last reset (game start or task completion).
    last_reset: Instant,
    /// Paused duration accumulator (for countdown phase).
    paused_duration: Duration,
    /// Whether the timer is currently paused.
    paused: bool,
    /// Last restore info for "+time" popup display.
    pub last_restore: Option<f64>,
}

impl Energy {
    pub fn new(max_seconds: f64) -> Self {
        Self {
            max_seconds,
            last_reset: Instant::now(),
            paused_duration: Duration::ZERO,
            paused: true, // Start paused (countdown phase)
            last_restore: None,
        }
    }

    /// Create with default timer duration.
    pub fn default_new() -> Self {
        Self::new(DEFAULT_TIMER_SECS)
    }

    /// Seconds remaining on the timer.
    pub fn remaining_seconds(&self) -> f64 {
        let elapsed = if self.paused {
            self.paused_duration
        } else {
            self.paused_duration + self.last_reset.elapsed()
        };
        (self.max_seconds - elapsed.as_secs_f64()).max(0.0)
    }

    /// Is the timer depleted (game over)?
    pub fn is_depleted(&self) -> bool {
        self.remaining_seconds() <= 0.0
    }

    /// Current timer as a percentage (0.0 to 100.0).
    pub fn percentage(&self) -> f64 {
        if self.max_seconds <= 0.0 {
            return 0.0;
        }
        (self.remaining_seconds() / self.max_seconds * 100.0).clamp(0.0, 100.0)
    }

    /// Start the timer (after countdown finishes).
    pub fn start(&mut self) {
        self.paused = false;
        self.paused_duration = Duration::ZERO;
        self.last_reset = Instant::now();
    }

    /// Reset the timer to full (on task completion).
    pub fn restore_task(&mut self) {
        let remaining = self.remaining_seconds();
        self.last_restore = Some(self.max_seconds - remaining);
        self.paused_duration = Duration::ZERO;
        self.last_reset = Instant::now();
    }

    /// Reset everything (for new level / restart).
    pub fn reset(&mut self) {
        self.paused_duration = Duration::ZERO;
        self.last_reset = Instant::now();
        self.paused = true;
        self.last_restore = None;
    }

    /// Pause the timer (for practice mode).
    pub fn pause(&mut self) {
        if !self.paused {
            self.paused_duration += self.last_reset.elapsed();
            self.paused = true;
        }
    }

    /// Resume the timer (leaving practice mode).
    pub fn resume(&mut self) {
        if self.paused {
            self.paused = false;
            self.last_reset = Instant::now();
        }
    }

    /// Whether the timer is currently paused.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Clear the last restore popup.
    pub fn clear_restore_popup(&mut self) {
        self.last_restore = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_initial_state() {
        let e = Energy::default_new();
        assert!((e.remaining_seconds() - DEFAULT_TIMER_SECS).abs() < 0.1);
        assert!((e.percentage() - 100.0).abs() < 1.0);
        assert!(!e.is_depleted());
    }

    #[test]
    fn test_timer_stays_paused() {
        let e = Energy::new(1.0); // 1 second timer, paused
        thread::sleep(Duration::from_millis(50));
        // Should still be full because paused
        assert!((e.remaining_seconds() - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_timer_drains_when_started() {
        let mut e = Energy::new(2.0);
        e.start();
        thread::sleep(Duration::from_millis(100));
        assert!(e.remaining_seconds() < 2.0);
        assert!(!e.is_depleted());
    }

    #[test]
    fn test_restore_resets_timer() {
        let mut e = Energy::new(2.0);
        e.start();
        thread::sleep(Duration::from_millis(100));
        let before = e.remaining_seconds();
        e.restore_task();
        assert!(e.remaining_seconds() > before);
        assert!((e.remaining_seconds() - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_depleted() {
        let mut e = Energy::new(0.05); // 50ms timer
        e.start();
        thread::sleep(Duration::from_millis(100));
        assert!(e.is_depleted());
    }

    #[test]
    fn test_reset() {
        let mut e = Energy::new(2.0);
        e.start();
        thread::sleep(Duration::from_millis(100));
        e.reset();
        assert!((e.remaining_seconds() - 2.0).abs() < 0.1);
    }
}
