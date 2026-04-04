use std::time::{Duration, Instant};

/// Default time limit per task (seconds).
const DEFAULT_TIMER_SECS: f64 = 20.0;

/// Energy mode determines how the survival mechanic works.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyMode {
    /// Classic timer-based: countdown in real time, resets on task completion.
    Timer,
    /// Motion-count-based (World 1): each motion costs 1, resets on target.
    /// Budget is set per-target from BFS optimal path + buffer.
    MotionCount,
}

/// A countdown timer — the core survival mechanic.
///
/// In Timer mode: counts down in real time. Completing a task resets it.
/// In MotionCount mode: each motion costs 1 energy. Budget is set per target.
pub struct Energy {
    // --- Timer mode fields ---
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

    // --- MotionCount mode fields ---
    /// Current energy mode.
    pub mode: EnergyMode,
    /// Current motion count budget (MotionCount mode).
    pub motion_budget: usize,
    /// Motions used so far for the current target.
    pub motions_used: usize,
    /// Errors committed (motions that didn't move closer to target).
    pub errors: usize,
    /// Maximum errors allowed before death.
    pub max_errors: usize,
}

impl Energy {
    pub fn new(max_seconds: f64) -> Self {
        Self {
            max_seconds,
            last_reset: Instant::now(),
            paused_duration: Duration::ZERO,
            paused: true, // Start paused (countdown phase)
            last_restore: None,
            mode: EnergyMode::Timer,
            motion_budget: 0,
            motions_used: 0,
            errors: 0,
            max_errors: 10,
        }
    }

    /// Create with default timer duration.
    pub fn default_new() -> Self {
        Self::new(DEFAULT_TIMER_SECS)
    }

    /// Create a motion-count energy system (World 1).
    pub fn new_motion_count(max_errors: usize) -> Self {
        Self {
            max_seconds: 0.0,
            last_reset: Instant::now(),
            paused_duration: Duration::ZERO,
            paused: false,
            last_restore: None,
            mode: EnergyMode::MotionCount,
            motion_budget: 0,
            motions_used: 0,
            errors: 0,
            max_errors,
        }
    }

    // --- Motion count methods ---

    /// Set the energy budget for the next target.
    pub fn set_budget(&mut self, budget: usize) {
        self.motion_budget = budget;
        self.motions_used = 0;
    }

    /// Use one motion. Returns true if still alive.
    pub fn use_motion(&mut self) -> bool {
        self.motions_used += 1;
        !self.is_depleted()
    }

    /// Record an error (motion that didn't move closer to target).
    /// Returns true if still alive.
    pub fn record_error(&mut self) -> bool {
        self.errors += 1;
        self.errors <= self.max_errors
    }

    /// Check if energy is depleted based on current mode.
    pub fn is_over_budget(&self) -> bool {
        if self.mode == EnergyMode::MotionCount {
            self.motion_budget > 0 && self.motions_used > self.motion_budget
        } else {
            false
        }
    }

    /// Check if errors exceeded max allowed.
    pub fn errors_exceeded(&self) -> bool {
        self.errors > self.max_errors
    }

    /// Energy remaining as a fraction (for MotionCount bar display).
    pub fn motion_fraction(&self) -> f64 {
        if self.motion_budget == 0 {
            return 1.0;
        }
        let remaining = self.motion_budget.saturating_sub(self.motions_used);
        remaining as f64 / self.motion_budget as f64
    }

    /// Reset energy for next target in MotionCount mode.
    pub fn reset_for_target(&mut self, budget: usize) {
        self.motion_budget = budget;
        self.motions_used = 0;
    }

    // --- Timer mode methods ---

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
        match self.mode {
            EnergyMode::Timer => self.remaining_seconds() <= 0.0,
            EnergyMode::MotionCount => {
                self.errors_exceeded()
            }
        }
    }

    /// Current timer as a percentage (0.0 to 100.0).
    pub fn percentage(&self) -> f64 {
        match self.mode {
            EnergyMode::Timer => {
                if self.max_seconds <= 0.0 {
                    return 0.0;
                }
                (self.remaining_seconds() / self.max_seconds * 100.0).clamp(0.0, 100.0)
            }
            EnergyMode::MotionCount => {
                (self.motion_fraction() * 100.0).clamp(0.0, 100.0)
            }
        }
    }

    /// Start the timer (after countdown finishes).
    pub fn start(&mut self) {
        self.paused = false;
        self.paused_duration = Duration::ZERO;
        self.last_reset = Instant::now();
    }

    /// Reset the timer to full (on task completion).
    pub fn restore_task(&mut self) {
        match self.mode {
            EnergyMode::Timer => {
                let remaining = self.remaining_seconds();
                self.last_restore = Some(self.max_seconds - remaining);
                self.paused_duration = Duration::ZERO;
                self.last_reset = Instant::now();
            }
            EnergyMode::MotionCount => {
                // Budget will be set separately via set_budget / reset_for_target
                self.motions_used = 0;
            }
        }
    }

    /// Reset everything (for new level / restart).
    pub fn reset(&mut self) {
        self.paused_duration = Duration::ZERO;
        self.last_reset = Instant::now();
        self.paused = true;
        self.last_restore = None;
        self.motions_used = 0;
        self.motion_budget = 0;
        self.errors = 0;
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

    #[test]
    fn test_motion_count_basic() {
        let mut e = Energy::new_motion_count(3);
        e.set_budget(5);
        assert_eq!(e.motions_used, 0);
        assert!(!e.is_depleted());

        e.use_motion();
        e.use_motion();
        assert_eq!(e.motions_used, 2);
        assert!(!e.is_depleted());
    }

    #[test]
    fn test_motion_count_errors() {
        let mut e = Energy::new_motion_count(2);
        e.set_budget(10);
        assert!(e.record_error()); // 1 error, max 2: alive
        assert!(e.record_error()); // 2 errors: alive
        assert!(!e.record_error()); // 3 errors: dead
        assert!(e.is_depleted());
    }

    #[test]
    fn test_motion_count_reset_for_target() {
        let mut e = Energy::new_motion_count(5);
        e.set_budget(3);
        e.use_motion();
        e.use_motion();
        assert_eq!(e.motions_used, 2);
        e.reset_for_target(5);
        assert_eq!(e.motions_used, 0);
        assert_eq!(e.motion_budget, 5);
    }
}
