//! BFS pathfinder for optimal motion path between consecutive targets.
//!
//! Before a level starts, we pre-calculate the shortest path (fewest motions)
//! between every consecutive pair of targets. Each motion costs 1 regardless
//! of count prefix (`6j` = 1 motion). BFS on (line, col) state space.
//!
//! This gives us:
//! 1. Energy budget per target
//! 2. Death hints (compare player's actual motions to optimal)
//! 3. Level validation (guarantee every target is reachable)

use std::collections::{HashMap, HashSet, VecDeque};

use crate::vim::buffer::Buffer;
use crate::vim::cursor::Cursor;
use crate::vim::motions;

/// A motion that can be taken in the BFS.
#[derive(Debug, Clone)]
pub struct BfsMotion {
    /// Human-readable name for hints (e.g., "6j", "w", "fa").
    pub name: String,
    /// The action to simulate (closure would be ideal but we use enum dispatch).
    pub kind: MotionKind,
}

/// Kinds of motions we simulate in BFS.
#[derive(Debug, Clone)]
pub enum MotionKind {
    // Basic: h j k l
    MoveLeft,
    MoveDown,
    MoveUp,
    MoveRight,
    // Counted: {n}j, {n}k, {n}h, {n}l
    CountedDown(usize),
    CountedUp(usize),
    CountedLeft(usize),
    CountedRight(usize),
    // Word motions
    WordForward,
    WordBackward,
    WordEnd,
    BigWordForward,
    BigWordBackward,
    // Counted word motions
    CountedWordForward(usize),
    CountedWordBackward(usize),
    CountedWordEnd(usize),
    CountedBigWordForward(usize),
    CountedBigWordBackward(usize),
    // Find/till char
    FindCharForward(char),
    FindCharBackward(char),
    TillCharForward(char),
    TillCharBackward(char),
    // Counted find
    CountedFindCharForward(usize, char),
    CountedFindCharBackward(usize, char),
    // Line position
    LineStart,
    LineEnd,
}

/// Simulate a motion on the buffer and return the resulting cursor position.
fn simulate_motion(motion: &MotionKind, cursor: &Cursor, buffer: &Buffer) -> Cursor {
    match motion {
        MotionKind::MoveLeft => motions::move_left(cursor, buffer),
        MotionKind::MoveDown => motions::move_down(cursor, buffer),
        MotionKind::MoveUp => motions::move_up(cursor, buffer),
        MotionKind::MoveRight => motions::move_right(cursor, buffer),
        MotionKind::CountedDown(n) => {
            let mut c = *cursor;
            for _ in 0..*n { c = motions::move_down(&c, buffer); }
            c
        }
        MotionKind::CountedUp(n) => {
            let mut c = *cursor;
            for _ in 0..*n { c = motions::move_up(&c, buffer); }
            c
        }
        MotionKind::CountedLeft(n) => {
            let mut c = *cursor;
            for _ in 0..*n { c = motions::move_left(&c, buffer); }
            c
        }
        MotionKind::CountedRight(n) => {
            let mut c = *cursor;
            for _ in 0..*n { c = motions::move_right(&c, buffer); }
            c
        }
        MotionKind::WordForward => motions::word_forward(cursor, buffer),
        MotionKind::WordBackward => motions::word_backward(cursor, buffer),
        MotionKind::WordEnd => motions::word_end(cursor, buffer),
        MotionKind::BigWordForward => motions::big_word_forward(cursor, buffer),
        MotionKind::BigWordBackward => motions::big_word_backward(cursor, buffer),
        MotionKind::CountedWordForward(n) => {
            let mut c = *cursor;
            for _ in 0..*n { c = motions::word_forward(&c, buffer); }
            c
        }
        MotionKind::CountedWordBackward(n) => {
            let mut c = *cursor;
            for _ in 0..*n { c = motions::word_backward(&c, buffer); }
            c
        }
        MotionKind::CountedWordEnd(n) => {
            let mut c = *cursor;
            for _ in 0..*n { c = motions::word_end(&c, buffer); }
            c
        }
        MotionKind::CountedBigWordForward(n) => {
            let mut c = *cursor;
            for _ in 0..*n { c = motions::big_word_forward(&c, buffer); }
            c
        }
        MotionKind::CountedBigWordBackward(n) => {
            let mut c = *cursor;
            for _ in 0..*n { c = motions::big_word_backward(&c, buffer); }
            c
        }
        MotionKind::FindCharForward(ch) => motions::find_char_forward(cursor, buffer, *ch),
        MotionKind::FindCharBackward(ch) => motions::find_char_backward(cursor, buffer, *ch),
        MotionKind::TillCharForward(ch) => motions::till_char_forward(cursor, buffer, *ch),
        MotionKind::TillCharBackward(ch) => motions::till_char_backward(cursor, buffer, *ch),
        MotionKind::CountedFindCharForward(n, ch) => {
            let mut c = *cursor;
            for _ in 0..*n { c = motions::find_char_forward(&c, buffer, *ch); }
            c
        }
        MotionKind::CountedFindCharBackward(n, ch) => {
            let mut c = *cursor;
            for _ in 0..*n { c = motions::find_char_backward(&c, buffer, *ch); }
            c
        }
        MotionKind::LineStart => motions::line_start(cursor, buffer),
        MotionKind::LineEnd => motions::line_end(cursor, buffer),
    }
}

/// Build the set of motions available for BFS given the World 1 sub-level.
/// These are the "edges" in the BFS graph.
fn build_motions_for_level(level: usize, buffer: &Buffer, from: &Cursor, to: &Cursor) -> Vec<BfsMotion> {
    let mut motions = Vec::new();

    // Level 1: h j k l only
    // Level 2: + w W b B e + count prefixes
    // Level 3: + f F t T
    // Level 4/5: all World 1 motions + $ 0

    // Always available: h j k l
    motions.push(BfsMotion { name: "h".into(), kind: MotionKind::MoveLeft });
    motions.push(BfsMotion { name: "j".into(), kind: MotionKind::MoveDown });
    motions.push(BfsMotion { name: "k".into(), kind: MotionKind::MoveUp });
    motions.push(BfsMotion { name: "l".into(), kind: MotionKind::MoveRight });

    if level >= 2 {
        // Word motions
        motions.push(BfsMotion { name: "w".into(), kind: MotionKind::WordForward });
        motions.push(BfsMotion { name: "b".into(), kind: MotionKind::WordBackward });
        motions.push(BfsMotion { name: "e".into(), kind: MotionKind::WordEnd });
        motions.push(BfsMotion { name: "W".into(), kind: MotionKind::BigWordForward });
        motions.push(BfsMotion { name: "B".into(), kind: MotionKind::BigWordBackward });

        // Count prefixes: generate counted motions based on distance
        let line_dist = if to.line > from.line {
            to.line - from.line
        } else {
            from.line - to.line
        };
        let max_count = (line_dist + 5).min(30);

        for n in 2..=max_count {
            motions.push(BfsMotion {
                name: format!("{}j", n),
                kind: MotionKind::CountedDown(n),
            });
            motions.push(BfsMotion {
                name: format!("{}k", n),
                kind: MotionKind::CountedUp(n),
            });
        }

        // Counted word motions (2-5)
        for n in 2..=5 {
            motions.push(BfsMotion {
                name: format!("{}w", n),
                kind: MotionKind::CountedWordForward(n),
            });
            motions.push(BfsMotion {
                name: format!("{}b", n),
                kind: MotionKind::CountedWordBackward(n),
            });
            motions.push(BfsMotion {
                name: format!("{}e", n),
                kind: MotionKind::CountedWordEnd(n),
            });
        }
    }

    if level >= 3 {
        // f/F/t/T: generate for chars on the target line
        let target_line = buffer.line(to.line).unwrap_or_default();
        let chars_on_target: HashSet<char> = target_line.chars()
            .filter(|c| !c.is_whitespace())
            .collect();

        // Also chars on source line (for F/T)
        let source_line = buffer.line(from.line).unwrap_or_default();
        let chars_on_source: HashSet<char> = source_line.chars()
            .filter(|c| !c.is_whitespace())
            .collect();

        let all_chars: HashSet<char> = chars_on_target.union(&chars_on_source).copied().collect();

        for ch in &all_chars {
            motions.push(BfsMotion {
                name: format!("f{}", ch),
                kind: MotionKind::FindCharForward(*ch),
            });
            motions.push(BfsMotion {
                name: format!("F{}", ch),
                kind: MotionKind::FindCharBackward(*ch),
            });
            motions.push(BfsMotion {
                name: format!("t{}", ch),
                kind: MotionKind::TillCharForward(*ch),
            });
            motions.push(BfsMotion {
                name: format!("T{}", ch),
                kind: MotionKind::TillCharBackward(*ch),
            });

            // Counted finds (2-3)
            for n in 2..=3 {
                motions.push(BfsMotion {
                    name: format!("{}f{}", n, ch),
                    kind: MotionKind::CountedFindCharForward(n, *ch),
                });
            }
        }
    }

    if level >= 4 {
        // $ and 0
        motions.push(BfsMotion { name: "0".into(), kind: MotionKind::LineStart });
        motions.push(BfsMotion { name: "$".into(), kind: MotionKind::LineEnd });
    }

    motions
}

/// Result of a BFS pathfinding between two targets.
#[derive(Debug, Clone)]
pub struct PathResult {
    /// The optimal number of motions (BFS depth).
    pub optimal_motions: usize,
    /// The sequence of motion names in the optimal path.
    pub path: Vec<String>,
    /// Whether the target was reachable at all.
    pub reachable: bool,
}

/// Find the shortest path (fewest motions) from `start` to `goal` using BFS.
/// Each motion costs 1 regardless of count prefix.
///
/// `level` is the World 1 sub-level (1-5), which determines available motions.
pub fn find_optimal_path(
    buffer: &Buffer,
    start: Cursor,
    goal: Cursor,
    level: usize,
) -> PathResult {
    if start == goal {
        return PathResult {
            optimal_motions: 0,
            path: Vec::new(),
            reachable: true,
        };
    }

    let available_motions = build_motions_for_level(level, buffer, &start, &goal);

    // BFS
    let start_state = (start.line, start.col);
    let goal_state = (goal.line, goal.col);

    let mut visited: HashMap<(usize, usize), (usize, usize, String)> = HashMap::new();
    let mut queue: VecDeque<(usize, usize)> = VecDeque::new();

    visited.insert(start_state, (usize::MAX, usize::MAX, String::new())); // sentinel parent
    queue.push_back(start_state);

    let mut found = false;

    // Limit BFS to prevent infinite loops (state space: lines * max_cols)
    let max_iterations = buffer.line_count() * 200;
    let mut iterations = 0;

    while let Some((line, col)) = queue.pop_front() {
        iterations += 1;
        if iterations > max_iterations {
            break;
        }

        if (line, col) == goal_state {
            found = true;
            break;
        }

        let cursor = Cursor::new(line, col);

        for motion in &available_motions {
            let new_cursor = simulate_motion(&motion.kind, &cursor, buffer);
            let new_state = (new_cursor.line, new_cursor.col);

            // Skip if we didn't actually move
            if new_state == (line, col) {
                continue;
            }

            if !visited.contains_key(&new_state) {
                visited.insert(new_state, (line, col, motion.name.clone()));
                queue.push_back(new_state);
            }
        }
    }

    if !found {
        return PathResult {
            optimal_motions: 0,
            path: Vec::new(),
            reachable: false,
        };
    }

    // Reconstruct path
    let mut path = Vec::new();
    let mut current = goal_state;
    while current != start_state {
        let (prev_line, prev_col, motion_name) = visited.get(&current).unwrap().clone();
        path.push(motion_name);
        current = (prev_line, prev_col);
    }
    path.reverse();

    PathResult {
        optimal_motions: path.len(),
        path,
        reachable: true,
    }
}

/// Pre-calculate optimal paths for all consecutive target pairs in a level.
/// Returns a vector of PathResults, one per target (path from previous position to target).
pub fn calculate_level_paths(
    buffer: &Buffer,
    targets: &[(usize, usize)], // (line, col) of each target
    level: usize,
) -> Vec<PathResult> {
    let mut results = Vec::new();
    let mut prev = Cursor::new(0, 0); // start at buffer origin

    for &(line, col) in targets {
        let goal = Cursor::new(line, col);
        let result = find_optimal_path(buffer, prev, goal, level);
        results.push(result);
        prev = goal;
    }

    results
}

/// Generate a death hint by comparing player's last motions to the optimal path.
pub fn generate_death_hint(
    player_motions: &[String],
    optimal_path: &PathResult,
) -> String {
    if !optimal_path.reachable {
        return "This target might not be reachable with your current keys.".to_string();
    }

    if optimal_path.path.is_empty() {
        return "You were already at the target!".to_string();
    }

    let optimal_count = optimal_path.optimal_motions;
    let player_count = player_motions.len();

    // Check for repeated single-char motions that could be replaced by counted motions
    if player_count > 3 {
        // Count consecutive identical motions
        let mut runs: Vec<(String, usize)> = Vec::new();
        for m in player_motions {
            if let Some(last) = runs.last_mut() {
                if &last.0 == m {
                    last.1 += 1;
                    continue;
                }
            }
            runs.push((m.clone(), 1));
        }

        for (motion, count) in &runs {
            if *count >= 3 {
                return format!(
                    "Did you know `{}{}` moves {} times in 1 motion instead of {}?",
                    count, motion, count, count
                );
            }
        }
    }

    // Suggest the optimal path
    let optimal_str = optimal_path.path.join(" ");
    if player_count > optimal_count + 2 {
        format!(
            "Try `{}` -- that's {} motion{} instead of {}!",
            optimal_str,
            optimal_count,
            if optimal_count == 1 { "" } else { "s" },
            player_count,
        )
    } else if !optimal_path.path.is_empty() {
        format!(
            "Optimal path: `{}` ({} motion{})",
            optimal_str,
            optimal_count,
            if optimal_count == 1 { "" } else { "s" },
        )
    } else {
        "Keep practicing those motions!".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_path_down() {
        let buf = Buffer::from_str("line 1\nline 2\nline 3\nline 4");
        let result = find_optimal_path(&buf, Cursor::new(0, 0), Cursor::new(3, 0), 1);
        assert!(result.reachable);
        assert_eq!(result.optimal_motions, 3); // j j j
    }

    #[test]
    fn test_simple_path_right() {
        let buf = Buffer::from_str("hello world");
        let result = find_optimal_path(&buf, Cursor::new(0, 0), Cursor::new(0, 5), 1);
        assert!(result.reachable);
        assert_eq!(result.optimal_motions, 5); // l l l l l
    }

    #[test]
    fn test_counted_motion_level2() {
        let buf = Buffer::from_str("line 1\nline 2\nline 3\nline 4\nline 5\nline 6");
        let result = find_optimal_path(&buf, Cursor::new(0, 0), Cursor::new(5, 0), 2);
        assert!(result.reachable);
        // With count prefixes, 5j = 1 motion
        assert_eq!(result.optimal_motions, 1);
    }

    #[test]
    fn test_word_motion_level2() {
        let buf = Buffer::from_str("hello world foo");
        let result = find_optimal_path(&buf, Cursor::new(0, 0), Cursor::new(0, 6), 2);
        assert!(result.reachable);
        // w moves to "world" at col 6 in 1 motion
        assert_eq!(result.optimal_motions, 1);
    }

    #[test]
    fn test_same_position() {
        let buf = Buffer::from_str("hello");
        let result = find_optimal_path(&buf, Cursor::new(0, 0), Cursor::new(0, 0), 1);
        assert!(result.reachable);
        assert_eq!(result.optimal_motions, 0);
    }

    #[test]
    fn test_death_hint_repeated_motions() {
        let player = vec!["j".into(), "j".into(), "j".into(), "j".into(), "j".into()];
        let optimal = PathResult {
            optimal_motions: 1,
            path: vec!["5j".into()],
            reachable: true,
        };
        let hint = generate_death_hint(&player, &optimal);
        assert!(hint.contains("5") || hint.contains("motion"));
    }
}
