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
}
