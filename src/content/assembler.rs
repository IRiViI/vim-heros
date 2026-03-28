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

/// Optional level context for generating hint comments in the runway.
pub struct LevelContext {
    pub world: usize,
    pub level: usize,
    pub name: String,
    pub language: String,
}

/// Import runway pools per language. Each entry is one import line.
/// The assembler randomly picks 5–10 of these to prepend before the first segment.
fn import_pool(language: &str) -> Vec<&'static str> {
    match language {
        "python" => vec![
            "import os",
            "import sys",
            "import json",
            "import math",
            "import random",
            "import datetime",
            "import itertools",
            "import functools",
            "import collections",
            "import pathlib",
            "import re",
            "import typing",
            "from collections import defaultdict",
            "from collections import Counter",
            "from typing import List, Optional",
            "from typing import Dict, Tuple",
            "from dataclasses import dataclass",
            "from pathlib import Path",
            "from enum import Enum",
            "from functools import lru_cache",
        ],
        "typescript" | "javascript" => vec![
            "import fs from 'fs';",
            "import path from 'path';",
            "import { readFile } from 'fs/promises';",
            "import { EventEmitter } from 'events';",
            "import { Request, Response } from 'express';",
            "import { useState, useEffect } from 'react';",
            "import { z } from 'zod';",
            "import { createClient } from '@supabase/supabase-js';",
            "import type { Config } from './types';",
            "import type { User, Session } from './models';",
            "import { logger } from './utils/logger';",
            "import { db } from './db';",
            "import { validateInput } from './validation';",
            "import { formatDate, parseISO } from 'date-fns';",
            "import { clsx } from 'clsx';",
            "import { v4 as uuidv4 } from 'uuid';",
            "import { Router } from 'express';",
            "import { PrismaClient } from '@prisma/client';",
            "import { describe, it, expect } from 'vitest';",
            "import { render, screen } from '@testing-library/react';",
        ],
        "rust" => vec![
            "use std::collections::HashMap;",
            "use std::collections::HashSet;",
            "use std::io::{self, Read, Write};",
            "use std::fs;",
            "use std::path::PathBuf;",
            "use std::sync::{Arc, Mutex};",
            "use std::time::{Duration, Instant};",
            "use serde::{Deserialize, Serialize};",
            "use anyhow::{Context, Result};",
            "use tokio::sync::mpsc;",
        ],
        "cpp" | "c" => vec![
            "#include <iostream>",
            "#include <vector>",
            "#include <string>",
            "#include <map>",
            "#include <algorithm>",
            "#include <memory>",
            "#include <functional>",
            "#include <optional>",
            "#include <cassert>",
            "#include <numeric>",
        ],
        _ => vec![],
    }
}

/// Build an import runway: 5–10 randomly selected import lines for the language.
fn build_runway(language: &str) -> String {
    let pool = import_pool(language);
    if pool.is_empty() {
        return String::new();
    }

    let mut rng = thread_rng();
    let count = 3 + (rand::random::<usize>() % 3); // 3..=5
    let count = count.min(pool.len());

    let mut selected: Vec<&str> = pool.clone();
    selected.shuffle(&mut rng);
    selected.truncate(count);

    let mut runway = selected.join("\n");
    runway.push('\n');
    runway
}

/// Build a comment block explaining the key Vim motions for this level.
/// Returns empty string if no level context is provided.
fn build_level_hints(ctx: &LevelContext) -> String {
    let comment = match ctx.language.as_str() {
        "python" => "#",
        _ => "//",
    };

    let motions: &[&str] = match (ctx.world, ctx.level) {
        (1, 1) => &[
            "h / l      move left / right",
            "j / k      move down / up",
            "w          jump to next word",
            "b          jump back a word",
        ],
        (1, 2) => &[
            "w / b      next word / back a word",
            "e          jump to end of word",
            "0 / $      start / end of line",
            "f<char>    jump to character on line",
        ],
        (1, 3) => &[
            "0 / $      start / end of line",
            "^ / g_     first / last non-blank",
            "gg / G     top / bottom of file",
            "<n>G       go to line n",
        ],
        (2, 1) => &[
            "x          delete character",
            "dd         delete entire line",
            "dw         delete word",
            "D          delete to end of line",
        ],
        (2, 2) => &[
            "yy         yank (copy) line",
            "yw         yank word",
            "p / P      paste after / before",
            "dd + p     cut and paste a line",
        ],
        (3, 1) => &[
            "f<c> / t<c>  find / till character",
            ";            repeat last f/t",
            "/pattern     search forward",
            "n / N        next / prev match",
        ],
        (3, 2) => &[
            "cw         change word",
            "ci\"        change inside quotes",
            "ci(        change inside parens",
            "diw        delete inner word",
        ],
        (3, 3) => &[
            "/pattern   search for text",
            "n          jump to next match",
            "dw / dd    delete word / line",
            "combine search + delete for speed",
        ],
        (4, _) => &[
            "combine all motions for speed!",
            "f/t + ; for fast horizontal moves",
            "/pattern for long-distance jumps",
            "operator + motion = power",
        ],
        _ => &[
            "h/j/k/l    basic movement",
            "w/b/e      word motions",
            "f<c>       find character",
        ],
    };

    let bar = format!("{} {}", comment, "═".repeat(42));
    let thin = format!("{} {}", comment, "─".repeat(42));

    let mut lines = Vec::new();
    lines.push(bar.clone());
    lines.push(format!(
        "{} LEVEL {}-{}: {}",
        comment, ctx.world, ctx.level, ctx.name
    ));
    lines.push(thin);
    lines.push(format!("{} KEY MOTIONS FOR ★★★:", comment));
    for motion in motions {
        lines.push(format!("{}   {}", comment, motion));
    }
    lines.push(bar);

    let mut result = lines.join("\n");
    result.push('\n');
    result
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
/// Prepends level hints and an import runway (task-free lines) before the first segment.
pub fn assemble(segments: &[&Segment], level_ctx: Option<&LevelContext>) -> AssembledLevel {
    if segments.is_empty() {
        return AssembledLevel {
            buffer: Buffer::from_str(""),
            tasks: Vec::new(),
        };
    }

    let language = &segments[0].meta.language;
    let sep = separator(language);

    // Start with level hints (if context provided) + import runway
    let mut full_code = String::new();
    if let Some(ctx) = level_ctx {
        full_code.push_str(&build_level_hints(ctx));
    }
    full_code.push_str(&build_runway(language));
    if !full_code.is_empty() {
        full_code.push_str(sep);
    }

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
            if let Some(mut task) = resolve_segment_task(seg_task, &code_buffer, line_offset) {
                task.good_keys = seg_task.optimal_keys;
                task.perfect_keys = seg_task.perfect_keys;
                all_tasks.push(task);
            }
        }
    }

    // Sort tasks top-to-bottom
    all_tasks.sort_by_key(|t| (t.target_line, t.target_col));

    let buffer = Buffer::from_str(&full_code);

    // Fill gaps larger than MAX_GAP lines with auto-generated navigation tasks
    fill_gaps(&buffer, &mut all_tasks, MAX_GAP);

    // Recalculate keystroke budgets based on available motions for this world
    if let Some(ctx) = level_ctx {
        recalculate_keystroke_budgets(&buffer, &mut all_tasks, ctx.world);
    }

    AssembledLevel {
        buffer,
        tasks: all_tasks,
    }
}

/// Maximum gap (in lines) allowed between consecutive tasks before auto-fill kicks in.
const MAX_GAP: usize = 10;

/// Boring tokens to skip when auto-generating navigation tasks.
const SKIP_TOKENS: &[&str] = &[
    "import", "from", "def", "return", "class", "self", "None", "True", "False",
    "const", "let", "var", "function", "return", "export", "default", "async",
    "await", "type", "interface", "enum", "struct", "impl", "pub", "use", "mod",
    "crate", "super", "where", "trait", "match", "else", "elif", "except",
    "finally", "with", "pass", "break", "continue", "yield", "raise", "assert",
    "print", "println", "console", "void", "null", "undefined", "typeof",
    "instanceof", "this", "new", "try", "catch", "throw", "throws",
];

/// Fill gaps larger than `max_gap` lines between consecutive tasks by inserting
/// auto-generated MoveTo tasks targeting interesting identifiers in the code.
fn fill_gaps(buffer: &Buffer, tasks: &mut Vec<Task>, max_gap: usize) {
    // Build list of (start_line, end_line) gaps to fill.
    // Include gap before first task and after last task.
    let mut gaps: Vec<(usize, usize)> = Vec::new();

    if tasks.is_empty() {
        return;
    }

    // Gap before first task
    if tasks[0].target_line > max_gap {
        gaps.push((0, tasks[0].target_line));
    }

    // Gaps between consecutive tasks
    for i in 0..tasks.len() - 1 {
        let gap_start = tasks[i].target_line;
        let gap_end = tasks[i + 1].target_line;
        if gap_end - gap_start > max_gap {
            gaps.push((gap_start + 1, gap_end));
        }
    }

    // Gap after last task (no need — level ends soon after last task)

    let mut new_tasks: Vec<Task> = Vec::new();

    for (gap_start, gap_end) in gaps {
        let gap_size = gap_end - gap_start;
        // How many filler tasks to inject: roughly 1 per max_gap lines
        let fill_count = gap_size / max_gap;
        if fill_count == 0 {
            continue;
        }

        // Collect candidate tokens from lines in this gap
        let mut candidates: Vec<(usize, usize, String)> = Vec::new(); // (line, col, token)
        for line_idx in gap_start..gap_end {
            if let Some(line_text) = buffer.line(line_idx) {
                let trimmed = line_text.trim();
                // Skip empty lines, comment-only lines, separator lines
                if trimmed.is_empty()
                    || trimmed.starts_with('#')
                    || trimmed.starts_with("//")
                    || trimmed == "---"
                {
                    continue;
                }
                // Extract word-like tokens
                for token in extract_identifiers(&line_text) {
                    if token.text.len() >= 4
                        && !SKIP_TOKENS.contains(&token.text.as_str())
                    {
                        candidates.push((line_idx, token.col, token.text));
                    }
                }
            }
        }

        if candidates.is_empty() {
            continue;
        }

        // Pick candidates at roughly even intervals through the gap
        let step = candidates.len() / (fill_count + 1);
        if step == 0 {
            // Not enough candidates; just pick the middle one
            let mid = candidates.len() / 2;
            let (line, col, ref token) = candidates[mid];
            new_tasks.push(Task::move_to(
                line,
                col,
                format!("Navigate to '{}'", token),
                "NAV",
                25,
            ));
        } else {
            for i in 0..fill_count {
                let idx = step * (i + 1);
                let idx = idx.min(candidates.len() - 1);
                let (line, col, ref token) = candidates[idx];
                new_tasks.push(Task::move_to(
                    line,
                    col,
                    format!("Navigate to '{}'", token),
                    "NAV",
                    25,
                ));
            }
        }
    }

    tasks.extend(new_tasks);
    tasks.sort_by_key(|t| (t.target_line, t.target_col));
}

/// Recalculate keystroke budgets for MoveTo tasks based on actual distance between
/// consecutive tasks and which Vim motions are available.
///
/// `perfect_keys` = absolute optimal (any Vim motion: count prefixes, f/t, /search).
///   Computed from actual distance. Takes the min of computed vs TOML (only tighter).
///
/// `good_keys` = world-constrained optimal + buffer.
///   World 1-2: basic motions only (h/j/k/l/w/b/e, no count prefixes, no f/t, no search).
///   World 3+:  keeps TOML values (player has f/t, /search).
fn recalculate_keystroke_budgets(buffer: &Buffer, tasks: &mut [Task], world: usize) {
    use crate::game::task::TaskKind;

    if tasks.is_empty() {
        return;
    }

    let mut prev_line: usize = 0;

    for task in tasks.iter_mut() {
        if task.kind != TaskKind::MoveTo {
            prev_line = task.target_line;
            continue;
        }

        let line_dist = if task.target_line >= prev_line {
            task.target_line - prev_line
        } else {
            prev_line - task.target_line
        };

        let word_hops = count_word_hops_to_col(buffer, task.target_line, task.target_col);

        // --- perfect_keys: absolute optimal (all motions, all worlds) ---
        // Vertical: count prefix + j/k (e.g., "3j" = 2 keys, "15j" = 3 keys)
        let vertical_perfect = if line_dist == 0 {
            0
        } else if line_dist == 1 {
            1 // just "j" or "k"
        } else {
            digit_count(line_dist) + 1 // e.g., "3j" = 2, "12j" = 3
        };
        // Horizontal: f<char> = 2 keys if not at col 0, else 0
        let horizontal_perfect = if word_hops == 0 {
            0
        } else {
            word_hops.min(2) // f<char> is 2 keys and usually available
        };
        let computed_perfect = vertical_perfect + horizontal_perfect;

        // Only tighten: use the minimum of computed vs TOML
        if task.perfect_keys == 0 || computed_perfect < task.perfect_keys {
            task.perfect_keys = computed_perfect;
        }

        // --- good_keys: world-constrained optimal ---
        if world <= 2 {
            // World 1-2: no count prefixes, no f/t, no search.
            // Vertical: one j/k per line. Horizontal: one w per word boundary.
            let world_optimal = line_dist + word_hops;
            let good = world_optimal + 2; // small buffer for imperfect play

            // Only widen: use the max of computed vs TOML
            if good > task.good_keys {
                task.good_keys = good;
            }
        }

        prev_line = task.target_line;
    }
}

/// Number of decimal digits in a positive integer (e.g., 3 → 1, 12 → 2, 100 → 3).
fn digit_count(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    let mut count = 0;
    let mut v = n;
    while v > 0 {
        count += 1;
        v /= 10;
    }
    count
}

/// Count the minimum number of 'w' (word-forward) presses needed to reach
/// `target_col` from column 0 on the given line. Returns 0 if target is at col 0.
fn count_word_hops_to_col(buffer: &Buffer, line_idx: usize, target_col: usize) -> usize {
    if target_col == 0 {
        return 0;
    }

    let line = match buffer.line(line_idx) {
        Some(l) => l,
        None => return 0,
    };

    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return 0;
    }

    // Simulate word-forward motion: skip current word, skip whitespace, land on next word start
    let mut col = 0;
    let mut hops = 0;

    while col < target_col && col < chars.len() {
        // Skip current word (non-whitespace, same word class)
        let start_is_alnum = chars[col].is_alphanumeric() || chars[col] == '_';
        if start_is_alnum {
            while col < chars.len() && (chars[col].is_alphanumeric() || chars[col] == '_') {
                col += 1;
            }
        } else if !chars[col].is_whitespace() {
            // Punctuation word
            while col < chars.len() && !chars[col].is_whitespace()
                && !(chars[col].is_alphanumeric() || chars[col] == '_')
            {
                col += 1;
            }
        }
        // Skip whitespace
        while col < chars.len() && chars[col].is_whitespace() {
            col += 1;
        }
        hops += 1;

        if col >= target_col {
            break;
        }
    }

    hops
}

/// A token found in a line of code.
struct Token {
    col: usize,
    text: String,
}

/// Extract identifier-like tokens from a line of code.
fn extract_identifiers(line: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = line.char_indices().peekable();

    while let Some(&(i, c)) = chars.peek() {
        if c.is_alphabetic() || c == '_' {
            let start = i;
            let mut end = i;
            let mut text = String::new();
            while let Some(&(j, ch)) = chars.peek() {
                if ch.is_alphanumeric() || ch == '_' {
                    text.push(ch);
                    end = j;
                    chars.next();
                } else {
                    break;
                }
            }
            let _ = end; // suppress unused warning
            tokens.push(Token { col: start, text });
        } else {
            chars.next();
        }
    }

    tokens
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

    let abs_line = line + line_offset;

    match seg_task.task_type.as_str() {
        "delete_line" => {
            let original = code_buffer.line(line).unwrap_or_default();
            Some(Task::delete_line(
                abs_line,
                original,
                &seg_task.description,
                "DEL LINE",
                seg_task.points,
            ))
        }
        "delete_word" => {
            Some(Task::delete_word(
                abs_line,
                col,
                &seg_task.anchor.pattern,
                &seg_task.description,
                "DEL",
                seg_task.points,
            ))
        }
        "change_word" => {
            let new_text = seg_task.new_text.as_deref().unwrap_or("???");
            let gutter = format!("CHG \u{2192} {}", new_text);
            Some(Task::change_word(
                abs_line,
                col,
                &seg_task.anchor.pattern,
                new_text,
                &seg_task.description,
                gutter,
                seg_task.points,
            ))
        }
        "replace_char" => {
            let expected = seg_task
                .replace_with
                .as_ref()
                .and_then(|s| s.chars().next())
                .unwrap_or('?');
            Some(Task::replace_char(
                abs_line,
                col,
                expected,
                &seg_task.description,
                "FIX",
                seg_task.points,
            ))
        }
        "change_inside" => {
            let delimiter = seg_task
                .delimiter
                .as_ref()
                .and_then(|s| s.chars().next())
                .unwrap_or('"');
            let new_text = seg_task.new_text.as_deref().unwrap_or("???");
            let gutter = format!("ci{} \u{2192} {}", delimiter, new_text);
            Some(Task::change_inside(
                abs_line,
                col,
                delimiter,
                new_text,
                &seg_task.description,
                gutter,
                seg_task.points,
            ))
        }
        "yank_paste" => {
            let expected = seg_task.expected_text.as_deref().unwrap_or("???");
            Some(Task::yank_paste(
                abs_line,
                col,
                expected,
                &seg_task.description,
                "YANK+P",
                seg_task.points,
            ))
        }
        "delete_block" => {
            let n = seg_task.line_count.unwrap_or(1);
            let mut original_lines = Vec::new();
            for i in 0..n {
                if let Some(l) = code_buffer.line(line + i) {
                    original_lines.push(l);
                }
            }
            Some(Task::delete_block(
                abs_line,
                original_lines,
                &seg_task.description,
                "DEL BLK",
                seg_task.points,
            ))
        }
        "indent" => {
            let expected = seg_task.expected_indent.as_deref().unwrap_or("    ");
            Some(Task::indent(
                abs_line,
                expected,
                &seg_task.description,
                "INDENT",
                seg_task.points,
            ))
        }
        _ => {
            // Default: move_to
            Some(Task::move_to(
                abs_line,
                col,
                &seg_task.description,
                "MOVE",
                seg_task.points,
            ))
        }
    }
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
        let result = assemble(&[&seg], None);
        assert!(result.buffer.line_count() >= 1);
        assert_eq!(result.tasks.len(), 1);
        assert_eq!(result.tasks[0].description, "Move to 'Alice'");
    }

    #[test]
    fn test_assemble_multiple_segments() {
        let s1 = make_segment("s1", "x = 1\ny = 2", "x");
        let s2 = make_segment("s2", "a = 10\nb = 20", "b");
        let result = assemble(&[&s1, &s2], None);

        assert_eq!(result.tasks.len(), 2);
        // First task should be on an earlier line than the second
        assert!(result.tasks[0].target_line < result.tasks[1].target_line);
    }

    #[test]
    fn test_assemble_empty() {
        let result = assemble(&[], None);
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
