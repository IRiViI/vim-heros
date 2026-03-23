use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::App;
use crate::game::engine::GameState;
use crate::game::task::{Task, TaskState};
use crate::vim::mode::Mode;

/// Render the full game view into a ratatui frame.
pub fn render(frame: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // HUD bar
            Constraint::Min(3),    // buffer area
            Constraint::Length(1), // status bar
        ])
        .split(frame.area());

    render_hud(frame, app, chunks[0]);
    render_buffer(frame, app, chunks[1]);
    render_status_bar(frame, app, chunks[2]);

    if matches!(app.engine.state, GameState::GameOver | GameState::LevelComplete) {
        render_results(frame, app, chunks[1]);
    }

    if app.engine.state == GameState::Countdown {
        render_countdown(frame, app, chunks[1]);
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

/// Task highlight style for the single target character.
fn task_char_style(task: &Task) -> Style {
    match task.state {
        TaskState::Pending | TaskState::Active => {
            Style::default().bg(Color::Red).fg(Color::White).add_modifier(Modifier::BOLD)
        }
        TaskState::Completed => {
            Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD)
        }
        TaskState::Missed => {
            Style::default().bg(Color::Yellow).fg(Color::Black)
        }
    }
}

/// Gutter annotation for a task.
fn task_annotation(task: &Task) -> (String, Color) {
    match task.state {
        TaskState::Pending | TaskState::Active => (
            format!("  \u{25c0} {} ", task.gutter_text),
            Color::Red,
        ),
        TaskState::Completed => (
            " \u{2713} DONE ".to_string(),
            Color::Green,
        ),
        TaskState::Missed => (
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
        let line_num = format!("{:>width$} ", line_idx + 1, width = gutter_width - 1);

        // Check if this line has a task
        let task = task_for_line(&app.tasks, line_idx);
        let is_cursor_line = line_idx == cursor.line;

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

            // Build spans character by character only when needed,
            // otherwise use sliced spans for performance.
            if task_col.is_some() || cursor_col.is_some() {
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

                    if is_cursor || is_task_target {
                        // Flush any accumulated normal text
                        flush_run(&mut spans, &line_content, run_start, byte_pos);

                        let ch_str = line_content[byte_pos..next_byte].to_string();
                        if is_cursor {
                            // Cursor always wins visually
                            spans.push(Span::styled(
                                ch_str,
                                Style::default().bg(Color::White).fg(Color::Black),
                            ));
                        } else {
                            // Task target character
                            spans.push(Span::styled(ch_str, task_style.unwrap()));
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

/// Render the status bar showing mode, cursor position, score, and scroll progress.
fn render_status_bar(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let mode_str = match app.mode {
        Mode::Normal => " NORMAL ",
        Mode::Insert => " INSERT ",
        Mode::Replace => " REPLACE ",
    };
    let mode_color = match app.mode {
        Mode::Normal => Color::Blue,
        Mode::Insert => Color::Green,
        Mode::Replace => Color::Red,
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
    let stars = s.star_display();
    let tasks_missed = app.tasks.iter().filter(|t| t.state == TaskState::Missed).count();
    let is_complete = app.engine.state == GameState::LevelComplete;

    let (title_text, title_color, border_color, border_title) = if is_complete {
        ("LEVEL COMPLETE!", Color::Green, Color::Green, " Results ")
    } else {
        ("GAME OVER", Color::Red, Color::Red, " Results ")
    };

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            title_text,
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            stars,
            Style::default()
                .fg(Color::Yellow)
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
