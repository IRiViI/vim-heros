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

    if app.engine.state == GameState::GameOver {
        render_game_over(frame, app, chunks[1]);
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
            " Level 1-1 ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("\"First Steps\""),
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

/// Get the style info for a task line: (background color, annotation text, annotation color).
fn task_line_style(task: &Task) -> (Color, String, Color) {
    match task.state {
        TaskState::Pending | TaskState::Active => (
            Color::Rgb(60, 20, 20),
            format!("  \u{25c0} {} ", task.gutter_text),
            Color::Red,
        ),
        TaskState::Completed => (
            Color::Rgb(20, 60, 20),
            " \u{2713} DONE ".to_string(),
            Color::Green,
        ),
        TaskState::Missed => (
            Color::Rgb(60, 60, 20),
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
        let task_bg = task.map(|t| task_line_style(t));

        if line_idx == cursor.line {
            // Cursor line — build span by span
            let mut spans = vec![Span::styled(
                line_num,
                Style::default().fg(Color::Yellow),
            )];

            let cursor_style = Style::default().bg(Color::White).fg(Color::Black);
            // Task background for non-cursor characters on this line
            let content_style = match &task_bg {
                Some((bg, _, _)) => Style::default().bg(*bg),
                None => Style::default(),
            };

            if line_content.is_empty() {
                spans.push(Span::styled(" ".to_string(), cursor_style));
            } else {
                let cursor_col = cursor.col.min(line_content.len().saturating_sub(1));

                if cursor_col > 0 {
                    spans.push(Span::styled(
                        line_content[..cursor_col].to_string(),
                        content_style,
                    ));
                }

                let cursor_char = if cursor_col < line_content.len() {
                    line_content[cursor_col..cursor_col + 1].to_string()
                } else {
                    " ".to_string()
                };
                spans.push(Span::styled(cursor_char, cursor_style));

                if cursor_col + 1 < line_content.len() {
                    spans.push(Span::styled(
                        line_content[cursor_col + 1..].to_string(),
                        content_style,
                    ));
                }
            }

            // Append task annotation if present
            if let Some((_, ref annotation, ann_color)) = task_bg {
                spans.push(Span::styled(annotation.clone(), Style::default().fg(ann_color)));
            }

            lines.push(Line::from(spans));
        } else if let Some((bg, ref annotation, ann_color)) = task_bg {
            // Task line (no cursor)
            lines.push(Line::from(vec![
                Span::styled(line_num, Style::default().fg(Color::DarkGray)),
                Span::styled(line_content, Style::default().bg(bg)),
                Span::styled(annotation.clone(), Style::default().fg(ann_color)),
            ]));
        } else {
            // Normal line
            lines.push(Line::from(vec![
                Span::styled(line_num, Style::default().fg(Color::DarkGray)),
                Span::raw(line_content),
            ]));
        }
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
    };
    let mode_color = match app.mode {
        Mode::Normal => Color::Blue,
        Mode::Insert => Color::Green,
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

/// Render the game over / results overlay.
fn render_game_over(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let s = &app.scoring;
    let stars = s.star_display();
    let tasks_missed = app.tasks.iter().filter(|t| t.state == TaskState::Missed).count();

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "GAME OVER",
            Style::default()
                .fg(Color::Red)
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
            "R to retry \u{2502} Q to quit",
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
        .border_style(Style::default().fg(Color::Red))
        .title(" Results ");

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}
