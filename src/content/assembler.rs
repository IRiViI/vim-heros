use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::game::task::{self, Task};
use crate::vim::buffer::Buffer;

use super::segment::{Segment, SegmentTask};

/// Result of assembling segments into a playable level.
pub struct AssembledLevel {
    pub buffer: Buffer,
    pub tasks: Vec<Task>,
}

/// Comment separator between segments, keyed by language.
fn separator(language: &str) -> &'static str {
    match language {
        "python" => "\n# ---\n",
        "typescript" | "javascript" => "\n// ---\n",
        "rust" | "cpp" | "c" => "\n// ---\n",
        _ => "\n// ---\n",
    }
}

/// Select `count` segments randomly from the pool, avoiding `recently_seen` IDs.
pub fn select_segments<'a>(
    pool: &'a [Segment],
    count: usize,
    recently_seen: &[String],
) -> Vec<&'a Segment> {
    let mut rng = thread_rng();

    // Prefer segments not recently seen
    let mut available: Vec<&Segment> = pool
        .iter()
        .filter(|s| !recently_seen.contains(&s.meta.id))
        .collect();

    // If not enough fresh segments, allow repeats
    if available.len() < count {
        available = pool.iter().collect();
    }

    available.shuffle(&mut rng);
    available.into_iter().take(count).collect()
}

/// Assemble selected segments into a single buffer with resolved tasks.
pub fn assemble(segments: &[&Segment]) -> AssembledLevel {
    if segments.is_empty() {
        return AssembledLevel {
            buffer: Buffer::from_str(""),
            tasks: Vec::new(),
        };
    }

    let language = &segments[0].meta.language;
    let sep = separator(language);

    let mut full_code = String::new();
    let mut all_tasks: Vec<Task> = Vec::new();

    for (i, segment) in segments.iter().enumerate() {
        if i > 0 {
            full_code.push_str(sep);
        }

        let line_offset = full_code.lines().count();
        // If we appended a separator, it ended with a newline, so next content
        // starts at the line count.
        let code = segment.code.content.trim_start_matches('\n');
        full_code.push_str(code);

        // Ensure trailing newline
        if !full_code.ends_with('\n') {
            full_code.push('\n');
        }

        // Resolve tasks: find anchors in the segment's code and offset
        let code_buffer = Buffer::from_str(code);
        for seg_task in &segment.tasks {
            if let Some(task) = resolve_segment_task(seg_task, &code_buffer, line_offset) {
                all_tasks.push(task);
            }
        }
    }

    // Sort tasks top-to-bottom
    all_tasks.sort_by_key(|t| (t.target_line, t.target_col));

    AssembledLevel {
        buffer: Buffer::from_str(&full_code),
        tasks: all_tasks,
    }
}

/// Resolve a segment task's anchor to an absolute position and create a Task.
fn resolve_segment_task(
    seg_task: &SegmentTask,
    code_buffer: &Buffer,
    line_offset: usize,
) -> Option<Task> {
    let (line, col) = task::resolve_pattern(
        code_buffer,
        &seg_task.anchor.pattern,
        seg_task.anchor.occurrence,
    )?;

    let gutter = match seg_task.task_type.as_str() {
        "move_to" => "MOVE".to_string(),
        "delete_line" => "DEL".to_string(),
        "delete_word" => "DEL".to_string(),
        "change_word" => {
            if let Some(ref new) = seg_task.new_text {
                format!("CHG \u{2192} {}", new)
            } else {
                "CHG".to_string()
            }
        }
        "replace_char" => "FIX".to_string(),
        other => other.to_uppercase(),
    };

    Some(Task::move_to(
        line + line_offset,
        col,
        &seg_task.description,
        gutter,
        seg_task.points,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::segment::Segment;

    fn make_segment(id: &str, code: &str, pattern: &str) -> Segment {
        let toml = format!(
            r#"
[meta]
id = "{id}"
zone = "starter"
language = "python"

[code]
content = """
{code}
"""

[[tasks]]
type = "move_to"
anchor = {{ pattern = "{pattern}", occurrence = 1 }}
description = "Move to '{pattern}'"
points = 50
"#
        );
        Segment::from_toml(&toml).unwrap()
    }

    #[test]
    fn test_assemble_single_segment() {
        let seg = make_segment("s1", "name = \"Alice\"", "Alice");
        let result = assemble(&[&seg]);
        assert!(result.buffer.line_count() >= 1);
        assert_eq!(result.tasks.len(), 1);
        assert_eq!(result.tasks[0].description, "Move to 'Alice'");
    }

    #[test]
    fn test_assemble_multiple_segments() {
        let s1 = make_segment("s1", "x = 1\ny = 2", "x");
        let s2 = make_segment("s2", "a = 10\nb = 20", "b");
        let result = assemble(&[&s1, &s2]);

        assert_eq!(result.tasks.len(), 2);
        // First task should be on an earlier line than the second
        assert!(result.tasks[0].target_line < result.tasks[1].target_line);
    }

    #[test]
    fn test_assemble_empty() {
        let result = assemble(&[]);
        assert_eq!(result.tasks.len(), 0);
    }

    #[test]
    fn test_select_segments_avoids_recent() {
        let s1 = make_segment("s1", "x = 1", "x");
        let s2 = make_segment("s2", "y = 2", "y");
        let s3 = make_segment("s3", "z = 3", "z");
        let pool = vec![s1, s2, s3];

        let selected = select_segments(&pool, 2, &["s1".to_string()]);
        assert_eq!(selected.len(), 2);
        // s1 should not be selected
        for seg in &selected {
            assert_ne!(seg.meta.id, "s1");
        }
    }

    #[test]
    fn test_select_segments_allows_repeats_if_needed() {
        let s1 = make_segment("s1", "x = 1", "x");
        let pool = vec![s1];

        // Even though s1 is "recently seen", it's the only option
        let selected = select_segments(&pool, 1, &["s1".to_string()]);
        assert_eq!(selected.len(), 1);
    }
}
