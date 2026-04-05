use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Color;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::content::assembler;
use crate::content::loader;
use crate::game::energy::Energy;
use crate::game::engine::{Engine, GameState};
use crate::game::pathfinder::{self, PathResult};
use crate::game::scoring::Scoring;
use crate::game::task::{self, Task, TaskKind, TaskState};
use crate::game::viewport::Viewport;
use crate::game::worlds::{self, VimSkill};
use crate::vim::buffer::Buffer;
use crate::vim::command::{self, Action, CommandParser, ParseResult};
use crate::vim::cursor::Cursor;
use crate::vim::mode::Mode;
use crate::vim::register::RegisterFile;
use crate::vim::search::{self, SearchDirection, SearchState};
use crate::vim::undo::UndoHistory;

const SAMPLE_CODE: &str = r#"use std::collections::HashMap;

fn main() {
    let greeting = "Hello, Vim Heroes!";
    println!("{}", greeting);

    // Calculate some numbers
    let numbers = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let total: i32 = numbers.iter().sum();
    let average = total as f64 / numbers.len() as f64;
    println!("Total: {}, Average: {:.1}", total, average);

    // Fizzbuzz
    for i in 1..=20 {
        let result = match (i % 3, i % 5) {
            (0, 0) => "FizzBuzz".to_string(),
            (0, _) => "Fizz".to_string(),
            (_, 0) => "Buzz".to_string(),
            _ => i.to_string(),
        };
        println!("{}: {}", i, result);
    }

    // Build a frequency map
    let words = vec!["hello", "world", "hello", "rust", "world", "hello"];
    let mut freq: HashMap<&str, usize> = HashMap::new();
    for word in &words {
        *freq.entry(word).or_insert(0) += 1;
    }
    println!("Word frequencies: {:?}", freq);

    // Fibonacci sequence
    let fibs = fibonacci(10);
    println!("Fibonacci: {:?}", fibs);

    // String manipulation
    let sentence = "the quick brown fox jumps over the lazy dog";
    let title_case: String = sentence
        .split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    println!("Title case: {}", title_case);

    // Pattern matching with enums
    let shapes = vec![
        Shape::Circle(5.0),
        Shape::Rectangle(4.0, 6.0),
        Shape::Triangle(3.0, 4.0, 5.0),
    ];
    for shape in &shapes {
        println!("Area: {:.2}", shape.area());
    }
}

fn fibonacci(n: usize) -> Vec<u64> {
    let mut fibs = vec![0, 1];
    for i in 2..n {
        let next = fibs[i - 1] + fibs[i - 2];
        fibs.push(next);
    }
    fibs
}

enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
    Triangle(f64, f64, f64),
}

impl Shape {
    fn area(&self) -> f64 {
        match self {
            Shape::Circle(r) => std::f64::consts::PI * r * r,
            Shape::Rectangle(w, h) => w * h,
            Shape::Triangle(a, b, c) => {
                let s = (a + b + c) / 2.0;
                (s * (s - a) * (s - b) * (s - c)).sqrt()
            }
        }
    }
}"#;

const DEFAULT_SCROLL_SPEED_MS: u64 = 2000;
const SEGMENTS_PER_LEVEL: usize = 4;

/// Manhattan-ish distance between two cursor positions (for error detection).
fn cursor_distance(l1: usize, c1: usize, l2: usize, c2: usize) -> usize {
    let line_dist = if l1 > l2 { l1 - l2 } else { l2 - l1 };
    let col_dist = if c1 > c2 { c1 - c2 } else { c2 - c1 };
    line_dist + col_dist
}

/// Level metadata.
pub struct LevelInfo {
    pub world: usize,
    pub level: usize,
    pub name: String,
    pub zone: String,
    pub language: String,
    pub scroll_speed_ms: u64,
    /// World 1 difficulty (1-5). 0 means not World 1 (use classic mechanics).
    pub w1_difficulty: u8,
}

impl LevelInfo {
    pub fn display_id(&self) -> String {
        format!("{}-{}", self.world, self.level)
    }

    /// Whether this level uses World 1 mechanics (motion-count energy, no countdown).
    pub fn is_world1(&self) -> bool {
        self.world == 1
    }

    /// Whether this level uses World 2 mechanics (split-screen, timer energy, insert mode).
    pub fn is_basic_edit(&self) -> bool {
        self.world == 2
    }
}

/// All available levels — 14 worlds × 5 levels each.
/// Level X-5 in each world is a boss level (1.5× scroll speed).
fn level_list() -> Vec<LevelInfo> {
    vec![
        // ── World 1: Motion ──
        // Scroll speed comes from difficulty, default = difficulty 2 (:wq Survivor = 2000ms)
        LevelInfo { world: 1, level: 1, name: "Basic Movement".into(), zone: "nav".into(), language: "python".into(), scroll_speed_ms: 2000, w1_difficulty: 2 },
        LevelInfo { world: 1, level: 2, name: "Word Jumps & Counts".into(), zone: "nav".into(), language: "python".into(), scroll_speed_ms: 2000, w1_difficulty: 2 },
        LevelInfo { world: 1, level: 3, name: "Line Targeting".into(), zone: "nav".into(), language: "python".into(), scroll_speed_ms: 2000, w1_difficulty: 2 },
        LevelInfo { world: 1, level: 4, name: "Restricted Zones".into(), zone: "nav".into(), language: "python".into(), scroll_speed_ms: 2000, w1_difficulty: 2 },
        LevelInfo { world: 1, level: 5, name: "Perfect Motions".into(), zone: "nav".into(), language: "python".into(), scroll_speed_ms: 2000, w1_difficulty: 2 },

        // ── World 2: Basic Edit (i/a/I/A/o/O) ──
        LevelInfo { world: 2, level: 1, name: "Insert Basics".into(), zone: "basic_edit".into(), language: "python".into(), scroll_speed_ms: 0, w1_difficulty: 2 },
        LevelInfo { world: 2, level: 2, name: "Line Edges".into(), zone: "basic_edit".into(), language: "python".into(), scroll_speed_ms: 0, w1_difficulty: 2 },
        LevelInfo { world: 2, level: 3, name: "New Lines".into(), zone: "basic_edit".into(), language: "python".into(), scroll_speed_ms: 0, w1_difficulty: 2 },
        LevelInfo { world: 2, level: 4, name: "Restricted Entry".into(), zone: "basic_edit".into(), language: "python".into(), scroll_speed_ms: 0, w1_difficulty: 2 },
        LevelInfo { world: 2, level: 5, name: "Perfect Entry".into(), zone: "basic_edit".into(), language: "python".into(), scroll_speed_ms: 0, w1_difficulty: 2 },

        // ── World 3: Word Surfer (w/b/e) ──
        LevelInfo { world: 3, level: 1, name: "Word Jumps".into(), zone: "starter".into(), language: "python".into(), scroll_speed_ms: 2400, w1_difficulty: 0 },
        LevelInfo { world: 3, level: 2, name: "Big Words".into(), zone: "starter".into(), language: "typescript".into(), scroll_speed_ms: 2400, w1_difficulty: 0 },
        LevelInfo { world: 3, level: 3, name: "Word Counts".into(), zone: "starter".into(), language: "python".into(), scroll_speed_ms: 2300, w1_difficulty: 0 },
        LevelInfo { world: 3, level: 4, name: "Word Mix".into(), zone: "starter".into(), language: "typescript".into(), scroll_speed_ms: 2300, w1_difficulty: 0 },
        LevelInfo { world: 3, level: 5, name: "The Marathon".into(), zone: "starter".into(), language: "python".into(), scroll_speed_ms: 1600, w1_difficulty: 0 },

        // ── World 4: Line Rider (0/^/$/f/t) ──
        LevelInfo { world: 4, level: 1, name: "Line Ends".into(), zone: "starter".into(), language: "python".into(), scroll_speed_ms: 2300, w1_difficulty: 0 },
        LevelInfo { world: 4, level: 2, name: "Find Char".into(), zone: "starter".into(), language: "typescript".into(), scroll_speed_ms: 2300, w1_difficulty: 0 },
        LevelInfo { world: 4, level: 3, name: "Repeat Find".into(), zone: "starter".into(), language: "python".into(), scroll_speed_ms: 2200, w1_difficulty: 0 },
        LevelInfo { world: 4, level: 4, name: "Line Precision".into(), zone: "starter".into(), language: "typescript".into(), scroll_speed_ms: 2200, w1_difficulty: 0 },
        LevelInfo { world: 4, level: 5, name: "The Sniper Range".into(), zone: "starter".into(), language: "python".into(), scroll_speed_ms: 1533, w1_difficulty: 0 },

        // ── World 5: The Writer (R/r — advanced insert) ──
        LevelInfo { world: 5, level: 1, name: "Replace Chars".into(), zone: "starter".into(), language: "python".into(), scroll_speed_ms: 2200, w1_difficulty: 0 },
        LevelInfo { world: 5, level: 2, name: "Replace Mode".into(), zone: "starter".into(), language: "typescript".into(), scroll_speed_ms: 2200, w1_difficulty: 0 },
        LevelInfo { world: 5, level: 3, name: "Mixed Replace".into(), zone: "starter".into(), language: "python".into(), scroll_speed_ms: 2100, w1_difficulty: 0 },
        LevelInfo { world: 5, level: 4, name: "Replace + Insert".into(), zone: "starter".into(), language: "typescript".into(), scroll_speed_ms: 2100, w1_difficulty: 0 },
        LevelInfo { world: 5, level: 5, name: "The Overwriter".into(), zone: "starter".into(), language: "python".into(), scroll_speed_ms: 1467, w1_difficulty: 0 },

        // ── World 6: The Destroyer (x/dd/D) ──
        LevelInfo { world: 6, level: 1, name: "Delete Chars".into(), zone: "starter".into(), language: "python".into(), scroll_speed_ms: 2100, w1_difficulty: 0 },
        LevelInfo { world: 6, level: 2, name: "Delete Lines".into(), zone: "starter".into(), language: "typescript".into(), scroll_speed_ms: 2100, w1_difficulty: 0 },
        LevelInfo { world: 6, level: 3, name: "Delete to End".into(), zone: "starter".into(), language: "python".into(), scroll_speed_ms: 2000, w1_difficulty: 0 },
        LevelInfo { world: 6, level: 4, name: "Mixed Deletions".into(), zone: "starter".into(), language: "typescript".into(), scroll_speed_ms: 2000, w1_difficulty: 0 },
        LevelInfo { world: 6, level: 5, name: "The Cleanup".into(), zone: "starter".into(), language: "python".into(), scroll_speed_ms: 1400, w1_difficulty: 0 },

        // ── World 7: Verb + Noun (d/c{motion}, ., >>/<<) ──
        LevelInfo { world: 7, level: 1, name: "Delete Motions".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 2000, w1_difficulty: 0 },
        LevelInfo { world: 7, level: 2, name: "Change Motions".into(), zone: "junior".into(), language: "typescript".into(), scroll_speed_ms: 2000, w1_difficulty: 0 },
        LevelInfo { world: 7, level: 3, name: "Dot Repeat".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1900, w1_difficulty: 0 },
        LevelInfo { world: 7, level: 4, name: "Indent Power".into(), zone: "junior".into(), language: "typescript".into(), scroll_speed_ms: 1900, w1_difficulty: 0 },
        LevelInfo { world: 7, level: 5, name: "The Refactor".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1333, w1_difficulty: 0 },

        // ── World 8: Copy Ninja (yy/yw/p/P) ──
        LevelInfo { world: 8, level: 1, name: "Yank Lines".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1900, w1_difficulty: 0 },
        LevelInfo { world: 8, level: 2, name: "Yank Words".into(), zone: "junior".into(), language: "typescript".into(), scroll_speed_ms: 1900, w1_difficulty: 0 },
        LevelInfo { world: 8, level: 3, name: "Paste Before".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1800, w1_difficulty: 0 },
        LevelInfo { world: 8, level: 4, name: "Cut & Move".into(), zone: "junior".into(), language: "typescript".into(), scroll_speed_ms: 1800, w1_difficulty: 0 },
        LevelInfo { world: 8, level: 5, name: "The Rearrangement".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1267, w1_difficulty: 0 },

        // ── World 9: The Selector (v/V + operators) ──
        LevelInfo { world: 9, level: 1, name: "Visual Chars".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1800, w1_difficulty: 0 },
        LevelInfo { world: 9, level: 2, name: "Visual Lines".into(), zone: "junior".into(), language: "typescript".into(), scroll_speed_ms: 1800, w1_difficulty: 0 },
        LevelInfo { world: 9, level: 3, name: "Visual + Counts".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1700, w1_difficulty: 0 },
        LevelInfo { world: 9, level: 4, name: "Mixed Visual".into(), zone: "junior".into(), language: "typescript".into(), scroll_speed_ms: 1700, w1_difficulty: 0 },
        LevelInfo { world: 9, level: 5, name: "The Bulk Edit".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1200, w1_difficulty: 0 },

        // ── World 10: Text Object Surgeon (iw/aw/i"/ci(/da{) ──
        LevelInfo { world: 10, level: 1, name: "Inner Word".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1700, w1_difficulty: 0 },
        LevelInfo { world: 10, level: 2, name: "Inside Quotes".into(), zone: "junior".into(), language: "typescript".into(), scroll_speed_ms: 1700, w1_difficulty: 0 },
        LevelInfo { world: 10, level: 3, name: "Inside Brackets".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1600, w1_difficulty: 0 },
        LevelInfo { world: 10, level: 4, name: "Operator + Object".into(), zone: "junior".into(), language: "typescript".into(), scroll_speed_ms: 1600, w1_difficulty: 0 },
        LevelInfo { world: 10, level: 5, name: "The Nested Beast".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1133, w1_difficulty: 0 },

        // ── World 11: Code Navigator ({/}/%/marks) ──
        LevelInfo { world: 11, level: 1, name: "Paragraphs".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1600, w1_difficulty: 0 },
        LevelInfo { world: 11, level: 2, name: "Bracket Match".into(), zone: "junior".into(), language: "typescript".into(), scroll_speed_ms: 1600, w1_difficulty: 0 },
        LevelInfo { world: 11, level: 3, name: "Marks".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1500, w1_difficulty: 0 },
        LevelInfo { world: 11, level: 4, name: "Navigate All".into(), zone: "junior".into(), language: "typescript".into(), scroll_speed_ms: 1500, w1_difficulty: 0 },
        LevelInfo { world: 11, level: 5, name: "The Labyrinth".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1067, w1_difficulty: 0 },

        // ── World 12: Search & Destroy (/?/n/N/*/#) ──
        LevelInfo { world: 12, level: 1, name: "Search Forward".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1500, w1_difficulty: 0 },
        LevelInfo { world: 12, level: 2, name: "Search Backward".into(), zone: "junior".into(), language: "typescript".into(), scroll_speed_ms: 1500, w1_difficulty: 0 },
        LevelInfo { world: 12, level: 3, name: "Word Search".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1400, w1_difficulty: 0 },
        LevelInfo { world: 12, level: 4, name: "Search + Operate".into(), zone: "junior".into(), language: "typescript".into(), scroll_speed_ms: 1400, w1_difficulty: 0 },
        LevelInfo { world: 12, level: 5, name: "The Bug Hunt".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1000, w1_difficulty: 0 },

        // ── World 13: Time Traveler (u/Ctrl-R/registers) ──
        LevelInfo { world: 13, level: 1, name: "Undo / Redo".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1400, w1_difficulty: 0 },
        LevelInfo { world: 13, level: 2, name: "Named Registers".into(), zone: "junior".into(), language: "typescript".into(), scroll_speed_ms: 1400, w1_difficulty: 0 },
        LevelInfo { world: 13, level: 3, name: "Undo Branches".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1300, w1_difficulty: 0 },
        LevelInfo { world: 13, level: 4, name: "Time Recovery".into(), zone: "junior".into(), language: "typescript".into(), scroll_speed_ms: 1300, w1_difficulty: 0 },
        LevelInfo { world: 13, level: 5, name: "The Time Paradox".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 933, w1_difficulty: 0 },

        // ── World 14: Macro Wizard (q/@ /@@) ──
        LevelInfo { world: 14, level: 1, name: "Record & Play".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1300, w1_difficulty: 0 },
        LevelInfo { world: 14, level: 2, name: "Replay Last".into(), zone: "junior".into(), language: "typescript".into(), scroll_speed_ms: 1300, w1_difficulty: 0 },
        LevelInfo { world: 14, level: 3, name: "Counted Replay".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1200, w1_difficulty: 0 },
        LevelInfo { world: 14, level: 4, name: "Macro Chains".into(), zone: "junior".into(), language: "typescript".into(), scroll_speed_ms: 1200, w1_difficulty: 0 },
        LevelInfo { world: 14, level: 5, name: "The Assembly Line".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 867, w1_difficulty: 0 },

        // ── World 15: The Grandmaster (Grand Finale) ──
        LevelInfo { world: 15, level: 1, name: "Fix Bubble Sort".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1200, w1_difficulty: 0 },
        LevelInfo { world: 15, level: 2, name: "Fix Binary Search".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1200, w1_difficulty: 0 },
        LevelInfo { world: 15, level: 3, name: "Fix Quicksort".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1100, w1_difficulty: 0 },
        LevelInfo { world: 15, level: 4, name: "Fix Merge Sort".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 1100, w1_difficulty: 0 },
        LevelInfo { world: 15, level: 5, name: "The Final Boss".into(), zone: "junior".into(), language: "python".into(), scroll_speed_ms: 800, w1_difficulty: 0 },
    ]
}

fn default_level() -> LevelInfo {
    level_list().into_iter().next().unwrap()
}

/// A repeatable edit for dot (.) repeat.
#[derive(Debug, Clone)]
struct RepeatableEdit {
    /// The initial action that triggered the edit.
    action: Action,
    /// How many times to repeat the initial action.
    count: usize,
    /// Characters typed during insert mode (for change/insert actions).
    insert_text: Vec<char>,
}

/// Why the game ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameOverReason {
    /// Timer countdown reached 0.
    TimerExpired,
    /// Player scrolled off the viewport (World 1).
    ScrolledOff,
    /// Too many errors / non-optimal motions (World 1).
    ErrorsExceeded,
    /// Not game over (or level complete).
    None,
}

pub struct App {
    pub buffer: Buffer,
    pub cursor: Cursor,
    pub mode: Mode,
    pub running: bool,
    pub viewport: Viewport,
    pub engine: Engine,
    pub scoring: Scoring,
    pub energy: Energy,
    pub game_over_reason: GameOverReason,
    pub tasks: Vec<Task>,
    pub level: LevelInfo,
    parser: CommandParser,
    recently_seen: Vec<String>,
    registers: RegisterFile,
    undo: UndoHistory,
    pub search: SearchState,
    /// Anchor cursor for visual mode selection (where v/V was pressed).
    pub visual_anchor: Cursor,
    /// Last repeatable edit for dot (.) repeat.
    last_edit: Option<RepeatableEdit>,
    /// Buffer for collecting insert-mode keystrokes (for dot repeat).
    insert_chars: Vec<char>,
    /// Macro register storage: maps register char -> recorded keystrokes.
    macro_regs: HashMap<char, Vec<KeyEvent>>,
    /// Current macro recording buffer (None = not recording).
    macro_recording: Option<(char, Vec<KeyEvent>)>,
    /// Last played macro register (for @@ repeat).
    last_macro_reg: Option<char>,
    /// Command-line input buffer (for : commands). None = not active.
    pub cmdline: Option<String>,
    level_index: usize,
    /// Keystrokes since last task completion (for per-task optimal tracking).
    task_keystrokes: usize,
    /// When the last task was completed (for catch-up scroll delay).
    last_task_completion: Option<Instant>,
    /// Flash popup for task completion quality (text, when, color).
    pub completion_flash: Option<(String, Instant, Color)>,
    /// Practice mode: show expected commands, disable timer.
    pub practice_mode: bool,
    /// Index of the task we last auto-scrolled the camera for.
    /// Once we've scrolled to show a new task, the player can freely move around.
    camera_task_index: Option<usize>,
    /// Skills unlocked for the current world (cumulative from all prior worlds).
    unlocked_skills: HashSet<VimSkill>,
    /// Flash popup for locked key attempts (text, when, color).
    pub locked_key_flash: Option<(String, Instant, Color)>,
    /// Keys already shown as locked this level (only flash once per key per level).
    locked_keys_shown: HashSet<&'static str>,

    // ── World 1 specific fields ──
    /// Pre-calculated BFS optimal paths for each target (World 1).
    pub w1_paths: Vec<PathResult>,
    /// Player motion names since last target (for death hints).
    pub w1_player_motions: Vec<String>,
    /// Index of the current target in w1_paths (0 = first target).
    pub w1_current_target: usize,
    /// Death hint text (set on game over).
    pub death_hint: Option<String>,
    /// World 2: target buffer (the "answer key"). None for non-W2 worlds.
    pub target_buffer: Option<Buffer>,
    /// World 2: viewport for the target buffer.
    pub target_viewport: Viewport,
}

impl App {
    pub fn new(viewport_height: usize) -> Self {
        let level = default_level();
        let (buffer, tasks, seen, w2_target) = Self::load_level(&level, &[]);
        let tasks_total = tasks.len();

        let is_w1 = level.is_world1();
        let is_w2 = level.is_basic_edit();

        // World 1: pre-calculate optimal paths
        let w1_paths = if is_w1 {
            let targets: Vec<(usize, usize)> = tasks.iter()
                .map(|t| (t.target_line, t.target_col))
                .collect();
            pathfinder::calculate_level_paths(&buffer, &targets, level.level)
        } else {
            Vec::new()
        };

        // Engine and energy setup per world type
        let difficulty = worlds::w1_difficulty(level.w1_difficulty);
        let engine = if is_w1 || is_w2 {
            Engine::new_waiting(if is_w1 { difficulty.scroll_ms } else { 0 })
        } else {
            Engine::new(level.scroll_speed_ms)
        };
        let energy = if is_w1 {
            let mut e = Energy::new_motion_count(difficulty.max_errors);
            if let Some(first_path) = w1_paths.first() {
                e.set_budget(first_path.optimal_motions + 3);
            }
            e
        } else if is_w2 {
            Energy::new(30.0) // 30 seconds timer for World 2
        } else {
            Energy::default_new()
        };

        // Skill gating per world type
        let unlocked = if is_w1 {
            worlds::w1_allowed_skills(level.level)
        } else {
            // World 2+: use cumulative skills (InsertMode unlocked at W2)
            worlds::skills_for_world(level.world)
        };

        Self {
            buffer,
            cursor: Cursor::new(0, 0),
            mode: Mode::Normal,
            running: true,
            viewport: Viewport::new(viewport_height),
            engine,
            scoring: Scoring::new(tasks_total),
            energy,
            game_over_reason: GameOverReason::None,
            tasks,
            level,
            parser: CommandParser::new(),
            recently_seen: seen,
            registers: RegisterFile::new(),
            undo: UndoHistory::new(),
            search: SearchState::new(),
            visual_anchor: Cursor::new(0, 0),
            last_edit: None,
            insert_chars: Vec::new(),
            macro_regs: HashMap::new(),
            macro_recording: None,
            last_macro_reg: None,
            cmdline: None,
            level_index: 0,
            task_keystrokes: 0,
            last_task_completion: None,
            completion_flash: None,
            practice_mode: false,
            camera_task_index: None,
            unlocked_skills: unlocked,
            locked_key_flash: None,
            locked_keys_shown: HashSet::new(),
            w1_paths,
            w1_player_motions: Vec::new(),
            w1_current_target: 0,
            death_hint: None,
            target_buffer: w2_target,
            target_viewport: Viewport::new(viewport_height),
        }
    }

    /// Load a level from content segments. Returns (buffer, tasks, segment IDs used).
    fn load_level(
        level: &LevelInfo,
        recently_seen: &[String],
    ) -> (Buffer, Vec<Task>, Vec<String>, Option<Buffer>) {
        // World 2 (Basic Edit): use assemble_w2 for split-screen mode
        if level.is_basic_edit() {
            return Self::load_level_w2(level);
        }

        let pool = loader::load_segments(&level.language, &level.zone);

        if pool.is_empty() {
            // Fallback to hardcoded SAMPLE_CODE
            let buffer = Buffer::from_str(SAMPLE_CODE);
            let tasks = task::hardcoded_tasks(&buffer);
            return (buffer, tasks, Vec::new(), None);
        }

        // Load tutorial intro segment: always for World 1, only level X-1 for others
        let level_id = format!("{}-{}", level.world, level.level);
        let intro = if level.world == 1 || level.level == 1 {
            loader::load_intro_segment(&level.language, &level_id)
        } else {
            None
        };

        let regular_count = if intro.is_some() {
            SEGMENTS_PER_LEVEL.saturating_sub(1)
        } else {
            SEGMENTS_PER_LEVEL
        };

        let selected = assembler::select_segments(&pool, regular_count, recently_seen);

        // Build final segment list: intro first (if any), then regular segments
        let mut all_segments: Vec<&_> = Vec::new();
        if let Some(ref intro_seg) = intro {
            all_segments.push(intro_seg);
        }
        all_segments.extend(selected.iter());

        let ids: Vec<String> = all_segments.iter().map(|s| s.meta.id.clone()).collect();
        let ctx = assembler::LevelContext {
            world: level.world,
            level: level.level,
            name: level.name.clone(),
            language: level.language.clone(),
        };
        let assembled = assembler::assemble(&all_segments, Some(&ctx));
        (assembled.buffer, assembled.tasks, ids, None)
    }

    /// Load a World 2 (Basic Edit) level: returns player buffer, tasks, IDs, and target buffer.
    fn load_level_w2(level: &LevelInfo) -> (Buffer, Vec<Task>, Vec<String>, Option<Buffer>) {
        let level_id = format!("{}-{}", level.world, level.level);

        // Try to load the intro segment for this level (which contains the removals)
        if let Some(segment) = loader::load_intro_segment(&level.language, &level_id) {
            let ctx = assembler::LevelContext {
                world: level.world,
                level: level.level,
                name: level.name.clone(),
                language: level.language.clone(),
            };
            let assembled = assembler::assemble_w2(&segment, Some(&ctx));
            let ids = vec![segment.meta.id.clone()];
            return (
                assembled.player_buffer,
                assembled.tasks,
                ids,
                Some(assembled.target_buffer),
            );
        }

        // Fallback: try loading from the basic_edit zone
        let pool = loader::load_segments(&level.language, &level.zone);
        if let Some(segment) = pool.first() {
            let ctx = assembler::LevelContext {
                world: level.world,
                level: level.level,
                name: level.name.clone(),
                language: level.language.clone(),
            };
            let assembled = assembler::assemble_w2(segment, Some(&ctx));
            let ids = vec![segment.meta.id.clone()];
            return (
                assembled.player_buffer,
                assembled.tasks,
                ids,
                Some(assembled.target_buffer),
            );
        }

        // Final fallback
        let buffer = Buffer::from_str(SAMPLE_CODE);
        let tasks = task::hardcoded_tasks(&buffer);
        (buffer, tasks, Vec::new(), None)
    }

    /// Update viewport height when terminal is resized.
    pub fn update_viewport_height(&mut self, terminal_height: usize) {
        // terminal_height minus 2 (borders) minus 1 (HUD) minus 1 (status bar)
        self.viewport.height = terminal_height.saturating_sub(4);
    }

    /// Main tick: poll for input, handle scroll, check game over.
    /// Returns true if a frame should be rendered.
    pub fn tick(&mut self) -> bool {
        match self.engine.state {
            GameState::Countdown => self.tick_countdown(),
            GameState::WaitingForInput => self.tick_waiting_for_input(),
            GameState::Playing => self.tick_playing(),
            GameState::GameOver | GameState::LevelComplete => self.tick_game_over(),
        }
    }

    fn tick_countdown(&mut self) -> bool {
        // Check if countdown is done
        if self.engine.check_countdown() {
            if !self.practice_mode {
                self.energy.start(); // Start the timer when countdown finishes
            }
            return true;
        }

        // Consume input during countdown (allow quit, but don't penalize)
        if event::poll(Duration::from_millis(50)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && key.code == KeyCode::Char('c')
                {
                    self.running = false;
                    return true;
                }
                if key.code == KeyCode::Char('q') {
                    self.running = false;
                    return true;
                }
            }
        }

        // Re-render each tick so the countdown number updates
        true
    }

    /// World 1 start condition: waiting for the player's first keystroke.
    fn tick_waiting_for_input(&mut self) -> bool {
        if event::poll(Duration::from_millis(50)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && key.code == KeyCode::Char('c')
                {
                    self.running = false;
                    return true;
                }
                if key.code == KeyCode::Char('q') {
                    self.running = false;
                    return true;
                }
                // First keystroke: start the game and process this key
                self.engine.start_on_input();
                if !self.practice_mode {
                    self.energy.start();
                }
                // Process this key as a normal playing key
                return self.handle_key(key);
            }
        }
        // Always re-render so the "press any key" message shows
        true
    }

    fn tick_playing(&mut self) -> bool {
        let mut needs_render = false;
        let is_w1 = self.level.is_world1();
        let is_w2 = self.level.is_basic_edit();

        // Poll for input
        if event::poll(Duration::from_millis(50)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                needs_render = self.handle_key(key);
            }
        }

        // World 1: auto-scroll the viewport at engine speed
        if is_w1 && self.engine.should_scroll() {
            let max_line = self.buffer.line_count().saturating_sub(1);
            if self.viewport.top_line < max_line {
                self.viewport.scroll_down();
                self.engine.record_scroll();
                needs_render = true;

                // Check if cursor scrolled off the top of the viewport (game over)
                if self.cursor.line < self.viewport.top_line {
                    if !self.practice_mode {
                        self.engine.state = GameState::GameOver;
                        self.game_over_reason = GameOverReason::ScrolledOff;
                        // Generate death hint
                        if self.w1_current_target < self.w1_paths.len() {
                            self.death_hint = Some(pathfinder::generate_death_hint(
                                &self.w1_player_motions,
                                &self.w1_paths[self.w1_current_target],
                            ));
                        }
                        return true;
                    }
                }
            }
        }

        // World 1: update catching_up mode (4x scroll when target not in view)
        if is_w1 {
            if let Some(task) = self.tasks.iter().find(|t| t.is_completable()) {
                let target_in_view = self.viewport.contains(task.target_line);
                self.engine.catching_up = !target_in_view;
            }
        }

        // Camera behavior
        let max_line = self.buffer.line_count().saturating_sub(1);
        let current_task_index = self.tasks.iter().position(|t| t.is_completable());

        if is_w2 {
            // World 2: free navigation, camera follows cursor
            self.viewport.ensure_visible(self.cursor.line, 2, max_line);
            // Also scroll target viewport to show the active task's target line
            if let Some(ref target_buf) = self.target_buffer {
                if let Some(task_idx) = current_task_index {
                    let target_max = target_buf.line_count().saturating_sub(1);
                    self.target_viewport.ensure_visible(
                        self.tasks[task_idx].target_line, 2, target_max
                    );
                }
            }
            needs_render = true;
        } else if is_w1 {
            // World 1: viewport auto-scrolls. Just ensure cursor stays visible.
            self.viewport.ensure_visible(self.cursor.line, 2, max_line);
            needs_render = true;
        } else {
            // Classic mode: camera follows tasks
            if let Some(task_idx) = current_task_index {
                if self.camera_task_index != Some(task_idx) {
                    self.camera_task_index = Some(task_idx);
                    let next_task_line = self.tasks[task_idx].target_line;
                    let lo = self.cursor.line.min(next_task_line);
                    let hi = self.cursor.line.max(next_task_line);
                    let span = hi - lo;
                    let usable = self.viewport.height.saturating_sub(4);
                    if span <= usable {
                        let mid = lo + span / 2;
                        let half = self.viewport.height / 2;
                        let new_top = mid.saturating_sub(half);
                        let max_top = max_line.saturating_sub(self.viewport.height.saturating_sub(1));
                        self.viewport.top_line = new_top.min(max_top);
                    } else {
                        self.viewport.ensure_visible(next_task_line, 2, max_line);
                    }
                    needs_render = true;
                } else {
                    self.viewport.ensure_visible(self.cursor.line, 2, max_line);
                }
                needs_render = true;
            } else {
                self.viewport.ensure_visible(self.cursor.line, 2, max_line);
            }
        }

        // Activate the next incomplete task (tasks follow TOML order, not position)
        for task in &mut self.tasks {
            if task.is_completable() {
                if task.state == TaskState::Pending {
                    task.mark_active();
                }
                break;
            }
        }

        // Game over: energy depleted (skip in practice mode)
        if !self.practice_mode && self.energy.is_depleted() {
            self.engine.state = GameState::GameOver;
            self.game_over_reason = if is_w1 {
                GameOverReason::ErrorsExceeded
            } else {
                GameOverReason::TimerExpired
            };
            // Generate death hint for World 1
            if is_w1 && self.w1_current_target < self.w1_paths.len() {
                self.death_hint = Some(pathfinder::generate_death_hint(
                    &self.w1_player_motions,
                    &self.w1_paths[self.w1_current_target],
                ));
            }
        }

        // Check level complete: all tasks resolved
        let all_resolved = self.tasks.iter().all(|t| !t.is_completable());
        if all_resolved && !self.tasks.is_empty() {
            self.engine.state = GameState::LevelComplete;
            needs_render = true;
        }

        needs_render
    }

    fn tick_game_over(&mut self) -> bool {
        if !event::poll(Duration::from_millis(50)).unwrap_or(false) {
            return false;
        }

        let event = match event::read() {
            Ok(ev) => ev,
            Err(_) => return false,
        };

        match event {
            Event::Key(key) => {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && key.code == KeyCode::Char('c')
                {
                    self.running = false;
                    return true;
                }
                match key.code {
                    KeyCode::Char('q') => {
                        self.running = false;
                        true
                    }
                    KeyCode::Char('r') => {
                        self.restart();
                        true
                    }
                    KeyCode::Char('n') => {
                        self.next_level();
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn restart(&mut self) {
        let (buffer, tasks, seen, w2_target) = Self::load_level(&self.level, &self.recently_seen);
        self.buffer = buffer;
        self.tasks = tasks;
        self.recently_seen = seen;
        self.target_buffer = w2_target;
        self.cursor = Cursor::new(0, 0);
        self.mode = Mode::Normal;
        self.viewport = Viewport::new(self.viewport.height);

        let is_w1 = self.level.is_world1();
        let is_w2 = self.level.is_basic_edit();

        // World 1: pre-calculate paths
        self.w1_paths = if is_w1 {
            let targets: Vec<(usize, usize)> = self.tasks.iter()
                .map(|t| (t.target_line, t.target_col))
                .collect();
            pathfinder::calculate_level_paths(&self.buffer, &targets, self.level.level)
        } else {
            Vec::new()
        };

        // Reset engine and energy
        if is_w1 || is_w2 {
            let difficulty = worlds::w1_difficulty(self.level.w1_difficulty);
            self.engine = Engine::new_waiting(if is_w1 { difficulty.scroll_ms } else { 0 });
            if is_w1 {
                self.energy = Energy::new_motion_count(difficulty.max_errors);
                if let Some(first_path) = self.w1_paths.first() {
                    self.energy.set_budget(first_path.optimal_motions + 3);
                }
            } else {
                self.energy = Energy::new(30.0); // 30s timer for W2
            }
        } else {
            self.engine.reset();
            self.energy.reset();
        }

        self.scoring.reset(self.tasks.len());
        self.game_over_reason = GameOverReason::None;
        self.parser = CommandParser::new();
        self.registers = RegisterFile::new();
        self.undo = UndoHistory::new();
        self.search = SearchState::new();
        self.visual_anchor = Cursor::new(0, 0);
        self.last_edit = None;
        self.insert_chars.clear();
        self.macro_regs.clear();
        self.macro_recording = None;
        self.last_macro_reg = None;
        self.cmdline = None;
        self.task_keystrokes = 0;
        self.last_task_completion = None;
        self.completion_flash = None;

        self.unlocked_skills = if is_w1 {
            worlds::w1_allowed_skills(self.level.level)
        } else {
            worlds::skills_for_world(self.level.world)
        };

        self.locked_key_flash = None;
        self.locked_keys_shown.clear();
        self.camera_task_index = None;
        self.w1_player_motions.clear();
        self.w1_current_target = 0;
        self.death_hint = None;
        self.target_viewport = Viewport::new(self.viewport.height);
    }

    /// Move cursor (and viewport) by `lines` in a direction.
    fn scroll_cursor(&mut self, lines: usize, down: bool) {
        let max_line = self.buffer.line_count().saturating_sub(1);
        if down {
            self.cursor.line = (self.cursor.line + lines).min(max_line);
        } else {
            self.cursor.line = self.cursor.line.saturating_sub(lines);
        }
        self.cursor.clamp(&self.buffer, false);
        self.check_task_completion();
    }

    /// Return the target line of the next incomplete task (in task order).
    fn next_incomplete_task_line(&self) -> Option<usize> {
        self.tasks.iter().find(|t| t.is_completable()).map(|t| t.target_line)
    }

    fn next_level(&mut self) {
        let levels = level_list();
        self.level_index = (self.level_index + 1) % levels.len();
        self.level = levels.into_iter().nth(self.level_index).unwrap();
        self.recently_seen.clear();
        self.restart();
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.running = false;
            return true;
        }

        // Command-line mode intercepts all keys
        if self.cmdline.is_some() {
            return self.handle_cmdline_input(key);
        }

        // Search input mode intercepts all keys
        if self.search.active {
            return self.handle_search_input(key);
        }

        // Record keystrokes for macro (except the q that stops recording)
        let is_macro_stop = key.code == KeyCode::Char('q')
            && self.mode.is_normal()
            && self.macro_recording.is_some();
        if self.macro_recording.is_some() && !is_macro_stop {
            if let Some((_, ref mut keys)) = self.macro_recording {
                keys.push(key);
            }
        }

        match self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::Insert => self.handle_insert_key(key),
            Mode::Replace => self.handle_replace_key(key),
            Mode::Visual | Mode::VisualLine => self.handle_visual_key(key),
        }
    }

    fn handle_search_input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Enter => {
                if self.search.commit_input() {
                    // Execute the search
                    self.execute_search(self.search.direction);
                }
                true
            }
            KeyCode::Esc => {
                self.search.cancel_input();
                true
            }
            KeyCode::Backspace => {
                if self.search.input_buf.is_empty() {
                    self.search.cancel_input();
                } else {
                    self.search.pop_char();
                }
                true
            }
            KeyCode::Char(ch) => {
                self.search.push_char(ch);
                true
            }
            _ => false,
        }
    }

    fn handle_cmdline_input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Enter => {
                let cmd = self.cmdline.take().unwrap_or_default();
                self.execute_cmdline(&cmd);
                true
            }
            KeyCode::Esc => {
                self.cmdline = None;
                true
            }
            KeyCode::Backspace => {
                if let Some(ref mut buf) = self.cmdline {
                    if buf.is_empty() {
                        self.cmdline = None;
                    } else {
                        buf.pop();
                    }
                }
                true
            }
            KeyCode::Char(ch) => {
                if let Some(ref mut buf) = self.cmdline {
                    buf.push(ch);
                }
                true
            }
            _ => false,
        }
    }

    fn execute_cmdline(&mut self, cmd: &str) {
        let cmd = cmd.trim();
        match cmd {
            "q" => {
                // Quit current level — for now, quit the game
                self.running = false;
            }
            "q!" => {
                // Force quit
                self.running = false;
            }
            "r" | "restart" => {
                self.restart();
            }
            "n" | "next" => {
                self.next_level();
            }
            "practice" => {
                self.practice_mode = !self.practice_mode;
                if self.practice_mode {
                    // Pause the energy timer in practice mode
                    self.energy.pause();
                } else {
                    // Resume the energy timer when leaving practice mode
                    self.energy.resume();
                }
            }
            _ => {
                // Unknown command — just dismiss
            }
        }
    }

    fn execute_dot_repeat(&mut self, repeat_count: usize) {
        let edit = match &self.last_edit {
            Some(e) => e.clone(),
            None => return,
        };

        for _ in 0..repeat_count {
            self.undo.push(self.buffer.rope(), self.cursor);

            // Execute the initial action
            for _ in 0..edit.count {
                command::execute(
                    edit.action,
                    &mut self.buffer,
                    &mut self.cursor,
                    &mut self.mode,
                    &mut self.registers,
                );
            }

            // If the action entered insert mode, replay the insert text
            if !edit.insert_text.is_empty() && self.mode.is_insert() {
                for &ch in &edit.insert_text {
                    command::execute(
                        Action::InsertChar(ch),
                        &mut self.buffer,
                        &mut self.cursor,
                        &mut self.mode,
                        &mut self.registers,
                    );
                }
                // Return to normal mode
                command::execute(
                    Action::EnterNormalMode,
                    &mut self.buffer,
                    &mut self.cursor,
                    &mut self.mode,
                    &mut self.registers,
                );
            }
        }
        self.check_task_completion();
    }

    fn execute_macro(&mut self, reg: char, count: usize) {
        let keys = match self.macro_regs.get(&reg) {
            Some(k) => k.clone(),
            None => return,
        };
        // Temporarily stop recording to avoid nested recording
        let was_recording = self.macro_recording.take();
        for _ in 0..count {
            for key in &keys {
                self.handle_key(*key);
                if !self.running {
                    break;
                }
            }
        }
        self.macro_recording = was_recording;
    }

    fn execute_search(&mut self, direction: SearchDirection) {
        if !self.search.has_pattern() {
            return;
        }
        let result = search::search_next(
            &self.cursor,
            &self.buffer,
            &self.search.pattern,
            direction,
        );
        if let Some(new_cursor) = result {
            self.cursor = new_cursor;
            self.check_task_completion();
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> bool {
        // Ctrl key combos
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('r') => {
                    // Skill gating: Ctrl-R (redo) requires Undo skill
                    if !self.unlocked_skills.contains(&VimSkill::Undo) {
                        let key_name = "Ctrl-R";
                        if !self.locked_keys_shown.contains(key_name) {
                            self.locked_key_flash = Some((
                                format!("\u{1f512} {} \u{2014} unlock in World 12", key_name),
                                Instant::now(),
                                Color::Yellow,
                            ));
                            self.locked_keys_shown.insert(key_name);
                        }
                        return true;
                    }
                    self.scoring.record_keystroke();
                    self.task_keystrokes += 1;
                    if let Some((rope, cursor)) = self.undo.redo() {
                        self.buffer.set_rope(rope);
                        self.cursor = cursor;
                    }
                    return true;
                }
                // Page/half-page movements are free: no energy drain,
                // no task keystroke count. They're for catching up with
                // the scroll, not for task execution.
                KeyCode::Char('d') => {
                    self.scoring.record_keystroke();
                    self.scroll_cursor(self.viewport.height / 2, true);
                    return true;
                }
                KeyCode::Char('u') => {
                    self.scoring.record_keystroke();
                    self.scroll_cursor(self.viewport.height / 2, false);
                    return true;
                }
                KeyCode::Char('f') => {
                    self.scoring.record_keystroke();
                    self.scroll_cursor(self.viewport.height.saturating_sub(2), true);
                    return true;
                }
                KeyCode::Char('b') => {
                    self.scoring.record_keystroke();
                    self.scroll_cursor(self.viewport.height.saturating_sub(2), false);
                    return true;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Esc => {
                self.parser.cancel();
                true
            }
            KeyCode::Char(ch) => {
                self.scoring.record_keystroke();
                self.task_keystrokes += 1;
                // Timer-based: no keystroke drain

                match self.parser.feed(ch) {
                    ParseResult::Action(action, count) => {
                        // Skill gating: block actions the player hasn't unlocked yet
                        if let Some(skill) = worlds::skill_for_action(&action) {
                            if !self.unlocked_skills.contains(&skill) {
                                let key_name = worlds::skill_display_key(&action);
                                if !self.locked_keys_shown.contains(key_name) {
                                    let unlock_world = worlds::skill_unlock_world(skill);
                                    self.locked_key_flash = Some((
                                        format!("\u{1f512} {} \u{2014} unlock in World {}", key_name, unlock_world),
                                        Instant::now(),
                                        Color::Yellow,
                                    ));
                                    self.locked_keys_shown.insert(key_name);
                                }
                                return true;
                            }
                        }

                        // World 1 Level 4: zone-based horizontal restrictions
                        if self.level.is_world1() && self.level.level == 4 {
                            if let Some(skill) = worlds::skill_for_action(&action) {
                                // Get the active task's zone
                                let active_zone = self.tasks.iter()
                                    .find(|t| t.state == task::TaskState::Active)
                                    .and_then(|t| t.zone.as_deref());
                                if let Some(zone) = active_zone {
                                    let zone_skills = worlds::w1_zone_skills(zone);
                                    if !zone_skills.contains(&skill) {
                                        let zone_label = match zone {
                                            "hl" => "h/l",
                                            "wb" => "w/b",
                                            "ft" => "f/t",
                                            "line_edge" => "$/0",
                                            _ => zone,
                                        };
                                        self.locked_key_flash = Some((
                                            format!("\u{1f6ab} {} zone \u{2014} only {} keys allowed here",
                                                zone_label, zone_label),
                                            Instant::now(),
                                            Color::Red,
                                        ));
                                        return true;
                                    }
                                }
                            }
                        }

                        // World 2: per-level insert command gating
                        if self.level.is_basic_edit() && worlds::is_insert_entry(&action) {
                            let allowed = worlds::w2_allowed_insert_actions(self.level.level);
                            if !allowed.contains(&action) {
                                let key_name = worlds::skill_display_key(&action);
                                self.locked_key_flash = Some((
                                    format!("\u{1f512} {} \u{2014} not available in this level", key_name),
                                    Instant::now(),
                                    Color::Yellow,
                                ));
                                return true;
                            }

                            // Level 4: per-task restriction
                            if self.level.level == 4 {
                                let task_entry = self.tasks.iter()
                                    .find(|t| t.state == task::TaskState::Active)
                                    .and_then(|t| t.entry_point.as_deref());
                                if let Some(required) = task_entry {
                                    let action_key = worlds::skill_display_key(&action);
                                    if action_key != required {
                                        self.locked_key_flash = Some((
                                            format!("\u{1f6ab} USE: {} only", required),
                                            Instant::now(),
                                            Color::Red,
                                        ));
                                        return true;
                                    }
                                }
                            }
                        }

                        // Handle undo specially
                        if matches!(action, Action::Undo) {
                            if let Some((rope, cursor)) = self.undo.undo(self.buffer.rope(), self.cursor) {
                                self.buffer.set_rope(rope);
                                self.cursor = cursor;
                            }
                            return true;
                        }

                        // Handle visual mode entry — set anchor
                        if matches!(action, Action::EnterVisualMode | Action::EnterVisualLineMode) {
                            self.visual_anchor = self.cursor;
                        }

                        // Handle search/cmdline actions specially
                        match action {
                            Action::EnterCmdLine => {
                                self.cmdline = Some(String::new());
                                return true;
                            }
                            Action::SearchForward => {
                                self.search.start_input(SearchDirection::Forward);
                                return true;
                            }
                            Action::SearchBackward => {
                                self.search.start_input(SearchDirection::Backward);
                                return true;
                            }
                            Action::SearchNext => {
                                for _ in 0..count {
                                    self.execute_search(self.search.direction);
                                }
                                return true;
                            }
                            Action::SearchPrev => {
                                let reverse = match self.search.direction {
                                    SearchDirection::Forward => SearchDirection::Backward,
                                    SearchDirection::Backward => SearchDirection::Forward,
                                };
                                for _ in 0..count {
                                    self.execute_search(reverse);
                                }
                                return true;
                            }
                            Action::SearchWordForward => {
                                if let Some(word) = search::word_under_cursor(&self.cursor, &self.buffer) {
                                    self.search.pattern = word;
                                    self.search.direction = SearchDirection::Forward;
                                    self.execute_search(SearchDirection::Forward);
                                }
                                return true;
                            }
                            Action::SearchWordBackward => {
                                if let Some(word) = search::word_under_cursor(&self.cursor, &self.buffer) {
                                    self.search.pattern = word;
                                    self.search.direction = SearchDirection::Backward;
                                    self.execute_search(SearchDirection::Backward);
                                }
                                return true;
                            }
                            Action::DotRepeat => {
                                self.execute_dot_repeat(count);
                                return true;
                            }
                            Action::MacroRecord(reg) => {
                                self.macro_recording = Some((reg, Vec::new()));
                                return true;
                            }
                            Action::MacroStop => {
                                if let Some((reg, keys)) = self.macro_recording.take() {
                                    self.macro_regs.insert(reg, keys);
                                }
                                return true;
                            }
                            Action::MacroPlay(reg) => {
                                let actual_reg = if reg == '\0' {
                                    // @@ — replay last
                                    match self.last_macro_reg {
                                        Some(r) => r,
                                        None => return true,
                                    }
                                } else {
                                    reg
                                };
                                self.last_macro_reg = Some(actual_reg);
                                self.execute_macro(actual_reg, count);
                                return true;
                            }
                            _ => {}
                        }

                        // Push undo snapshot before editing actions
                        if action.is_edit() {
                            self.undo.push(self.buffer.rope(), self.cursor);
                        }

                        // Record repeatable edits for dot repeat
                        if action.is_edit() && !matches!(action, Action::InsertChar(_) | Action::Backspace) {
                            self.last_edit = Some(RepeatableEdit {
                                action,
                                count,
                                insert_text: Vec::new(),
                            });
                            self.insert_chars.clear();
                        }

                        // World 1: save cursor before motion for error detection
                        let w1_before = self.cursor;

                        for _ in 0..count {
                            command::execute(
                                action,
                                &mut self.buffer,
                                &mut self.cursor,
                                &mut self.mode,
                                &mut self.registers,
                            );
                        }

                        // World 1: track motion, energy, errors
                        if self.level.is_world1() && worlds::is_motion_action(&action) {
                            let motion_name = if count > 1 {
                                format!("{}{}", count, worlds::skill_display_key(&action))
                            } else {
                                worlds::skill_display_key(&action).to_string()
                            };
                            self.w1_player_motions.push(motion_name);

                            // Use 1 motion energy (each parsed action = 1 motion)
                            self.energy.use_motion();

                            // Error detection: check if we moved closer to the target
                            if let Some(task) = self.tasks.iter().find(|t| t.is_completable()) {
                                let target = (task.target_line, task.target_col);
                                let dist_before = cursor_distance(w1_before.line, w1_before.col, target.0, target.1);
                                let dist_after = cursor_distance(self.cursor.line, self.cursor.col, target.0, target.1);
                                // If we didn't move closer and didn't reach the target, it's an error
                                if dist_after >= dist_before && dist_after > 0 {
                                    self.energy.record_error();
                                }
                            }
                        }

                        self.check_task_completion();
                    }
                    ParseResult::Pending => {}
                    ParseResult::None => {}
                }
                true
            }
            _ => false,
        }
    }

    fn handle_insert_key(&mut self, key: KeyEvent) -> bool {
        let action = match key.code {
            KeyCode::Esc => {
                // Finalize the repeatable edit with collected insert chars
                if let Some(ref mut edit) = self.last_edit {
                    edit.insert_text = self.insert_chars.clone();
                }
                self.insert_chars.clear();
                Action::EnterNormalMode
            }
            KeyCode::Char(ch) => {
                self.insert_chars.push(ch);
                Action::InsertChar(ch)
            }
            KeyCode::Enter => {
                self.insert_chars.push('\n');
                Action::InsertChar('\n')
            }
            KeyCode::Backspace => Action::Backspace,
            _ => return false,
        };

        self.scoring.record_keystroke();
        self.task_keystrokes += 1;
        // Timer-based: no keystroke drain
        if action.is_edit() {
            self.undo.push(self.buffer.rope(), self.cursor);
        }
        command::execute(action, &mut self.buffer, &mut self.cursor, &mut self.mode, &mut self.registers);
        self.check_task_completion();
        true
    }

    fn handle_replace_key(&mut self, key: KeyEvent) -> bool {
        let action = match key.code {
            KeyCode::Esc => Action::EnterNormalMode,
            KeyCode::Char(ch) => Action::ReplaceOverwrite(ch),
            KeyCode::Enter => Action::ReplaceOverwrite('\n'),
            KeyCode::Backspace => Action::Backspace,
            _ => return false,
        };

        self.scoring.record_keystroke();
        self.task_keystrokes += 1;
        // Timer-based: no keystroke drain
        if action.is_edit() {
            self.undo.push(self.buffer.rope(), self.cursor);
        }
        command::execute(action, &mut self.buffer, &mut self.cursor, &mut self.mode, &mut self.registers);
        self.check_task_completion();
        true
    }

    fn handle_visual_key(&mut self, key: KeyEvent) -> bool {
        // Ctrl combos
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('d') => {
                    self.scoring.record_keystroke();
                    self.task_keystrokes += 1;
                    // Timer-based: no keystroke drain
                    self.scroll_cursor(self.viewport.height / 2, true);
                    return true;
                }
                KeyCode::Char('u') => {
                    self.scoring.record_keystroke();
                    self.task_keystrokes += 1;
                    // Timer-based: no keystroke drain
                    self.scroll_cursor(self.viewport.height / 2, false);
                    return true;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.parser.cancel();
                true
            }
            KeyCode::Char(ch) => {
                self.scoring.record_keystroke();
                self.task_keystrokes += 1;
                // Timer-based: no keystroke drain

                // Operators act on the visual selection
                match ch {
                    'd' | 'x' => {
                        self.visual_operator(command::Operator::Delete);
                        return true;
                    }
                    'c' | 's' => {
                        self.visual_operator(command::Operator::Change);
                        return true;
                    }
                    'y' => {
                        self.visual_operator(command::Operator::Yank);
                        return true;
                    }
                    // Toggle between visual and visual-line
                    'v' => {
                        if self.mode == Mode::Visual {
                            self.mode = Mode::Normal;
                        } else {
                            self.mode = Mode::Visual;
                            self.visual_anchor = self.cursor;
                        }
                        return true;
                    }
                    'V' => {
                        if self.mode == Mode::VisualLine {
                            self.mode = Mode::Normal;
                        } else {
                            self.mode = Mode::VisualLine;
                            self.visual_anchor = self.cursor;
                        }
                        return true;
                    }
                    _ => {}
                }

                // Motions: use the parser to interpret, then apply as cursor movement
                match self.parser.feed(ch) {
                    ParseResult::Action(action, count) => {
                        // Apply motion to move the cursor (extending selection)
                        for _ in 0..count {
                            command::execute(
                                action,
                                &mut self.buffer,
                                &mut self.cursor,
                                &mut self.mode,
                                &mut self.registers,
                            );
                        }
                        // Stay in visual mode (execute may have changed it for insert actions)
                        // Only if execution didn't change mode to something else
                    }
                    ParseResult::Pending => {}
                    ParseResult::None => {}
                }
                true
            }
            _ => false,
        }
    }

    /// Execute an operator on the visual selection.
    fn visual_operator(&mut self, op: command::Operator) {
        let anchor = self.visual_anchor;
        let cursor = self.cursor;
        let is_linewise = self.mode == Mode::VisualLine;

        self.undo.push(self.buffer.rope(), self.cursor);

        if is_linewise {
            let start_line = anchor.line.min(cursor.line);
            let end_line = anchor.line.max(cursor.line);

            match op {
                command::Operator::Delete => {
                    let text = self.buffer.delete_lines(start_line, end_line);
                    let reg_text = if text.ends_with('\n') { text } else { format!("{}\n", text) };
                    self.registers.delete(None, super::vim::register::RegisterContent::Linewise(reg_text));
                    self.cursor.line = start_line.min(self.buffer.line_count().saturating_sub(1));
                    self.cursor.clamp(&self.buffer, false);
                }
                command::Operator::Change => {
                    let text = self.buffer.delete_lines(start_line, end_line);
                    let reg_text = if text.ends_with('\n') { text } else { format!("{}\n", text) };
                    self.registers.delete(None, super::vim::register::RegisterContent::Linewise(reg_text));
                    if start_line < self.buffer.line_count() {
                        self.buffer.insert_char(start_line, 0, '\n');
                        self.cursor.line = start_line;
                    }
                    self.cursor.col = 0;
                    self.mode = Mode::Insert;
                    self.check_task_completion();
                    return;
                }
                command::Operator::Yank => {
                    let mut text = String::new();
                    for line_idx in start_line..=end_line {
                        if let Some(line) = self.buffer.line(line_idx) {
                            text.push_str(&line);
                            text.push('\n');
                        }
                    }
                    self.registers.yank(None, super::vim::register::RegisterContent::Linewise(text));
                    self.cursor.line = start_line;
                    self.cursor.clamp(&self.buffer, false);
                }
            }
        } else {
            // Charwise visual
            let (start, end) = if (anchor.line, anchor.col) <= (cursor.line, cursor.col) {
                (anchor, cursor)
            } else {
                (cursor, anchor)
            };

            // End is inclusive in charwise visual — make it exclusive for the range
            let end_exclusive = if end.col + 1 <= self.buffer.line_len(end.line) {
                Cursor::new(end.line, end.col + 1)
            } else if end.line + 1 < self.buffer.line_count() {
                Cursor::new(end.line + 1, 0)
            } else {
                Cursor::new(end.line, self.buffer.line_len(end.line))
            };

            let text = self.buffer.text_range(
                start.line,
                start.col,
                end_exclusive.line,
                end_exclusive.col,
            );

            match op {
                command::Operator::Delete => {
                    self.buffer.delete_range(
                        start.line, start.col,
                        end_exclusive.line, end_exclusive.col,
                    );
                    self.registers.delete(None, super::vim::register::RegisterContent::Charwise(text));
                    self.cursor = start;
                    self.cursor.clamp(&self.buffer, false);
                }
                command::Operator::Change => {
                    self.buffer.delete_range(
                        start.line, start.col,
                        end_exclusive.line, end_exclusive.col,
                    );
                    self.registers.delete(None, super::vim::register::RegisterContent::Charwise(text));
                    self.cursor = start;
                    self.mode = Mode::Insert;
                    self.check_task_completion();
                    return;
                }
                command::Operator::Yank => {
                    self.registers.yank(None, super::vim::register::RegisterContent::Charwise(text));
                    self.cursor = start;
                }
            }
        }

        self.mode = Mode::Normal;
        self.check_task_completion();
    }

    fn check_task_completion(&mut self) {
        for task in &mut self.tasks {
            if !task.is_completable() {
                continue;
            }
            // Only check the first completable task (the active one)
            if task.state != TaskState::Active {
                break;
            }
            let completed = match &task.kind {
                TaskKind::MoveTo => {
                    self.cursor.line == task.target_line
                        && self.cursor.col == task.target_col
                }
                TaskKind::DeleteLine { original_content } => {
                    // Line is deleted if the content at that line no longer matches
                    match self.buffer.line(task.target_line) {
                        Some(line) => line.trim() != original_content.trim(),
                        None => true, // line doesn't exist anymore = deleted
                    }
                }
                TaskKind::DeleteWord { word } => {
                    // Word is deleted if the line no longer contains it
                    match self.buffer.line(task.target_line) {
                        Some(line) => !line.contains(word.as_str()),
                        None => true,
                    }
                }
                TaskKind::ChangeWord { new_text, .. } => {
                    // Completed when the line contains the new_text
                    match self.buffer.line(task.target_line) {
                        Some(line) => line.contains(new_text.as_str()),
                        None => false,
                    }
                }
                TaskKind::ReplaceChar { expected } => {
                    // Completed when the char at position matches expected
                    self.buffer
                        .char_at(task.target_line, task.target_col)
                        .map(|ch| ch == *expected)
                        .unwrap_or(false)
                }
                TaskKind::ChangeInside { new_text, .. } => {
                    // Completed when the line contains the new_text
                    match self.buffer.line(task.target_line) {
                        Some(line) => line.contains(new_text.as_str()),
                        None => false,
                    }
                }
                TaskKind::YankPaste { expected_text } => {
                    // Completed when the target line contains the expected text
                    match self.buffer.line(task.target_line) {
                        Some(line) => line.contains(expected_text.as_str()),
                        None => false,
                    }
                }
                TaskKind::DeleteBlock { original_lines } => {
                    // Completed when none of the original lines exist at their positions
                    original_lines.iter().enumerate().all(|(i, orig)| {
                        let line_idx = task.target_line + i;
                        match self.buffer.line(line_idx) {
                            Some(line) => line.trim() != orig.trim(),
                            None => true,
                        }
                    })
                }
                TaskKind::Indent { expected_indent } => {
                    // Completed when the line starts with the expected indentation
                    match self.buffer.line(task.target_line) {
                        Some(line) => line.starts_with(expected_indent.as_str()),
                        None => false,
                    }
                }
                TaskKind::InsertLine { expected_content, near_line } => {
                    // Scan ±3 lines around near_line for matching trimmed content
                    let start = near_line.saturating_sub(3);
                    let end = (*near_line + 4).min(self.buffer.line_count());
                    (start..end).any(|i| {
                        self.buffer.line(i)
                            .map(|l| l.trim() == expected_content.trim())
                            .unwrap_or(false)
                    })
                }
                TaskKind::InsertText { expected_text } => {
                    // Completed when the target line contains the expected text
                    match self.buffer.line(task.target_line) {
                        Some(line) => line.contains(expected_text.as_str()),
                        None => false,
                    }
                }
            };
            if completed {
                use crate::game::task::CompletionQuality;
                task.mark_completed();
                // Determine completion quality: Perfect > Great > Done
                let is_perfect = task.perfect_keys > 0
                    && self.task_keystrokes <= task.perfect_keys;
                let is_great = task.good_keys > 0
                    && self.task_keystrokes <= task.good_keys;
                if is_perfect {
                    task.quality = CompletionQuality::Perfect;
                    self.scoring.award_perfect();
                } else if is_great {
                    task.quality = CompletionQuality::Great;
                    self.scoring.award_great();
                }
                self.scoring.complete_task(task.points);
                self.energy.restore_task();
                // Flash popup for completion quality
                let (flash_text, flash_color) = match task.quality {
                    CompletionQuality::Perfect => ("\u{2605} PERFECT".to_string(), Color::Rgb(255, 215, 0)),
                    CompletionQuality::Great => ("\u{2713} GREAT".to_string(), Color::Cyan),
                    CompletionQuality::Done => ("\u{2713} DONE".to_string(), Color::Green),
                };
                self.completion_flash = Some((flash_text, Instant::now(), flash_color));
                self.task_keystrokes = 0;
                self.last_task_completion = Some(Instant::now());

                // World 1: advance to next target, reset motion tracking, set new budget
                if self.level.is_world1() {
                    self.w1_current_target += 1;
                    self.w1_player_motions.clear();
                    if self.w1_current_target < self.w1_paths.len() {
                        let next_optimal = self.w1_paths[self.w1_current_target].optimal_motions;
                        // Level 5: budget = optimal exactly (must reach in 1 motion)
                        let budget = if self.level.level == 5 {
                            next_optimal
                        } else {
                            next_optimal + 3
                        };
                        self.energy.reset_for_target(budget);
                    }
                }
            }
        }
    }
}
