use std::collections::HashSet;

/// A Vim skill that can be unlocked by reaching a world.
/// Skills are cumulative — reaching World N unlocks all skills from Worlds 1..=N.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VimSkill {
    // World 1 — First Steps
    MoveLeft,        // h
    MoveDown,        // j
    MoveUp,          // k
    MoveRight,       // l
    Count,           // number prefix (5j, 3k, etc.)
    GotoLine,        // gg, G, {n}G

    // World 2 — Word Surfer
    WordForward,     // w
    WordBackward,    // b
    WordEnd,         // e
    BigWordForward,  // W
    BigWordBackward, // B
    BigWordEnd,      // E

    // World 3 — Line Rider
    LineStart,       // 0
    LineFirstChar,   // ^
    LineEnd,         // $
    FindChar,        // f/F
    TillChar,        // t/T
    RepeatFind,      // ;/,

    // World 4 — The Writer
    InsertMode,      // i, a, I, A, o, O
    ReplaceMode,     // R, r

    // World 5 — The Destroyer
    DeleteChar,      // x, X
    DeleteLine,      // dd
    DeleteToEnd,     // D

    // World 6 — Verb + Noun
    OperatorMotion,  // d{motion}, c{motion}
    DotRepeat,       // .
    Indent,          // >>, <<

    // World 7 — Copy Ninja
    YankLine,        // yy
    YankMotion,      // y{motion}
    Paste,           // p, P

    // World 8 — The Selector
    VisualMode,      // v
    VisualLineMode,  // V

    // World 9 — Text Object Surgeon
    TextObject,      // iw, aw, i", a", i(, a{, etc.

    // World 10 — Code Navigator
    Paragraph,       // {, }
    MatchBracket,    // %

    // World 11 — Search & Destroy
    Search,          // /, ?, n, N
    SearchWord,      // *, #

    // World 12 — Time Traveler
    Undo,            // u, Ctrl-R
    NamedRegister,   // "a, "b, etc.

    // World 13 — Macro Wizard
    Macro,           // q{reg}, @{reg}, @@

    // World 14 — The Grandmaster (no new skills, everything unlocked)

    // Always available (viewport navigation)
    ScrollPage,      // Ctrl-d/u/f/b
}

/// World metadata.
pub struct WorldDef {
    pub number: usize,
    pub name: &'static str,
    pub subtitle: &'static str,
    /// Skills introduced in this world (not cumulative).
    pub new_skills: &'static [VimSkill],
}

/// All 14 worlds.
pub const WORLDS: &[WorldDef] = &[
    WorldDef {
        number: 1,
        name: "First Steps",
        subtitle: "Learn to move",
        new_skills: &[
            VimSkill::MoveLeft, VimSkill::MoveDown,
            VimSkill::MoveUp, VimSkill::MoveRight,
            VimSkill::Count, VimSkill::GotoLine,
            VimSkill::ScrollPage,
        ],
    },
    WorldDef {
        number: 2,
        name: "Word Surfer",
        subtitle: "Ride the words",
        new_skills: &[
            VimSkill::WordForward, VimSkill::WordBackward, VimSkill::WordEnd,
            VimSkill::BigWordForward, VimSkill::BigWordBackward, VimSkill::BigWordEnd,
        ],
    },
    WorldDef {
        number: 3,
        name: "Line Rider",
        subtitle: "Precision movement",
        new_skills: &[
            VimSkill::LineStart, VimSkill::LineFirstChar, VimSkill::LineEnd,
            VimSkill::FindChar, VimSkill::TillChar, VimSkill::RepeatFind,
        ],
    },
    WorldDef {
        number: 4,
        name: "The Writer",
        subtitle: "Enter insert mode",
        new_skills: &[
            VimSkill::InsertMode, VimSkill::ReplaceMode,
        ],
    },
    WorldDef {
        number: 5,
        name: "The Destroyer",
        subtitle: "Delete with precision",
        new_skills: &[
            VimSkill::DeleteChar, VimSkill::DeleteLine, VimSkill::DeleteToEnd,
        ],
    },
    WorldDef {
        number: 6,
        name: "Verb + Noun",
        subtitle: "The most important lesson",
        new_skills: &[
            VimSkill::OperatorMotion, VimSkill::DotRepeat, VimSkill::Indent,
        ],
    },
    WorldDef {
        number: 7,
        name: "Copy Ninja",
        subtitle: "Yank and paste",
        new_skills: &[
            VimSkill::YankLine, VimSkill::YankMotion, VimSkill::Paste,
        ],
    },
    WorldDef {
        number: 8,
        name: "The Selector",
        subtitle: "Visual mode",
        new_skills: &[
            VimSkill::VisualMode, VimSkill::VisualLineMode,
        ],
    },
    WorldDef {
        number: 9,
        name: "Text Object Surgeon",
        subtitle: "Inner and around",
        new_skills: &[
            VimSkill::TextObject,
        ],
    },
    WorldDef {
        number: 10,
        name: "Code Navigator",
        subtitle: "Structural movement",
        new_skills: &[
            VimSkill::Paragraph, VimSkill::MatchBracket,
        ],
    },
    WorldDef {
        number: 11,
        name: "Search & Destroy",
        subtitle: "Find anything",
        new_skills: &[
            VimSkill::Search, VimSkill::SearchWord,
        ],
    },
    WorldDef {
        number: 12,
        name: "Time Traveler",
        subtitle: "Undo and registers",
        new_skills: &[
            VimSkill::Undo, VimSkill::NamedRegister,
        ],
    },
    WorldDef {
        number: 13,
        name: "Macro Wizard",
        subtitle: "Record and replay",
        new_skills: &[
            VimSkill::Macro,
        ],
    },
    WorldDef {
        number: 14,
        name: "The Grandmaster",
        subtitle: "The final challenge",
        new_skills: &[], // No new skills — everything already unlocked
    },
];

/// Get the cumulative set of skills unlocked at a given world number.
pub fn skills_for_world(world: usize) -> HashSet<VimSkill> {
    let mut skills = HashSet::new();
    for w in WORLDS.iter() {
        if w.number <= world {
            for &skill in w.new_skills {
                skills.insert(skill);
            }
        }
    }
    skills
}

/// Map an Action to the VimSkill it requires.
/// Returns None for actions that are always allowed (Esc, None, etc.).
use crate::vim::command::{Action, Operator};

pub fn skill_for_action(action: &Action) -> Option<VimSkill> {
    match action {
        // World 1 — basic movement
        Action::MoveLeft => Some(VimSkill::MoveLeft),
        Action::MoveDown => Some(VimSkill::MoveDown),
        Action::MoveUp => Some(VimSkill::MoveUp),
        Action::MoveRight => Some(VimSkill::MoveRight),
        Action::GotoFirstLine | Action::GotoLastLine | Action::GotoLine(_) => {
            Some(VimSkill::GotoLine)
        }

        // World 2 — word motions
        Action::WordForward => Some(VimSkill::WordForward),
        Action::WordBackward => Some(VimSkill::WordBackward),
        Action::WordEnd => Some(VimSkill::WordEnd),
        Action::BigWordForward => Some(VimSkill::BigWordForward),
        Action::BigWordBackward => Some(VimSkill::BigWordBackward),
        Action::BigWordEnd => Some(VimSkill::BigWordEnd),

        // World 3 — line precision
        Action::LineStart => Some(VimSkill::LineStart),
        Action::LineFirstChar => Some(VimSkill::LineFirstChar),
        Action::LineEnd => Some(VimSkill::LineEnd),
        Action::FindCharForward(_) | Action::FindCharBackward(_) => Some(VimSkill::FindChar),
        Action::TillCharForward(_) | Action::TillCharBackward(_) => Some(VimSkill::TillChar),

        // World 4 — insert mode
        Action::EnterInsertMode | Action::InsertAfter
        | Action::InsertAtStart | Action::InsertAtEnd
        | Action::OpenLineBelow | Action::OpenLineAbove => Some(VimSkill::InsertMode),
        Action::EnterReplaceMode | Action::ReplaceChar(_) => Some(VimSkill::ReplaceMode),

        // World 5 — simple deletion
        Action::DeleteChar | Action::DeleteCharBefore => Some(VimSkill::DeleteChar),
        Action::DeleteLine => Some(VimSkill::DeleteLine),
        Action::DeleteToEnd => Some(VimSkill::DeleteToEnd),

        // World 6 — operator + motion
        Action::OperatorMotion(op, _, _) => match op {
            Operator::Delete | Operator::Change => Some(VimSkill::OperatorMotion),
            Operator::Yank => Some(VimSkill::YankMotion),
        },
        Action::ChangeToEnd => Some(VimSkill::OperatorMotion),
        Action::DotRepeat => Some(VimSkill::DotRepeat),

        // World 6 — operator + text object
        Action::OperatorTextObject(op, _) => match op {
            Operator::Delete | Operator::Change => Some(VimSkill::TextObject),
            Operator::Yank => Some(VimSkill::TextObject),
        },

        // World 7 — yank and paste
        Action::YankLine => Some(VimSkill::YankLine),
        Action::PasteAfter | Action::PasteBefore => Some(VimSkill::Paste),

        // World 8 — visual mode
        Action::EnterVisualMode => Some(VimSkill::VisualMode),
        Action::EnterVisualLineMode => Some(VimSkill::VisualLineMode),

        // World 10 — code navigation
        Action::ParagraphForward | Action::ParagraphBackward => Some(VimSkill::Paragraph),
        Action::MatchBracket => Some(VimSkill::MatchBracket),

        // World 11 — search
        Action::SearchForward | Action::SearchBackward
        | Action::SearchNext | Action::SearchPrev => Some(VimSkill::Search),
        Action::SearchWordForward | Action::SearchWordBackward => Some(VimSkill::SearchWord),

        // World 12 — undo
        Action::Undo | Action::Redo => Some(VimSkill::Undo),

        // World 13 — macros
        Action::MacroRecord(_) | Action::MacroStop | Action::MacroPlay(_) => {
            Some(VimSkill::Macro)
        }

        // Scroll is always available
        Action::ScrollHalfDown | Action::ScrollHalfUp
        | Action::ScrollFullDown | Action::ScrollFullUp => Some(VimSkill::ScrollPage),

        // Always allowed
        Action::EnterNormalMode | Action::EnterCmdLine
        | Action::InsertChar(_) | Action::Backspace
        | Action::ReplaceOverwrite(_) | Action::None => None,
    }
}

/// Human-readable name for a skill (used in lock flash).
pub fn skill_display_key(action: &Action) -> &'static str {
    match action {
        Action::MoveLeft => "h",
        Action::MoveDown => "j",
        Action::MoveUp => "k",
        Action::MoveRight => "l",
        Action::GotoFirstLine => "gg",
        Action::GotoLastLine | Action::GotoLine(_) => "G",
        Action::WordForward => "w",
        Action::WordBackward => "b",
        Action::WordEnd => "e",
        Action::BigWordForward => "W",
        Action::BigWordBackward => "B",
        Action::BigWordEnd => "E",
        Action::LineStart => "0",
        Action::LineFirstChar => "^",
        Action::LineEnd => "$",
        Action::FindCharForward(_) => "f",
        Action::FindCharBackward(_) => "F",
        Action::TillCharForward(_) => "t",
        Action::TillCharBackward(_) => "T",
        Action::EnterInsertMode => "i",
        Action::InsertAfter => "a",
        Action::InsertAtStart => "I",
        Action::InsertAtEnd => "A",
        Action::OpenLineBelow => "o",
        Action::OpenLineAbove => "O",
        Action::EnterReplaceMode => "R",
        Action::ReplaceChar(_) => "r",
        Action::DeleteChar => "x",
        Action::DeleteCharBefore => "X",
        Action::DeleteLine => "dd",
        Action::DeleteToEnd => "D",
        Action::ChangeToEnd => "C",
        Action::DotRepeat => ".",
        Action::YankLine => "yy",
        Action::PasteAfter => "p",
        Action::PasteBefore => "P",
        Action::EnterVisualMode => "v",
        Action::EnterVisualLineMode => "V",
        Action::ParagraphForward => "}",
        Action::ParagraphBackward => "{",
        Action::MatchBracket => "%",
        Action::SearchForward => "/",
        Action::SearchBackward => "?",
        Action::SearchNext => "n",
        Action::SearchPrev => "N",
        Action::SearchWordForward => "*",
        Action::SearchWordBackward => "#",
        Action::Undo => "u",
        Action::Redo => "Ctrl-R",
        Action::MacroRecord(_) => "q",
        Action::MacroStop => "q",
        Action::MacroPlay(_) => "@",
        Action::OperatorMotion(Operator::Delete, _, _) => "d{motion}",
        Action::OperatorMotion(Operator::Change, _, _) => "c{motion}",
        Action::OperatorMotion(Operator::Yank, _, _) => "y{motion}",
        Action::OperatorTextObject(Operator::Delete, _) => "d{object}",
        Action::OperatorTextObject(Operator::Change, _) => "c{object}",
        Action::OperatorTextObject(Operator::Yank, _) => "y{object}",
        _ => "?",
    }
}

/// Which world introduces a given skill.
pub fn skill_unlock_world(skill: VimSkill) -> usize {
    for w in WORLDS.iter() {
        if w.new_skills.contains(&skill) {
            return w.number;
        }
    }
    1 // ScrollPage and fallback
}

// ── World 1 difficulty settings ──────────────────────────────────

/// Difficulty level for World 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W1Difficulty {
    pub level: u8,
    pub name: &'static str,
    /// Scroll speed: milliseconds per line.
    pub scroll_ms: u64,
    /// Maximum errors (non-optimal motions) allowed.
    pub max_errors: usize,
}

pub const W1_DIFFICULTIES: &[W1Difficulty] = &[
    W1Difficulty { level: 1, name: "Nano User",       scroll_ms: 5000, max_errors: 10 },
    W1Difficulty { level: 2, name: ":wq Survivor",    scroll_ms: 2000, max_errors: 5 },
    W1Difficulty { level: 3, name: "Keyboard Warrior", scroll_ms: 1000, max_errors: 3 },
    W1Difficulty { level: 4, name: "10x Engineer",    scroll_ms: 500,  max_errors: 1 },
    W1Difficulty { level: 5, name: "Uses Arch btw",   scroll_ms: 200,  max_errors: 0 },
];

/// Get World 1 difficulty by level (1-indexed). Defaults to difficulty 1.
pub fn w1_difficulty(difficulty: u8) -> &'static W1Difficulty {
    W1_DIFFICULTIES.iter()
        .find(|d| d.level == difficulty)
        .unwrap_or(&W1_DIFFICULTIES[0])
}

// ── World 1 per-level allowed motions ───────────────────────────

/// Restriction zone for World 1 Level 4 (horizontal keys restricted per zone).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W1Zone {
    /// h/l only for horizontal
    HlOnly,
    /// w/W/b/B/e only for horizontal
    WordOnly,
    /// f/F/t/T only for horizontal
    FindOnly,
    /// $/0 only for horizontal
    LineEdgeOnly,
    /// No restriction (between zones)
    Any,
}

/// Get the set of allowed VimSkills for a World 1 sub-level.
pub fn w1_allowed_skills(level: usize) -> HashSet<VimSkill> {
    let mut skills = HashSet::new();
    // Always available
    skills.insert(VimSkill::ScrollPage);

    match level {
        1 => {
            // h j k l only
            skills.insert(VimSkill::MoveLeft);
            skills.insert(VimSkill::MoveDown);
            skills.insert(VimSkill::MoveUp);
            skills.insert(VimSkill::MoveRight);
        }
        2 => {
            // + w W b B e + count prefixes
            skills.insert(VimSkill::MoveLeft);
            skills.insert(VimSkill::MoveDown);
            skills.insert(VimSkill::MoveUp);
            skills.insert(VimSkill::MoveRight);
            skills.insert(VimSkill::WordForward);
            skills.insert(VimSkill::WordBackward);
            skills.insert(VimSkill::WordEnd);
            skills.insert(VimSkill::BigWordForward);
            skills.insert(VimSkill::BigWordBackward);
            skills.insert(VimSkill::Count);
        }
        3 => {
            // + f F t T
            skills.insert(VimSkill::MoveLeft);
            skills.insert(VimSkill::MoveDown);
            skills.insert(VimSkill::MoveUp);
            skills.insert(VimSkill::MoveRight);
            skills.insert(VimSkill::WordForward);
            skills.insert(VimSkill::WordBackward);
            skills.insert(VimSkill::WordEnd);
            skills.insert(VimSkill::BigWordForward);
            skills.insert(VimSkill::BigWordBackward);
            skills.insert(VimSkill::Count);
            skills.insert(VimSkill::FindChar);
            skills.insert(VimSkill::TillChar);
        }
        4 | 5 => {
            // All World 1 motions
            skills.insert(VimSkill::MoveLeft);
            skills.insert(VimSkill::MoveDown);
            skills.insert(VimSkill::MoveUp);
            skills.insert(VimSkill::MoveRight);
            skills.insert(VimSkill::WordForward);
            skills.insert(VimSkill::WordBackward);
            skills.insert(VimSkill::WordEnd);
            skills.insert(VimSkill::BigWordForward);
            skills.insert(VimSkill::BigWordBackward);
            skills.insert(VimSkill::Count);
            skills.insert(VimSkill::FindChar);
            skills.insert(VimSkill::TillChar);
            skills.insert(VimSkill::LineStart);
            skills.insert(VimSkill::LineEnd);
        }
        _ => {
            // Default: everything in world 1
            for &skill in &[
                VimSkill::MoveLeft, VimSkill::MoveDown, VimSkill::MoveUp,
                VimSkill::MoveRight, VimSkill::WordForward, VimSkill::WordBackward,
                VimSkill::WordEnd, VimSkill::BigWordForward, VimSkill::BigWordBackward,
                VimSkill::Count, VimSkill::FindChar, VimSkill::TillChar,
                VimSkill::LineStart, VimSkill::LineEnd,
            ] {
                skills.insert(skill);
            }
        }
    }
    skills
}

/// Check whether an action is a motion (moves the cursor without editing).
pub fn is_motion_action(action: &Action) -> bool {
    matches!(action,
        Action::MoveLeft | Action::MoveDown | Action::MoveUp | Action::MoveRight
        | Action::WordForward | Action::WordBackward | Action::WordEnd
        | Action::BigWordForward | Action::BigWordBackward | Action::BigWordEnd
        | Action::LineStart | Action::LineFirstChar | Action::LineEnd
        | Action::FindCharForward(_) | Action::FindCharBackward(_)
        | Action::TillCharForward(_) | Action::TillCharBackward(_)
        | Action::GotoFirstLine | Action::GotoLastLine | Action::GotoLine(_)
        | Action::ParagraphForward | Action::ParagraphBackward
        | Action::MatchBracket
        | Action::SearchForward | Action::SearchBackward
        | Action::SearchNext | Action::SearchPrev
        | Action::SearchWordForward | Action::SearchWordBackward
        | Action::ScrollHalfDown | Action::ScrollHalfUp
        | Action::ScrollFullDown | Action::ScrollFullUp
    )
}

/// Format skill names for the level hint block.
/// Returns lines like "w / b      next word / back a word".
pub fn skill_hint_lines(world: usize) -> Vec<&'static str> {
    match world {
        1 => vec![
            "h / l      move left / right",
            "j / k      move down / up",
            "5j / 3k    move with counts",
            "gg / G     top / bottom of file",
        ],
        2 => vec![
            "w / b      next word / back a word",
            "e          jump to end of word",
            "W / B / E  big-word motions",
            "3w / 2b    word motions with counts",
        ],
        3 => vec![
            "0 / $      start / end of line",
            "^          first non-blank character",
            "f<c> / t<c>  find / till character",
            ";          repeat last f/t",
        ],
        4 => vec![
            "i / a      insert before / after cursor",
            "I / A      insert at line start / end",
            "o / O      open line below / above",
            "r<c>       replace single character",
        ],
        5 => vec![
            "x          delete character under cursor",
            "dd         delete entire line",
            "D          delete to end of line",
        ],
        6 => vec![
            "dw / d$    delete word / to end",
            "cw / c$    change word / to end",
            ".          repeat last change",
            ">> / <<    indent / dedent line",
        ],
        7 => vec![
            "yy         yank (copy) line",
            "yw         yank word",
            "p / P      paste after / before",
        ],
        8 => vec![
            "v          visual character mode",
            "V          visual line mode",
            "v + d/c/y  select then operate",
        ],
        9 => vec![
            "iw / aw    inner / around word",
            "i\" / a\"    inside / around quotes",
            "ci( / di{  change/delete inside brackets",
        ],
        10 => vec![
            "{ / }      jump between paragraphs",
            "%          jump to matching bracket",
        ],
        11 => vec![
            "/pattern   search forward",
            "?pattern   search backward",
            "n / N      next / prev match",
            "* / #      search word under cursor",
        ],
        12 => vec![
            "u          undo",
            "Ctrl-R     redo",
            "\"ayy       yank into register a",
            "\"ap        paste from register a",
        ],
        13 => vec![
            "qa ... q   record macro into register a",
            "@a         play macro from register a",
            "@@         replay last macro",
            "5@a        play macro 5 times",
        ],
        _ => vec![
            "all motions unlocked!",
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skills_cumulative() {
        let w1 = skills_for_world(1);
        assert!(w1.contains(&VimSkill::MoveLeft));
        assert!(!w1.contains(&VimSkill::WordForward));

        let w2 = skills_for_world(2);
        assert!(w2.contains(&VimSkill::MoveLeft)); // still has W1
        assert!(w2.contains(&VimSkill::WordForward));
        assert!(!w2.contains(&VimSkill::FindChar));
    }

    #[test]
    fn test_world_14_has_everything() {
        let all = skills_for_world(14);
        // Should have skills from every world
        assert!(all.contains(&VimSkill::MoveLeft));
        assert!(all.contains(&VimSkill::Macro));
        assert!(all.contains(&VimSkill::TextObject));
    }

    #[test]
    fn test_skill_for_action_basic() {
        assert_eq!(skill_for_action(&Action::MoveLeft), Some(VimSkill::MoveLeft));
        assert_eq!(skill_for_action(&Action::WordForward), Some(VimSkill::WordForward));
        assert_eq!(skill_for_action(&Action::None), None);
    }

    #[test]
    fn test_unlock_world() {
        assert_eq!(skill_unlock_world(VimSkill::MoveLeft), 1);
        assert_eq!(skill_unlock_world(VimSkill::WordForward), 2);
        assert_eq!(skill_unlock_world(VimSkill::FindChar), 3);
        assert_eq!(skill_unlock_world(VimSkill::Macro), 13);
    }
}
