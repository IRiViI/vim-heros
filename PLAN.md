# Vim Heroes — Master Plan

A Guitar Hero–inspired terminal game that teaches Vim through escalating, real-code
challenges. Text scrolls down like a note highway; your cursor must keep up or it's
game over. Fewer keystrokes = more points. Powered by real Vim commands.

**Design philosophy**: Slay the Spire meets Guitar Hero for Vim. A full skilled run
takes ~30 minutes. Failing and restarting is the learning loop — both for the game
mechanics and for real Vim skills.

---

## Table of Contents

1. [Game Design](#1-game-design)
2. [World & Level System](#2-world--level-system)
3. [Game Modes](#3-game-modes)
4. [Content System](#4-content-system)
5. [Achievements System](#5-achievements-system)
6. [Easter Eggs & Vim Wisdom](#6-easter-eggs--vim-wisdom)
7. [Keybinding / Config System](#7-keybinding--config-system)
8. [Technical Architecture](#8-technical-architecture)
9. [Multi-Buffer System](#9-multi-buffer-system)
10. [Ideas Parking Lot](#10-ideas-parking-lot)
11. [Phased Implementation Plan](#11-phased-implementation-plan)

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
  Energy = 0? ──▶ GAME OVER
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
- **Energy bar**: every keystroke drains energy, completing tasks restores it.
  Hit 0 energy = game over. This makes efficiency the core survival mechanic.

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
| Each scroll tick    | +10         | Baseline survival reward                 |
| Task completed      | +50 to +500 | Scales with task complexity              |
| Optimal solution    | +100 bonus  | Used ≤ optimal number of keystrokes      |
| Combo multiplier    | ×1.5 / ×2 / ×3 | Consecutive optimal task completions |
| Missed task         | −50         | Task scrolled off-screen uncompleted     |

Tasks must be worth significantly more than survival points so the optimal strategy
is "complete tasks efficiently," not "hold j and ignore everything."

Note: The old keystroke penalty (−2 per key) is replaced by the **energy bar system**
(Section 1.3). Keystrokes drain energy instead of directly reducing score.

### 1.3 Energy Bar System

The energy bar is the core survival mechanic. It replaces the simple keystroke
penalty with a visible, visceral pressure system.

#### How it works
- Start each level with a full energy bar (e.g., 100 energy)
- **Every keystroke drains energy** (configurable cost per action type)
- **Completing tasks restores energy** (more for optimal completion)
- **Energy also drains over time** (tied to scroll speed — harder levels drain faster)
- **Hit 0 energy = Game Over** (restart from checkpoint)

#### Configurable costs (stored in a tuning config file)

All values live in a single config/TOML file for tuning without recompile:

```toml
[energy]
max = 100
start = 100

[energy.drain]
keystroke_base = 1        # basic cost per key press
time_drain_per_tick = 0.5 # passive drain per scroll tick

[energy.restore]
task_complete = 15        # energy restored on task completion
task_optimal = 25         # energy restored on optimal completion
combo_bonus = 5           # extra per combo level (combo 3 = +15 extra)

[energy.difficulty_multipliers]
# Multipliers applied to drain values per difficulty
nano_user = { drain: 0.5, restore: 1.5 }
wq_survivor = { drain: 0.75, restore: 1.25 }
keyboard_warrior = { drain: 1.0, restore: 1.0 }
ten_x_engineer = { drain: 1.5, restore: 0.75 }
uses_arch_btw = { drain: 2.0, restore: 0.5 }
```

#### Visual design
- Energy bar in the HUD (top bar), next to score
- Color gradient: green (>60%) → yellow (30-60%) → red (<30%)
- Pulsing/flashing when critically low (<15%)
- Brief "+15" popup when energy restored by task completion

#### Energy between levels
- Energy **resets to full** at the start of each level
- No attrition across levels — each level is a fresh challenge
- The roguelike tension comes from "fail = restart from checkpoint", not gradual drain
- Energy does NOT drain during countdown or import runway (grace period)
- Energy does NOT drain for unlearned keys (skill-gated keys are free)

#### Relationship to scoring
- Score system (points, combos, stars) still exists for ranking/leaderboard
- Energy = survival mechanic (hit 0 = die)
- Score = mastery mechanic (how well you survived)

### 1.4 Star Rating & Checkpoints (per level)

- ★☆☆ — Completed the level (survived to the end)
- ★★☆ — Completed all tasks
- ★★★ — Completed all tasks within the optimal keystroke budget

**Difficulty does NOT affect stars.** Stars measure completion quality, not speed:
- ★ = survived (possible on any difficulty)
- ★★ = all tasks completed (possible on any difficulty)
- ★★★ = all tasks completed optimally (possible on any difficulty)

Difficulty affects scroll speed, energy drain/restore, and **score**. A 3-star
run on "Uses Arch btw" earns the same checkpoint as "Nano User" but a much higher
score. Beginners can progress on easier difficulty; experts chase high scores.

**Checkpoint mechanic:** Every 3-starred level becomes a permanent checkpoint.
The checkpoint is determined by your highest **consecutive** 3-star chain starting
from level 1-1. Example: 3-starred 1-1 through 4-3 → next run starts at 4-4.
If you 3-star 1-1, 1-2, skip 1-3, and 3-star 1-4 — your checkpoint is still 1-3
(the chain broke). This incentivizes going back to perfect skipped levels.

### 1.5 Visual Design (Terminal)

```
┌──────────────────────────────────────────────────────┐
│ ★★☆  Level 2-3  "Word Surfer"   Score: 1,250  ×2    │  ← HUD
│ Energy: ████████████░░░░░  68%                       │  ← Energy bar
├──────────────────────────────────────────────────────┤
│  14 │   for (const item of items) {                  │
│  15 │     sum += item.price;                         │
│  16 │ ██  sum += item.tax  ██       CHG → "cost"     │  ← red = pending task
│  17 │   }                                            │
│  18 │   return sum;               █                  │  ← cursor
│  19 │ }                                              │
│  20 │ ▓▓  console.log(total)  ▓▓       ✓ DONE       │  ← green = completed
│  21 │                                                │
├──────────────────────────────────────────────────────┤
│ NORMAL │ Keys: 12 │ ▸ Change "tax" → "cost"         │  ← status bar
└──────────────────────────────────────────────────────┘
```

- **Red background**: task pending, with a short annotation in the right gutter.
- **Green background**: task completed.
- **Yellow background**: partially done / cursor is on it.
- **Energy bar**: color gradient from green to red, pulsing when critically low.
- **Status bar**: current mode, keystroke count, current task description.
- **HUD**: level name, star progress, score, combo multiplier, energy.

### 1.6 Game Over Screen

Show: final score, stars earned, tasks completed/total, keystrokes used vs optimal,
energy remaining, and a breakdown of commands used.

```
┌──────────────────────────────────────────┐
│            GAME OVER                     │
│                                          │
│  World 5-3  "The Destroyer"              │
│  Score: 2,450        Stars: ★★☆          │
│  Tasks: 7/9          Energy: 0%          │
│  Keystrokes: 84      Optimal: 61         │
│                                          │
│  Your checkpoint: World 4-4              │
│                                          │
│  [R] Restart from checkpoint             │
│  [B] Back to beginning (1-1)             │
│  [Q] Quit to menu                        │
└──────────────────────────────────────────┘
```

"Restart from checkpoint" goes to the level after your highest consecutive 3-star.
Stars earned during the failed run are still saved (progress isn't lost, just position).

### 1.6b Between-Level Flow

What happens between levels during a run:

**1. Level Complete → Results screen** (mandatory 2 seconds, then press any key):
```
┌──────────────────────────────────────────┐
│         LEVEL COMPLETE!                  │
│                                          │
│  World 3-2  "Line Rider"                 │
│  Score: 1,850        Stars: ★★★          │
│  Tasks: 6/6          Energy: 72%         │
│  Keystrokes: 42      Optimal: 38         │
│                                          │
│  ✓ NEW CHECKPOINT UNLOCKED!              │
│                                          │
│     Press any key to continue...         │
└──────────────────────────────────────────┘
```

If a new achievement unlocks: "ACHIEVEMENT UNLOCKED: Wordsmith" shown below stats.

**2. Next Level loading screen** (press any key to start):
```
┌──────────────────────────────────────────┐
│       World 3, Level 3                   │
│       "Line Rider"                       │
│                                          │
│  Available: h j k l 5j w b e 0 ^ $ f t  │
│                                          │
│     Press any key to start...            │
└──────────────────────────────────────────┘
```

**3. Countdown** (3... 2... 1...) → level starts with full energy.

**Pause menu** (press `Esc` during gameplay):
- [Resume] — continue playing
- [Quit Run] — return to main menu (stars saved, run ends)
- No "Restart Level" option — in roguelike mode, you either complete or die

### 1.7 Tutorial-as-Gameplay: Intro Segments

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

#### Example: Level 2-1 intro (introduces `w` and `b`)

```
  1 │ # ═══════════════════════════════════
  2 │ # WORLD 2: Word Surfer
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
| 1-1   | Full intro | `h` `j` `k` `l` + counts — how the game works, viewport, tasks, scoring, energy |
| 2-1   | Full intro | `w` `b` `e` `W` `B` `E` — word motions |
| 3-1   | Full intro | `0` `^` `$` `f` `t` `;` `,` — line precision |
| 4-1   | Full intro | `i` `a` `I` `A` `o` `O` `R` — insert mode entry points |
| 5-1   | Full intro | `x` `X` `dd` `D` — deletion commands |
| 6-1   | Full intro | `d{motion}` `c{motion}` `.` `>>` `<<` — **the verb+noun paradigm** (most important tutorial) |
| 7-1   | Full intro | `y` `yy` `p` `P` — yank and paste |
| 8-1   | Full intro | `v` `V` + operators — visual mode |
| 9-1   | Full intro | `iw` `aw` `i"` `ci(` `da{` — text objects |
| 10-1  | Full intro | `{` `}` `%` + marks — code navigation |
| 11-1  | Full intro | `/` `?` `n` `N` `*` `#` — search |
| 12-1  | Full intro | `u` `Ctrl-R` + named registers `"a`-`"z` — time travel |
| 13-1  | Full intro | `q{reg}` `@{reg}` `@@` — macros |
| 14-1  | Short intro | Everything — no new commands, just the final challenge |

The very first intro (1-1) is special — it also explains the game mechanics:
the scrolling viewport, what happens when you fall behind, what the colored
highlights mean, the energy bar, and the scoring system. This is the only "meta-tutorial."

#### World 6 intro: The most important tutorial

World 6 "Verb + Noun" is where Vim "clicks" for most people. The intro should
be exceptional:

```
# ═══════════════════════════════════════
# THE MOST IMPORTANT LESSON IN VIM:
# ═══════════════════════════════════════
#
# Everything you've learned is a NOUN:
#   w (word), $ (end of line), 3j (3 lines down)
#
# Now meet the VERBS:
#   d (delete), c (change), y (yank)
#
# Verb + Noun = Action:
#   d + w  = delete a word
#   c + $  = change to end of line
#   d + 3j = delete 3 lines down
#
# And the magic: '.' repeats your last verb+noun.
# ═══════════════════════════════════════
```

#### Segment format addition

```toml
[meta]
id = "intro-2-1-word-motions"
zone = "starter"
language = "python"        # one intro per language per level
tags = ["words"]
difficulty = 1
intro = true               # marks this as a level intro segment
intro_level = "2-1"        # which level this intro belongs to

[code]
content = """
# ═══════════════════════════════════
# WORLD 2: Word Surfer
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

### 1.8 Vim Hints System

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
│         World 2-1: Word Surfer          │
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
| S10 | `f{char}` finds the next occurrence of {char} on the line. `fa` jumps to the next 'a'. |
| S11 | `t{char}` jumps to just before {char}. `tf` stops one character before 'f'. |
| S12 | `;` repeats your last `f` or `t` search forward. `,` repeats it backward. |
| S13 | `i` enters insert mode before the cursor. `a` enters after. |
| S14 | `I` inserts at the start of the line. `A` appends at the end. |
| S15 | `o` opens a new line below and enters insert mode. `O` opens above. |
| S16 | Press `Esc` to return to Normal mode from anywhere. Always your safe home. |
| S17 | In Vim, you spend most of your time in Normal mode, not Insert mode. |
| S18 | Counts work with motions: `3w` jumps 3 words forward. `5j` moves 5 lines down. |
| S19 | `5G` or `5gg` jumps to line 5. Any number works. |
| S20 | `f` is a sniper rifle — it finds any character on the line instantly. |

##### Zone: Junior

| ID | Hint |
|----|------|
| J01 | `x` deletes the character under the cursor. Like a tiny eraser. |
| J02 | `dd` deletes the entire current line. Quick and clean. |
| J03 | `D` deletes from cursor to end of line. Shorthand for `d$`. |
| J04 | `dw` = delete a word. Operators (`d`) + motions (`w`) combine into power moves. |
| J05 | `d$` deletes from cursor to end of line. `d0` deletes to the start. |
| J06 | `cw` changes a word — deletes it and drops you into insert mode. |
| J07 | `c$` changes from cursor to end of line. You can also use `C` as a shortcut. |
| J08 | `.` repeats your last change. Did `cw`+typed "foo"+`Esc`? Now `.` does it again. |
| J09 | The dot command `.` is one of Vim's most powerful features. Master it. |
| J10 | Think in terms of "verb + noun": `d` (delete) + `w` (word) = delete word. |
| J11 | `>>` indents the current line. `<<` dedents. These are operators too! |
| J12 | Counts work with operators too: `3dd` deletes 3 lines. `2dw` deletes 2 words. |
| J13 | `yy` yanks (copies) the entire current line. |
| J14 | `p` pastes after the cursor. `P` pastes before. |
| J15 | After `dd`, press `p` to paste the deleted line somewhere else — it's a cut! |
| J16 | `Y` yanks the entire line (same as `yy`). `yw` yanks one word. |
| J17 | `v` starts visual mode (character-wise). `V` selects whole lines. |
| J18 | In visual mode, select text then press `d` to delete, `c` to change, or `y` to yank. |
| J19 | `r{char}` replaces the character under the cursor without entering insert mode. |
| J20 | `R` enters Replace mode — every character you type overwrites the existing text. |

##### Zone: Medior

| ID | Hint |
|----|------|
| M01 | `ciw` = change inner word. Deletes the word and enters insert mode. Works anywhere in the word. |
| M02 | `ci"` changes everything inside double quotes. `ci'` for single quotes. |
| M03 | `ci(` changes inside parentheses. Also works: `ci{`, `ci[`, `ci<`. |
| M04 | `di"` deletes inside quotes. `da"` deletes the quotes too (around). |
| M05 | `i` = inner (inside the delimiters). `a` = around (includes the delimiters). |
| M06 | `vi{` visually selects everything inside curly braces. Great for function bodies. |
| M07 | Text objects understand nesting: `ci(` inside `f(g(x))` changes the innermost `()`. |
| M08 | `%` jumps to the matching bracket: `(↔)`, `{↔}`, `[↔]`. |
| M09 | `{` jumps to the previous empty line. `}` jumps to the next. Great for navigating between functions. |
| M10 | Marks let you save positions: `ma` saves, `'a` jumps back. Like bookmarks in code. |
| M11 | `/pattern` searches forward. `?pattern` searches backward. |
| M12 | After searching, `n` goes to the next match, `N` goes to the previous. |
| M13 | `*` searches for the word under your cursor. `#` searches backward for it. |
| M14 | `.` after `ciw`+type+`Esc` lets you change the next occurrence instantly. |
| M15 | `diw` then `w` then `.` — delete words one at a time, wherever you want. |
| M16 | `dt{char}` deletes from cursor up to (but not including) {char}. |
| M17 | `yiw` yanks a word without moving the cursor. Great for copy-paste workflows. |
| M18 | After `y`, the cursor stays where it was. Use `p` to paste at the destination. |
| M19 | Think of text objects as "what", motions as "where": `d` + `iw` = delete [what: inner word]. |
| M20 | `>i{` indents everything inside braces. Operators + text objects = precision. |

##### Zone: Senior

| ID | Hint |
|----|------|
| X01 | `u` undoes your last change. `Ctrl-r` redoes it. Your safety net. |
| X02 | Vim's undo is a **tree**, not a line. Undo then edit = new branch. The old branch is still there. |
| X03 | `g-` goes to an older text state. `g+` goes to a newer one. They walk the undo tree chronologically. |
| X04 | `:earlier 5m` reverts to how the file looked 5 minutes ago. `:later 5m` goes forward. Time travel. |
| X05 | `"ayy` yanks a line into register 'a'. `"ap` pastes from register 'a'. |
| X06 | Registers `a-z` store text. Uppercase `"Ayy` appends to register 'a' instead of replacing. |
| X07 | `"0` always holds your last yank. `""` holds the last delete or yank. |
| X08 | Undo + registers = time travel. Undo to a past state, yank something, redo back, paste it. |
| X09 | `qa` starts recording a macro into register 'a'. `q` stops recording. `@a` replays it. |
| X10 | `@@` replays the last macro. `5@a` replays macro 'a' five times. |
| X11 | Plan your macro: get to a consistent starting position first, end ready for the next `@a`. |
| X12 | Macros ARE register contents. `"ap` pastes macro 'a' as text. Edit it, then `"ayy` to save back. |
| X13 | A well-crafted macro + `100@a` can refactor an entire file in one move. |
| X14 | If you're doing the same edit more than twice, you should be recording a macro. |
| X15 | `Ctrl-a` increments a number under the cursor. `Ctrl-x` decrements it. |
| X16 | `=` auto-indents. `=i{` fixes indentation inside a block. `gg=G` re-indents the whole file. |
| X17 | `xp` swaps two characters. `ddp` swaps two lines. Quick micro-refactors. |
| X18 | `:s/old/new/g` substitutes on the current line. `:%s/old/new/g` does the whole file. |
| X19 | `"+y` yanks to the system clipboard. `"+p` pastes from it. |
| X20 | The best Vim command is the one that gets the job done in the fewest keystrokes. |

---

## 2. World & Level System

### 2.1 Four Zones, 14 Worlds

The game has 14 worlds organized into 4 skill zones. Each world focuses on a
specific category of Vim motions. Within a world, **only that world's skills +
all previous worlds' skills are available.** Keys for unlearned motions are ignored
(no penalty, no energy drain).

#### STARTER ZONE (Worlds 1-4) — "Learning to Walk"

| World | Name | Core Skills | Teaching Goal |
|-------|------|-------------|---------------|
| 1 | **"First Steps"** | `h j k l` + counts (`5j`, `8l`) + `gg G {num}G` | Move around code. Counts multiply everything. |
| 2 | **"Word Surfer"** | `w b e W B E` | Navigate by words — never spam `l` again. |
| 3 | **"Line Rider"** | `0 ^ $ f t F T ; ,` | Precision within lines. `f` is a sniper rifle. |
| 4 | **"The Writer"** | `i a I A o O R` + `Esc` | Enter/exit insert mode efficiently. Know which entry point to use. |

#### JUNIOR ZONE (Worlds 5-8) — "Learning to Fight"

| World | Name | Core Skills | Teaching Goal |
|-------|------|-------------|---------------|
| 5 | **"The Destroyer"** | `x X dd D` | Simple deletions. Remove what doesn't belong. |
| 6 | **"Verb + Noun"** | `d{motion} c{motion} . >> <<` | The operator paradigm — Vim's biggest "aha" moment. `d3w`, `c$`, `.` to repeat. Indentation as operators. |
| 7 | **"Copy Ninja"** | `y yy p P` | Yank and paste workflows. Rearrange code. |
| 8 | **"The Selector"** | `v V` + operators on selections | Visual mode — select then act. Bulk operations. |

#### MEDIOR ZONE (Worlds 9-11) — "Learning to Master"

| World | Name | Core Skills | Teaching Goal |
|-------|------|-------------|---------------|
| 9 | **"Text Object Surgeon"** | `iw aw i" a" i( a( i{ a{` | Inner/around objects. Edit INSIDE things. Combine with operators from World 6. |
| 10 | **"Code Navigator"** | `{ } %` + marks `m` `'` `` ` `` | Structural code navigation. Jump between functions, matching brackets, saved positions. |
| 11 | **"Search & Destroy"** | `/ ? n N * #` | Find patterns across the file. Hunt bugs. |

#### SENIOR ZONE (Worlds 12-14) — "Becoming the Master"

| World | Name | Core Skills | Teaching Goal |
|-------|------|-------------|---------------|
| 12 | **"Time Traveler"** | `u Ctrl-R` + undo tree (`g-` `g+` `:earlier` `:later`) + named registers `"ayy "bp` | Undo tree branching — navigate between undo branches. Named registers as multiple clipboards. Fish things from the past. |
| 13 | **"Macro Wizard"** | `q{reg} @{reg} @@` | Record, replay, automate. The ultimate efficiency tool. |
| 14 | **"The Grandmaster"** | Everything | The finale — all skills combined under maximum pressure. |

#### Zone progression summary

```
STARTER (1-4)  →  JUNIOR (5-8)  →  MEDIOR (9-11)  →  SENIOR (12-14)
  movement          editing          precision          mastery
  ~8 min            ~8 min           ~6 min             ~8 min
```

Every 3-starred level = permanent checkpoint. Your "high water mark" advances
as you master levels.

Total: ~30 min for a skilled run, ~45 min including retries on harder worlds.

### 2.2 Worlds & Levels in Detail

Each world has 5 levels. Level X-1 has a full tutorial intro. Level X-5 is a
world boss with a unique mechanic.

#### World 1 — First Steps

| Level | Focus | Task Types |
|-------|-------|------------|
| 1-1   | `h` `j` `k` `l` + game mechanics | Move cursor to marked positions |
| 1-2   | Counts: `5j` `8l` `3k` | Navigate using counts |
| 1-3   | `gg` `G` `{num}G` | Jump to specific lines |
| 1-4   | All basic movement combined | Mixed movement challenges |
| 1-5   | **BOSS: "The Maze"** | Navigate deeply nested ASCII code art |

#### World 2 — Word Surfer

| Level | Focus | Task Types |
|-------|-------|------------|
| 2-1   | `w` `b` `e` | Jump to highlighted words |
| 2-2   | `W` `B` `E` (big words) | Navigate through symbols and punctuation |
| 2-3   | Word motions + counts (`3w`, `2b`) | Longer jumps |
| 2-4   | All word motions combined | Mixed word navigation |
| 2-5   | **BOSS: "The Marathon"** | Sprint through camelCaseVariableNamesThatGoOnForever |

#### World 3 — Line Rider

| Level | Focus | Task Types |
|-------|-------|------------|
| 3-1   | `0` `^` `$` | Line start/end positioning |
| 3-2   | `f` `t` `F` `T` | Find characters on a line |
| 3-3   | `;` `,` | Repeat find motions |
| 3-4   | All line motions combined | Mixed precision |
| 3-5   | **BOSS: "The Sniper Range"** | 120+ character lines, targets scattered across configs |

#### World 4 — The Writer

| Level | Focus | Task Types |
|-------|-------|------------|
| 4-1   | `i` `a` (basic insert) | Insert text at positions |
| 4-2   | `I` `A` (line start/end insert) | Insert at line boundaries |
| 4-3   | `o` `O` (open lines) | Open lines and insert |
| 4-4   | `R` (replace mode) | Overwrite text efficiently |
| 4-5   | **BOSS: "The Blank Page"** | Code with many missing pieces, heavy insert mode |

#### World 5 — The Destroyer

| Level | Focus | Task Types |
|-------|-------|------------|
| 5-1   | `x` `X` | Delete specific characters |
| 5-2   | `dd` | Delete entire lines |
| 5-3   | `D` (delete to EOL) | Partial line deletion |
| 5-4   | Mixed deletions | Combined deletion challenges |
| 5-5   | **BOSS: "The Cleanup"** | File full of dead code, comments, debug prints — strip it all |

#### World 6 — Verb + Noun *(Most important world)*

| Level | Focus | Task Types |
|-------|-------|------------|
| 6-1   | `dw` `db` `d$` `d0` | Delete with motions |
| 6-2   | `cw` `cb` `c$` `C` | Change with motions |
| 6-3   | `.` (dot repeat) | Repeat last change efficiently |
| 6-4   | `>>` `<<` (indentation) | Indent/dedent as operators |
| 6-5   | **BOSS: "The Refactor"** | Systematic changes with verb+noun combos, heavy `.` repeat |

#### World 7 — Copy Ninja

| Level | Focus | Task Types |
|-------|-------|------------|
| 7-1   | `yy` `p` | Yank and paste lines |
| 7-2   | `yw` `p` | Yank and paste words |
| 7-3   | `P` (paste before) | Paste positioning |
| 7-4   | `dd` + `p` workflows (cut) | Move code around |
| 7-5   | **BOSS: "The Rearrangement"** | Code blocks in wrong order, rearrange correctly |

#### World 8 — The Selector

| Level | Focus | Task Types |
|-------|-------|------------|
| 8-1   | `v` + motions + `d`/`c`/`y` | Visual character selection |
| 8-2   | `V` + operators | Visual line selection |
| 8-3   | Visual + counts (`v3w` then `d`) | Combining visual with motions |
| 8-4   | Mixed visual operations | Combined visual challenges |
| 8-5   | **BOSS: "The Bulk Edit"** | Multiple sections needing identical operations |

#### World 9 — Text Object Surgeon

| Level | Focus | Task Types |
|-------|-------|------------|
| 9-1   | `iw` `aw` | Inner/around word |
| 9-2   | `i"` `i'` `a"` `a'` | Inside/around quotes |
| 9-3   | `i(` `i{` `i[` `a(` etc | Inside/around brackets |
| 9-4   | `ci"` `di(` `yi{` | Operators + text objects |
| 9-5   | **BOSS: "The Nested Beast"** | 5+ levels of nesting (JSON, callbacks), deep `ci{` `da(` |

#### World 10 — Code Navigator

| Level | Focus | Task Types |
|-------|-------|------------|
| 10-1  | `{` `}` (paragraph motions) | Jump between functions |
| 10-2  | `%` (bracket matching) | Navigate matching pairs |
| 10-3  | `m{a-z}` marks + `'` jumps | Save and restore positions |
| 10-4  | All navigation combined | Mixed structural navigation |
| 10-5  | **BOSS: "The Labyrinth"** | 100+ line file, jump between functions/brackets/marks |

#### World 11 — Search & Destroy

| Level | Focus | Task Types |
|-------|-------|------------|
| 11-1  | `/` `?` (basic search) | Search forward/backward |
| 11-2  | `n` `N` (next/prev match) | Navigate search results |
| 11-3  | `*` `#` (word search) | Search word under cursor |
| 11-4  | Search + operators (`d/pattern`) | Delete/change to search match |
| 11-5  | **BOSS: "The Bug Hunt"** | Multiple instances of same bug pattern, use `*` `n` to find and fix |

#### World 12 — Time Traveler

Vim's undo system is a **tree**, not a line. When you undo and then make a new
edit, you create a branch. Most editors lose the undone work — Vim keeps it all.
This world teaches the player to navigate that tree and use registers as
multi-clipboard time travel.

**How the game creates undo branches:** The game performs **setup edits** on the
buffer during the countdown phase (before the player takes control). These edits
are shown as a brief "previously on..." montage — the code visibly changes a few
times, creating an undo history with branches. When the countdown ends, the player
inherits a buffer with a pre-built undo tree to navigate. For later levels (12-4,
12-5), the game also auto-executes "mistake" edits mid-level (shown as
`⚡ AUTO-EDIT: function deleted!` flash) that the player must undo/recover from.

| Level | Focus | Task Types |
|-------|-------|------------|
| 12-1  | `u` `Ctrl-R` (basic undo/redo) | Simple undo/redo tasks. Game makes a "mistake" mid-scroll, player undoes it. |
| 12-2  | Named registers (`"ayy` `"bp`) | Juggle multiple code pieces using registers a-z. Yank into `"a`, `"b`, paste elsewhere. |
| 12-3  | Undo tree branching (`g-` `g+` `:earlier` `:later`) | Pre-built undo tree from setup edits. Navigate between branches to recover "lost" edits. |
| 12-4  | Combine undo tree + registers | "The Accident" — game auto-deletes code mid-level. Undo to past state, yank into register, redo back, paste. Fish code from the past. |
| 12-5  | **BOSS: "The Time Paradox"** | Complex pre-built undo tree with 3+ branches. Reconstruct the correct code version by navigating branches and assembling pieces from registers. No hints. |

#### World 13 — Macro Wizard

| Level | Focus | Task Types |
|-------|-------|------------|
| 13-1  | `qa` ... `q` `@a` (record + play) | Basic macro recording |
| 13-2  | `@@` (replay last) | Replay efficiency |
| 13-3  | `5@a` (counted replay) | Batch macro execution |
| 13-4  | Complex macro chains (multi-step) | Multi-command macros |
| 13-5  | **BOSS: "The Assembly Line"** | 20+ identical changes, record one macro, replay 20 times |

#### World 14 — The Grandmaster (Grand Finale)

See Section 2.4 for full finale design.

| Level | Focus | Task Types |
|-------|-------|------------|
| 14-1  | Fix Bubble Sort | Simple edits |
| 14-2  | Fix Binary Search | Precision edits with `ciw`, `r` |
| 14-3  | Fix Quicksort | Multiple related bugs, needs search |
| 14-4  | Fix Merge Sort | Operators + text objects |
| 14-5  | **FINAL BOSS**: All algorithms combined | 15+ bugs, 100+ lines, fast scroll |

### 2.3 World Bosses — Unique Mechanics

Each world boss (level X-5) has a thematic twist that tests mastery of that
world's core skills. Bosses differ from regular levels in **both content and rules**:

**All bosses share:**
- **1.5x scroll speed** compared to regular levels in that world
- **More tasks per segment** (denser challenges)
- **No tutorial hints** in the code — you're on your own
- **No import runway** — first task appears immediately

**Each boss also has one unique content/rule twist** (see table):

| World | Boss Name | Unique Mechanic |
|-------|-----------|----------------|
| 1 | **"The Maze"** | Code structured as a navigation puzzle — deeply nested blocks, ASCII art. Pure `hjkl` + counts. |
| 2 | **"The Marathon"** | Extremely long camelCaseVariableNamesThatGoOnForever. `w`/`b`/`e` sprint through them. |
| 3 | **"The Sniper Range"** | 120+ character lines. Targets scattered across long strings/configs. `f`/`t` precision. |
| 4 | **"The Blank Page"** | Code with many missing pieces. Heavy insert mode — pick the right entry point every time. |
| 5 | **"The Cleanup"** | File full of dead code, commented-out blocks, debug prints. Strip it all down. |
| 6 | **"The Refactor"** | Systematic changes across a function. Each uses a different verb+noun combo. Heavy `.` repeat. |
| 7 | **"The Rearrangement"** | Code blocks in wrong order. Yank/delete/paste to rearrange everything correctly. |
| 8 | **"The Bulk Edit"** | Multiple sections need identical operations. Visual select + operate. |
| 9 | **"The Nested Beast"** | 5+ levels of nesting (JSON, nested callbacks). `ci{`, `da(`, `yi"` deep inside structures. |
| 10 | **"The Labyrinth"** | 100+ line file. Jump between functions, matching brackets, saved marks. Structural navigation. |
| 11 | **"The Bug Hunt"** | Multiple instances of the same bug pattern. Use `/`, `*`, `n` to find all, then fix. |
| 12 | **"The Time Paradox"** | Complex undo/register puzzle. Undo to recover deleted code, store in registers, redo, reassemble. |
| 13 | **"The Assembly Line"** | 20+ identical repetitive changes. Record one macro, replay 20 times. The efficiency ultimate. |
| 14 | **"The Final Boss"** | See Grand Finale — 15+ bugs across all sorting algorithms. |

### 2.4 Grand Finale — World 14: "The Grandmaster"

The finale uses **fix broken algorithms** — which exercises the full Vim toolkit
(navigate, edit, search, undo, repeat). Writing code from scratch is just typing;
*fixing* buggy code is where Vim mastery shines.

| Level | Algorithm | Bugs to Fix | Why it's a good test |
|-------|-----------|-------------|---------------------|
| 14-1 | Bubble Sort | Wrong comparison, missing swap, off-by-one loop bound | Warm-up. Simple edits. |
| 14-2 | Binary Search | Off-by-one, wrong midpoint calc (`/` vs `//`), incorrect return | Precision edits with `ciw`, `r` |
| 14-3 | Quicksort | Partition logic wrong, bad pivot, missing recursion base case | Multiple related bugs — needs `/` and `n` to find them |
| 14-4 | Merge Sort | Incorrect merge step, missing base case, wrong slice indices | Deep understanding of operators + text objects |
| 14-5 | **FINAL BOSS**: Full program using all 4 algorithms | 15+ bugs across 100+ lines. Fast scroll. | Everything. All skills. Maximum pressure. |

**Cherry on the cake — the code "runs":**

When the player completes level 14-5, the game triggers a victory sequence:

1. Buffer stops scrolling. Cursor disappears.
2. Screen clears to a clean code view of the fixed algorithms.
3. A simulated terminal output appears below, line by line (typewriter effect):

```
$ python sorting.py
Running bubble sort...    [1, 2, 3, 4, 5, 6, 7, 8, 9, 10] ✓
Running binary search...  found 7 at index 6 ✓
Running quicksort...      [1, 2, 3, 4, 5, 6, 7, 8, 9, 10] ✓
Running merge sort...     [1, 2, 3, 4, 5, 6, 7, 8, 9, 10] ✓

ALL TESTS PASSED ✓
```

4. Final congratulations screen:

```
┌──────────────────────────────────────────────┐
│                                              │
│        Congratulations, Grandmaster.         │
│                                              │
│   You didn't just play a game — you fixed    │
│   real code with pure Vim mastery.           │
│                                              │
│   Total time:    28:43                       │
│   Total keys:    1,247                       │
│   Stars earned:  ★★★ × 62 / 70              │
│   Achievements:  8 / 11                      │
│                                              │
│        Press any key...                      │
└──────────────────────────────────────────────┘
```

5. Optional: fun credits scroll with Vim facts and contributor names.

### 2.5 Roguelike Progression

**Fail = restart from checkpoint.**

- No "retry just this level" in the default mode
- Game is short enough (~30 min skilled) that restarting IS the learning loop
- If you can't pass World 3, you need more practice with Worlds 1-2 anyway
- Creates real tension — every keystroke matters
- Each run you get better at Vim, both in-game and in real life

**Checkpoint system:** Highest consecutive 3-star chain from 1-1 = your checkpoint.
- When you 3-star a level, it's permanently recorded
- Your checkpoint = the level AFTER your highest consecutive 3-star chain from 1-1
- Example: 3-starred 1-1 through 4-3 → checkpoint is 4-4
- If you 3-star 1-1, 1-2, miss 1-3, 3-star 1-4 → checkpoint is still 1-3 (chain broke)
- This incentivizes going back to perfect skipped levels to extend the chain
- Stars earned during a failed run are still saved (progress isn't lost, only position)

**Death UX flow:**
1. Game Over screen shows score, stats, and current checkpoint
2. Options: [R] Restart from checkpoint / [B] Start from 1-1 / [Q] Quit to menu
3. Starting from checkpoint = full energy, fresh run from that level forward
4. All levels between checkpoint and death lose their in-progress state (but saved stars persist)

**World map** (main menu):
- Visual grid showing all 14 worlds × 5 levels
- Gold star = 3-starred (part of checkpoint chain)
- Silver star = 1-2 stars (reached but not mastered)
- Lock icon = not yet reached
- Checkpoint marker shows where next run starts
- Current run progress shown as a path through the map

### 2.6 Skill Gating

- Unlocked motions are cumulative (World 5 player has all Worlds 1-5 skills)
- Pressing an unlearned key does nothing (no penalty, no energy drain)
- Forces players to solve problems with the right tools
- Tutorial intro for each world teaches: "You now have access to X, Y, Z"
- Skills are unlocked by reaching worlds, not purchased

**UX when pressing an unlearned key:**
- Brief subtle flash on the status bar: `🔒 w — unlock in World 2`
- Disappears after 0.5 seconds
- Only shown **once per key per level** (don't nag the player)
- No energy drain, no score penalty, no sound — just a gentle hint
- Teaches players what's coming next and builds anticipation
- In the results screen, if the player pressed locked keys frequently:
  *"You tried 'w' 12 times — it unlocks in World 2!"*

---

## 3. Game Modes

### 3.1 Story Mode

The default experience. Player picks a **difficulty level** (which controls scroll
speed and energy drain/restore multipliers) and plays through the 14 worlds with
progressive vim motion unlocking per world. Motions are gated by the world system —
reach a world to unlock its skills.

### 3.2 Endless Mode

All vim motions unlocked. Player picks a **language**, then plays infinite scrolling
code with random tasks. No difficulty selection — speed ramps automatically based on
lines of code scrolled.

**Zone system (line-based):**
- Every **1000 lines** = new zone with a speed increase
- Each zone boundary triggers a **30-row break banner** — no tasks, just the zone
  name and flavor text. Gives the player a breather and a sense of achievement.
- After "Uses Arch btw", zones scale exponentially: 100X, 1000X, 10^NX Engineer...
- Banner is styled as code comments in the player's selected language:

```
│  998 │     return result                              │
│  999 │ }                                              │
│ 1000 │                                                │
│ 1001 │ // =========================================== │
│ 1002 │ //                                             │
│ 1003 │ //         :wq Survivor                        │
│ 1004 │ //                                             │
│ 1005 │ //   "You can exit Vim. That's more than       │
│ 1006 │ //    most."                                   │
│ 1007 │ //                                             │
│ 1008 │ //         Score: 4,230                        │
│ 1009 │ //                                             │
│ 1010 │ // =========================================== │
│ 1011 │                                                │
│ 1012 │ fn process_batch(items: &[Item]) -> Vec<Out> { │
```

| Lines     | Zone              | Flavor Text                                              |
|-----------|-------------------|----------------------------------------------------------|
| 0–999     | Nano User         | *"Welcome. You can do this."*                            |
| 1000–1999 | :wq Survivor      | *"You can exit Vim. That's more than most."*             |
| 2000–2999 | Keyboard Warrior  | *"Your fingers are starting to blur."*                   |
| 3000–3999 | 10x Engineer      | *"Your terminal fears you."*                             |
| 4000–4999 | Uses Arch btw     | *"You don't use Vim. Vim uses you."*                     |
| 5000–5999 | 100X Engineer     | *"At this point you're compiling by hand."*              |
| 6000–6999 | 1000X Engineer    | *"The machine code writes itself when you're near."*     |
| 7000+     | 10^NX Engineer    | Keeps scaling: 10^4X, 10^5X, 10^6X...                   |

**Death conditions:**
1. Cursor scrolls off the top of the viewport
2. Energy reaches 0

**Scoring:** Linear — no multipliers. Score is score. Clean leaderboard comparison.

**Personal ghost:** Faint marker showing where the player's previous best run died.
*"You passed your ghost at line 4,230."*

**Nerd crowd meter:** Fun persistent element — Stack Overflow rep, GitHub stars,
NPM downloads, PR approvals. Shown on break banners or as HUD element.

### 3.3 Rhythm Mode — "Vim Drop"

A separate game mode where vim commands fall down a single lane, queue up, and
the player types them in order — watching each command execute live on a code
buffer. Combines rhythm-game feel with real vim editing practice.

#### Core Concept

```
  Code buffer (left)              │  Command lane (right)
                                  │
    def calculate(items):         │      ciw
      old_price = 0        ←highlight   j
      for item in items:          │      dw
        old_price += item.cost    │      w
      tmp = old_price * 1.1       │
      return tmp                  │  ═══════════════
                                  │  → ciw  ← type this
                                  │    j       (queued)
                                  │    dw      (queued)
                                  │
  Input: c          STREAK: 14    │  Nano User  Score: 91
```

- **Single lane**: all commands fall down one column
- **Queue**: commands stack at the bottom strike zone, processed in order
- **Live execution**: each correct command visibly modifies the code buffer
- **Wrong keys ignored**: don't execute, but reset multiplier to 0
- **Commands stay queued** until correctly typed

#### Two Sub-Modes

**Mode A — Guided Drop**: You see the vim commands falling and queued. Type them
as they arrive. Teaches command recognition and muscle memory.

**Mode B — Blind Drop**: Commands are **hidden**. Instead, the code buffer shows
highlights + gutter descriptions of what needs to happen (e.g., highlight
`old_price` with hint "change to `new_price`"). You must figure out the correct
vim command. Tests true vim fluency.

#### Scoring System

**0-indexed, base-1 — a coder's scoring system.**

```
multiplier starts at 0

Per correct command:  score += 1 × multiplier, then multiplier += 1
On wrong key:        multiplier = 0  (brutal reset)
On timeout:          multiplier -= 1 (min 0)
End of level:        score -= total_time_seconds × coefficient
```

**Progression example (perfect streak):**
```
Hit 1:  1 × 0 = 0 pts   (multiplier becomes 1)
Hit 2:  1 × 1 = 1 pts   (multiplier becomes 2)
Hit 3:  1 × 2 = 2 pts   (multiplier becomes 3)
...
Hit 50: 1 × 49 = 49 pts (multiplier becomes 50)

Total after 50 perfect hits: 0+1+2+...+49 = 1225 (triangular number)
```

One mistake at hit 50? Back to multiplier 0. Accuracy is king.

**Three scoring layers:**

| Layer | Mechanic | Effect |
|-------|----------|--------|
| Accuracy | Multiplier resets to 0 on wrong key | Dominates scoring. Streaks are everything. |
| Time decay | Multiplier -1 every `timeout` seconds of inactivity | Keeps pressure on per difficulty level |
| Time tax | Final score -= total_seconds × coefficient | Rewards overall speed |

#### Difficulty Levels

| Level | Name | Timeout | Vibe |
|-------|------|---------|------|
| 1 | **Nano User** | 10s | Learning, hunting for keys |
| 2 | **:wq Survivor** | 5s | Can exit vim, still thinking |
| 3 | **Keyboard Warrior** | 2s | Knows the commands, building speed |
| 4 | **10x Engineer** | 0.5s | Muscle memory, rapid fire |
| 5 | **Uses Arch btw** | 0.2s | Meme-tier. Barely human reaction time. |

#### Architecture

**Top-Level Mode Dispatch:**

```rust
enum AppMode {
    StoryMode(App),        // existing game, untouched
    RhythmMode(RhythmApp), // new
}
```

`main.rs` holds `AppMode` and dispatches `tick()` + `render()`. Existing `App`
stays as-is — pure additive change.

**New Module Structure:**

```
src/rhythm/
  mod.rs          — pub mod declarations
  app.rs          — RhythmApp state machine (Guided + Blind sub-modes)
  engine.rs       — timing, note spawning, queue management
  note.rs         — Note, QueuedNote structs
  scoring.rs      — 0-indexed multiplier, time decay, time tax
  input.rs        — keystroke matching, multi-key sequence handling
  song.rs         — Song/level definition + TOML deser
src/content/
  rhythm_loader.rs — load songs from content/rhythm/ via include_dir!
src/ui/
  rhythm_view.rs   — split layout: code buffer (left) + command lane (right)
```

**Key Data Structures:**

```rust
struct Note {
    keys: String,           // "ciw", "dw", "j", "3w", etc.
    description: String,    // "change inner word to 'new_price'"
    target_line: usize,     // line in code buffer this affects
    target_col: usize,      // column
    difficulty: Difficulty,
}

struct QueuedNote {
    note: Note,
    state: NoteState,       // Falling | Queued | Active | Completed
    y_position: f64,        // for falling animation
}

enum RhythmSubMode {
    Guided,  // commands visible
    Blind,   // commands hidden, hints shown in code
}

struct RhythmApp {
    sub_mode: RhythmSubMode,
    code_buffer: Buffer,       // reuse existing vim Buffer
    cursor: Cursor,            // reuse existing vim Cursor
    falling_notes: Vec<QueuedNote>,
    queue: VecDeque<QueuedNote>,
    active_note: Option<QueuedNote>,
    input_buffer: String,      // for multi-key sequences
    scoring: RhythmScoring,
    difficulty: Difficulty,
    song: Song,
    engine: RhythmEngine,
}
```

**Rendering — Split Layout:**

```
┌─────────────────────────────────┬──────────────────┐
│                                 │   Command Lane   │
│        Code Buffer              │                  │
│   (with cursor, highlights,     │    dw  (falling) │
│    live execution visible)      │    j             │
│                                 │    ciw           │
│                                 │  ════════════    │
│                                 │  → dw  (active)  │
│                                 │    j   (queued)  │
│                                 │    ciw (queued)  │
├─────────────────────────────────┴──────────────────┤
│ Input: d     STREAK: 14    Nano User    Score: 91  │
└────────────────────────────────────────────────────┘
```

In **Blind mode**, the command lane shows `???` instead of command text, and the
code buffer shows highlighted regions with gutter hints like
`◀ change to 'new_price'`.

**Reuse from Existing Code:**

- **`vim::Buffer`** + **`vim::Cursor`** — code buffer and cursor for live execution
- **`vim::command::CommandParser`** — parse player input into Actions
- **`vim::motions`** — execute parsed motions/commands on the buffer
- **`content::loader`** pattern — `include_dir!` for loading rhythm song TOML files
- **`game::scoring`** pattern — structural reference for RhythmScoring
- **`ui::game_view`** patterns — ratatui layout, HUD rendering, highlight styling

**Song Content Format:**

```toml
# content/rhythm/basic-motions/first-steps.toml
[meta]
id = "basic-motions-1"
name = "First Steps"
description = "Basic movement on a simple function"
difficulty = "easy"
language = "python"

[code]
content = """
def calculate(items):
    old_price = 0
    for item in items:
        old_price += item.cost
    return old_price
"""

[[commands]]
keys = "w"
description = "Jump to next word"
target = { line = 1, col = 0 }

[[commands]]
keys = "dw"
description = "Delete 'old_'"
target = { line = 2, col = 4 }

[[commands]]
keys = "j"
description = "Move down"
target = { line = 2, col = 4 }
```

### 3.4 Difficulty Levels

Shared difficulty names used across Story Mode and Rhythm Mode:

| Level | Name                 | Story (scroll speed) | Story (energy) | Rhythm (timeout) |
|-------|----------------------|----------------------|----------------|------------------|
| 1     | **Nano User**        | Very slow            | Low drain, high restore | 10s |
| 2     | **:wq Survivor**     | Slow                 | Medium drain/restore | 5s |
| 3     | **Keyboard Warrior** | Medium               | Balanced | 2s |
| 4     | **10x Engineer**     | Fast                 | High drain, low restore | 0.5s |
| 5     | **Uses Arch btw**    | Brutal               | Extreme drain, minimal restore | 0.2s |

Endless Mode does not use this difficulty system — its speed is determined entirely
by line-based zone progression.

### 3.5 Future Mode Ideas

- **Practice Mode**: No scrolling, no energy drain. Just tasks on a static buffer.
  Useful for drilling specific skills without restart pressure. Could be unlocked
  after reaching a world in story mode.
- **Daily Challenge**: One shared level per day, same for everyone. Leaderboard.

---

## 4. Content System

### 4.1 Languages

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

### 4.2 Segment Pool Architecture

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

Segments should also be tagged by **world** so that the segment pool for World 5
("The Destroyer") contains segments with delete-heavy tasks, World 9 segments have
deeply nested structures for text object practice, etc.

### 4.3 Segment Format

```toml
[meta]
id = "py-junior-api-fetch"
zone = "junior"
language = "python"
tags = ["functions", "error-handling", "http"]
world_tags = ["destroyer", "verb-noun"]    # which worlds this segment suits
difficulty = 3                    # 1-5 within the zone
hints = ["J04", "J06"]           # Hint IDs from Section 1.8 to embed as comments

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

### 4.4 Task Types

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

### 4.5 Assembly Algorithm

When a level starts:

1. Determine zone and world from level number.
2. Load segment pool for the player's chosen language + zone, filtered by world tags.
3. Randomly select 3–6 segments, weighted by:
   - Tags matching the level's target commands.
   - Not recently seen (tracked in save file, last 3 playthroughs).
4. Stitch segments with natural separators (blank lines, comments like
   `// ---` or `# ---`).
5. Resolve task anchors → absolute line/column positions in assembled buffer.
6. Order tasks top-to-bottom to match scroll direction.

### 4.6 Code Complexity by Zone

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

### 4.7 Adding New Content

New segments, languages, and task types can be added using the
`add-content` skill file (see `.claude/skills/add-content.md`). The skill
guides contributors through the segment format, validation rules, and
naming conventions.

---

## 5. Achievements System

Achievements are optional goals that completionists chase. They replace the
"constraint level" concept — instead of blocking progression, they're bragging
rights with cosmetic rewards.

### Achievement Ideas

| Achievement | Condition | Difficulty |
|-------------|-----------|-----------|
| **"Wordsmith"** | Complete World 2 without using `h` or `l` | Medium |
| **"One Shot"** | Complete any level with 0 wasted keystrokes | Hard |
| **"Speed Demon"** | Complete World 1 in under 60 seconds | Medium |
| **"Pacifist"** | Complete a level without deleting anything | Easy |
| **"No Arrow Keys"** | Never press arrow keys during an entire run | Easy |
| **"The Floor is Lava"** | Complete a level without ever stopping (no idle ticks) | Hard |
| **"Untouchable"** | Complete a world without energy ever dropping below 50% | Hard |
| **"Minimalist"** | Complete a level using fewer keystrokes than par | Hard |
| **"Marathon Runner"** | Reach line 5000 in Endless Mode | Medium |
| **"Perfect World"** | 3-star every level in a single world | Medium |
| **"Grandmaster"** | Complete World 14 | Hard |

### Achievement Rewards

- Terminal color themes
- Cursor style unlocks
- Fun HUD skins
- Title next to player name on leaderboard

---

## 6. Easter Eggs & Vim Wisdom

**Not formal unlocks** — fun comments and references scattered throughout level
content as part of the code segments. These serve double duty: they make the game
feel alive AND teach deeper Vim understanding.

### Vim Wisdom (educational easter eggs)

Sprinkled as code comments that teach deeper understanding:
- `// Fun fact: 'dd' is actually 'd' + 'd' — the delete operator applied to a line motion`
- `# Pro tip: 'ciw' is just 'c' + 'iw' — change + inner-word. All operators work with text objects!`
- `// 'diw' deletes the word, 'daw' deletes the word AND the space. The 'a' = "around"`

### Nerdy References

- `// TODO: exit vim` (World 1)
- `i_use_arch_btw = True` (variable name)
- `# This code was written by someone who definitely uses Emacs` (task: delete this line)
- `// The cake is a lie` (Portal reference)
- `sudo_rm_rf = "please don't"`
- `# There are only 10 types of people...` (binary joke)
- `// It works on my machine ¯\_(ツ)_/¯`
- A function called `turnOffAndOnAgain()` (IT Crowd)
- `# First, solve the problem. Then, write the code. — John Johnson`
- Hidden Konami code reference in a comment
- `// I'm not saying it's aliens, but it's aliens` (in auto-generated code)
- `# rm -rf / was here` in a comment the player must delete

---

## 7. Keybinding / Config System

### 7.1 Config File Location

`~/.vim-heroes/config.toml` — created on first launch with sensible defaults
and comments explaining every option.

### 7.2 Config Structure

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

### 7.3 Key Syntax

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

### 7.4 Built-in Presets

| Preset       | Movement keys | Notes                            |
|--------------|---------------|----------------------------------|
| `qwerty`     | `h j k l`     | Default, stock Vim               |
| `colemak`    | `h n e i`     | Common Colemak Vim remap         |
| `colemak-dh` | `m n e i`     | Colemak-DH variant               |
| `dvorak`     | `d h t n`     | Common Dvorak Vim remap          |
| `workman`    | `y n e o`     | Workman layout                   |

Presets remap the full set of keys for that layout. Individual overrides in
`[keymap]` take precedence over the preset.

### 7.5 Architecture Integration

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

## 8. Technical Architecture

### 8.1 Stack

| Component        | Choice             | Rationale                              |
|------------------|--------------------|----------------------------------------|
| Language         | Rust               | Single binary, fast rendering, no runtime |
| Terminal UI      | ratatui + crossterm | Production-grade TUI (powers lazygit, etc.) |
| Text buffer      | ropey              | Efficient rope DS for insert/delete    |
| Config parsing   | serde + toml       | Ergonomic TOML parsing                 |
| Content embed    | include_dir        | Bake segments into the binary          |
| Save data        | serde + bincode    | Fast local save to ~/.vim-heroes/      |

### 8.2 Project Structure

```
vim-heroes/
├── src/
│   ├── main.rs                  # Entry point, terminal init/cleanup, AppMode dispatch
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
│   │   ├── energy.rs            # Energy bar system, drain/restore logic
│   │   ├── task.rs              # Task state machine: pending → active → done/missed
│   │   ├── worlds.rs            # World definitions, skill gating, progression
│   │   └── level.rs             # Level metadata, progression logic
│   │
│   ├── rhythm/                  # Rhythm Mode (Vim Drop)
│   │   ├── mod.rs
│   │   ├── app.rs               # RhythmApp state machine (Guided + Blind)
│   │   ├── engine.rs            # Timing, note spawning, queue management
│   │   ├── note.rs              # Note, QueuedNote structs
│   │   ├── scoring.rs           # 0-indexed multiplier, time decay, time tax
│   │   ├── input.rs             # Keystroke matching, multi-key sequences
│   │   └── song.rs              # Song/level definition + TOML deser
│   │
│   ├── content/                 # Content management
│   │   ├── mod.rs
│   │   ├── segment.rs           # Segment struct, TOML parsing
│   │   ├── assembler.rs         # Stitch segments into a level buffer
│   │   ├── anchor.rs            # Resolve pattern anchors → buffer positions
│   │   ├── rhythm_loader.rs     # Load rhythm songs from content/rhythm/
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
│   │   ├── rhythm_view.rs       # Rhythm mode split layout
│   │   ├── menu.rs              # Main menu, level select, language picker
│   │   ├── hud.rs               # Score, combo, stars, energy bar, level info
│   │   ├── task_overlay.rs      # Red/green/yellow highlights + gutter annotations
│   │   ├── results.rs           # End-of-level results screen
│   │   └── theme.rs             # Color themes
│   │
│   └── progress/                # Player progress
│       ├── mod.rs
│       ├── save.rs              # Stars, high scores, checkpoints → ~/.vim-heroes/save.dat
│       ├── achievements.rs      # Achievement tracking and unlock logic
│       └── unlocks.rs           # Cosmetics, achievement rewards
│
├── config/
│   └── energy.toml              # Tunable energy bar values
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
│   ├── cpp/
│   │   └── ...
│   └── rhythm/                  # Rhythm Mode songs
│       ├── basic-motions/
│       └── ...
│
├── Cargo.toml
├── PLAN.md                      # This file
└── README.md
```

### 8.3 Key Dependencies

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

### 8.4 Game Loop

```rust
// Pseudocode
loop {
    // 1. Non-blocking input
    if poll_input(timeout: 33ms) {        // ~30 fps
        let key = read_key();
        if !world_allows_key(key, current_world) {
            continue;                     // skill gating: ignore unlearned keys
        }
        keystroke_count += 1;
        energy -= keystroke_drain;
        let action = keymap.resolve(key); // physical → logical
        vim_engine.execute(action);
        check_task_completion(&mut tasks, &buffer, &cursor);
    }

    // 2. Scroll tick
    if elapsed >= scroll_interval {
        viewport.scroll_down(1);
        energy -= time_drain;
        if cursor.line < viewport.top_line {
            return GameOver;
        }
        if energy <= 0 {
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
        render_hud(frame, &game_state, &energy);
        render_buffer(frame, &buffer, &viewport, &cursor, &tasks);
        render_statusbar(frame, &vim_engine, &keystroke_count, &current_task);
    });

    // 5. Check level complete
    if viewport.bottom_line >= buffer.len_lines() {
        return LevelComplete;
    }
}
```

### 8.5 Distribution

| Channel              | Tool / Method          | Audience              |
|----------------------|------------------------|-----------------------|
| `cargo install`      | crates.io              | Rust developers       |
| Homebrew             | homebrew-tap           | macOS / Linux         |
| AUR                  | PKGBUILD               | Arch Linux            |
| `.deb` package       | cargo-deb              | Debian / Ubuntu       |
| GitHub Releases      | Prebuilt binaries      | Everyone              |
| Snap / Flatpak       | snapcraft / flatpak    | Universal Linux       |

---

## 9. Multi-Buffer System

Starting from later worlds, levels can use multiple code files (buffers). This
teaches real-world multi-file Vim workflow and adds a context-switching challenge.

### 9.1 Buffer Count Progression

| Zone    | Buffers | Switch Trigger        | Notes                           |
|---------|---------|----------------------|---------------------------------|
| Starter | 1 | —                  | Single file, learn the basics   |
| Junior  | 1-2 | Event-driven       | Game forces swaps at set points |
| Medior  | 2-3 | Event + player-initiated | Cross-file tasks (yank from A, paste in B) |
| Senior  | 3-4 | Player-initiated   | Register juggling across files  |

### 9.2 HUD: Buffer Line

When multiple buffers are active, the HUD shows a buffer line:

```
┌──────────────────────────────────────────────────┐
│ [1] main.py  │  2  utils.py  │  3  config.py    │  ← buffer line
├──────────────────────────────────────────────────┤
│ ★★☆  Level 10-2                  Score: 2,100  ×2│  ← HUD
│ Energy: ████████████████░░░  82%                 │
├──────────────────────────────────────────────────┤
│  14 │   for item in items:                       │
│  ...                                             │
```

- Active buffer is highlighted (e.g., `[1]` with brackets and bold).
- Buffer line is always visible when there are 2+ buffers.
- Modified buffers show a `+` indicator: `[1+] main.py`.

### 9.3 Event-Driven Swaps (Earlier Levels)

In earlier multi-buffer levels, the game controls when swaps happen. A swap
event scrolls into view as a visual marker:

```
  24 │ }
  25 │
  26 │ ══════════════════════════════════════
  27 │  ▸ SWITCH TO: utils.py
  28 │ ══════════════════════════════════════
  29 │
```

When the viewport reaches the swap marker, the buffer switches automatically.

### 9.4 Player-Initiated Swaps (Later Levels)

In later levels, tasks require the player to actively switch buffers:

- *"Yank the function signature from `utils.py` and paste it in `main.py`"*
- *"The variable name in `config.py` is wrong — switch there and fix it"*

### 9.5 Segment Format Addition

```toml
[meta]
id = "py-medior-multi-api"
zone = "medior"
language = "python"
buffers = ["main.py", "utils.py"]

[code.main_py]
content = """
from utils import fetch_data
..."""

[code.utils_py]
content = """
import requests
..."""

[[tasks]]
buffer = "main.py"
type = "change_word"
anchor = { pattern = "fetch_data", occurrence = 1 }
...

[[swap_events]]
after_line = 24
from = "main.py"
to = "utils.py"
```

---

## 10. Ideas Parking Lot

Good ideas that don't belong in Story Mode but could be great minigames,
features, or separate modes in the future.

### Ghost Racer (Minigame)
- Race against your previous best run (ghost cursor)
- Race against "optimal" theoretical ghost
- Could be a standalone mode with its own menu entry
- Replayability: "I know I can beat my ghost on 3-2"

### Combo Zones (Endless Mode / Minigame)
- Highlighted sections where perfect play = massive bonus
- Screen flashes, particles on perfect combo zone completion
- Too arcade-y for story mode, perfect for endless/rhythm modes

### Daily Gauntlet (Minigame)
- One randomly generated level per day
- Global leaderboard by keystroke efficiency
- Keeps people coming back after finishing campaign
- Could use any world's skill set randomly
- Would need some form of online leaderboard (simple API?)

### Cosmetics Shop
- Credits earned from achievements or completing worlds
- Cursor styles (green, blue, amber, underscore, pipe, blinking)
- Color themes (Gruvbox, Catppuccin, Solarized, Dracula)
- HUD skins (Stock Vim, NeoVim, SpaceVim, LazyVim)
- Relative line numbers, nerd font icons, task complete sparkle

### Personal Code: GitHub Integration
- Fetch player's public repos and code files via GitHub REST API
- Auto-slicer: split fetched files into game-sized segments with auto-generated tasks
- "Your Code" mode in level select
- Cache locally, graceful degradation if API unreachable
- No auth tokens needed, only public repos

### Multiplayer
- Split-screen race mode (who finishes the level with more points)
- Asynchronous leaderboards per level

---

## 11. Phased Implementation Plan

Each phase is a self-contained deliverable. Complete one before starting the next.

### Phase 1 — Project Skeleton & Vim Buffer ✅

**Goal**: Rust project with text buffer and basic Vim motions.

**Deliverables**: `vim/buffer.rs`, `vim/cursor.rs`, `vim/mode.rs`,
`vim/motions.rs` (hjkl), `vim/command.rs`, unit tests.

---

### Phase 2 — Terminal Rendering & Input ✅

**Goal**: See the buffer on screen, move cursor with real keystrokes.

**Deliverables**: `main.rs`, `ui/game_view.rs`, basic keymap, input loop, `app.rs`.

---

### Phase 3 — Scrolling Viewport & Game Over ✅

**Goal**: Core Guitar Hero mechanic — viewport scrolls, cursor must keep up.

**Deliverables**: `game/viewport.rs`, `game/engine.rs`, game over detection,
scroll indicator, basic game over screen.

---

### Phase 4 — Expanded Motions ✅

**Goal**: Enough Vim commands to make movement interesting.

**Deliverables**: w/b/e/W/B/E, 0/^/$, gg/G/{num}G, f/t/F/T/;/,, {/}, %.

---

### Phase 5 — Task System ✅

**Goal**: Tasks appear in the buffer and can be completed.

**Deliverables**: `game/task.rs`, `content/anchor.rs`, `ui/task_overlay.rs`,
gutter annotations, `move_to` task type.

---

### Phase 6 — Scoring & HUD ✅

**Goal**: Points, combos, feedback.

**Deliverables**: `game/scoring.rs`, `ui/hud.rs`, `ui/statusbar.rs`,
`ui/results.rs`, score formula.

---

### Phase 7 — Content System & Segment Loader ✅

**Goal**: Levels assembled from content segments, not hardcoded.

**Deliverables**: `content/segment.rs`, `content/assembler.rs`,
`content/history.rs`, `include_dir!`, level metadata.

---

### Phase 8 — First Content: Python & TypeScript (Starter + Junior) ✅

**Goal**: Enough content for a real playable demo.

**Deliverables**: ~80 Python segments, ~80 TypeScript segments, import runway,
3-second countdown, language selection, hint comments.

---

### Phase 9 — Editing Commands & Advanced Tasks ✅

**Goal**: Full operator + motion system.

**Deliverables**: d/c/y operators, text objects, insert mode, r/R, dot repeat,
u/Ctrl-r, visual mode, registers, macros, search, new task types.

---

### Phase 10 — Energy Bar System

**Goal**: Replace keystroke penalty with energy bar as core survival mechanic.

**Deliverables**:
- `game/energy.rs`: energy state, drain/restore logic
- `config/energy.toml`: tunable values (see Section 1.3)
- Energy bar rendering in HUD (color gradient, pulse on low)
- Energy drain on keystroke + scroll tick
- Energy restore on task completion (more for optimal)
- Difficulty multipliers for drain/restore rates
- Game over when energy reaches 0
- "+N" popup animation on energy restore

**Exit criteria**: Playing a level shows energy bar draining per keystroke,
refilling on task completion, and game over when hitting 0.

---

### Phase 11 — World System & Skill Gating

**Goal**: 14 themed worlds with progressive skill unlocking.

**Deliverables**:
- `game/worlds.rs`: world definitions (14 worlds, skills per world)
- Skill gating: unlearned keys are ignored (no penalty, no drain)
- World-aware level metadata (replaces old zone-only system)
- Content segments tagged with `world_tags` for appropriate selection
- World-based level assembly (World 5 gets delete-heavy segments)
- Tutorial intro segments for each world's first level

**Exit criteria**: Starting World 1 only allows hjkl+counts. Starting World 6
allows all prior skills + operators. Pressing `w` in World 1 does nothing.

---

### Phase 12 — Roguelike Progression & Checkpoints

**Goal**: Fail = restart, with 3-star checkpoints.

**Deliverables**:
- `progress/save.rs`: persist per-level stars, checkpoint tracking
- Game over → restart from highest consecutive 3-star checkpoint
- Level select showing world map with star progress
- 3-star a level → permanent checkpoint (next run can skip to it)
- Results screen: show checkpoint advancement

**Exit criteria**: Dying sends player back to their checkpoint. 3-starring a
level advances the checkpoint. Progress persists across sessions.

---

### Phase 13 — Full Content: All Languages, All Zones

**Goal**: Complete content library for 14 worlds.

**Deliverables**:
- Medior + senior segments for Python and TypeScript
- All 4 zones for Rust (~160 segments)
- All 4 zones for C++ (~160 segments)
- World boss content (unique segments for each X-5 level)
- Grand Finale content (buggy sorting algorithms for World 14)
- Easter egg comments and Vim wisdom scattered throughout
- Total: ~640 segments across all languages

**Exit criteria**: Every world/level combination works in every language.
World bosses have unique mechanics. Grand Finale is playable.

---

### Phase 14 — Achievements & Menus

**Goal**: Achievement system and polished menu UI.

**Deliverables**:
- `progress/achievements.rs`: achievement tracking, condition checking
- Achievement notification popup on unlock
- `ui/menu.rs`: main menu (Story Mode, Endless, Rhythm Mode, Achievements)
- World select screen with star progress and checkpoints
- Settings screen: difficulty, language, keybinds, theme, hints

**Exit criteria**: Achievements trigger correctly, persist across sessions.
Menu navigation is smooth and shows all relevant progress.

---

### Phase 15 — Rhythm Mode (Vim Drop)

**Goal**: Full Rhythm Mode as described in Section 3.3.

**Deliverables**:
- `src/rhythm/` module (app, engine, note, scoring, input, song)
- `src/ui/rhythm_view.rs`: split layout rendering
- `src/content/rhythm_loader.rs`: TOML song loading
- Guided and Blind sub-modes
- 0-indexed scoring with multiplier
- 5+ handcrafted songs
- Song selection menu
- Difficulty selection (Nano User → Uses Arch btw)
- Results screen: score, accuracy, streak

**Exit criteria**: Both Guided and Blind modes playable from song selection.
Scoring works correctly. 5+ songs available.

---

### Phase 16 — Multi-Buffer Levels

**Goal**: Levels with multiple code files and buffer-switching.

**Deliverables**:
- `vim/buffers.rs`: multi-buffer manager
- Buffer commands: `:e`, `:bn`, `:bp`, `:ls`, `Ctrl-^`
- Extended TOML format with multi-buffer support
- Event-driven swap markers (auto-switch)
- Buffer line in HUD
- Cross-file tasks

**Exit criteria**: Multi-buffer levels work with event-driven and player-initiated
swaps. Buffer line shows active file.

---

### Phase 17 — Polish & Endless Mode

**Goal**: Ship-quality experience.

**Deliverables**:
- Endless Mode (line-based zones, break banners, speed ramp, ghost)
- Cosmetics rendering (cursor styles, themes, HUD skins)
- Stats dashboard (command usage, accuracy trends, time played)
- Sound effects (optional)
- Config system (full keybinding customization, presets)
- First-run experience

**Exit criteria**: The game feels complete and polished.

---

### Phase 18 — Distribution & Release

**Goal**: Players can install easily on any platform.

**Deliverables**:
- CI/CD pipeline (GitHub Actions)
- `cargo install vim-heroes` → crates.io
- Homebrew formula
- `.deb` package, AUR PKGBUILD
- GitHub Releases with prebuilt binaries
- README with install instructions, screenshots, GIF demo

---

### Phase 19 — Personal Code: GitHub Integration

**Goal**: Let players practice Vim on their own code.

**Deliverables**:
- `content/github.rs`: fetch public repos via GitHub REST API
- Auto-slicer: split files into game-sized segments with auto-generated tasks
- "Your Code" mode in level select
- Local cache in `~/.vim-heroes/github-cache/`
- HTTP via `ureq` crate (no OpenSSL dependency)

**Exit criteria**: Player enters GitHub username, sees repos, plays levels
built from their own code.

---

### Future Ideas (Post-Release)

- Ghost Racer mode
- Daily Gauntlet with global leaderboard
- Multiplayer split-screen race
- Custom level packs
- More languages (Go, Java, Ruby, Zig, Haskell)
- Neovim plugin integration
- Community content submissions
- Seasonal cosmetics
- Replay analysis (view optimal solution for completed levels)
