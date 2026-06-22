//! Rendering. The layout is: header (title + progress), a middle row split into
//! the selectable tool list and a detail panel, a scrolling log, and a help bar.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Padding, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, Level};
use mokio_core::installer::Status;

pub fn draw(f: &mut Frame, app: &mut App) {
    let area: Rect = f.area();
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(14),
            Constraint::Length(1),
        ])
        .split(area);

    draw_header(f, app, vert[0]);
    let mid = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(vert[1]);
    draw_list(f, app, mid[0]);
    draw_detail(f, app, mid[1]);
    draw_log(f, app, vert[2]);
    draw_footer(f, vert[3]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let title = Line::from(vec![
        Span::styled(
            " Mokio ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            "one-click dev toolchain bootstrap",
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    let block = Block::default().borders(Borders::ALL).title(title);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let installed = app
        .statuses
        .values()
        .filter(|s| s.is_installed())
        .count();
    let total = app.infos.len();

    if app.running {
        let pct = if app.total_count > 0 {
            (app.done_count as u16 * 100) / app.total_count as u16
        } else {
            0
        };
        let label = format!(
            "Installing {} ({}/{})",
            app.current.as_deref().unwrap_or("…"),
            app.done_count,
            app.total_count
        );
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(Color::Cyan))
            .percent(pct)
            .label(label);
        f.render_widget(gauge, inner);
    } else {
        let line = Line::from(vec![
            Span::styled(
                format!(" {installed}/{total} tools installed "),
                Style::default().fg(Color::Green),
            ),
            Span::raw("  "),
            Span::styled(
                if app.finished {
                    "✓ run finished — press <r> to re-detect, <i> to run again"
                } else {
                    "ready — <Space> toggle · <i> install · <r> re-detect · <q> quit"
                },
                Style::default().fg(Color::Gray),
            ),
        ]);
        f.render_widget(Paragraph::new(line), inner);
    }
}

fn draw_list(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Tools ",
            Style::default().add_modifier(Modifier::BOLD),
        ))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line> = app
        .infos
        .iter()
        .enumerate()
        .map(|(idx, info)| {
            let selected = app.selected.contains(&info.id);
            let is_cursor = idx == app.cursor;
            let check = if selected { "✓" } else { " " };
            let (glyph, gcolor) = status_glyph(app.statuses.get(&info.id));

            let mut spans = vec![
                Span::styled(
                    format!("{check} "),
                    Style::default().fg(if selected {
                        Color::Cyan
                    } else {
                        Color::DarkGray
                    }),
                ),
                Span::styled(format!("{glyph} "), Style::default().fg(gcolor)),
                Span::styled(
                    info.name.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ];
            let mut tags = String::new();
            if info.id == "homebrew" {
                tags.push_str(" [foundation]");
            }
            if !info.requires.is_empty() {
                tags.push_str(&format!("  needs: {}", info.requires.join(", ")));
            }
            if !tags.is_empty() {
                spans.push(Span::styled(
                    tags,
                    Style::default().fg(Color::DarkGray),
                ));
            }

            let mut line = Line::from(spans);
            if is_cursor {
                line = line.style(Style::default().bg(Color::DarkGray));
            }
            line
        })
        .collect();

    let para = Paragraph::new(lines);
    f.render_widget(para, inner);
}

fn draw_detail(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Details ",
            Style::default().add_modifier(Modifier::BOLD),
        ))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let info = match app.cursor_info() {
        Some(i) => i,
        None => return,
    };
    let (glyph, gcolor) = status_glyph(app.statuses.get(&info.id));
    let selected = app.selected.contains(&info.id);

    let lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{glyph}  "),
                Style::default().fg(gcolor),
            ),
            Span::styled(
                info.name.clone(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            info.description.clone(),
            Style::default().fg(Color::Gray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Category:  ", Style::default().fg(Color::DarkGray)),
            Span::raw(info.category.label()),
        ]),
        Line::from(vec![
            Span::styled("Homepage:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(info.homepage.clone(), Style::default().fg(Color::Blue)),
        ]),
        Line::from(vec![
            Span::styled("Selected:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if selected { "yes" } else { "no" },
                Style::default().fg(if selected {
                    Color::Green
                } else {
                    Color::DarkGray
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("Status:    ", Style::default().fg(Color::DarkGray)),
            Span::raw(status_text(app.statuses.get(&info.id))),
        ]),
    ];

    let para = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(para, inner);
}

fn draw_log(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Log ",
            Style::default().add_modifier(Modifier::BOLD),
        ))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let height = inner.height as usize;
    let take = app.log.len().saturating_sub(height);
    let visible: Vec<&crate::app::LogLine> = app.log.iter().skip(take).collect();
    let lines: Vec<Line> = visible
        .iter()
        .map(|l| {
            let (prefix, color) = match l.level {
                Level::Phase => ("▶ ", Color::Cyan),
                Level::Info => ("  ", Color::Gray),
                Level::Log => ("  ", Color::DarkGray),
                Level::Warn => ("! ", Color::Red),
            };
            Line::from(Span::styled(
                format!("{prefix}{}", l.text),
                Style::default().fg(color),
            ))
        })
        .collect();

    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_footer(f: &mut Frame, area: Rect) {
    let help = " ↑/↓ move · Space toggle · i install · a all · n none · r re-detect · q quit ";
    let para = Paragraph::new(Span::styled(
        help,
        Style::default().fg(Color::DarkGray),
    ))
    .alignment(Alignment::Center);
    f.render_widget(para, area);
}

fn status_glyph(status: Option<&Status>) -> (&'static str, Color) {
    match status {
        Some(Status::Installed { .. }) => ("✓", Color::Green),
        Some(Status::NotInstalled) => ("○", Color::Yellow),
        Some(Status::Unknown) | None => ("·", Color::DarkGray),
    }
}

fn status_text(status: Option<&Status>) -> String {
    match status {
        Some(Status::Installed { version: Some(v) }) => format!("installed ({v})"),
        Some(Status::Installed { version: None }) => "installed".to_string(),
        Some(Status::NotInstalled) => "not installed".to_string(),
        Some(Status::Unknown) | None => "unknown".to_string(),
    }
}
