use serde::Deserialize;

/// Difficulty multipliers for energy drain and restore rates.
#[derive(Debug, Clone, Deserialize)]
pub struct DifficultyMultiplier {
    pub drain: f64,
    pub restore: f64,
}

/// Configuration for the energy system, loaded from TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct EnergyConfig {
    pub max: f64,
    pub start: f64,
    pub drain: DrainConfig,
    pub restore: RestoreConfig,
    pub difficulty_multipliers: DifficultyMultipliers,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DrainConfig {
    pub keystroke_base: f64,
    pub time_drain_per_tick: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RestoreConfig {
    pub task_complete: f64,
    pub task_optimal: f64,
    pub combo_bonus: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DifficultyMultipliers {
    pub nano_user: DifficultyMultiplier,
    pub wq_survivor: DifficultyMultiplier,
    pub keyboard_warrior: DifficultyMultiplier,
    pub ten_x_engineer: DifficultyMultiplier,
    pub uses_arch_btw: DifficultyMultiplier,
}

impl Default for EnergyConfig {
    fn default() -> Self {
        Self {
            max: 100.0,
            start: 100.0,
            drain: DrainConfig {
                keystroke_base: 1.0,
                time_drain_per_tick: 0.5,
            },
            restore: RestoreConfig {
                task_complete: 15.0,
                task_optimal: 25.0,
                combo_bonus: 5.0,
            },
            difficulty_multipliers: DifficultyMultipliers {
                nano_user: DifficultyMultiplier { drain: 0.5, restore: 1.5 },
                wq_survivor: DifficultyMultiplier { drain: 0.75, restore: 1.25 },
                keyboard_warrior: DifficultyMultiplier { drain: 1.0, restore: 1.0 },
                ten_x_engineer: DifficultyMultiplier { drain: 1.5, restore: 0.75 },
                uses_arch_btw: DifficultyMultiplier { drain: 2.0, restore: 0.5 },
            },
        }
    }
}

/// Difficulty level selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    NanoUser,
    WqSurvivor,
    KeyboardWarrior,
    TenXEngineer,
    UsesArchBtw,
}

impl Difficulty {
    pub fn name(&self) -> &'static str {
        match self {
            Difficulty::NanoUser => "Nano User",
            Difficulty::WqSurvivor => ":wq Survivor",
            Difficulty::KeyboardWarrior => "Keyboard Warrior",
            Difficulty::TenXEngineer => "10x Engineer",
            Difficulty::UsesArchBtw => "Uses Arch btw",
        }
    }

    fn multiplier<'a>(&self, config: &'a EnergyConfig) -> &'a DifficultyMultiplier {
        match self {
            Difficulty::NanoUser => &config.difficulty_multipliers.nano_user,
            Difficulty::WqSurvivor => &config.difficulty_multipliers.wq_survivor,
            Difficulty::KeyboardWarrior => &config.difficulty_multipliers.keyboard_warrior,
            Difficulty::TenXEngineer => &config.difficulty_multipliers.ten_x_engineer,
            Difficulty::UsesArchBtw => &config.difficulty_multipliers.uses_arch_btw,
        }
    }
}

/// The energy bar — core survival mechanic.
///
/// Energy drains on every keystroke and every scroll tick.
/// Completing tasks restores energy. Hit 0 = game over.
pub struct Energy {
    pub current: f64,
    pub max: f64,
    config: EnergyConfig,
    difficulty: Difficulty,
    /// Last energy restore amount, for "+N" popup display.
    pub last_restore: Option<f64>,
}

impl Energy {
    pub fn new(config: EnergyConfig, difficulty: Difficulty) -> Self {
        let max = config.max;
        let start = config.start.min(max);
        Self {
            current: start,
            max,
            config,
            difficulty,
            last_restore: None,
        }
    }

    /// Create with default config and keyboard warrior difficulty.
    pub fn default_new() -> Self {
        Self::new(EnergyConfig::default(), Difficulty::KeyboardWarrior)
    }

    /// Drain energy for a keystroke. Returns the amount drained.
    pub fn drain_keystroke(&mut self) -> f64 {
        let mult = self.difficulty.multiplier(&self.config);
        let drain = self.config.drain.keystroke_base * mult.drain;
        self.current = (self.current - drain).max(0.0);
        self.last_restore = None;
        drain
    }

    /// Drain energy for a scroll tick (passive time drain).
    pub fn drain_tick(&mut self) -> f64 {
        let mult = self.difficulty.multiplier(&self.config);
        let drain = self.config.drain.time_drain_per_tick * mult.drain;
        self.current = (self.current - drain).max(0.0);
        drain
    }

    /// Restore energy on task completion.
    /// `optimal` = true if the task was completed within optimal keystrokes.
    /// `combo` = current combo count (for combo bonus).
    pub fn restore_task(&mut self, optimal: bool, combo: usize) -> f64 {
        let mult = self.difficulty.multiplier(&self.config);
        let base = if optimal {
            self.config.restore.task_optimal
        } else {
            self.config.restore.task_complete
        };
        let combo_bonus = self.config.restore.combo_bonus * combo as f64;
        let restore = (base + combo_bonus) * mult.restore;
        self.current = (self.current + restore).min(self.max);
        self.last_restore = Some(restore);
        restore
    }

    /// Is the energy depleted (game over)?
    pub fn is_depleted(&self) -> bool {
        self.current <= 0.0
    }

    /// Current energy as a percentage (0.0 to 100.0).
    pub fn percentage(&self) -> f64 {
        if self.max <= 0.0 {
            return 0.0;
        }
        (self.current / self.max) * 100.0
    }

    /// Reset energy to full (for new level).
    pub fn reset(&mut self) {
        self.current = self.config.start.min(self.max);
        self.last_restore = None;
    }

    /// Clear the last restore popup.
    pub fn clear_restore_popup(&mut self) {
        self.last_restore = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_energy() -> Energy {
        Energy::new(EnergyConfig::default(), Difficulty::KeyboardWarrior)
    }

    #[test]
    fn test_initial_state() {
        let e = test_energy();
        assert_eq!(e.current, 100.0);
        assert_eq!(e.max, 100.0);
        assert_eq!(e.percentage(), 100.0);
        assert!(!e.is_depleted());
    }

    #[test]
    fn test_drain_keystroke() {
        let mut e = test_energy();
        // KeyboardWarrior has drain multiplier 1.0, keystroke_base = 1.0
        e.drain_keystroke();
        assert_eq!(e.current, 99.0);
        assert_eq!(e.percentage(), 99.0);
    }

    #[test]
    fn test_drain_tick() {
        let mut e = test_energy();
        // KeyboardWarrior drain 1.0, time_drain_per_tick = 0.5
        e.drain_tick();
        assert_eq!(e.current, 99.5);
    }

    #[test]
    fn test_restore_task_normal() {
        let mut e = test_energy();
        e.current = 50.0;
        // task_complete = 15.0, combo = 0, restore mult = 1.0
        let restored = e.restore_task(false, 0);
        assert_eq!(restored, 15.0);
        assert_eq!(e.current, 65.0);
    }

    #[test]
    fn test_restore_task_optimal() {
        let mut e = test_energy();
        e.current = 50.0;
        // task_optimal = 25.0, combo = 0, restore mult = 1.0
        let restored = e.restore_task(true, 0);
        assert_eq!(restored, 25.0);
        assert_eq!(e.current, 75.0);
    }

    #[test]
    fn test_restore_with_combo() {
        let mut e = test_energy();
        e.current = 50.0;
        // task_complete = 15.0, combo_bonus = 5.0 * 3 = 15.0, total = 30.0
        let restored = e.restore_task(false, 3);
        assert_eq!(restored, 30.0);
        assert_eq!(e.current, 80.0);
    }

    #[test]
    fn test_restore_capped_at_max() {
        let mut e = test_energy();
        e.current = 95.0;
        let restored = e.restore_task(true, 5); // would be 25 + 25 = 50
        assert_eq!(e.current, 100.0); // capped at max
        assert_eq!(restored, 50.0); // restore amount before cap
    }

    #[test]
    fn test_drain_capped_at_zero() {
        let mut e = test_energy();
        e.current = 0.5;
        e.drain_keystroke(); // drains 1.0, but capped at 0
        assert_eq!(e.current, 0.0);
        assert!(e.is_depleted());
    }

    #[test]
    fn test_difficulty_nano_user() {
        let mut e = Energy::new(EnergyConfig::default(), Difficulty::NanoUser);
        // drain mult = 0.5, so keystroke drains 0.5
        e.drain_keystroke();
        assert_eq!(e.current, 99.5);
        // restore mult = 1.5, so task_complete = 15 * 1.5 = 22.5
        e.current = 50.0;
        let restored = e.restore_task(false, 0);
        assert_eq!(restored, 22.5);
    }

    #[test]
    fn test_difficulty_uses_arch_btw() {
        let mut e = Energy::new(EnergyConfig::default(), Difficulty::UsesArchBtw);
        // drain mult = 2.0, so keystroke drains 2.0
        e.drain_keystroke();
        assert_eq!(e.current, 98.0);
        // restore mult = 0.5, so task_complete = 15 * 0.5 = 7.5
        e.current = 50.0;
        let restored = e.restore_task(false, 0);
        assert_eq!(restored, 7.5);
    }

    #[test]
    fn test_reset() {
        let mut e = test_energy();
        e.current = 10.0;
        e.last_restore = Some(15.0);
        e.reset();
        assert_eq!(e.current, 100.0);
        assert!(e.last_restore.is_none());
    }

    #[test]
    fn test_last_restore_popup() {
        let mut e = test_energy();
        e.current = 50.0;
        e.restore_task(false, 0);
        assert_eq!(e.last_restore, Some(15.0));
        e.drain_keystroke();
        assert!(e.last_restore.is_none()); // cleared on keystroke
    }
}
