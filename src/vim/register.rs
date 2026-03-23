use std::collections::HashMap;

/// Whether register content was yanked/deleted as whole lines or characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterContent {
    /// Character-wise content (paste inline).
    Charwise(String),
    /// Line-wise content (paste as new lines).
    Linewise(String),
}

impl RegisterContent {
    pub fn text(&self) -> &str {
        match self {
            RegisterContent::Charwise(s) => s,
            RegisterContent::Linewise(s) => s,
        }
    }

    pub fn is_linewise(&self) -> bool {
        matches!(self, RegisterContent::Linewise(_))
    }
}

impl Default for RegisterContent {
    fn default() -> Self {
        RegisterContent::Charwise(String::new())
    }
}

/// Vim register file.
///
/// - `""` unnamed register (default target for d/c/y/p)
/// - `"0` last yank register
/// - `"a`–`"z` named registers
/// - `"A`–`"Z` append to named registers (write-only; reads from lowercase)
pub struct RegisterFile {
    unnamed: RegisterContent,
    last_yank: RegisterContent,
    named: HashMap<char, RegisterContent>,
}

impl RegisterFile {
    pub fn new() -> Self {
        Self {
            unnamed: RegisterContent::default(),
            last_yank: RegisterContent::default(),
            named: HashMap::new(),
        }
    }

    /// Read from a register. `None` means unnamed.
    pub fn get(&self, reg: Option<char>) -> &RegisterContent {
        match reg {
            None => &self.unnamed,
            Some('0') => &self.last_yank,
            Some(c) if c.is_ascii_lowercase() => {
                self.named.get(&c).unwrap_or(&self.unnamed)
            }
            Some(c) if c.is_ascii_uppercase() => {
                let lower = c.to_ascii_lowercase();
                self.named.get(&lower).unwrap_or(&self.unnamed)
            }
            Some(_) => &self.unnamed,
        }
    }

    /// Store content from a yank operation.
    /// Updates unnamed register and last-yank register.
    pub fn yank(&mut self, reg: Option<char>, content: RegisterContent) {
        self.last_yank = content.clone();
        self.store(reg, content);
    }

    /// Store content from a delete/change operation.
    /// Updates unnamed register (but NOT last-yank).
    pub fn delete(&mut self, reg: Option<char>, content: RegisterContent) {
        self.store(reg, content);
    }

    fn store(&mut self, reg: Option<char>, content: RegisterContent) {
        self.unnamed = content.clone();
        match reg {
            None => {}
            Some(c) if c.is_ascii_lowercase() => {
                self.named.insert(c, content);
            }
            Some(c) if c.is_ascii_uppercase() => {
                let lower = c.to_ascii_lowercase();
                let existing = self.named.entry(lower).or_default();
                let combined = format!("{}{}", existing.text(), content.text());
                let new_content = if content.is_linewise() {
                    RegisterContent::Linewise(combined)
                } else {
                    RegisterContent::Charwise(combined)
                };
                self.named.insert(lower, new_content);
            }
            Some(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unnamed_register() {
        let mut rf = RegisterFile::new();
        rf.delete(None, RegisterContent::Charwise("hello".into()));
        assert_eq!(rf.get(None).text(), "hello");
    }

    #[test]
    fn test_named_register() {
        let mut rf = RegisterFile::new();
        rf.yank(Some('a'), RegisterContent::Charwise("foo".into()));
        assert_eq!(rf.get(Some('a')).text(), "foo");
    }

    #[test]
    fn test_uppercase_appends() {
        let mut rf = RegisterFile::new();
        rf.yank(Some('a'), RegisterContent::Charwise("hello".into()));
        rf.yank(Some('A'), RegisterContent::Charwise(" world".into()));
        assert_eq!(rf.get(Some('a')).text(), "hello world");
    }

    #[test]
    fn test_yank_sets_last_yank() {
        let mut rf = RegisterFile::new();
        rf.yank(None, RegisterContent::Linewise("line\n".into()));
        assert_eq!(rf.get(Some('0')).text(), "line\n");
        assert!(rf.get(Some('0')).is_linewise());
    }

    #[test]
    fn test_delete_does_not_set_last_yank() {
        let mut rf = RegisterFile::new();
        rf.yank(None, RegisterContent::Charwise("first".into()));
        rf.delete(None, RegisterContent::Charwise("second".into()));
        // Unnamed has "second" but last_yank still has "first"
        assert_eq!(rf.get(None).text(), "second");
        assert_eq!(rf.get(Some('0')).text(), "first");
    }

    #[test]
    fn test_linewise_flag_preserved() {
        let mut rf = RegisterFile::new();
        rf.yank(None, RegisterContent::Linewise("line1\nline2\n".into()));
        assert!(rf.get(None).is_linewise());
    }

    #[test]
    fn test_unknown_named_returns_unnamed() {
        let mut rf = RegisterFile::new();
        rf.delete(None, RegisterContent::Charwise("x".into()));
        assert_eq!(rf.get(Some('z')).text(), "x");
    }

    #[test]
    fn test_read_uppercase_reads_lowercase() {
        let mut rf = RegisterFile::new();
        rf.yank(Some('b'), RegisterContent::Charwise("data".into()));
        assert_eq!(rf.get(Some('B')).text(), "data");
    }
}
