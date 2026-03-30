/// Tracks which portion of the buffer is visible on screen.
pub struct Viewport {
    /// First visible line (0-indexed).
    pub top_line: usize,
    /// Number of visible lines.
    pub height: usize,
}

impl Viewport {
    pub fn new(height: usize) -> Self {
        Self {
            top_line: 0,
            height,
        }
    }

    /// Last visible line (inclusive).
    pub fn bottom_line(&self) -> usize {
        self.top_line + self.height.saturating_sub(1)
    }

    /// Check if a line index is within the visible viewport.
    pub fn contains(&self, line: usize) -> bool {
        line >= self.top_line && line <= self.bottom_line()
    }

    /// Scroll down by one line.
    pub fn scroll_down(&mut self) {
        self.top_line += 1;
    }

    /// Scroll up by one line (clamped at 0).
    pub fn scroll_up(&mut self) {
        self.top_line = self.top_line.saturating_sub(1);
    }

    /// Scroll the viewport so that `line` is visible with at least `padding`
    /// lines of margin from the top and bottom edges. Scrolls up or down as
    /// needed. `max_line` is the last line in the buffer (used to clamp).
    pub fn ensure_visible(&mut self, line: usize, padding: usize, max_line: usize) {
        let pad_top = self.top_line + padding;
        let pad_bottom = self.bottom_line().saturating_sub(padding);

        if line < pad_top {
            // Need to scroll up
            self.top_line = line.saturating_sub(padding);
        } else if line > pad_bottom {
            // Need to scroll down
            let desired = line + padding + 1;
            let new_top = desired.saturating_sub(self.height);
            let max_top = max_line.saturating_sub(self.height.saturating_sub(1));
            self.top_line = new_top.min(max_top);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_viewport() {
        let vp = Viewport::new(20);
        assert_eq!(vp.top_line, 0);
        assert_eq!(vp.height, 20);
        assert_eq!(vp.bottom_line(), 19);
    }

    #[test]
    fn test_contains() {
        let vp = Viewport {
            top_line: 5,
            height: 10,
        };
        assert!(!vp.contains(4));
        assert!(vp.contains(5));
        assert!(vp.contains(14));
        assert!(!vp.contains(15));
    }

    #[test]
    fn test_scroll_down() {
        let mut vp = Viewport::new(10);
        vp.scroll_down();
        assert_eq!(vp.top_line, 1);
        assert_eq!(vp.bottom_line(), 10);
        assert!(!vp.contains(0));
        assert!(vp.contains(1));
    }

    #[test]
    fn test_scroll_up() {
        let mut vp = Viewport { top_line: 5, height: 10 };
        vp.scroll_up();
        assert_eq!(vp.top_line, 4);

        // Clamps at 0
        let mut vp2 = Viewport::new(10);
        vp2.scroll_up();
        assert_eq!(vp2.top_line, 0);
    }

    #[test]
    fn test_ensure_visible_scrolls_down() {
        let mut vp = Viewport::new(20); // lines 0-19
        vp.ensure_visible(25, 2, 100);
        // Line 25 should be within viewport with padding
        assert!(vp.contains(25));
        assert!(25 <= vp.bottom_line().saturating_sub(2));
    }

    #[test]
    fn test_ensure_visible_scrolls_up() {
        let mut vp = Viewport { top_line: 30, height: 20 }; // lines 30-49
        vp.ensure_visible(5, 2, 100);
        assert!(vp.contains(5));
        assert!(5 >= vp.top_line + 2);
    }

    #[test]
    fn test_ensure_visible_no_scroll_when_in_view() {
        let mut vp = Viewport { top_line: 10, height: 20 }; // lines 10-29
        let orig_top = vp.top_line;
        vp.ensure_visible(20, 2, 100);
        assert_eq!(vp.top_line, orig_top);
    }
}
