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
    let count = 5 + (rand::random::<usize>() % 6); // 5..=10
    let count = count.min(pool.len());

    let mut selected: Vec<&str> = pool.clone();
    selected.shuffle(&mut rng);
    selected.truncate(count);

    let mut runway = selected.join("\n");
    runway.push('\n');
    runway
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
/// Prepends an import runway (task-free lines) before the first segment.
pub fn assemble(segments: &[&Segment]) -> AssembledLevel {
    if segments.is_empty() {
        return AssembledLevel {
            buffer: Buffer::from_str(""),
            tasks: Vec::new(),
        };
    }

    let language = &segments[0].meta.language;
    let sep = separator(language);

    // Start with import runway
    let mut full_code = build_runway(language);
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
