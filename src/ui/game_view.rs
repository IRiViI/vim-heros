use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::{App, GameOverReason};
use crate::game::engine::GameState;
use crate::game::task::{CompletionQuality, Task, TaskState};
use crate::vim::mode::Mode;

/// Render the full game view into a ratatui frame.
pub fn render(frame: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // HUD bar
            Constraint::Length(1), // Energy bar
            Constraint::Min(3),    // buffer area
            Constraint::Length(1), // status bar
        ])
        .split(frame.area());

    render_hud(frame, app, chunks[0]);
    render_energy_bar(frame, app, chunks[1]);
    render_buffer(frame, app, chunks[2]);
    render_status_bar(frame, app, chunks[3]);

    if matches!(app.engine.state, GameState::GameOver | GameState::LevelComplete) {
        render_results(frame, app, chunks[2]);
    }

    if app.engine.state == GameState::Countdown {
        render_countdown(frame, app, chunks[2]);
    }
}

/// Render the HUD bar: stars, level, score, combo.
fn render_hud(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let stars = app.scoring.star_display();
    let score = format!("  Score: {}  ", app.scoring.score);
    let combo = app.scoring.combo_display();

    let tasks_done = app.tasks.iter().filter(|t| t.state == TaskState::Completed).count();
    let tasks_total = app.tasks.len();

    let spans = vec![
        Span::styled(
            format!(" {} ", stars),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" Level {} ", app.level.display_id()),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("\"{}\" ", app.level.name),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(score, Style::default().fg(Color::Green)),
        Span::styled(
            format!("Tasks: {}/{} ", tasks_done, tasks_total),
            Style::default().fg(Color::Magenta),
        ),
        if combo.is_empty() {
            Span::raw("")
        } else {
            Span::styled(
                format!(" {} ", combo),
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            )
        },
    ];

    let hud = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::Rgb(25, 25, 40)));
    frame.render_widget(hud, area);
}

/// Render the energy bar: colored bar with percentage and optional restore popup.
fn render_energy_bar(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let pct = app.energy.percentage();
    let bar_width = area.width.saturating_sub(20) as usize; // reserve space for label + pct
    let filled = ((pct / 100.0) * bar_width as f64).round() as usize;
    let empty = bar_width.saturating_sub(filled);

    // Color based on energy level
    let bar_color = if pct > 60.0 {
        Color::Green
    } else if pct > 30.0 {
        Color::Yellow
    } else if pct > 15.0 {
        Color::Red
    } else {
        // Pulsing effect at critical levels: alternate between red and dark red
        let secs = app.engine.elapsed_secs();
        if secs % 2 == 0 {
            Color::Red
        } else {
            Color::Rgb(180, 0, 0)
        }
    };

    let filled_str = "\u{2588}".repeat(filled);
    let empty_str = "\u{2591}".repeat(empty);

    let mut spans = vec![
        Span::styled(" Energy: ", Style::default().fg(Color::DarkGray)),
        Span::styled(filled_str, Style::default().fg(bar_color)),
        Span::styled(empty_str, Style::default().fg(Color::Rgb(60, 60, 60))),
        Span::styled(
            format!(" {:>5.1}% ", pct),
            Style::default().fg(bar_color).add_modifier(Modifier::BOLD),
        ),
    ];

    // Show "+N" restore popup if available
    if let Some(restore) = app.energy.last_restore {
        spans.push(Span::styled(
            format!("+{:.0}", restore),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ));
    }

    let bar = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::Rgb(20, 20, 35)));
    frame.render_widget(bar, area);
}

/// Gold color for perfect completions.
const GOLD: Color = Color::Rgb(255, 215, 0);

/// Task highlight style for the single target character.
fn task_char_style(task: &Task) -> Style {
    match (task.state, task.quality) {
        (TaskState::Pending | TaskState::Active, _) => {
            Style::default().bg(Color::Red).fg(Color::White).add_modifier(Modifier::BOLD)
        }
        (TaskState::Completed, CompletionQuality::Perfect) => {
            Style::default().bg(GOLD).fg(Color::Black).add_modifier(Modifier::BOLD)
        }
        (TaskState::Completed, CompletionQuality::Great) => {
            Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)
        }
        (TaskState::Completed, CompletionQuality::Done) => {
            Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD)
        }
        (TaskState::Missed, _) => {
            Style::default().bg(Color::Yellow).fg(Color::Black)
        }
    }
}

/// Gutter annotation for a task.
fn task_annotation(task: &Task) -> (String, Color) {
    match (task.state, task.quality) {
        (TaskState::Pending | TaskState::Active, _) => (
            format!("  \u{25c0} {} ", task.gutter_text),
            Color::Red,
        ),
        (TaskState::Completed, CompletionQuality::Perfect) => (
            " \u{2605} PERFECT ".to_string(),
            GOLD,
        ),
        (TaskState::Completed, CompletionQuality::Great) => (
            " \u{2713} GREAT ".to_string(),
            Color::Cyan,
        ),
        (TaskState::Completed, CompletionQuality::Done) => (
            " \u{2713} DONE ".to_string(),
            Color::Green,
        ),
        (TaskState::Missed, _) => (
            " \u{2717} MISSED ".to_string(),
            Color::Yellow,
        ),
    }
}

/// Find the task (if any) for a given line index.
fn task_for_line(tasks: &[Task], line_idx: usize) -> Option<&Task> {
    tasks.iter().find(|t| t.target_line == line_idx)
}

/// Render the text buffer with line numbers, cursor, and task overlays.
fn render_buffer(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let buffer = &app.buffer;
    let cursor = &app.cursor;
    let viewport = &app.viewport;

    let inner_height = area.height.saturating_sub(2) as usize; // 2 for top+bottom border
    let line_count = buffer.line_count();

    let max_line_num = line_count;
    let gutter_width = format!("{}", max_line_num).len() + 1; // +1 for separator space

    let scroll_top = viewport.top_line;

    let mut lines: Vec<Line> = Vec::with_capacity(inner_height);

    for i in 0..inner_height {
        let line_idx = scroll_top + i;
        if line_idx >= line_count {
            let gutter = format!("{:>width$} ", "~", width = gutter_width - 1);
            lines.push(Line::from(vec![Span::styled(
                gutter,
                Style::default().fg(Color::DarkGray),
            )]));
            continue;
        }

        let line_content = buffer.line(line_idx).unwrap_or_default();

        // Check if this line has a task
        let task = task_for_line(&app.tasks, line_idx);
        let is_cursor_line = line_idx == cursor.line;

        // Relative line numbers: absolute on cursor line, distance on others
        let line_num = if is_cursor_line {
            format!("{:>width$} ", line_idx + 1, width = gutter_width - 1)
        } else {
            let rel = (line_idx as isize - cursor.line as isize).unsigned_abs();
            format!("{:>width$} ", rel, width = gutter_width - 1)
        };

        let line_num_style = if is_cursor_line {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let mut spans = vec![Span::styled(line_num, line_num_style)];

        if line_content.is_empty() {
            if is_cursor_line {
                let cursor_style = Style::default().bg(Color::White).fg(Color::Black);
                spans.push(Span::styled(" ".to_string(), cursor_style));
            }
        } else {
            let cursor_col = if is_cursor_line {
                Some(cursor.col.min(line_content.len().saturating_sub(1)))
            } else {
                None
            };

            let task_col = task.map(|t| t.target_col);
            let task_style = task.map(|t| task_char_style(t));

            // Compute visual selection range for this line
            let visual_range = if app.mode.is_visual() {
                visual_line_range(app, line_idx, line_content.len())
            } else {
                None
            };

            // Build spans character by character only when needed,
            // otherwise use sliced spans for performance.
            if task_col.is_some() || cursor_col.is_some() || visual_range.is_some() {
                // We need character-level control
                let chars: Vec<char> = line_content.chars().collect();
                let mut col = 0;
                let mut run_start = 0;

                // Helper: flush a run of normal characters
                let flush_run = |spans: &mut Vec<Span>, content: &str, start: usize, end: usize| {
                    if end > start {
                        spans.push(Span::raw(content[start..end].to_string()));
                    }
                };

                for (i, _ch) in chars.iter().enumerate() {
                    let byte_pos = line_content
                        .char_indices()
                        .nth(i)
                        .map(|(b, _)| b)
                        .unwrap_or(0);
                    let next_byte = line_content
                        .char_indices()
                        .nth(i + 1)
                        .map(|(b, _)| b)
                        .unwrap_or(line_content.len());

                    let is_cursor = cursor_col == Some(i);
                    let is_task_target = task_col == Some(i) && task_style.is_some();
                    let is_visual = visual_range
                        .map(|(vs, ve)| i >= vs && i < ve)
                        .unwrap_or(false);

                    if is_cursor || is_task_target || is_visual {
                        // Flush any accumulated normal text
                        flush_run(&mut spans, &line_content, run_start, byte_pos);

                        let ch_str = line_content[byte_pos..next_byte].to_string();
                        if is_cursor {
                            // Cursor always wins visually
                            spans.push(Span::styled(
                                ch_str,
                                Style::default().bg(Color::White).fg(Color::Black),
                            ));
                        } else if is_task_target {
                            // Task target character
                            spans.push(Span::styled(ch_str, task_style.unwrap()));
                        } else {
                            // Visual selection highlight
                            spans.push(Span::styled(
                                ch_str,
                                Style::default().bg(Color::Rgb(80, 80, 140)).fg(Color::White),
                            ));
                        }
                        run_start = next_byte;
                    }
                    col = i;
                }
                // Flush remaining
                if run_start < line_content.len() {
                    spans.push(Span::raw(line_content[run_start..].to_string()));
                }
                let _ = col; // suppress unused warning
            } else {
                // No cursor, no task — simple raw span
                spans.push(Span::raw(line_content.clone()));
            }
        }

        // Append task annotation
        if let Some(t) = task {
            let (annotation, ann_color) = task_annotation(t);
            spans.push(Span::styled(annotation, Style::default().fg(ann_color)));
        }

        lines.push(Line::from(spans));
    }

    let title = " Vim Heroes ";
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

/// Compute the column range (start_col, end_col_exclusive) of the visual selection on a line.
/// Returns None if this line is not part of the selection.
fn visual_line_range(app: &App, line_idx: usize, line_len: usize) -> Option<(usize, usize)> {
    let anchor = app.visual_anchor;
    let cursor = app.cursor;

    match app.mode {
        Mode::Visual => {
            let (start, end) = if (anchor.line, anchor.col) <= (cursor.line, cursor.col) {
                (anchor, cursor)
            } else {
                (cursor, anchor)
            };

            if line_idx < start.line || line_idx > end.line {
                return None;
            }

            let col_start = if line_idx == start.line { start.col } else { 0 };
            let col_end = if line_idx == end.line {
                (end.col + 1).min(line_len)
            } else {
                line_len
            };

            if col_start >= col_end && col_start >= line_len {
                return None;
            }
            Some((col_start, col_end.max(col_start + 1)))
        }
        Mode::VisualLine => {
            let start_line = anchor.line.min(cursor.line);
            let end_line = anchor.line.max(cursor.line);

            if line_idx >= start_line && line_idx <= end_line {
                Some((0, line_len.max(1)))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Render the status bar showing mode, cursor position, score, and scroll progress.
fn render_status_bar(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    // If command line is active, show it
    if let Some(ref cmd) = app.cmdline {
        let cmd_line = Paragraph::new(Line::from(vec![
            Span::styled(
                ":",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                cmd.clone(),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                "\u{2588}",
                Style::default().fg(Color::Yellow),
            ),
        ]))
        .style(Style::default().bg(Color::Rgb(30, 30, 30)));
        frame.render_widget(cmd_line, area);
        return;
    }

    // If search input is active, show the search line instead
    if app.search.active {
        let prompt = app.search.prompt_char();
        let input = &app.search.input_buf;
        let search_line = Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{}", prompt),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                input.clone(),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                "\u{2588}",
                Style::default().fg(Color::Yellow),
            ),
        ]))
        .style(Style::default().bg(Color::Rgb(30, 30, 30)));
        frame.render_widget(search_line, area);
        return;
    }

    let mode_str = match app.mode {
        Mode::Normal => " NORMAL ",
        Mode::Insert => " INSERT ",
        Mode::Replace => " REPLACE ",
        Mode::Visual => " VISUAL ",
        Mode::VisualLine => " V-LINE ",
    };
    let mode_color = match app.mode {
        Mode::Normal => Color::Blue,
        Mode::Insert => Color::Green,
        Mode::Replace => Color::Red,
        Mode::Visual | Mode::VisualLine => Color::Magenta,
    };

    let position = format!(
        " Ln {}, Col {} ",
        app.cursor.line + 1,
        app.cursor.col + 1
    );
    let score = format!(" Score: {} ", app.scoring.score);
    let scroll_progress = format!(
        " {}/{} ",
        app.viewport.top_line + 1,
        app.buffer.line_count()
    );
    let keys = format!(" Keys: {} ", app.scoring.keystrokes);

    // Find the next active/pending task description
    let next_task = app
        .tasks
        .iter()
        .find(|t| matches!(t.state, TaskState::Active | TaskState::Pending));
    let task_info = match next_task {
        Some(t) => format!(" \u{25b8} {} ", t.description),
        None => String::new(),
    };

    let tasks_done = app.tasks.iter().filter(|t| t.state == TaskState::Completed).count();
    let tasks_total = app.tasks.len();
    let task_progress = format!(" Tasks: {}/{} ", tasks_done, tasks_total);

    let spans = vec![
        Span::styled(
            mode_str,
            Style::default()
                .bg(mode_color)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(position, Style::default().fg(Color::White)),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(score, Style::default().fg(Color::Green)),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(keys, Style::default().fg(Color::Yellow)),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(scroll_progress, Style::default().fg(Color::Cyan)),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(task_progress, Style::default().fg(Color::Magenta)),
        Span::styled(task_info, Style::default().fg(Color::Red)),
    ];

    let status_line = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::Rgb(30, 30, 30)));
    frame.render_widget(status_line, area);
}

/// Render the results overlay (game over or level complete).
fn render_results(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let s = &app.scoring;
    let stars = s.star_display_full();
    let star_color = if s.is_perfect() { GOLD } else { Color::Yellow };
    let tasks_missed = app.tasks.iter().filter(|t| t.state == TaskState::Missed).count();
    let is_complete = app.engine.state == GameState::LevelComplete;

    let (title_text, title_color, border_color, border_title) = if is_complete && s.is_perfect() {
        ("PERFECT RUN!", GOLD, GOLD, " Results ")
    } else if is_complete {
        ("LEVEL COMPLETE!", Color::Green, Color::Green, " Results ")
    } else {
        ("GAME OVER", Color::Red, Color::Red, " Results ")
    };

    let reason_text = match app.game_over_reason {
        GameOverReason::MissedTask => "Task missed!",
        GameOverReason::CursorOffScreen => "Cursor left the screen!",
        GameOverReason::EnergyDepleted => "Out of energy!",
        GameOverReason::None => "",
    };

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            title_text,
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        )),
        if reason_text.is_empty() {
            Line::from("")
        } else {
            Line::from(Span::styled(
                reason_text,
                Style::default().fg(Color::Red),
            ))
        },
        Line::from(""),
        Line::from(Span::styled(
            stars,
            Style::default()
                .fg(star_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Score: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", s.score), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Tasks: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}/{}", s.tasks_completed, s.tasks_total),
                Style::default().fg(Color::Magenta),
            ),
            if tasks_missed > 0 {
                Span::styled(
                    format!(" ({} missed)", tasks_missed),
                    Style::default().fg(Color::Yellow),
                )
            } else {
                Span::raw("")
            },
        ]),
        Line::from(vec![
            Span::styled("Great: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", s.tasks_great),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled("  Perfect: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", s.tasks_perfect),
                Style::default().fg(if s.is_perfect() { GOLD } else { Color::Yellow }),
            ),
            Span::styled(
                format!("  /{}", s.tasks_total),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("Keys: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", s.keystrokes),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("Max combo: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", s.max_combo),
                Style::default().fg(Color::Red),
            ),
        ]),
        Line::from(vec![
            Span::styled("Time: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}s", app.engine.elapsed_secs()),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("Energy: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.0}%", app.energy.percentage()),
                Style::default().fg(if app.energy.percentage() > 30.0 {
                    Color::Green
                } else {
                    Color::Red
                }),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            format!("Level {}", app.level.display_id()),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "R retry \u{2502} N next level \u{2502} Q quit",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let width: u16 = 34;
    let height: u16 = text.len() as u16 + 2;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect::new(x, y, width, height);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(border_title);

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}

/// Render the countdown overlay (3... 2... 1...).
fn render_countdown(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let remaining = app.engine.countdown_remaining();
    let num_str = if remaining == 0 {
        "GO!".to_string()
    } else {
        format!("{}", remaining)
    };

    let color = match remaining {
        3 => Color::Red,
        2 => Color::Yellow,
        1 => Color::Green,
        _ => Color::Green,
    };

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!(" Level {} ", app.level.display_id()),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("\"{}\"", app.level.name),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            num_str,
            Style::default()
                .fg(color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    let width: u16 = 28;
    let height: u16 = text.len() as u16 + 2;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect::new(x, y, width, height);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Get Ready ");

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}
