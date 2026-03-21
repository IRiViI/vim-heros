---
name: add-content
description: Create new code segments, add languages, and expand game content for Vim Heroes
---

# Skill: Add Content to Vim Heroes

You are a content creator for **Vim Heroes**, a Guitar Hero–inspired Vim training game.
Your job is to create code segments that serve as playable levels, add new programming
languages, or expand existing content pools.

## Before You Start

1. Read `PLAN.md` sections 3 (Content System) and 2 (Zone & Level System) to understand
   the full content architecture.
2. Check what content already exists:
   ```
   find content/ -name "*.toml" | head -40
   ```
3. Identify gaps: which zone/language combinations need more segments.

## Content Directory Structure

```
content/
├── {language}/
│   ├── starter/       # Simple scripts, hello world, basic syntax
│   ├── junior/        # Functions, classes, basic patterns
│   ├── medior/        # Design patterns, async, generics, moderate complexity
│   └── senior/        # Production-grade, complex architectures, advanced patterns
```

## Segment File Format

Each segment is a TOML file at `content/{language}/{zone}/{id}.toml`.

### File naming convention

`{short_descriptive_name}.toml` — lowercase, underscores, no language prefix.
Examples: `fizzbuzz.toml`, `lru_cache.toml`, `middleware_chain.toml`

### Required structure

```toml
[meta]
id = "{lang}-{zone}-{short_name}"    # Globally unique ID
zone = "{starter|junior|medior|senior}"
language = "{python|typescript|rust|cpp}"
tags = ["tag1", "tag2"]              # Used for level-matching (see Tag List below)
difficulty = 3                        # 1-5 within the zone
hints = ["S03", "S07"]               # Hint IDs from PLAN.md Section 1.6 (0-2 per segment)

[code]
content = """
# The actual code goes here.
# Must be 15-40 lines.
# Must be syntactically valid in the target language.
# Must be idiomatic — write code the way a real developer would.
# No placeholder comments like "// TODO" unless they ARE the task.
# Include 0-2 VIM TIP comments from the hint catalog (see Hints section below).
"""

[[tasks]]
type = "{task_type}"                 # See Task Types below
anchor = { pattern = "exact text to find", occurrence = 1 }
description = "Human-readable task instruction"
points = 100                         # 50-500, scale with difficulty
optimal_keys = 5                     # Minimum keystrokes for an expert

# Type-specific fields (see Task Types section)
```

### Intro segments (tutorial-as-gameplay)

Every level starts with an intro segment that teaches the player what they need.
There is no separate tutorial — the tutorial IS the first part of the level.

**File naming**: `intro_{level}.toml` — e.g., `intro_1_2.toml`
**Location**: same directory as regular segments (`content/{language}/{zone}/`)

```toml
[meta]
id = "intro-{lang}-{level}"         # e.g., "intro-py-1-2"
zone = "starter"
language = "python"
tags = ["words"]
difficulty = 1
intro = true                         # REQUIRED: marks as intro segment
intro_level = "1-2"                  # REQUIRED: which level this belongs to

[code]
content = """
# ═══════════════════════════════════
# LEVEL 1-2: Word Motions
# ═══════════════════════════════════
#
# Pressing 'l' many times is slow.
# Use 'w' to jump to the next word!
# Use 'b' to jump back a word.
#
# Try it — move to 'target' below:

name = "hello"
target = "world"
result = name + target

# Great! Use 'w' and 'b' from now on.
# ═══════════════════════════════════
"""

[[tasks]]
type = "move_to"
anchor = { pattern = "target", occurrence = 1 }
description = "Use 'w' to jump to 'target'"
points = 25                          # Intro tasks are worth less (25-50)
optimal_keys = 2
```

#### Intro segment rules

- **Level X-1** (first level of each world): **Full intro** — explain the new
  commands with examples, 2-3 simple practice tasks, visual separators
- **Level X-2 through X-5**: **Short reminder** — just a 3-5 line header comment
  listing available commands, then straight into code
- **Level 1-1 is special**: also explains game mechanics (viewport scrolling,
  tasks, colors, scoring). This is the only meta-tutorial.
- Intro task points are low (25-50) — they're practice, not the challenge
- Use clear comment separators (`# ═══`) to visually distinguish the intro
- The intro must work at the level's scroll speed — keep it short enough that
  the player can read AND do tasks before it scrolls past
- Each language needs its own intros (comment syntax differs, code examples differ)

## Task Types Reference

### move_to
Move cursor to a specific position. Simplest task type, used mainly in starter zone.
```toml
[[tasks]]
type = "move_to"
anchor = { pattern = "target_word", occurrence = 1 }
description = "Move to 'target_word'"
points = 50
optimal_keys = 3
```

### delete_line
Delete an entire line.
```toml
[[tasks]]
type = "delete_line"
anchor = { pattern = "line content to match", occurrence = 1 }
description = "Delete this comment"
points = 75
optimal_keys = 2     # dd
```

### delete_word
Delete a specific word.
```toml
[[tasks]]
type = "delete_word"
anchor = { pattern = "the_word", occurrence = 1 }
description = "Delete 'the_word'"
points = 75
optimal_keys = 3     # daw or dw depending on context
```

### change_word
Replace a word with new text.
```toml
[[tasks]]
type = "change_word"
anchor = { pattern = "old_word", occurrence = 1 }
new_text = "new_word"
description = "Change 'old_word' to 'new_word'"
points = 100
optimal_keys = 5     # ciw + type + Esc
```

### change_inside
Change content inside delimiters (quotes, brackets, etc.).
```toml
[[tasks]]
type = "change_inside"
anchor = { pattern = '"old string content"', occurrence = 1 }
delimiter = '"'                      # or (, {, [, ', <
new_text = "new string content"
description = "Change the string to 'new string content'"
points = 120
optimal_keys = 5     # ci" + type + Esc
```

### insert_text
Insert text at a specific position.
```toml
[[tasks]]
type = "insert_text"
anchor = { pattern = "line to insert after", occurrence = 1 }
position = "after"                   # "before" or "after" the anchor line
new_text = "    new_line_content"
description = "Add a return statement after this line"
points = 100
optimal_keys = 5     # o + type + Esc
```

### yank_paste
Copy text from one location and paste it at another.
```toml
[[tasks]]
type = "yank_paste"
anchor = { pattern = "line to yank", occurrence = 1 }
paste_target = { pattern = "paste after this", occurrence = 1 }
description = "Copy this line to below the return statement"
points = 150
optimal_keys = 6     # yy + navigate + p
```

### delete_block
Delete multiple consecutive lines.
```toml
[[tasks]]
type = "delete_block"
anchor = { pattern = "first line of block", occurrence = 1 }
line_count = 3                       # How many lines to delete
description = "Delete this if-block (3 lines)"
points = 100
optimal_keys = 3     # 3dd or V2jd
```

### replace_char
Replace a single character (typo fix).
```toml
[[tasks]]
type = "replace_char"
anchor = { pattern = "word_with_tpyo", occurrence = 1 }
char_offset = 10                     # Character position within the match
new_char = "y"
description = "Fix the typo: 'tpyo' → 'typo'"
points = 50
optimal_keys = 3     # f{char}r{new}
```

### indent
Fix indentation of a line or block.
```toml
[[tasks]]
type = "indent"
anchor = { pattern = "wrongly indented line", occurrence = 1 }
direction = "right"                  # "left" or "right"
count = 1                            # Number of indent levels
description = "Indent this line one level"
points = 75
optimal_keys = 2     # >>
```

## Tag List

Tags determine which segments get selected for which levels. Use 1-3 tags per segment.

### Movement tags
- `hjkl` — basic character movement
- `words` — w/b/e word motions
- `line-position` — 0/^/$ line navigation
- `line-jump` — gg/G/{num}G
- `find-char` — f/t/F/T/;/,
- `search` — //?/n/N/*/#
- `brackets` — % bracket matching
- `paragraphs` — {/} paragraph motion

### Editing tags
- `delete-char` — x/X
- `delete-line` — dd
- `yank-paste` — yy/p/P
- `insert-mode` — i/a/I/A/o/O
- `replace` — r/R
- `operators` — d/c/y + motions
- `text-objects` — iw/aw/i"/a(/i{ etc.
- `dot-repeat` — . repeat last change
- `undo-redo` — u/Ctrl-r
- `visual` — v/V/Ctrl-v
- `macros` — q/@ macro record/replay
- `registers` — named register operations
- `indent` — >/<  indentation

### Code topic tags (for thematic variety)
- `functions`, `classes`, `error-handling`, `http`, `async`, `generics`,
  `data-structures`, `algorithms`, `io`, `strings`, `math`, `testing`,
  `config`, `logging`, `database`, `cli`, `types`, `traits`, `templates`

## Vim Hints in Code Segments

Each segment should include **0–2 Vim tip comments** embedded naturally in the code.
These teach concepts while the player reads the scrolling code. Use the correct
comment syntax for the language (`#` for Python, `//` for TS/Rust/C++).

### Format

```python
# VIM TIP: Use 'w' to jump forward by word — much faster than 'llllll'
```

```typescript
// VIM TIP: 'ci"' changes everything inside double quotes in one move
```

### Rules

- Always prefix with `VIM TIP:` so the renderer can highlight them
- Use hints **from the same zone or earlier** — don't show senior hints in starter code
- Hints should relate to the tasks in the segment when possible
- Keep them to one line (two max)
- Don't put hints on consecutive lines — space them out in the code
- Reference the hint ID in `meta.hints` so the system can track which hints the player has seen

### Hint IDs by zone

Pick hints from PLAN.md Section 1.6. Use the ID prefix to match zones:
- **Starter**: S01–S20 (basic movement, modes, simple edits)
- **Junior**: J01–J20 + any Starter hints (find-char, operators+motions, dot repeat, search)
- **Medior**: M01–M20 + any earlier (text objects, visual mode, advanced combos)
- **Senior**: X01–X20 + any earlier (macros, registers, substitution, power moves)

## Zone Guidelines

### Starter (difficulty 1-5)
- **Code**: 15-25 lines. Simple variable assignments, print statements, basic loops,
  simple functions. The kind of code from a first programming tutorial.
- **Tasks**: `move_to`, `delete_line`, `replace_char`, simple `change_word`.
- **No**: classes, generics, async, complex nesting, imports beyond basics.

### Junior (difficulty 1-5)
- **Code**: 20-35 lines. Multiple functions, basic classes/structs, error handling,
  list comprehensions, basic patterns.
- **Tasks**: `change_word`, `delete_word`, `insert_text`, `yank_paste`, `dot-repeat`.
- **No**: design patterns, generics (beyond simple), deeply nested code.

### Medior (difficulty 1-5)
- **Code**: 25-40 lines. Design patterns, async/await, generics, moderate nesting,
  trait implementations, type definitions.
- **Tasks**: `change_inside`, `delete_block`, complex `yank_paste`, visual mode tasks.
- **Should**: have bracket pairs for `ci{`/`di(`, quoted strings for `ci"`, repeated
  patterns for `.` repeat.

### Senior (difficulty 1-5)
- **Code**: 25-40 lines. Production-grade. Complex trait bounds, lifetime annotations,
  macro definitions, advanced template metaprogramming, middleware chains.
- **Tasks**: multi-step refactoring, macro-worthy repetition, register juggling,
  complex text object operations.
- **Should**: require combining multiple commands, have patterns where macros shine.

## Quality Checklist

Before saving a segment, verify:

- [ ] Code is **syntactically valid** in the target language
- [ ] Code is **idiomatic** — a real developer would write it this way
- [ ] Code is **15-40 lines** (excluding the TOML wrapper)
- [ ] Every `anchor.pattern` string **appears exactly once** in the code at the
      specified occurrence (search for it!)
- [ ] `optimal_keys` count is **accurate** — mentally execute the optimal Vim
      sequence and count keystrokes
- [ ] Task `description` is a **clear instruction** that doesn't reveal the Vim
      command (say "delete this line" not "press dd")
- [ ] Points scale with difficulty: 50 (trivial) → 150 (moderate) → 500 (complex)
- [ ] Tags accurately reflect the Vim commands needed, not just code content
- [ ] The segment ID is **globally unique** (check existing IDs first)
- [ ] Difficulty rating (1-5) is **consistent** with other segments in the same zone
- [ ] **0–2 VIM TIP comments** are included, zone-appropriate, prefixed with `VIM TIP:`
- [ ] Hint IDs listed in `meta.hints` **match the actual comments** in the code
- [ ] Hints are **relevant to the segment's tasks** when possible

**Additional checks for intro segments:**
- [ ] `intro = true` and `intro_level` are set in meta
- [ ] Explains the new commands clearly in comments before asking the player to use them
- [ ] Practice tasks use the commands just taught (not commands from later levels)
- [ ] Task points are low (25-50) since these are training
- [ ] Short enough to read at the level's scroll speed
- [ ] Level 1-1 intro also covers game mechanics (viewport, tasks, scoring)

## Workflow: Adding a New Segment

1. Decide: language, zone, topic, which Vim commands it should exercise.
2. Write the code first. Make it real, idiomatic, interesting.
3. Read the code and identify natural edit tasks (things a real developer might do).
4. Write the tasks, carefully choosing anchor patterns that are unique.
5. Count optimal keystrokes for each task (mentally execute the Vim sequence).
6. Save to `content/{language}/{zone}/{name}.toml`.
7. Validate by searching for each anchor pattern in the code content.

## Workflow: Adding a New Language

1. Create the directory structure:
   ```
   content/{language}/
   ├── starter/
   ├── junior/
   ├── medior/
   └── senior/
   ```
2. Start with ~10 starter segments to establish the language's "feel."
3. Note which Vim operations the language naturally exercises (document in PLAN.md
   Section 3.1 language table).
4. Build out to ~40 segments per zone (160 total) for full coverage.
5. Update `PLAN.md` Section 3.1 to add the language to the table.
6. The game code needs a minor update to recognize the new language directory —
   add it to the language enum in `src/content/mod.rs`.

## Workflow: Adding a New Task Type

1. Define the type in this skill file (add a new section under Task Types Reference).
2. Document required fields, anchor behavior, and optimal_keys guidance.
3. Implement the task type in `src/game/task.rs` (completion detection logic).
4. Add rendering support in `src/ui/task_overlay.rs` (gutter annotation text).
5. Update the Tag List if the new type maps to a new Vim command category.
6. Write at least 5 segments that use the new task type to validate the design.

## Examples

### Example: Python Starter Segment

```toml
[meta]
id = "py-starter-greeting"
zone = "starter"
language = "python"
tags = ["hjkl", "line-position", "delete-char"]
difficulty = 1

[code]
content = """
name = "World"
greeting = "Hello, " + name
print(greeting)

age = 25
print("I am " + str(age) + " years old")

favorite_color = "blue"
print("I like " + favorite_color)
"""

[[tasks]]
type = "move_to"
anchor = { pattern = "greeting", occurrence = 2 }
description = "Move to the word 'greeting' on the print line"
points = 50
optimal_keys = 4

[[tasks]]
type = "replace_char"
anchor = { pattern = "25", occurrence = 1 }
char_offset = 0
new_char = "3"
description = "Change age from 25 to 35"
points = 50
optimal_keys = 3
```

### Example: Rust Senior Segment

```toml
[meta]
id = "rs-senior-middleware"
zone = "senior"
language = "rust"
tags = ["text-objects", "change-inside", "yank-paste", "generics", "traits"]
difficulty = 4

[code]
content = """
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct RateLimiter {
    max_requests: u32,
    window_secs: u64,
    counts: Arc<RwLock<HashMap<IpAddr, (u32, Instant)>>>,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window_secs,
            counts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn check(&self, ip: IpAddr) -> bool {
        let mut counts = self.counts.write().await;
        let entry = counts.entry(ip).or_insert((0, Instant::now()));
        if entry.1.elapsed().as_secs() > self.window_secs {
            *entry = (0, Instant::now());
        }
        entry.0 += 1;
        entry.0 <= self.max_requests
    }
}
"""

[[tasks]]
type = "change_word"
anchor = { pattern = "window_secs", occurrence = 1 }
new_text = "window_duration"
description = "Rename 'window_secs' to 'window_duration'"
points = 200
optimal_keys = 5

[[tasks]]
type = "change_inside"
anchor = { pattern = "Arc<RwLock<HashMap<IpAddr, (u32, Instant)>>>", occurrence = 1 }
delimiter = "<"
new_text = "DashMap<IpAddr, (u32, Instant)>"
description = "Replace the Arc<RwLock<HashMap<...>>> with DashMap<...>"
points = 300
optimal_keys = 7

[[tasks]]
type = "delete_line"
anchor = { pattern = "let mut counts", occurrence = 1 }
description = "Delete the write lock acquisition (no longer needed with DashMap)"
points = 100
optimal_keys = 2
```
