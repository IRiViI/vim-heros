# Rhythm Mode — "Vim Drop" Game Mode

## Context

New game mode where vim commands fall down a single lane, queue up, and the player types them in order — watching each command execute live on a code buffer. Combines rhythm-game feel with real vim editing practice. Two sub-modes: Guided (see commands) and Blind (see hints, guess the command).

## Core Concept

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

## Two Sub-Modes

### Mode A — Guided Drop
You see the vim commands falling and queued. Type them as they arrive. Teaches command recognition and muscle memory.

### Mode B — Blind Drop
Commands are **hidden**. Instead, the code buffer shows highlights + gutter descriptions of what needs to happen (e.g., highlight `old_price` with hint "change to `new_price`"). You must figure out the correct vim command. Tests true vim fluency.

## Scoring System

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

One mistake at hit 50? Back to multiplier 0. Next hit earns nothing. Accuracy is king.

**Three scoring layers:**
| Layer | Mechanic | Effect |
|-------|----------|--------|
| Accuracy | Multiplier resets to 0 on wrong key | Dominates scoring. Streaks are everything. |
| Time decay | Multiplier -1 every `timeout` seconds of inactivity | Keeps pressure on per difficulty level |
| Time tax | Final score -= total_seconds × coefficient | Rewards overall speed |

## Difficulty Levels

| Level | Name | Timeout | Vibe |
|-------|------|---------|------|
| 1 | **Nano User** | 10s | Learning, hunting for keys |
| 2 | **:wq Survivor** | 5s | Can exit vim, still thinking |
| 3 | **Keyboard Warrior** | 2s | Knows the commands, building speed |
| 4 | **10x Engineer** | 0.5s | Muscle memory, rapid fire |
| 5 | **Uses Arch btw** | 0.2s | Meme-tier. Barely human reaction time. |

## Architecture

### Top-Level Mode Dispatch

```rust
enum AppMode {
    StoryMode(App),        // existing game, untouched
    RhythmMode(RhythmApp), // new
}
```

`main.rs` holds `AppMode` and dispatches `tick()` + `render()`. Existing `App` stays as-is — pure additive change.

### New Module Structure

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

### Key Data Structures

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

### Rendering — Split Layout

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

In **Blind mode**, the command lane shows `???` instead of command text, and the code buffer shows highlighted regions with gutter hints like `◀ change to 'new_price'`.

### Reuse from Existing Code

- **`vim::Buffer`** + **`vim::Cursor`** — the code buffer and cursor for live command execution
- **`vim::command::CommandParser`** — parse player input into Actions for execution on the buffer
- **`vim::motions`** — execute parsed motions/commands on the buffer
- **`content::loader`** pattern — `include_dir!` for loading rhythm song TOML files
- **`game::scoring`** pattern — structural reference for the new RhythmScoring
- **`ui::game_view`** patterns — ratatui layout, HUD rendering, highlight styling

### Song Content Format

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
# anchor in code where cursor should be when this executes
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

## Implementation Phases

### Phase R1: Skeleton + Static Rendering
- Create `src/rhythm/` module with Note, RhythmApp structs
- `src/ui/rhythm_view.rs`: split layout — code buffer left, command lane right
- Render hardcoded notes in the lane + code in the buffer
- Wire into `main.rs` with `--rhythm` flag
- **Exit**: See the split layout with code and command lane. No input.

### Phase R2: Queue + Falling Animation
- `RhythmEngine` with Instant-based timing for falling notes
- Notes fall down the lane, queue at strike zone
- Active note highlighted differently from queued notes
- Countdown before start
- **Exit**: Notes fall and queue up. No input handling yet.

### Phase R3: Input + Live Execution (Guided Mode)
- Keystroke matching against active note (multi-key sequence support)
- On correct: execute command on buffer via existing vim engine, advance queue
- On wrong: ignore input, reset multiplier to 0
- 0-indexed scoring with multiplier display
- Difficulty selection (timeout for multiplier decay)
- **Exit**: Full guided mode playable. Type commands, watch code change.

### Phase R4: Blind Mode + Hints
- `RhythmSubMode::Blind` — hide command text in lane (show `???`)
- Show highlights + gutter descriptions in code buffer
- Player types what they think the command is
- Match against expected command
- **Exit**: Both Guided and Blind modes playable.

### Phase R5: Song Loading + Results
- TOML song format + `rhythm_loader.rs` using `include_dir!`
- Song complete screen: score, accuracy, streak, time, difficulty
- 5+ handcrafted songs (basic motions, word motions, editing)
- Song selection menu
- **Exit**: Full play-through from TOML songs, results, retry.

### Phase R6: Integration + Polish
- Mode selection at startup (Story Mode / Rhythm Mode)
- Guided/Blind toggle in rhythm mode menu
- Difficulty selector with names (Nano User through Uses Arch btw)
- Visual polish: color coding, streak effects
- Shared progression with story mode if credit shop exists
