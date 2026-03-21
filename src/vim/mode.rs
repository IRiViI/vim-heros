/// The current Vim editing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
}

impl Mode {
    pub fn is_insert(self) -> bool {
        self == Mode::Insert
    }

    pub fn is_normal(self) -> bool {
        self == Mode::Normal
    }
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Normal
    }
}
