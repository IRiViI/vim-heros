# Worlds

Every world surrounds a different concept of vim motions.

Every level has a content block in the beginning explaining the game mode and all the elements.
Also it says the keys that are expected to use in order to finish it.

## World 1: Motion

### Game mechanic

Code scrolls down the screen over time. The player must navigate to highlighted targets using only the allowed motions. Each motion typed costs 1 energy, regardless of distance covered (so `6j` costs the same as `j`). Energy resets when the player reaches the next target.

### Start condition

No countdown. The game starts on the player's first keystroke.

### Target rules

- Targets appear as grey boxes; the active target is highlighted with colour.
- Every next target is always within 20 lines of the previous one, so the jump is always reachable.
- Targets can be on the same line (before or after cursor), or on a line above — forcing use of `h`, `k`, and `b`, not just forward movement.
- The task description should contain the suggested Vim command as a hint (not a separate "command" field).

### Scroll speed

Scroll speed and error tolerance are tied to difficulty level:

| Difficulty | Name | Scroll speed | Errors allowed |
|---|---|---|---|
| 1 | Nano User | 1 line per 5s | 10 |
| 2 | :wq Survivor | 1 line per 2s | 5 |
| 3 | Keyboard Warrior | 1 line per 1s | 3 |
| 4 | 10x Engineer | 1 line per 0.5s | 1 |
| 5 | Uses Arch btw | 1 line per 0.2s | 0 |

An "error" is any motion that doesn't move the cursor closer to the target along the optimal path.

- When the next target is not in view, scroll speed increases by 4x until it is.

### Death hints

When the player dies, the game identifies the reason for death and shows a targeted hint. Examples:

- "Did you know `6j` moves 6 lines down? That's 1 motion instead of 6!"
- "Try `w` to jump to the next word instead of pressing `l` repeatedly."
- "Use `B` to jump back to the previous WORD — saves energy over `h h h h`."

The hint system should analyse the player's last sequence of motions and suggest the more efficient alternative.

### Levels

#### Level 1 — Basic movement
**Allowed:** `h` `j` `k` `l`

**Intro:**
```
# ═══════════════════════════════════
# WORLD 1: Motion
# ═══════════════════════════════════
#
# Welcome to Vim Heroes!
#
# Code scrolls down. Reach the targets
# before they scroll away.
#
# Your keys:
#   h — left
#   j — down
#   k — up
#   l — right
#
# Each motion costs 1 energy.
# Energy resets when you hit a target.
#
# The highlighted target below is your
# first objective. Go!
# ═══════════════════════════════════
```

Targets are close together. Teaches arrow-key equivalents. Targets mostly below and to the right, with a few same-line and backward targets to introduce `h` and `k`.

#### Level 2 — Word jumps and counts
**Allowed:** `h` `j` `k` `l` `w` `W` `b` `B` `e` + count prefixes (e.g. `5j`, `3w`)

**Intro:**
```
# ═══════════════════════════════════
# Level 1-2: Word jumps & counts
# ═══════════════════════════════════
#
# Pressing 'l' 20 times is slow.
# There's a better way:
#
#   w — jump to next word
#   b — jump back a word
#   W — jump to next WORD (skip punctuation)
#   B — jump back a WORD
#   e — jump to end of word
#
# And counts multiply ANY motion:
#   5j — move 5 lines down (1 motion!)
#   3w — jump 3 words forward
#
# Targets are further apart now.
# You'll need these to survive.
# ═══════════════════════════════════
```

Targets are further apart. Reaching them with just `h`/`j`/`k`/`l` would burn too much energy — player is forced to use word motions and counts to survive.

#### Level 3 — Line targeting
**Allowed:** `h` `j` `k` `l` `w` `W` `b` `B` `e` `f` `F` `t` `T` + count prefixes

**Intro:**
```
# ═══════════════════════════════════
# Level 1-3: Line targeting
# ═══════════════════════════════════
#
# New precision tools:
#
#   f{char} — jump forward TO {char}
#   F{char} — jump backward TO {char}
#   t{char} — jump forward UNTIL {char}
#   T{char} — jump backward UNTIL {char}
#
# Examples:
#   fa — jump to the next 'a'
#   2fe — jump to the second 'e'
#
# These are sniper rifles for
# horizontal movement. Use them.
# ═══════════════════════════════════
```

Introduces `f`/`F`/`t`/`T` for precise horizontal jumps. Targets are placed on specific characters within lines. Count prefixes work here too (e.g. `2fe` jumps to the second `e`).

#### Level 4 — Restricted zones
**Allowed:** All World 1 motions, but **zones restrict horizontal keys**.

**Intro:**
```
# ═══════════════════════════════════
# Level 1-4: Restricted zones
# ═══════════════════════════════════
#
# You know all the motions now.
# But do you REALLY know them?
#
# This level has ZONES. In each zone
# only certain horizontal keys work:
#
#   h/l zones  — character by character
#   w/b zones  — word by word
#   f/t zones  — find characters
#   $/0 zones  — line edges only
#
# j and k always work (vertical).
# No falling back on familiar keys.
# Prove you've mastered each one.
# ═══════════════════════════════════
```

`j` and `k` are always available (only vertical motions in Vim). But horizontal movement is restricted per zone:

- **h/l zones** — only `h` and `l` for horizontal movement
- **w/b zones** — only `w` `W` `b` `B` `e` for horizontal movement
- **f/t zones** — only `f` `F` `t` `T` for horizontal movement
- **$/0 zones** — only `$` and `0` for horizontal movement. Since the cursor usually starts at the beginning of a line, `$` targets come first, then `0` to get back. Teaches line-edge jumping.

Forces mastery of each horizontal technique by removing the fallback to familiar keys.

#### Level 5 — Perfect motions
**Allowed:** All World 1 motions.

**Intro:**
```
# ═══════════════════════════════════
# Level 1-5: Perfect motions
# ═══════════════════════════════════
#
# Final test.
#
# Every target must be reached in
# exactly ONE motion. No mistakes.
#
# You know the tools:
#   h j k l w W b B e f F t T $ 0
#   ...and counts (6j, 3fe, 2w)
#
# Pick the right one. Every time.
# ═══════════════════════════════════
```

Every target must be reached in exactly 1 motion. No wasted keystrokes — if you don't land on it perfectly, you lose energy. Every target is designed to be reachable in a single motion from the previous one (e.g. `6j`, `3fe`, `$`, `w`). The ultimate graduation test for World 1.

### Optimal path pre-calculation

Before a level starts, the game renders the full buffer and pre-calculates the optimal path between every consecutive pair of targets using BFS on the (line, col) state space.

- Each state is a cursor position (line, col).
- Edges are all allowed motions for the current level, simulated on the actual buffer content.
- Each motion costs 1 regardless of count prefix (`6j` = 1 motion, but simulates 6 `j` movements to determine the landing position).
- `j`/`k` column snapping is accounted for — moving vertically through short lines alters the column, so vertical and horizontal movement can't be calculated independently.
- BFS finds the shortest path (fewest motions) from cursor to target.
- The state space is small (lines x max columns), so BFS is instant.

This gives us:
1. **Energy budget per target** — the optimal motion count becomes the baseline.
2. **Death hints for free** — compare the player's actual motions to the optimal path and suggest the better alternative.
3. **Level validation** — guarantees every target is reachable within the allowed keys before the player starts.

## World 2: Basic Edit

### Game mechanic

Split-screen view. The right window shows the complete target code with highlighted differences. The left window shows the player's incomplete code. The player must add the missing text using insert mode commands, typing real code.

No auto-scrolling. The player navigates freely. Energy drains over time — every X seconds, energy decreases. Completing a task restores energy. Hit 0 energy = game over.

All World 1 motions are available for navigation (cumulative skill unlock).

### Start condition

No countdown. The game starts on the player's first keystroke.

### Content theme

Old school / legacy scripts: sorting algorithms (bubble sort, selection sort), fibonacci, factorial, binary search, linked list operations. Classic functions every programmer has written.

### Split-screen rules

- Left window: player's code (editable). Title: "Your Code".
- Right window: target code (read-only). Title: "Target".
- Differences are highlighted in the right window only — missing lines get a coloured background, missing inline text gets character-level highlighting.
- Both viewports scroll independently. The target viewport auto-scrolls to show the next task's relevant line.

### Energy

Energy drains over time (not per motion). Same difficulty tiers as World 1:

| Difficulty | Name | Energy drain | Errors allowed |
|---|---|---|---|
| 1 | Nano User | slow | 10 |
| 2 | :wq Survivor | moderate | 5 |
| 3 | Keyboard Warrior | fast | 3 |
| 4 | 10x Engineer | very fast | 1 |
| 5 | Uses Arch btw | extreme | 0 |

Completing a task restores energy. An "error" is using the wrong insert entry point (e.g., using `i` when `A` would be optimal).

### Levels

#### Level 1 — Insert basics
**Allowed insert commands:** `i` `a` `Esc`

**Intro:**
```
# ═══════════════════════════════════
# WORLD 2: Basic Edit
# ═══════════════════════════════════
#
# Split screen! Right side shows the
# target code. Left side is yours.
#
# Add the missing parts using:
#   i — insert before cursor
#   a — insert after cursor
#   Esc — back to normal mode
#
# Type the actual code. Energy drains
# over time — be quick!
# ═══════════════════════════════════
```

Adding code top-to-bottom, one insertion per task. Simple completions: fill in parameters, values, operators. The player navigates to the right spot, enters insert mode with `i` or `a`, types the missing text, and presses `Esc`.

#### Level 2 — Line edges
**Allowed insert commands:** `i` `a` `I` `A` `Esc`

**Intro:**
```
# ═══════════════════════════════════
# Level 2-2: Line edges
# ═══════════════════════════════════
#
# New entry points:
#
#   I — insert at beginning of line
#   A — insert at end of line
#
# Use I to add text at the start.
# Use A to append at the end.
# Much faster than navigating there
# with h/l first.
# ═══════════════════════════════════
```

Still top-to-bottom, one line at a time. Tasks require inserting at line edges: add a comment prefix with `I`, append a closing bracket or semicolon with `A`. Player learns that `I` and `A` are shortcuts for "go to start/end of line + insert mode".

#### Level 3 — New lines
**Allowed insert commands:** `i` `a` `I` `A` `o` `O` `Esc`

**Intro:**
```
# ═══════════════════════════════════
# Level 2-3: New lines
# ═══════════════════════════════════
#
# Now you can create new lines:
#
#   o — open line below, enter insert
#   O — open line above, enter insert
#
# This is where it gets real.
# Build entire functions line by line.
# ═══════════════════════════════════
```

Adding lines within code blocks — not top-to-bottom anymore. The player builds functions in a natural coding style: for example, start with a skeleton (function signature + for loops), then fill in the body logic using `o`/`O`. The content is designed so the line insertions make sense as real coding steps.

#### Level 4 — Restricted entry points
**Allowed insert commands:** All (`i` `a` `I` `A` `o` `O` `Esc`), but per-task restrictions.

**Intro:**
```
# ═══════════════════════════════════
# Level 2-4: Restricted entry points
# ═══════════════════════════════════
#
# You know all the insert commands.
# Now prove it.
#
# Each task tells you which command
# to use. A message in the HUD shows
# the allowed keys for the current
# task.
#
# No falling back on familiar ones.
# ═══════════════════════════════════
```

Each task specifies which insert command to use. The HUD shows a message like "USE: I only" or "USE: o/O only". This forces the player to use the right tool for each situation instead of always defaulting to `i`.

#### Level 5 — Perfect entry
**Allowed insert commands:** All.

**Intro:**
```
# ═══════════════════════════════════
# Level 2-5: Perfect entry
# ═══════════════════════════════════
#
# Final test.
#
# Every insertion must use the OPTIMAL
# entry command. No wasted keystrokes.
#
#   End of line? Use A, not l l l i.
#   New line below? Use o, not A Enter.
#   Start of line? Use I, not 0 i.
#
# One command. Every time.
# ═══════════════════════════════════
```

Every task must be completed using the optimal insert entry point. If the target text is at end of line, `A` is required — not `l l l i`. If a new line is needed below, `o` is required — not `A` then `Enter`. The game tracks which insert command was used and rejects non-optimal ones.

### Content format

TOML defines the target (complete) code in `[code]`. A `[[removals]]` list specifies what to remove to create the player's starting (incomplete) code. Each removal becomes a task.

- `type = "inline"`: text is removed from within a line. Player must insert it back with `i`/`a`/`I`/`A`.
- `type = "whole_line"`: entire line is removed. Player must add it with `o`/`O`.
- `entry_point`: the optimal insert command for this task (for level 4 restrictions and level 5 enforcement).

## Missing / To Do

Motions and commands not yet assigned to a world or level:

**Navigation:**
- `gg` — jump to the first line of the file
- `G` — jump to the last line of the file
- `{n}G` — jump to line n (e.g. `42G` goes to line 42)
- `^` — jump to first non-blank character on line
- `;` `,` — repeat last f/t search forward/backward
- `{` `}` — jump to previous/next empty line (paragraph motions)
- `%` — jump to matching bracket
- `R` — enter replace mode (overwrite characters)

**Deletion:**
- `x` — delete character under cursor
- `X` — delete character before cursor
- `dd` — delete entire line
- `D` — delete from cursor to end of line

**Operators (verb + noun):**
- `d{motion}` — delete with motion (e.g. `dw`, `d$`, `d3j`)
- `c{motion}` — change with motion (e.g. `cw`, `c$`)
- `.` — repeat last change
- `>>` `<<` — indent / dedent line
- `r{char}` — replace character under cursor

**Yank / paste:**
- `y` `yy` — yank (copy) text / yank line
- `p` `P` — paste after / paste before

**Visual mode:**
- `v` `V` — visual mode (character / line)
- operators on visual selections

**Text objects:**
- `iw` `aw` — inner word / around word
- `i"` `a"` `i(` `a(` `i{` `a{` — inner/around delimiters

**Search:**
- `/` `?` — search forward / backward
- `n` `N` — next / previous match
- `*` `#` — search word under cursor forward / backward

**Undo / registers:**
- `u` — undo
- `Ctrl-R` — redo
- `"a`-`"z` — named registers

**Macros:**
- `q{reg}` — start recording macro
- `@{reg}` — replay macro
- `@@` — replay last macro

**Marks:**
- `m{a-z}` — set mark
- `'{a-z}` — jump to mark
