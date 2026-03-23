# Vim Heroes — Master Plan

A Guitar Hero–inspired terminal game that teaches Vim through escalating, real-code
challenges. Text scrolls down like a note highway; your cursor must keep up or it's
game over. Fewer keystrokes = more points. Powered by real Vim commands.

---

## Table of Contents

1. [Game Design](#1-game-design)
2. [Zone & Level System](#2-zone--level-system)
3. [Content System](#3-content-system)
4. [Keybinding / Config System](#4-keybinding--config-system)
5. [Technical Architecture](#5-technical-architecture)
6. [Credit Shop & Progression](#6-credit-shop--progression)
7. [Multi-Buffer System](#7-multi-buffer-system)
8. [Phased Implementation Plan](#8-phased-implementation-plan)

---

## 1. Game Design

### 1.1 Core Loop

```
┌─────────────┐      ┌───────────────┐      ┌────────────┐
│  Viewport   │─────▶│  Player edits │─────▶│  Scoring   │
│  scrolls    │      │  buffer with  │      │  evaluated │
│  down       │      │  Vim commands │      │            │
└─────────────┘      └───────────────┘      └────────────┘
       │                                          │
       ▼                                          ▼
  Cursor out of                             Task done?
  viewport? ──▶ GAME OVER                  ──▶ Points + combo
```

- The **text buffer is static** — it's a real code file assembled from segments.
- The **viewport scrolls down** at a steady rate (line-by-line at intervals).
- The player's **cursor must stay within the visible viewport** — if the viewport
  scrolls past the cursor, it's game over.
- **Tasks** appear as highlighted regions in the code ahead of the cursor. The player
  must navigate to them and execute the correct edit before they scroll away.
- **Player-driven scroll boost**: when the cursor moves past the bottom of the
  viewport, the viewport snaps forward by the same distance. This lets skilled
  players speed things up — e.g. `6j` at the bottom edge scrolls the viewport
  6 lines forward instantly. The auto-scroll timer resets after a boost so the
  player isn't immediately punished.

### 1.1b Level Start: Countdown & Import Runway

Every level starts with two grace mechanisms so the player isn't thrown straight
into tasks:

**1. Import runway** — The assembler prepends 5–10 lines of language-appropriate
import statements before the first code segment. These lines never contain tasks.
They give the player a visual "runway" to orient themselves in the code while the
viewport scrolls through harmless content.

```python
import os
import sys
from collections import defaultdict
from typing import List, Optional

# ---
# (first code segment with tasks starts here)
```

The imports are randomly selected from a pool per language so they don't feel
repetitive. They also double as realistic code context.

**2. Countdown** — A 3-second countdown (`3... 2... 1...`) is displayed as an
overlay before scrolling begins. During the countdown the viewport is frozen and
no keystrokes are penalized. After "1" disappears, scrolling starts and the game
is live. This mirrors Guitar Hero / Rock Band's pre-song countdown.

Combined, these mean the first task is always at least ~10 lines into the buffer
and the player has 3 seconds of orientation time before anything moves.

### 1.2 Scoring

| Source              | Points      | Notes                                    |
|---------------------|-------------|------------------------------------------|
| Each second alive   | +10         | Baseline survival reward                 |
| Each keystroke      | −2          | Penalizes inefficiency                   |
| Task completed      | +50 to +500 | Scales with task complexity              |
| Optimal solution    | +100 bonus  | Used ≤ optimal number of keystrokes      |
| Combo multiplier    | ×1.5 / ×2 / ×3 | Consecutive optimal task completions |
| Missed task         | −50         | Task scrolled off-screen uncompleted     |

Tasks must be worth significantly more than survival points so the optimal strategy
is "complete tasks efficiently," not "hold j and ignore everything."

### 1.3 Star Rating (per level)

- ★☆☆ — Completed the level (survived to the end)
- ★★☆ — Completed all tasks
- ★★★ — Completed all tasks within the optimal keystroke budget

### 1.4 Visual Design (Terminal)

```
┌──────────────────────────────────────────────────┐
│ ★★☆  Level 2-3  "Word Hopping"   Score: 1,250  ×2│  ← HUD
├──────────────────────────────────────────────────┤
│  14 │   for (const item of items) {              │
│  15 │     sum += item.price;                     │
│  16 │ ██  sum += item.tax  ██       CHG → "cost" │  ← red = pending task
│  17 │   }                                        │
│  18 │   return sum;               █              │  ← cursor
│  19 │ }                                          │
│  20 │ ▓▓  console.log(total)  ▓▓       ✓ DONE   │  ← green = completed
│  21 │                                            │
├──────────────────────────────────────────────────┤
│ NORMAL │ Keys: 12 │ ▸ Change "tax" → "cost"     │  ← status bar
└──────────────────────────────────────────────────┘
```

- **Red background**: task pending, with a short annotation in the right gutter.
- **Green background**: task completed.
- **Yellow background**: partially done / cursor is on it.
- **Status bar**: current mode, keystroke count, current task description.
- **HUD**: level name, star progress, score, combo multiplier.

### 1.5 Game Over Screen

Show: final score, stars earned, tasks completed/total, keystrokes used vs optimal,
and a breakdown of commands used. Offer: retry, next level, back to menu.

### 1.6 Tutorial-as-Gameplay: Intro Segments

There is no separate tutorial. Every level begins with an **intro segment** — a
specially crafted code snippet that teaches the player exactly what they need for
that level. The intro segment scrolls in just like regular code, but it's
structured as a guided walkthrough using comments.

The player learns by doing: read the instruction, do the thing, see it work —
all while the viewport is already scrolling. This creates a natural flow from
"learning" to "playing" within a single level.

#### How it works

1. Each level's first segment is always a **tutorial intro segment** (tagged
   `intro: true` in the TOML). It scrolls at the same speed as the rest.
2. The intro segment contains comment blocks that explain the new commands,
   interleaved with simple practice tasks.
3. After the intro segment, regular code segments follow with real tasks.
4. The intro tasks are worth fewer points (25–50) — they're training wheels.

#### Example: Level 1-2 intro (introduces `w` and `b`)

```
  1 │ # ═══════════════════════════════════
  2 │ # LEVEL 1-2: Word Motions
  3 │ # ═══════════════════════════════════
  4 │ #
  5 │ # You already know h/j/k/l.
  6 │ # But pressing 'lllllll' to cross a
  7 │ # line is slow. There's a better way:
  8 │ #
  9 │ # 'w' — jump to the next word
 10 │ # 'b' — jump back a word
 11 │ #
 12 │ # Try it! Move to 'target' below:
 13 │ #
 14 │ name = "hello"
 15 │ ██ target ██ = "world"        ◄ MOVE HERE
 16 │ result = name + target
 17 │ #
 18 │ # Nice! Now 'w' is your best friend.
 19 │ # The rest of this level uses real
 20 │ # code — use 'w' and 'b' to move
 21 │ # efficiently!
 22 │ #
 23 │ # ═══════════════════════════════════
 24 │
```

#### Intro segments per world

Each world introduces new commands, so each world's first level (X-1) has a
full intro. Later levels (X-2 through X-5) have shorter "reminder" intros
that just list the available commands in a brief header comment.

| Level | Intro type | What it teaches |
|-------|------------|-----------------|
| 1-1   | Full intro | `h` `j` `k` `l` — how the game works, what the viewport is, what tasks look like |
| 1-2   | Full intro | `w` `b` `e` — word motions |
| 1-3   | Short reminder | `0` `^` `$` |
| 1-4   | Short reminder | `gg` `G` `{num}G` |
| 1-5   | Short reminder | Mixed review |
| 2-1   | Full intro | `x` `X` — first editing commands |
| 2-2   | Full intro | `dd` `yy` `p` — delete/yank/paste |
| 2-3   | Short reminder | `i` `a` `I` `A` |
| ...   | ... | ... |
| 7-1   | Full intro | `q{reg}` `@{reg}` — macros |

The very first intro (1-1) is special — it also explains the game mechanics:
the scrolling viewport, what happens when you fall behind, what the colored
highlights mean, and the scoring system. This is the only "meta-tutorial."

#### Segment format addition

```toml
[meta]
id = "intro-1-2-word-motions"
zone = "starter"
language = "python"        # one intro per language per level
tags = ["words"]
difficulty = 1
intro = true               # marks this as a level intro segment
intro_level = "1-2"        # which level this intro belongs to

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
points = 25
optimal_keys = 2
```

### 1.7 Vim Hints System

Vim hints appear throughout the game to teach concepts alongside the hands-on
practice. Hints are delivered through three channels:

**1. Code comment hints** — Hints embedded as comments in the code segments
themselves. As the code scrolls by, the player reads real tips mixed into real
code. This is the primary hint delivery mechanism.

```python
# VIM TIP: Use 'w' to jump forward by word — much faster than 'llllll'
def calculate_total(items):
    total = 0
    # VIM TIP: 'ci"' changes everything inside quotes in one move
    label = "Grand Total"
```

**2. Level intro hints** — A short tip shown on the loading screen before each
level starts, introducing the new command(s) that level focuses on.

```
┌─────────────────────────────────────────┐
│         Level 1-2: Word Motions         │
│                                         │
│   NEW COMMAND: w                        │
│   Jump forward to the start of the      │
│   next word.                            │
│                                         │
│   Try it! Much faster than pressing     │
│   'l' many times.                       │
│                                         │
│        Press any key to start...        │
└─────────────────────────────────────────┘
```

**3. Post-level insights** — After completing a level, show a "did you know?"
tip based on what the player actually did. If they used 20 `l` presses where
`w` would have taken 3, show: *"You pressed 'l' 20 times — try 'w' to jump
by word and save keystrokes!"*

#### Hint Catalog

Hints are tiered by zone. Each zone introduces hints for the commands being
taught, plus general Vim wisdom at that skill level. Code segments in each zone
should include 0–2 comment hints per segment, drawn from that zone's pool.

##### Zone: Starter

| ID | Hint |
|----|------|
| S01 | `h` `j` `k` `l` — left, down, up, right. Your fingers never leave home row. |
| S02 | Think of `j` as having a downward hook — it moves down. |
| S03 | `w` jumps to the next word. Way faster than `llllll`. |
| S04 | `b` jumps back a word. The reverse of `w`. |
| S05 | `e` jumps to the end of the current word. |
| S06 | `0` goes to the first column. `^` goes to the first non-space character. |
| S07 | `$` goes to the end of the line. Think of it like regex. |
| S08 | `gg` goes to the top of the file. `G` goes to the bottom. |
| S09 | `5G` jumps to line 5. Works with any number. |
| S10 | `x` deletes the character under the cursor. Like a tiny eraser. |
| S11 | `dd` deletes the entire current line. Quick and clean. |
| S12 | `u` undoes your last change. `Ctrl-r` redoes it. |
| S13 | `i` enters insert mode before the cursor. `a` enters after. |
| S14 | `I` inserts at the start of the line. `A` appends at the end. |
| S15 | `o` opens a new line below and enters insert mode. `O` opens above. |
| S16 | `p` pastes after the cursor. `P` pastes before. |
| S17 | `yy` yanks (copies) the entire current line. |
| S18 | After `dd`, press `p` to paste the deleted line somewhere else — it's a cut! |
| S19 | Press `Esc` to return to Normal mode from anywhere. Always your safe home. |
| S20 | In Vim, you spend most of your time in Normal mode, not Insert mode. |

##### Zone: Junior

| ID | Hint |
|----|------|
| J01 | `f{char}` finds the next occurrence of {char} on the line. `fa` jumps to the next 'a'. |
| J02 | `t{char}` jumps to just before {char}. `tf` stops one character before 'f'. |
| J03 | `;` repeats your last `f` or `t` search forward. `,` repeats it backward. |
| J04 | `dw` = delete a word. Operators (`d`) + motions (`w`) combine into power moves. |
| J05 | `d$` deletes from cursor to end of line. `d0` deletes to the start. |
| J06 | `cw` changes a word — deletes it and drops you into insert mode. |
| J07 | `c$` changes from cursor to end of line. You can also use `C` as a shortcut. |
| J08 | `.` repeats your last change. Did `cw`+typed "foo"+`Esc`? Now `.` does it again. |
| J09 | The dot command `.` is one of Vim's most powerful features. Master it. |
| J10 | `r{char}` replaces the character under the cursor without entering insert mode. |
| J11 | `R` enters Replace mode — every character you type overwrites the existing text. |
| J12 | Counts work with motions: `3w` jumps 3 words forward. `5j` moves 5 lines down. |
| J13 | Counts work with operators too: `3dd` deletes 3 lines. `2dw` deletes 2 words. |
| J14 | `/pattern` searches forward. `?pattern` searches backward. |
| J15 | After searching, `n` goes to the next match, `N` goes to the previous. |
| J16 | `*` searches for the word under your cursor. `#` searches backward for it. |
| J17 | Think in terms of "verb + noun": `d` (delete) + `w` (word) = delete word. |
| J18 | `D` is shorthand for `d$`. `C` is shorthand for `c$`. Saves one keystroke. |
| J19 | `Y` yanks the entire line (same as `yy`). `yw` yanks one word. |
| J20 | Combine counts: `d3w` deletes 3 words. Every operator accepts a count. |

##### Zone: Medior

| ID | Hint |
|----|------|
| M01 | `ciw` = change inner word. Deletes the word and enters insert mode. Works anywhere in the word. |
| M02 | `ci"` changes everything inside double quotes. `ci'` for single quotes. |
| M03 | `ci(` changes inside parentheses. Also works: `ci{`, `ci[`, `ci<`. |
| M04 | `di"` deletes inside quotes. `da"` deletes the quotes too (around). |
| M05 | `i` = inner (inside the delimiters). `a` = around (includes the delimiters). |
| M06 | `vi{` visually selects everything inside curly braces. Great for function bodies. |
| M07 | `v` starts visual mode (character-wise). `V` selects whole lines. |
| M08 | In visual mode, press `o` to jump to the other end of your selection. |
| M09 | `Ctrl-v` enters visual block mode — select a rectangle of text. |
| M10 | In visual block mode, `I` inserts text on every selected line. |
| M11 | `%` jumps to the matching bracket: `(↔)`, `{↔}`, `[↔]`. |
| M12 | `>` indents, `<` dedents. `>>` indents the current line. `>i{` indents inside braces. |
| M13 | `gU` makes text uppercase. `gu` makes it lowercase. `gUiw` = uppercase the word. |
| M14 | Text objects understand nesting: `ci(` inside `f(g(x))` changes the innermost `()`. |
| M15 | `.` after `ciw`+type+`Esc` lets you change the next occurrence instantly. |
| M16 | `diw` then `w` then `.` — delete words one at a time, wherever you want. |
| M17 | `dt{char}` deletes from cursor up to (but not including) {char}. |
| M18 | `yiw` yanks a word without moving the cursor. Great for copy-paste workflows. |
| M19 | After `y`, the cursor stays where it was. Use `p` to paste at the destination. |
| M20 | Think of text objects as "what", motions as "where": `d` + `iw` = delete [what: inner word]. |

##### Zone: Senior

| ID | Hint |
|----|------|
| X01 | `qa` starts recording a macro into register 'a'. `q` stops recording. `@a` replays it. |
| X02 | `@@` replays the last macro. `5@a` replays macro 'a' five times. |
| X03 | Plan your macro: get to a consistent starting position first, end in a position ready for the next `@a`. |
| X04 | `"ayy` yanks a line into register 'a'. `"ap` pastes from register 'a'. |
| X05 | Registers `a-z` store text. Uppercase `"Ayy` appends to register 'a' instead of replacing. |
| X06 | `"0` always holds your last yank. `""` holds the last delete or yank. |
| X07 | `"+y` yanks to the system clipboard. `"+p` pastes from it. |
| X08 | `:reg` shows all register contents. Useful when juggling multiple registers. |
| X09 | Macros ARE register contents. `"ap` pastes macro 'a' as text. Edit it, then `"ayy` to save back. |
| X10 | `g;` goes to the last change position. `g,` goes to the next. Great for navigating edits. |
| X11 | `Ctrl-a` increments a number under the cursor. `Ctrl-x` decrements it. |
| X12 | `gf` opens the file path under the cursor. Useful in imports. |
| X13 | `=` auto-indents. `=i{` fixes indentation inside a block. `gg=G` re-indents the whole file. |
| X14 | A well-crafted macro + `100@a` can refactor an entire file in one move. |
| X15 | `c3w` vs `3cw` — both change 3 words, but muscle memory prefers one. Experiment. |
| X16 | `xp` swaps two characters. `ddp` swaps two lines. Quick micro-refactors. |
| X17 | `:s/old/new/g` substitutes on the current line. `:%s/old/new/g` does the whole file. |
| X18 | `:g/pattern/d` deletes all lines matching a pattern. `:v/pattern/d` keeps only matching lines. |
| X19 | If you're doing the same edit more than twice, you should be recording a macro. |
| X20 | The best Vim command is the one that gets the job done in the fewest keystrokes. |

---

## 2. Zone & Level System

### 2.1 Four Zones

Each zone maps to a skill tier and a code complexity tier. Zones contain multiple
worlds, each world contains 5 levels.

| Zone       | Worlds | Scroll Speed    | Vim Skills                        | Code Style            |
|------------|--------|-----------------|-----------------------------------|-----------------------|
| **Starter**  | 1–2    | 1 line / 2–3s   | Movement, basic editing           | Hello world, simple scripts |
| **Junior**   | 3–4    | 1 line / 1.5–2s | Word motions, operators+motions   | Functions, classes, basic patterns |
| **Medior**   | 5–6    | 1 line / 1–1.5s | Text objects, search, visual mode | Design patterns, async, generics |
| **Senior**   | 7–8    | 1 line / 0.5–1s | Macros, registers, advanced combos | Production-grade, complex architectures |

### 2.2 Worlds & Levels

Each world introduces specific commands, then tests them in 5 levels of increasing
density and speed.

#### Zone: Starter

**World 1 — First Steps**
| Level | New Commands         | Task Types                     |
|-------|----------------------|--------------------------------|
| 1-1   | `h` `j` `k` `l`     | Move cursor to marked positions |
| 1-2   | `w` `b` `e`         | Jump to highlighted words       |
| 1-3   | `0` `^` `$`         | Reach start/end of lines        |
| 1-4   | `gg` `G` `{num}G`   | Jump to specific line numbers   |
| 1-5   | All of the above     | Mixed movement challenges       |

**World 2 — First Edits**
| Level | New Commands         | Task Types                     |
|-------|----------------------|--------------------------------|
| 2-1   | `x` `X`             | Delete specific characters      |
| 2-2   | `dd` `yy` `p` `P`   | Delete/yank/paste lines         |
| 2-3   | `i` `a` `I` `A`     | Insert text at positions        |
| 2-4   | `o` `O`             | Open lines and insert           |
| 2-5   | All of the above     | Mixed basic editing             |

#### Zone: Junior

**World 3 — Precision**
| Level | New Commands         | Task Types                     |
|-------|----------------------|--------------------------------|
| 3-1   | `f` `t` `F` `T`     | Find characters on a line       |
| 3-2   | `;` `,`             | Repeat find motions             |
| 3-3   | `dw` `db` `d$` `d0` | Delete with motions             |
| 3-4   | `cw` `cb` `c$`      | Change with motions             |
| 3-5   | `.` (dot repeat)     | Repeat last change efficiently  |

**World 4 — Speed**
| Level | New Commands         | Task Types                     |
|-------|----------------------|--------------------------------|
| 4-1   | `r` `R`             | Replace characters              |
| 4-2   | `u` `Ctrl-r`        | Undo/redo (fix intentional mistakes) |
| 4-3   | `/` `?` `n` `N`     | Search navigation               |
| 4-4   | `*` `#`             | Word-under-cursor search        |
| 4-5   | All of the above     | Mixed precision + speed         |

#### Zone: Medior

**World 5 — Text Objects**
| Level | New Commands              | Task Types                   |
|-------|---------------------------|------------------------------|
| 5-1   | `iw` `aw`                | Inner/around word            |
| 5-2   | `i"` `i'` `a"` `a'`     | Inside/around quotes         |
| 5-3   | `i(` `i{` `i[` `a(` etc | Inside/around brackets       |
| 5-4   | `ci"` `di(` `yi{`       | Operators + text objects     |
| 5-5   | All of the above          | Mixed text object challenges |

**World 6 — Visual & Combine**
| Level | New Commands              | Task Types                   |
|-------|---------------------------|------------------------------|
| 6-1   | `v` + motions             | Visual character selection   |
| 6-2   | `V` + motions             | Visual line selection        |
| 6-3   | `Ctrl-v`                  | Visual block mode            |
| 6-4   | `%` (bracket matching)    | Navigate matching pairs      |
| 6-5   | All of the above          | Complex multi-step edits     |

#### Zone: Senior

**World 7 — Power User**
| Level | New Commands              | Task Types                   |
|-------|---------------------------|------------------------------|
| 7-1   | `q{reg}` ... `q` `@{reg}` | Record and replay macros    |
| 7-2   | `"{reg}y` `"{reg}p`      | Named registers              |
| 7-3   | `>` `<` indent operators  | Code indentation tasks       |
| 7-4   | `gU` `gu` case operators  | Case manipulation            |
| 7-5   | All of the above          | Mixed power user             |

**World 8 — Mastery**
| Level | New Commands              | Task Types                       |
|-------|---------------------------|----------------------------------|
| 8-1   | Complex macro chains      | Multi-step refactoring w/ macros |
| 8-2   | Register juggling         | Yank/paste across distant code   |
| 8-3   | Everything                | Real refactoring: rename vars    |
| 8-4   | Everything                | Real refactoring: restructure    |
| 8-5   | Everything                | Boss battle: full code overhaul  |

### 2.3 Special Modes

- **Practice Mode**: No scrolling, no timer. Just tasks on a static buffer. For
  learning new commands before tackling the real levels.
- **Endless Mode**: Infinite scrolling code with random tasks. Score-attack with a
  global leaderboard. Speed increases every 60 seconds.
- **Daily Challenge**: One shared level per day, same for everyone. Leaderboard.

---

## 3. Content System

### 3.1 Languages

Players choose their language before starting. Available languages:

| Language          | Priority | Ship in   |
|-------------------|----------|-----------|
| Python            | 1        | Phase 8   |
| TypeScript        | 1        | Phase 8   |
| Rust              | 2        | Phase 10  |
| C++               | 2        | Phase 10  |

Each language exercises different Vim strengths:

| Language       | Natural Vim focus                                          |
|----------------|----------------------------------------------------------|
| Python         | Indentation, `:` landmarks, f-strings, less bracket nesting |
| TypeScript     | Deep nesting (`ci{`), generics `<>`, template literals     |
| Rust           | Lifetimes `'a` as f-targets, `::` paths, match arms       |
| C++            | `<>` templates, `::` scope, `*&` pointers, `#` preprocessor |

### 3.2 Segment Pool Architecture

Code content is organized as a **pool of segments** — self-contained 15–40 line code
blocks. Each playthrough randomly selects and stitches segments together.

```
content/
├── python/
│   ├── starter/        # ~40 segments  (~800-1200 lines total)
│   ├── junior/         # ~40 segments
│   ├── medior/         # ~40 segments
│   └── senior/         # ~40 segments
├── typescript/
│   ├── starter/
│   └── ...
├── rust/
│   └── ...
└── cpp/
    └── ...
```

**Per language**: ~160 segments, ~3500-5000 lines of code.
**Total**: ~640 segments, ~15,000-20,000 lines across all languages.

Each playthrough uses 3–6 segments. Combined with the "no repeat from last 3
playthroughs" rule, players get thousands of unique combinations before repetition.

### 3.3 Segment Format

```toml
[meta]
id = "py-junior-api-fetch"
zone = "junior"
language = "python"
tags = ["functions", "error-handling", "http"]
difficulty = 3                    # 1-5 within the zone
hints = ["J04", "J06"]           # Hint IDs from Section 1.6 to embed as comments

[code]
content = """
import requests

# VIM TIP: 'dw' deletes a word. Operators + motions = power moves.
def fetch_user_profile(user_id: str) -> dict:
    url = f"https://api.example.com/users/{user_id}"
    response = requests.get(url, timeout=10)
    # VIM TIP: 'cw' changes a word — deletes it and drops you into insert mode.
    response.raise_for_status()
    return response.json()
"""

[[tasks]]
type = "change_word"
anchor = { pattern = '"full_name"', occurrence = 1 }
new_text = '"display_name"'
description = "Change 'full_name' to 'display_name'"
points = 100
optimal_keys = 5

[[tasks]]
type = "delete_line"
anchor = { pattern = "timeout=10", occurrence = 1 }
description = "Delete this line"
points = 75
optimal_keys = 2
```

### 3.4 Task Types

| Type           | Description                        | Example                          |
|----------------|------------------------------------|----------------------------------|
| `move_to`      | Place cursor on target             | "Move to the word 'fetch'"       |
| `delete_line`  | Delete an entire line              | "Delete this comment"            |
| `delete_word`  | Delete a word                      | "Delete the variable name"       |
| `change_word`  | Replace a word with new text       | "Change 'foo' to 'bar'"         |
| `change_inside`| Change content inside delimiters   | "Change the string contents"     |
| `insert_text`  | Insert text at a position          | "Add a return statement"         |
| `yank_paste`   | Copy from one place, paste another | "Copy this line to below line 20"|
| `delete_block` | Delete multiple lines              | "Delete this function body"      |
| `indent`       | Fix indentation                    | "Indent this block"              |
| `replace_char` | Replace a single character         | "Fix this typo: a → e"          |

### 3.5 Assembly Algorithm

When a level starts:

1. Determine zone from level number.
2. Load segment pool for the player's chosen language + zone.
3. Randomly select 3–6 segments, weighted by:
   - Tags matching the level's target commands.
   - Not recently seen (tracked in save file, last 3 playthroughs).
4. Stitch segments with natural separators (blank lines, comments like
   `// ---` or `# ---`).
5. Resolve task anchors → absolute line/column positions in assembled buffer.
6. Order tasks top-to-bottom to match scroll direction.

### 3.6 Code Complexity by Zone

**Starter** — Tutorial-level code:
```python
name = "Alice"
print("Hello, " + name)

numbers = [1, 2, 3, 4, 5]
total = 0
for n in numbers:
    total = total + n
print(total)
```

**Junior** — Structured code with functions and classes:
```python
from dataclasses import dataclass

@dataclass
class Product:
    name: str
    price: float
    quantity: int

    def total_value(self) -> float:
        return self.price * self.quantity
```

**Medior** — Design patterns, async, generics:
```typescript
class LRUCache<T> {
  private items = new Map<string, CacheEntry<T>>();

  get(key: string): T | undefined {
    const entry = this.items.get(key);
    if (!entry) return undefined;
    if (Date.now() > entry.expires_at) {
      this.items.delete(key);
      return undefined;
    }
    this.items.delete(key);
    this.items.set(key, entry);
    return entry.value;
  }
}
```

**Senior** — Production-grade, complex architecture:
```rust
trait Middleware: Send + Sync + 'static {
    fn handle<'a>(
        &'a self,
        req: Request,
        next: &'a dyn Fn(Request) -> BoxFuture<'a, Response>,
    ) -> BoxFuture<'a, Response>;
}
```

### 3.7 Adding New Content

New segments, languages, and task types can be added using the
`add-content` skill file (see `.claude/skills/add-content.md`). The skill
guides contributors through the segment format, validation rules, and
naming conventions.

---

## 4. Keybinding / Config System

### 4.1 Config File Location

`~/.vim-heroes/config.toml` — created on first launch with sensible defaults
and comments explaining every option.

### 4.2 Config Structure

```toml
[general]
language = "python"            # default language for new games
show_hints = true              # show command hints during gameplay
color_theme = "default"        # "default", "gruvbox", "solarized", "catppuccin"

[keymap]
# Optional: load a preset as a base, then override individual keys below.
# preset = "colemak-dh"        # "qwerty" (default), "colemak", "colemak-dh", "dvorak", "workman"

# Movement
move_left = "h"
move_down = "j"
move_up = "k"
move_right = "l"
word_forward = "w"
word_forward_big = "W"
word_back = "b"
word_back_big = "B"
word_end = "e"
word_end_big = "E"
line_start = "0"
line_first_char = "^"
line_end = "$"
goto_line_top = "gg"
goto_line_bottom = "G"
find_char_forward = "f"
find_char_backward = "F"
till_char_forward = "t"
till_char_backward = "T"
repeat_find = ";"
repeat_find_reverse = ","
match_bracket = "%"
paragraph_up = "{"
paragraph_down = "}"

# Operators
delete = "d"
change = "c"
yank = "y"
paste_after = "p"
paste_before = "P"
delete_char = "x"
replace_char = "r"
undo = "u"
redo = "C-r"

# Mode switching
insert_before = "i"
insert_after = "a"
insert_line_start = "I"
insert_line_end = "A"
open_below = "o"
open_above = "O"
visual_char = "v"
visual_line = "V"
visual_block = "C-v"
escape = "Escape"              # also supports arrays: ["Escape", "jk"]
command_mode = ":"

# Search
search_forward = "/"
search_backward = "?"
search_next = "n"
search_prev = "N"
search_word = "*"
search_word_back = "#"

# Other
repeat_last = "."
record_macro = "q"
play_macro = "@"
```

### 4.3 Key Syntax

| Syntax     | Meaning                              |
|------------|--------------------------------------|
| `"j"`      | Single character key                 |
| `"C-r"`    | Ctrl + r                             |
| `"S-k"`    | Shift + k                            |
| `"Escape"` | Special key name                     |
| `"Space"`  | Spacebar                             |
| `"Enter"`  | Enter/Return                         |
| `"gg"`     | Multi-key sequence (sequential)      |
| `["Escape", "jk"]` | Multiple bindings for one action |

### 4.4 Built-in Presets

| Preset       | Movement keys | Notes                            |
|--------------|---------------|----------------------------------|
| `qwerty`     | `h j k l`     | Default, stock Vim               |
| `colemak`    | `h n e i`     | Common Colemak Vim remap         |
| `colemak-dh` | `m n e i`     | Colemak-DH variant               |
| `dvorak`     | `d h t n`     | Common Dvorak Vim remap          |
| `workman`    | `y n e o`     | Workman layout                   |

Presets remap the full set of keys for that layout. Individual overrides in
`[keymap]` take precedence over the preset.

### 4.5 Architecture Integration

```
Raw keystroke (crossterm)
        │
        ▼
  ┌────────────┐
  │   Keymap    │  ← reads config.toml, resolves sequences with timeout
  │   Resolver  │     maps physical keys → logical Actions
  └────────────┘
        │
        ▼
  Logical Action (e.g., Action::WordForward)
        │
        ▼
  ┌────────────┐
  │    Vim      │  ← operates on logical actions only
  │   Engine    │     never sees raw keys
  └────────────┘
```

**Scoring** counts physical keystrokes, not logical actions — remapping doesn't
change your score.

**Hints and ghost replays** display keys in the player's active mapping.

---

## 5. Technical Architecture

### 5.1 Stack

| Component        | Choice             | Rationale                              |
|------------------|--------------------|----------------------------------------|
| Language         | Rust               | Single binary, fast rendering, no runtime |
| Terminal UI      | ratatui + crossterm | Production-grade TUI (powers lazygit, etc.) |
| Text buffer      | ropey              | Efficient rope DS for insert/delete    |
| Config parsing   | serde + toml       | Ergonomic TOML parsing                 |
| Content embed    | include_dir        | Bake segments into the binary          |
| Save data        | serde + bincode    | Fast local save to ~/.vim-heroes/      |

### 5.2 Project Structure

```
vim-heroes/
├── src/
│   ├── main.rs                  # Entry point, terminal init/cleanup
│   ├── app.rs                   # Top-level state machine: Menu → Playing → Results
│   │
│   ├── vim/                     # Vim emulation engine
│   │   ├── mod.rs
│   │   ├── buffer.rs            # Text buffer (rope-backed)
│   │   ├── cursor.rs            # Cursor position, clamping, movement
│   │   ├── mode.rs              # Normal / Insert / Visual / Operator-pending
│   │   ├── command.rs           # Keystroke → partial/complete command parser
│   │   ├── motions.rs           # h/j/k/l/w/b/e/f/t/G/gg/$/^/0 etc.
│   │   ├── operators.rs         # d/c/y + motion/text-object combinations
│   │   ├── text_objects.rs      # iw/aw/i"/a(/i{ etc.
│   │   ├── registers.rs         # Yank/delete registers ("a-z, unnamed, etc.)
│   │   ├── macros.rs            # Macro record/replay
│   │   └── buffers.rs           # Multi-buffer manager (list, switch, active index)
│   │
│   ├── game/                    # Game mechanics
│   │   ├── mod.rs
│   │   ├── engine.rs            # Core game loop: tick, scroll, input, render
│   │   ├── viewport.rs          # Viewport position, scroll speed, bounds check
│   │   ├── scoring.rs           # Points, combo, star calculation
│   │   ├── task.rs              # Task state machine: pending → active → done/missed
│   │   └── level.rs             # Level metadata, progression logic
│   │
│   ├── content/                 # Content management
│   │   ├── mod.rs
│   │   ├── segment.rs           # Segment struct, TOML parsing
│   │   ├── assembler.rs         # Stitch segments into a level buffer
│   │   ├── anchor.rs            # Resolve pattern anchors → buffer positions
│   │   └── history.rs           # Track recently-seen segments for variety
│   │
│   ├── config/                  # Configuration
│   │   ├── mod.rs
│   │   ├── keymap.rs            # Key mapping resolution, sequence timeout
│   │   ├── presets.rs           # Embedded layout presets
│   │   └── key_syntax.rs        # Parser for "C-r", "gg", ["Escape","jk"] etc.
│   │
│   ├── ui/                      # Terminal rendering
│   │   ├── mod.rs
│   │   ├── game_view.rs         # Main gameplay screen
│   │   ├── menu.rs              # Main menu, level select, language picker
│   │   ├── hud.rs               # Score, combo, stars, level info
│   │   ├── task_overlay.rs      # Red/green/yellow highlights + gutter annotations
│   │   ├── results.rs           # End-of-level results screen
│   │   ├── shop.rs              # Credit shop: motions, buffer cmds, cosmetics
│   │   └── theme.rs             # Color themes
│   │
│   └── progress/                # Player progress
│       ├── mod.rs
│       ├── save.rs              # Stars, high scores, unlocks → ~/.vim-heroes/save.dat
│       ├── credits.rs           # Credit balance, earn/spend logic
│       └── unlocks.rs           # Unlocked motions, cosmetics, buffer commands
│
├── content/                     # Code segments (embedded at compile time)
│   ├── python/
│   │   ├── starter/
│   │   │   ├── hello_world.toml
│   │   │   ├── fizzbuzz.toml
│   │   │   └── ...
│   │   ├── junior/
│   │   ├── medior/
│   │   └── senior/
│   ├── typescript/
│   │   └── ...
│   ├── rust/
│   │   └── ...
│   └── cpp/
│       └── ...
│
├── Cargo.toml
├── PLAN.md                      # This file
└── README.md
```

### 5.3 Key Dependencies

```toml
[dependencies]
ratatui = "0.29"
crossterm = "0.28"
ropey = "1.6"
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
include_dir = "0.7"
dirs = "5.0"
bincode = "1.3"
rand = "0.8"
```

Binary size target: ~5-8 MB (all content embedded).

### 5.4 Game Loop

```rust
// Pseudocode
loop {
    // 1. Non-blocking input
    if poll_input(timeout: 33ms) {        // ~30 fps
        let key = read_key();
        keystroke_count += 1;
        let action = keymap.resolve(key); // physical → logical
        vim_engine.execute(action);
        check_task_completion(&mut tasks, &buffer, &cursor);
    }

    // 2. Scroll tick
    if elapsed >= scroll_interval {
        viewport.scroll_down(1);
        if cursor.line < viewport.top_line {
            return GameOver;
        }
        score += SURVIVAL_POINTS;
    }

    // 3. Check for missed tasks
    for task in &mut tasks {
        if task.is_pending() && task.line < viewport.top_line {
            task.mark_missed();
            score -= MISS_PENALTY;
            combo = 0;
        }
    }

    // 4. Render
    terminal.draw(|frame| {
        render_hud(frame, &game_state);
        render_buffer(frame, &buffer, &viewport, &cursor, &tasks);
        render_statusbar(frame, &vim_engine, &keystroke_count, &current_task);
    });

    // 5. Check level complete
    if viewport.bottom_line >= buffer.len_lines() {
        return LevelComplete;
    }
}
```

### 5.5 Distribution

| Channel              | Tool / Method          | Audience              |
|----------------------|------------------------|-----------------------|
| `cargo install`      | crates.io              | Rust developers       |
| Homebrew             | homebrew-tap           | macOS / Linux         |
| AUR                  | PKGBUILD               | Arch Linux            |
| `.deb` package       | cargo-deb              | Debian / Ubuntu       |
| GitHub Releases      | Prebuilt binaries      | Everyone              |
| Snap / Flatpak       | snapcraft / flatpak    | Universal Linux       |

---

## 6. Credit Shop & Progression

The game uses a Tony Hawk Pro Skater–style unlock system. Players earn credits
after every level and spend them on new Vim motions, buffer commands, and
cosmetics. This aligns the game mechanic with how people actually learn Vim —
layering on commands over time — but gives the player agency over *what* to
learn next.

### 6.1 Earning Credits

| Source                    | Credits | Notes                                      |
|---------------------------|---------|---------------------------------------------|
| Level completed           | 50      | Baseline reward for surviving               |
| Per task completed        | 10–30   | Scales with task complexity                 |
| Star bonus (per star)     | 25      | Up to 75 bonus for a 3-star run             |
| Optimal solution bonus    | 15      | Per task solved within optimal keystrokes   |
| First-time level clear    | 100     | One-time bonus per level                    |

Credits are tracked in the save file alongside stars and scores.

### 6.2 Motion Unlock Tree

Players start with a minimal kit and buy new commands. Levels are designed so
the starter kit can always *complete* them (even if the score is low), but
unlocked motions enable higher scores and stars — the Tony Hawk loop of "same
park, new tricks, better score."

#### Tier 0 — Free Starter Kit

These are always available from the start:

| Command       | Description                      |
|---------------|----------------------------------|
| `h` `j` `k` `l` | Basic movement                |
| `i`           | Enter insert mode                |
| `Esc`         | Return to normal mode            |
| `x`           | Delete character under cursor    |

#### Tier 1 — Essentials (25–50 credits each)

| Command       | Cost | Description                      |
|---------------|------|----------------------------------|
| `w` `b`       | 25   | Word forward / back              |
| `e`           | 25   | End of word                      |
| `0` `$`       | 30   | Line start / end                 |
| `dd`          | 40   | Delete line                      |
| `yy` `p`      | 50   | Yank line / paste                |

#### Tier 2 — Bread & Butter (75–125 credits each)

| Command       | Cost | Description                      |
|---------------|------|----------------------------------|
| `^`           | 75   | First non-blank character        |
| `gg` `G`      | 75   | Top / bottom of file             |
| `o` `O`       | 80   | Open line below / above          |
| `a` `A`       | 80   | Append after cursor / end of line|
| `I`           | 80   | Insert at line start             |
| `dw` `db`     | 100  | Delete word forward / back       |
| `cw`          | 100  | Change word                      |
| `d$` `d0`     | 100  | Delete to end / start of line    |
| `P`           | 75   | Paste before cursor              |
| `u`           | 125  | Undo                             |

#### Tier 3 — Precision (150–250 credits each)

| Command         | Cost | Description                      |
|-----------------|------|----------------------------------|
| `f` `t` `F` `T` | 150  | Find / till character           |
| `;` `,`         | 100  | Repeat find forward / back      |
| `.`             | 200  | Dot repeat                       |
| `r`             | 150  | Replace character                |
| `/` `?`         | 200  | Search forward / back            |
| `n` `N`         | 150  | Next / previous search match     |
| `*` `#`         | 175  | Search word under cursor         |
| `c$` `C`        | 175  | Change to end of line            |
| `{num}G`        | 150  | Jump to line number              |

#### Tier 4 — Power Moves (300–500 credits each)

| Command              | Cost | Description                      |
|----------------------|------|----------------------------------|
| `ciw` `diw` `yiw`   | 300  | Inner word text objects          |
| `ci"` `di"` `ci(`   | 350  | Inside quotes / brackets         |
| `v` `V`             | 300  | Visual mode (char / line)        |
| `Ctrl-v`            | 400  | Visual block mode                |
| `%`                 | 300  | Match bracket                    |
| `>` `<`             | 350  | Indent / dedent                  |
| `gU` `gu`           | 300  | Uppercase / lowercase            |

#### Tier 5 — Endgame (500–750 credits each)

| Command              | Cost | Description                      |
|----------------------|------|----------------------------------|
| `q{reg}` `@{reg}`   | 500  | Record / replay macros           |
| `"{reg}y` `"{reg}p`  | 500  | Named registers                  |
| `@@`                 | 300  | Replay last macro                |
| `:s/old/new/g`       | 750  | Substitution                     |

#### Unlock Rules

- Unlocked motions are persisted in the save file and available across all levels.
- The shop is accessible from the main menu and from the end-of-level results screen.
- Levels display a "tip" when the player uses many keystrokes on something an
  unlockable motion would solve: *"This would be 2 keystrokes with `cw` — available
  in the shop for 100 credits."*
- The unlock tree is **not gated by tiers** — a player can save up and buy a Tier 4
  command early if they want to. Tiers just indicate pricing.
- Replaying earlier levels with newly unlocked motions is the primary way to improve
  scores and earn more credits (the Tony Hawk loop).

### 6.3 Cosmetics Shop

Cosmetics are purely visual. Keep the catalog small and focused.

#### Cursor Styles (50–150 credits each)

| Item              | Cost | Description                                 |
|-------------------|------|---------------------------------------------|
| Green cursor      | 50   | Green block cursor                          |
| Blue cursor       | 50   | Blue block cursor                           |
| Amber cursor      | 75   | Retro amber terminal look                   |
| Underscore cursor | 100  | `_` style instead of block                  |
| Pipe cursor       | 100  | `|` style (thin line)                       |
| Blinking cursor   | 150  | Block with blink animation                  |

#### Color Themes (100–200 credits each)

| Theme         | Cost | Description                                    |
|---------------|------|------------------------------------------------|
| Default       | Free | Ships with the game                            |
| Gruvbox       | 100  | Warm retro theme                               |
| Catppuccin    | 100  | Pastel modern theme                            |
| Solarized     | 150  | Classic solarized palette                      |
| Dracula       | 150  | Dark purple theme                              |

#### HUD Skins (200–400 credits each)

| Skin           | Cost | Description                                   |
|----------------|------|-----------------------------------------------|
| Stock Vim      | Free | Default — minimal, classic Vim aesthetic       |
| NeoVim         | 200  | Inspired by NeoVim's modern look              |
| SpaceVim       | 300  | SpaceVim's status line style with icons        |
| LazyVim        | 400  | LazyVim-inspired UI with rounded borders       |

#### Other Unlockables (100–250 credits each)

| Item                  | Cost | Description                               |
|-----------------------|------|-------------------------------------------|
| Relative line numbers | 100  | Show relative line numbers (useful for `5j`, `12G` etc.) |
| Nerd Font icons       | 150  | Use nerd font icons in HUD (requires nerd font installed) |
| Task complete sparkle | 250  | Animated flash effect when completing a task |

---

## 7. Multi-Buffer System

Starting from World 2, levels use multiple code files (buffers). This teaches
real-world multi-file Vim workflow and adds a context-switching challenge.

### 7.1 Buffer Count Progression

| World   | Buffers | Switch Trigger        | Notes                           |
|---------|---------|----------------------|---------------------------------|
| 1 (1-1 to 1-5)  | 1 | —                  | Single file, learn the basics   |
| 2 (2-1 to 2-5)  | 2 | Event-driven       | Game forces swaps at set points |
| 3–4              | 2–3 | Event-driven      | More frequent swaps             |
| 5–6              | 3–4 | Event + player-initiated | Cross-file tasks (yank from A, paste in B) |
| 7–8              | 4+  | Player-initiated   | Register juggling across files  |

### 7.2 Buffer Command Unlock Tree

Buffer commands are part of the credit shop (Section 6.2) but listed separately
because they form their own progression.

Players start with the most primitive method and unlock faster commands:

| Command              | Cost | Tier | Description                              |
|----------------------|------|------|------------------------------------------|
| `:e {filename}`      | Free | 0    | Open file by typing full name — slow but always available |
| `:bn` `:bp`          | 150  | 2    | Next / previous buffer                   |
| `:ls`                | 100  | 2    | List open buffers                        |
| `:b {partial}`       | 200  | 3    | Switch buffer by partial name match      |
| `Ctrl-^`             | 300  | 3    | Toggle to last buffer — instant swap     |
| `:b{num}`            | 250  | 3    | Switch to buffer by number               |

### 7.3 HUD: Buffer Line

When multiple buffers are active, the HUD shows a buffer line (standard Vim
concept — `:ls` output visualized):

```
┌──────────────────────────────────────────────────┐
│ [1] main.py  │  2  utils.py  │  3  config.py    │  ← buffer line
├──────────────────────────────────────────────────┤
│ ★★☆  Level 3-2  "Multi-File"    Score: 2,100  ×2│  ← HUD
├──────────────────────────────────────────────────┤
│  14 │   for item in items:                       │
│  ...                                             │
```

- Active buffer is highlighted (e.g., `[1]` with brackets and bold).
- Buffer line is always visible when there are 2+ buffers.
- Modified buffers show a `+` indicator: `[1+] main.py`.

### 7.4 Event-Driven Swaps (Early Levels)

In Worlds 2–4, the game controls when swaps happen. A swap event scrolls into
view as a visual marker in the code:

```
  24 │ }
  25 │
  26 │ ══════════════════════════════════════
  27 │  ▸ SWITCH TO: utils.py
  28 │ ══════════════════════════════════════
  29 │
```

When the viewport reaches the swap marker, the buffer switches automatically.
The player needs to orient themselves in the new file and continue completing
tasks. This teaches context-switching without requiring buffer commands.

### 7.5 Player-Initiated Swaps (Later Levels)

In Worlds 5+, tasks require the player to actively switch buffers:

- *"Yank the function signature from `utils.py` and paste it in `main.py`"*
- *"The variable name in `config.py` is wrong — switch there and fix it, then come back"*

These tasks have no swap marker — the player decides when and how to switch
using their unlocked buffer commands. This is where `:bn`, `Ctrl-^`, and
`:b {name}` become essential.

### 7.6 Segment Format Addition

Multi-buffer levels use an extended segment format:

```toml
[meta]
id = "py-junior-multi-api"
zone = "junior"
language = "python"
buffers = ["main.py", "utils.py"]       # declares multiple files

[code.main_py]                           # one [code.*] section per buffer
content = """
from utils import fetch_data
..."""

[code.utils_py]
content = """
import requests
..."""

[[tasks]]
buffer = "main.py"                       # which buffer the task is in
type = "change_word"
anchor = { pattern = "fetch_data", occurrence = 1 }
...

[[tasks]]
buffer = "utils.py"
type = "delete_line"
anchor = { pattern = "# TODO", occurrence = 1 }
...

[[swap_events]]                          # event-driven swap points
after_line = 24                          # swap triggers when viewport passes this line
from = "main.py"
to = "utils.py"
```

---

## 8. Phased Implementation Plan

Each phase is a self-contained deliverable. Complete one before starting the next.
Phases are designed so the game becomes playable as early as possible, then gains
features incrementally.

### Phase 1 — Project Skeleton & Vim Buffer ✅

**Goal**: A Rust project that can hold text in a buffer and move a cursor with
basic Vim motions.

**Deliverables**:
- `cargo init` with all dependencies in Cargo.toml
- `vim/buffer.rs`: rope-backed text buffer (load string, get line, insert, delete)
- `vim/cursor.rs`: cursor position with line/column, clamping to buffer bounds
- `vim/mode.rs`: Normal and Insert mode enum
- `vim/motions.rs`: `h`, `j`, `k`, `l` movement
- `vim/command.rs`: keystroke → action parser (Normal mode only)
- Unit tests for buffer operations and cursor movement

**No rendering yet** — this is pure logic. All testable via `cargo test`.

**Exit criteria**: Tests pass for loading a buffer, moving a cursor with hjkl,
and cursor stays clamped within buffer bounds.

---

### Phase 2 — Terminal Rendering & Input ✅

**Goal**: See the buffer on screen, move the cursor with real keystrokes.

**Deliverables**:
- `main.rs`: terminal setup (raw mode, alternate screen) and cleanup
- `ui/game_view.rs`: render buffer with line numbers, render cursor position
- `config/keymap.rs`: basic keymap (hardcoded qwerty for now)
- Input loop: read keystrokes → resolve to actions → update buffer → re-render
- `app.rs`: minimal state machine (just "Playing" state for now)

**Exit criteria**: Launch the game, see a hardcoded buffer, move cursor with hjkl
in real-time, quit with `q` or `Ctrl-c`.

---

### Phase 3 — Scrolling Viewport & Game Over ✅

**Goal**: The core Guitar Hero mechanic — viewport scrolls, cursor must keep up.

**Deliverables**:
- `game/viewport.rs`: viewport with configurable scroll speed, tracks top/bottom line
- `game/engine.rs`: game loop with tick-based scrolling (decoupled from frame rate)
- Game over detection: cursor above viewport top → game over screen
- `ui/game_view.rs`: only render lines within viewport, show scroll indicator
- Basic game over screen (score placeholder, "press R to retry")

**Exit criteria**: Buffer scrolls down automatically, player must press `j` to keep
up, game over triggers correctly when cursor leaves viewport. Moving past the
bottom of the viewport (e.g. `j`, `6j`) scrolls the viewport forward to match.

---

### Phase 4 — Expanded Motions ✅

**Goal**: Enough Vim commands to make movement interesting.

**Deliverables**:
- `vim/motions.rs` expanded: `w`, `b`, `e`, `W`, `B`, `E` (word motions)
- `0`, `^`, `$` (line position)
- `gg`, `G`, `{num}G` (line jumping)
- `f`, `t`, `F`, `T`, `;`, `,` (find character)
- `{`, `}` (paragraph)
- `%` (bracket matching)
- Unit tests for all motions

**Exit criteria**: All motions work correctly on various buffer contents.
Edge cases handled (empty lines, end of buffer, etc.).

---

### Phase 5 — Task System ✅

**Goal**: Tasks appear in the buffer and can be completed.

**Deliverables**:
- `game/task.rs`: task struct, states (pending/active/completed/missed)
- `content/anchor.rs`: resolve text-pattern anchors to buffer positions
- `ui/task_overlay.rs`: red/green/yellow highlighting on task lines
- Gutter annotations ("DEL", "CHG → 'foo'", "✓ DONE")
- Task completion detection: compare buffer state against expected outcome
- `move_to` task type (simplest — just move cursor to a position)

**Exit criteria**: Hardcoded tasks appear highlighted in the buffer, completing
a `move_to` task turns it green, missed tasks (scrolled past) are detected.

---

### Phase 6 — Scoring & HUD ✅

**Goal**: The game feels like a game — points, combos, feedback.

**Deliverables**:
- `game/scoring.rs`: point accumulation, combo tracking, star calculation
- `ui/hud.rs`: top bar with score, combo multiplier, level name, star progress
- `ui/statusbar.rs`: mode indicator, keystroke count, current task description
- `ui/results.rs`: end-of-level screen with score breakdown
- Score formula: survival points + task points − keystroke penalty + combo bonus

**Exit criteria**: Playing through a level shows a running score, combos work,
end-of-level screen shows meaningful results with star rating.

---

### Phase 7 — Content System & Segment Loader ✅

**Goal**: Levels are assembled from content segments, not hardcoded.

**Deliverables**:
- `content/segment.rs`: parse TOML segment files
- `content/assembler.rs`: randomly select and stitch segments into a buffer
- `content/history.rs`: track recently-seen segments in save file
- `levels/` directory structure with TOML format
- `include_dir!` macro to embed content in binary
- Level metadata: zone, world, level number, scroll speed, allowed commands

**Exit criteria**: Starting a level loads random segments from the correct
zone/language pool. Replaying gives different content.

---

### Phase 8 — First Content: Python & TypeScript (Starter + Junior) ✅

**Goal**: Enough content for a real playable demo across 2 languages, 2 zones.

**Deliverables**:
- ~40 Python starter segments
- ~40 Python junior segments
- ~40 TypeScript starter segments
- ~40 TypeScript junior segments
- Each segment has 1–3 well-designed tasks
- Each segment includes 0–2 Vim hint comments from Section 1.6 (zone-appropriate)
- Level intro screen showing the new command(s) for that level
- Post-level insight: compare player's keystrokes to optimal and suggest commands
- Language selection in menu
- Tasks cover: `move_to`, `delete_line`, `delete_word`, `change_word`, `insert_text`
- **Import runway**: assembler prepends 5–10 lines of language-appropriate imports
  before the first segment (see Section 1.1b). These lines never contain tasks.
- **3-second countdown**: overlay displayed before scrolling starts. Viewport is
  frozen and keystrokes are not penalized during countdown (see Section 1.1b).

**Exit criteria**: Worlds 1–4 are fully playable in Python and TypeScript with
varied content on each replay. Hints are visible in code and on level screens.
Countdown plays before each level. Import runway gives breathing room.

---

### Phase 9 — Editing Commands & Advanced Tasks ✅

**Goal**: Full operator + motion system, enabling medior/senior content.

**Deliverables**:
- `vim/operators.rs`: `d`, `c`, `y` combined with any motion
- `vim/text_objects.rs`: `iw`, `aw`, `i"`, `a"`, `i(`, `a(`, `i{`, `a{`, `i[`, `a[`
- Insert mode: `i`, `a`, `I`, `A`, `o`, `O`, typing text, `Escape` to return
- `r`, `R` (replace)
- `.` (dot repeat)
- `u`, `Ctrl-r` (undo/redo)
- Visual mode: `v`, `V`, `Ctrl-v` + operators
- Registers: unnamed, named `"a`-`"z`
- Macros: `q{reg}`, `@{reg}`
- New task types: `change_inside`, `yank_paste`, `delete_block`, `indent`
- Search: `/`, `?`, `n`, `N`, `*`, `#`

**Exit criteria**: All commands from the level tables (Section 2.2) are
implemented and tested.

---

### Phase 10 — Full Content: All Languages, All Zones

**Goal**: Complete content library.

**Deliverables**:
- Medior + senior segments for Python and TypeScript (~80 more per language)
- All 4 zones for Rust (~160 segments)
- All 4 zones for C++ (~160 segments)
- Total: ~640 segments across all languages
- All 8 worlds fully playable

**Exit criteria**: Every world/level combination works in every language with
no repetition for at least 3 playthroughs.

---

### Phase 11 — Config System & Presets

**Goal**: Full keybinding customization.

**Deliverables**:
- `config/mod.rs`: load/create `~/.vim-heroes/config.toml`
- `config/key_syntax.rs`: parse all key formats ("C-r", "gg", arrays, etc.)
- `config/presets.rs`: qwerty, colemak, colemak-dh, dvorak, workman
- Multi-key sequence resolution with configurable timeout (default 300ms)
- First-run prompt: choose layout or accept default
- Hints and ghost replay show keys in the player's active mapping

**Exit criteria**: A Colemak user can play the full game with correct bindings.
Config file changes are picked up on next launch.

---

### Phase 12 — Progress, Saves & Menus

**Goal**: Persistent player progression.

**Deliverables**:
- `progress/save.rs`: save/load to `~/.vim-heroes/save.dat`
- Track per-level: best score, stars earned, keystroke counts
- Track globally: total play time, commands used histogram
- `ui/menu.rs`: main menu (New Game, Continue, Level Select, Settings, Quit)
- New Game screen: choose game mode — **Story Mode** or **Free Play** (see Phase 13)
- Level select screen showing stars, locked/unlocked worlds
- World unlock: earn N stars in previous world to unlock next
- Settings screen: language picker, keybind preset, theme, hint toggle

**Exit criteria**: Player can quit and resume, see their star progress,
unlock new worlds by earning stars.

---

### Phase 13 — Credit Shop & Motion Unlocks

**Goal**: The Tony Hawk Pro Skater progression loop — earn credits, buy motions
and cosmetics, replay levels for better scores.

**Deliverables**:
- `progress/credits.rs`: credit balance tracking, earn/spend logic
- `progress/unlocks.rs`: unlocked motions and cosmetics, persisted in save file
- `ui/shop.rs`: shop screen accessible from main menu and results screen
  - Motion unlock tree (Tier 0–5 from Section 6.2)
  - Buffer command unlocks (Section 7.2)
  - Cosmetics catalog (Section 6.3)
- Credit rewards integrated into scoring (Section 6.1)
- `vim/command.rs`: gate command execution behind unlock checks — if the player
  presses an un-purchased motion, show a brief "locked" indicator
- Shop tip system: when a player uses many keystrokes on something an unlockable
  motion would solve, show a tip on the results screen
- Starter kit (Tier 0) available from first launch, no purchase needed
- `ui/results.rs`: add "credits earned" breakdown and "Visit Shop" option
- **Game mode toggle: Story Mode vs Free Play**
  - Selectable from main menu (added in Phase 12) and new game screen
  - **Story Mode**: default experience — start with Tier 0 starter kit, earn
    credits, buy motions progressively. The intended path for learners.
  - **Free Play**: all Vim motions unlocked from the start, no credit/shop
    gating. Designed for experienced vimmers who want to jump straight into
    the challenge. Credits and shop still function (for cosmetics), but all
    motions and buffer commands are pre-unlocked.
  - `progress/game_mode.rs`: mode enum, persisted per save slot
  - `vim/command.rs`: skip unlock check when mode is Free Play
  - Mode is set at save creation and displayed in the level select / HUD

**Exit criteria**: Player earns credits after every level, can browse and buy
motions/cosmetics in the shop, newly purchased motions work immediately in all
levels. Replaying earlier levels with better motions yields higher scores.
Free Play mode skips all motion gating — every command works from level one.

---

### Phase 14 — Multi-Buffer Levels

**Goal**: Levels with multiple code files and buffer-switching mechanics.

**Deliverables**:
- `vim/buffers.rs`: multi-buffer manager (list of buffers, active buffer index)
- `:e {filename}` command (free, always available) for manual file switching
- Unlockable buffer commands: `:bn`, `:bp`, `:ls`, `:b {partial}`, `Ctrl-^`, `:b{num}`
- `content/segment.rs`: extended TOML format with multi-buffer support (Section 7.6)
- `content/assembler.rs`: assemble multi-buffer levels from segments
- Event-driven swap markers (Section 7.4): visual markers that auto-switch buffers
- `ui/hud.rs`: buffer line showing open buffers with active indicator (Section 7.3)
- `game/engine.rs`: handle buffer switches mid-level, maintain per-buffer cursor/viewport
- Cross-file tasks: tasks that reference specific buffers (Section 7.5)
- World 2+ content segments updated to use multi-buffer format

**Exit criteria**: World 2 levels swap between 2 files via event markers. Later
worlds support player-initiated swaps. Buffer line in HUD shows active file.
Cross-file tasks (yank from A, paste in B) work in World 5+.

---

### Phase 15 — Polish & Extra Modes

**Goal**: Ship-quality experience.

**Deliverables**:
- Ghost replay: after level, show optimal keystrokes as a ghost cursor
- Practice mode: no scrolling, just tasks
- Endless mode: infinite random content, escalating speed
- Sound effects (optional, terminal bell or off)
- Stats dashboard: command usage, accuracy trends, time played
- Tutorial: interactive first-launch walkthrough of game mechanics
- Cosmetics rendering: cursor styles, color themes, HUD skins from shop

**Exit criteria**: The game feels complete and polished. A new player can
pick it up, understand the mechanics, and progress through all worlds.

---

### Phase 16 — Distribution & Release

**Goal**: Players can install the game easily on any platform.

**Deliverables**:
- CI/CD pipeline (GitHub Actions): build + test on Linux/macOS/Windows
- `cargo install vim-heroes` — publish to crates.io
- Homebrew formula in a tap repo
- `.deb` package via cargo-deb
- AUR PKGBUILD
- GitHub Releases with prebuilt binaries (linux-x64, macos-arm64, windows-x64)
- README with install instructions, screenshots, GIF demo

**Exit criteria**: `brew install vim-heroes`, `cargo install vim-heroes`, or
downloading a binary from GitHub Releases all work.

---

### Phase 17 — Personal Code: GitHub Integration

**Goal**: Let players practice Vim on their own code from their public GitHub repos.

**Deliverables**:
- `content/github.rs`: fetch player's public repos and code files via GitHub REST API
- GitHub username detection: parse `~/.gitconfig` for `user.name` → fallback to in-game
  prompt ("Enter your GitHub username")
- Fetch top repos by stars: `GET https://api.github.com/users/{name}/repos?sort=stars`
- Fetch code files: `GET https://api.github.com/repos/{owner}/{repo}/contents/{path}`
  filtered by supported extensions (.py, .ts, .rs, .cpp)
- Auto-slicer: split fetched files into game-sized segments (~20–60 lines) with
  auto-generated tasks (move_to, delete_line, change_word based on code patterns)
- "Your Code" mode in level select: separate section showing repo names as worlds
- Cache fetched content locally in `~/.vim-heroes/github-cache/` to avoid redundant
  API calls and respect rate limits
- HTTP via `ureq` crate (small, sync, no OpenSSL dependency — pure Rust TLS)

**Constraints**:
- No `gh` CLI dependency — uses raw HTTPS requests from the Rust binary
- No sudo / admin rights / elevated privileges required
- No auth tokens needed — only accesses public repos via unauthenticated API
- Unauthenticated GitHub API rate limit: 60 requests/hour (sufficient for ~10 repos
  worth of files per session)
- Graceful degradation: if API is unreachable or user has no public repos, skip
  silently and use built-in content

**Exit criteria**: Player enters their GitHub username, sees their top repos listed,
and can play levels built from their own code. Works on a fresh machine with no
GitHub tooling installed.

---

### Future Ideas (Post-Release)

- **Multiplayer**: split-screen race mode (who finishes the level with more points)
- **Daily challenge**: shared daily level with global leaderboard
- **Custom levels**: player-created segment packs (load from `~/.vim-heroes/custom/`)
- **More languages**: Go, Java, Ruby, Zig, Haskell
- **Neovim integration**: play inside Neovim as a plugin
- **Community content**: submit segments via PR, curated into releases
- **Boss battles**: multi-phase refactoring challenges
- **Achievements**: "Used 0 arrow keys", "100 combo", "All stars World 1", etc.
- **Replay analysis**: unlock ability to view the optimal solution for completed levels (shop item)
- **Seasonal cosmetics**: limited-time cursor/theme unlocks
