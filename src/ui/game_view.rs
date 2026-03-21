use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::vim::mode::Mode;

/// Render the full game view into a ratatui frame.
pub fn render(frame: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // buffer area
            Constraint::Length(1), // status bar
        ])
        .split(frame.area());

    render_buffer(frame, app, chunks[0]);
    render_status_bar(frame, app, chunks[1]);
}

/// Render the text buffer with line numbers and cursor.
fn render_buffer(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let buffer = &app.buffer;
    let cursor = &app.cursor;

    // How many lines we can display (inside the border)
    let inner_height = area.height.saturating_sub(2) as usize; // 2 for top+bottom border
    let line_count = buffer.line_count();

    // Calculate the gutter width (line numbers)
    let max_line_num = line_count;
    let gutter_width = format!("{}", max_line_num).len() + 1; // +1 for separator space

    // Determine which lines to show (scroll to keep cursor visible)
    let scroll_top = if cursor.line >= inner_height {
        cursor.line - inner_height + 1
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::with_capacity(inner_height);

    for i in 0..inner_height {
        let line_idx = scroll_top + i;
        if line_idx >= line_count {
            let gutter = format!("{:>width$} ", "~", width = gutter_width - 1);
            lines.push(Line::from(vec![
                Span::styled(gutter, Style::default().fg(Color::DarkGray)),
            ]));
            continue;
        }

        let line_content = buffer.line(line_idx).unwrap_or_default();
        let line_num = format!("{:>width$} ", line_idx + 1, width = gutter_width - 1);

        if line_idx == cursor.line {
            // This line has the cursor — build it span by span
            let mut spans = vec![
                Span::styled(line_num, Style::default().fg(Color::Yellow)),
            ];

            let cursor_style = Style::default().bg(Color::White).fg(Color::Black);

            if line_content.is_empty() {
                spans.push(Span::styled(" ".to_string(), cursor_style));
            } else {
                let cursor_col = cursor.col.min(line_content.len().saturating_sub(1));

                // Text before cursor
                if cursor_col > 0 {
                    spans.push(Span::raw(line_content[..cursor_col].to_string()));
                }

                // Cursor character
                let cursor_char = if cursor_col < line_content.len() {
                    line_content[cursor_col..cursor_col + 1].to_string()
                } else {
                    " ".to_string()
                };
                spans.push(Span::styled(cursor_char, cursor_style));

                // Text after cursor
                if cursor_col + 1 < line_content.len() {
                    spans.push(Span::raw(line_content[cursor_col + 1..].to_string()));
                }
            }

            lines.push(Line::from(spans));
        } else {
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

/// Render the status bar showing mode, cursor position, and keystroke count.
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
    let keys = format!(" Keys: {} ", app.keystroke_count);
    let quit_hint = match app.mode {
        Mode::Normal => " q:quit  i:insert ",
        Mode::Insert => " Esc:normal ",
    };

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
        Span::styled(keys, Style::default().fg(Color::Yellow)),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(quit_hint, Style::default().fg(Color::DarkGray)),
    ];

    let status_line = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::Rgb(30, 30, 30)));
    frame.render_widget(status_line, area);
}
