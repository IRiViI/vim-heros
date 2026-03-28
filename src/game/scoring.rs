/// Tracks combo multiplier, score, and star calculation.
pub struct Scoring {
    pub score: i64,
    pub combo: usize,
    pub max_combo: usize,
    pub tasks_completed: usize,
    pub tasks_total: usize,
    /// Tasks completed at Great quality or better (Great + Perfect).
    pub tasks_great: usize,
    /// Tasks completed at Perfect quality (absolute optimal).
    pub tasks_perfect: usize,
    pub keystrokes: usize,
}

impl Scoring {
    pub fn new(tasks_total: usize) -> Self {
        Self {
            score: 0,
            combo: 0,
            max_combo: 0,
            tasks_completed: 0,
            tasks_total,
            tasks_great: 0,
            tasks_perfect: 0,
            keystrokes: 0,
        }
    }

    /// Current combo multiplier: 1.0x, 1.5x, 2.0x, or 3.0x.
    pub fn combo_multiplier(&self) -> f64 {
        match self.combo {
            0 | 1 => 1.0,
            2 => 1.5,
            3 => 2.0,
            _ => 3.0,
        }
    }

    /// Award points for completing a task. Applies combo multiplier.
    pub fn complete_task(&mut self, base_points: i64) {
        self.combo += 1;
        if self.combo > self.max_combo {
            self.max_combo = self.combo;
        }
        self.tasks_completed += 1;
        let points = (base_points as f64 * self.combo_multiplier()) as i64;
        self.score += points;
    }

    /// Award bonus for Great completion (within world-constrained optimal).
    pub fn award_great(&mut self) {
        self.tasks_great += 1;
        self.score += 50;
    }

    /// Award bonus for Perfect completion (absolute optimal).
    pub fn award_perfect(&mut self) {
        self.tasks_great += 1;
        self.tasks_perfect += 1;
        self.score += 100;
    }

    /// Break the combo (missed task or non-optimal completion).
    pub fn break_combo(&mut self) {
        self.combo = 0;
    }

    /// Penalize for a missed task.
    pub fn miss_task(&mut self) {
        self.combo = 0;
        self.score -= 50;
    }

    /// Add survival points (called each scroll tick).
    pub fn award_survival(&mut self) {
        self.score += 10;
    }

    /// Record a keystroke (for stats display). No score penalty — energy handles that.
    pub fn record_keystroke(&mut self) {
        self.keystrokes += 1;
    }

    /// Calculate star rating (0-3).
    /// 0 stars: didn't complete all tasks
    /// 1 star: completed all tasks
    /// 2 stars: completed all tasks + ≥40% optimal
    /// 3 stars: completed all tasks + all but 2 optimal
    pub fn stars(&self) -> usize {
        if self.tasks_total == 0 {
            return 1;
        }
        if self.tasks_completed < self.tasks_total {
            return 0;
        }
        let great_threshold_2 = (self.tasks_total as f64 * 0.4).ceil() as usize;
        let great_threshold_3 = self.tasks_total.saturating_sub(2);
        if self.tasks_great >= great_threshold_3 {
            3
        } else if self.tasks_great >= great_threshold_2 {
            2
        } else {
            1
        }
    }

    /// Whether this is a perfect run (all tasks completed at Perfect quality).
    pub fn is_perfect(&self) -> bool {
        self.tasks_total > 0
            && self.tasks_completed == self.tasks_total
            && self.tasks_perfect == self.tasks_total
    }

    /// Star display string: "★★☆" etc.
    pub fn star_display(&self) -> String {
        let filled = self.stars();
        let empty = 3 - filled;
        "\u{2605}".repeat(filled) + &"\u{2606}".repeat(empty)
    }

    /// Star display with PERFECT suffix for perfect runs.
    pub fn star_display_full(&self) -> String {
        let base = self.star_display();
        if self.is_perfect() {
            format!("{} PERFECT", base)
        } else {
            base
        }
    }

    /// Combo display string for HUD.
    pub fn combo_display(&self) -> String {
        let mult = self.combo_multiplier();
        if mult > 1.0 {
            format!("\u{00d7}{:.1}", mult)
        } else {
            String::new()
        }
    }

    pub fn reset(&mut self, tasks_total: usize) {
        *self = Self::new(tasks_total);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let s = Scoring::new(5);
        assert_eq!(s.score, 0);
        assert_eq!(s.combo, 0);
        assert_eq!(s.combo_multiplier(), 1.0);
        assert_eq!(s.stars(), 0);
    }

    #[test]
    fn test_combo_multiplier() {
        let mut s = Scoring::new(5);
        s.complete_task(100); // combo=1, mult=1.0, +100
        assert_eq!(s.combo_multiplier(), 1.0);
        assert_eq!(s.score, 100);

        s.complete_task(100); // combo=2, mult=1.5, +150
        assert_eq!(s.combo_multiplier(), 1.5);
        assert_eq!(s.score, 250);

        s.complete_task(100); // combo=3, mult=2.0, +200
        assert_eq!(s.combo_multiplier(), 2.0);
        assert_eq!(s.score, 450);

        s.complete_task(100); // combo=4, mult=3.0, +300
        assert_eq!(s.combo_multiplier(), 3.0);
        assert_eq!(s.score, 750);
    }

    #[test]
    fn test_combo_break() {
        let mut s = Scoring::new(5);
        s.complete_task(100);
        s.complete_task(100);
        assert_eq!(s.combo, 2);
        s.break_combo();
        assert_eq!(s.combo, 0);
        assert_eq!(s.combo_multiplier(), 1.0);
    }

    #[test]
    fn test_miss_task() {
        let mut s = Scoring::new(5);
        s.complete_task(100);
        s.miss_task();
        assert_eq!(s.combo, 0);
        assert_eq!(s.score, 50); // 100 - 50
    }

    #[test]
    fn test_stars_none_completed() {
        let s = Scoring::new(5);
        assert_eq!(s.stars(), 0);
    }

    #[test]
    fn test_stars_all_completed_no_good() {
        let mut s = Scoring::new(5);
        s.tasks_completed = 5;
        s.tasks_great = 1; // 1/5 = 20%, below 40%
        assert_eq!(s.stars(), 1);
    }

    #[test]
    fn test_stars_40_percent_good() {
        let mut s = Scoring::new(5);
        s.tasks_completed = 5;
        s.tasks_great = 2; // 2/5 = 40%
        assert_eq!(s.stars(), 2);
    }

    #[test]
    fn test_stars_all_but_two_good() {
        let mut s = Scoring::new(5);
        s.tasks_completed = 5;
        s.tasks_great = 3; // 5-2 = 3
        assert_eq!(s.stars(), 3);
    }

    #[test]
    fn test_stars_3_stars_not_perfect() {
        let mut s = Scoring::new(5);
        s.tasks_completed = 5;
        s.tasks_great = 5; // all good
        s.tasks_perfect = 3; // but only 3 perfect
        assert_eq!(s.stars(), 3);
        assert!(!s.is_perfect());
    }

    #[test]
    fn test_stars_perfect_run() {
        let mut s = Scoring::new(5);
        s.tasks_completed = 5;
        s.tasks_great = 5;
        s.tasks_perfect = 5; // all perfect
        assert_eq!(s.stars(), 3);
        assert!(s.is_perfect());
    }

    #[test]
    fn test_stars_partial_completion() {
        let mut s = Scoring::new(5);
        s.tasks_completed = 4; // not all
        s.tasks_great = 4;
        assert_eq!(s.stars(), 0);
        assert!(!s.is_perfect());
    }

    #[test]
    fn test_award_great_and_perfect() {
        let mut s = Scoring::new(5);
        s.award_great();
        assert_eq!(s.tasks_great, 1);
        assert_eq!(s.tasks_perfect, 0);
        s.award_perfect();
        assert_eq!(s.tasks_great, 2); // perfect also counts as great
        assert_eq!(s.tasks_perfect, 1);
    }

    #[test]
    fn test_star_display() {
        let mut s = Scoring::new(5);
        assert_eq!(s.star_display(), "\u{2606}\u{2606}\u{2606}"); // 0 stars
        s.tasks_completed = 5;
        assert_eq!(s.star_display(), "\u{2605}\u{2606}\u{2606}"); // 1 star
        s.tasks_great = 2;
        assert_eq!(s.star_display(), "\u{2605}\u{2605}\u{2606}"); // 2 stars
        s.tasks_great = 3;
        assert_eq!(s.star_display(), "\u{2605}\u{2605}\u{2605}"); // 3 stars
    }

    #[test]
    fn test_star_display_perfect() {
        let mut s = Scoring::new(3);
        s.tasks_completed = 3;
        s.tasks_great = 3;
        s.tasks_perfect = 3;
        assert_eq!(s.star_display_full(), "\u{2605}\u{2605}\u{2605} PERFECT");
    }

    #[test]
    fn test_survival_and_keystroke() {
        let mut s = Scoring::new(0);
        s.award_survival();
        assert_eq!(s.score, 10);
        s.record_keystroke();
        assert_eq!(s.score, 10); // no score penalty — energy handles drain
        assert_eq!(s.keystrokes, 1);
    }

    #[test]
    fn test_max_combo_tracked() {
        let mut s = Scoring::new(5);
        s.complete_task(50);
        s.complete_task(50);
        s.complete_task(50);
        assert_eq!(s.max_combo, 3);
        s.break_combo();
        s.complete_task(50);
        assert_eq!(s.max_combo, 3); // max is still 3
    }
}
