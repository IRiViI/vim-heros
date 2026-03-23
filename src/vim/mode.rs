/// The current Vim editing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Replace,
}

impl Mode {
    pub fn is_insert(self) -> bool {
        self == Mode::Insert
    }

    pub fn is_normal(self) -> bool {
        self == Mode::Normal
    }

    pub fn is_replace(self) -> bool {
        self == Mode::Replace
    }
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Normal
    }
}
