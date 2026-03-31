use serde::Deserialize;

/// A code segment — a self-contained 15–40 line code block with tasks.
#[derive(Debug, Clone, Deserialize)]
pub struct Segment {
    pub meta: SegmentMeta,
    pub code: SegmentCode,
    #[serde(default)]
    pub tasks: Vec<SegmentTask>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SegmentMeta {
    pub id: String,
    pub zone: String,
    pub language: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_difficulty")]
    pub difficulty: u8,
    #[serde(default)]
    pub hints: Vec<String>,
    /// Marks this as a tutorial intro segment (first segment of a world's first level).
    #[serde(default)]
    pub intro: bool,
    /// Which level this intro belongs to, e.g. "2-1".
    #[serde(default)]
    pub intro_level: Option<String>,
}

fn default_difficulty() -> u8 {
    1
}

#[derive(Debug, Clone, Deserialize)]
pub struct SegmentCode {
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SegmentTask {
    #[serde(rename = "type")]
    pub task_type: String,
    pub anchor: TaskAnchor,
    pub description: String,
    #[serde(default = "default_points")]
    pub points: i64,
    #[serde(default)]
    pub optimal_keys: usize,
    /// Absolute optimal keystrokes (best possible with any vim command).
    #[serde(default)]
    pub perfect_keys: usize,
    /// For change_word tasks: the new text to change to.
    #[serde(default)]
    pub new_text: Option<String>,
    /// For replace_char tasks: the expected character after replacement.
    #[serde(default)]
    pub replace_with: Option<String>,
    /// For change_inside tasks: the delimiter character.
    #[serde(default)]
    pub delimiter: Option<String>,
    /// For yank_paste tasks: the expected text at the target.
    #[serde(default)]
    pub expected_text: Option<String>,
    /// For indent tasks: the expected leading whitespace.
    #[serde(default)]
    pub expected_indent: Option<String>,
    /// For delete_block tasks: the number of lines to delete.
    #[serde(default)]
    pub line_count: Option<usize>,
}

fn default_points() -> i64 {
    50
}

#[derive(Debug, Clone, Deserialize)]
pub struct TaskAnchor {
    pub pattern: String,
    #[serde(default = "default_occurrence")]
    pub occurrence: usize,
    /// When true, target the last character of the matched pattern (for `e` motion tasks).
    #[serde(default)]
    pub at_end: bool,
}

fn default_occurrence() -> usize {
    1
}

impl Segment {
    /// Parse a segment from TOML text.
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Number of lines in the code content.
    pub fn line_count(&self) -> usize {
        self.code.content.lines().count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TOML: &str = r#"
[meta]
id = "py-starter-hello"
zone = "starter"
language = "python"
tags = ["variables", "print"]
difficulty = 1

[code]
content = """
name = "Alice"
print("Hello, " + name)

numbers = [1, 2, 3]
total = sum(numbers)
print(total)
"""

[[tasks]]
type = "move_to"
anchor = { pattern = "Alice", occurrence = 1 }
description = "Move to 'Alice'"
points = 50
optimal_keys = 3

[[tasks]]
type = "move_to"
anchor = { pattern = "total", occurrence = 1 }
description = "Move to 'total'"
points = 75
optimal_keys = 5
"#;

    #[test]
    fn test_parse_segment() {
        let seg = Segment::from_toml(SAMPLE_TOML).unwrap();
        assert_eq!(seg.meta.id, "py-starter-hello");
        assert_eq!(seg.meta.zone, "starter");
        assert_eq!(seg.meta.language, "python");
        assert_eq!(seg.meta.tags, vec!["variables", "print"]);
        assert_eq!(seg.meta.difficulty, 1);
        assert!(seg.code.content.contains("Alice"));
        assert_eq!(seg.tasks.len(), 2);
        assert_eq!(seg.tasks[0].task_type, "move_to");
        assert_eq!(seg.tasks[0].anchor.pattern, "Alice");
        assert_eq!(seg.tasks[0].points, 50);
        assert_eq!(seg.tasks[1].anchor.pattern, "total");
    }

    #[test]
    fn test_parse_minimal_segment() {
        let toml = r#"
[meta]
id = "test"
zone = "starter"
language = "python"

[code]
content = "x = 1"
"#;
        let seg = Segment::from_toml(toml).unwrap();
        assert_eq!(seg.meta.difficulty, 1); // default
        assert!(seg.tasks.is_empty());
    }

    #[test]
    fn test_parse_at_end_anchor() {
        let toml = r#"
[meta]
id = "test-at-end"
zone = "starter"
language = "python"

[code]
content = "result = 42"

[[tasks]]
type = "move_to"
anchor = { pattern = "result", occurrence = 1, at_end = true }
description = "End of 'result'"
points = 50
"#;
        let seg = Segment::from_toml(toml).unwrap();
        assert_eq!(seg.tasks.len(), 1);
        assert!(seg.tasks[0].anchor.at_end);
    }

    #[test]
    fn test_at_end_defaults_false() {
        let seg = Segment::from_toml(SAMPLE_TOML).unwrap();
        assert!(!seg.tasks[0].anchor.at_end);
    }

    #[test]
    fn test_line_count() {
        let seg = Segment::from_toml(SAMPLE_TOML).unwrap();
        assert!(seg.line_count() >= 5);
    }
}
