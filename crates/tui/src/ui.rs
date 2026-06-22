//! Rendering. The layout is: header (title + progress), a middle row split into
//! the selectable tool list and a detail panel, a scrolling log, and a help bar.
//! All visible strings come from [`mokio_core::i18n`] using `app.lang`.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Padding, Paragraph, Wrap};
use ratatui::Frame;

use mokio_core::i18n;
use mokio_core::installer::Status;
use mokio_core::Lang;

use crate::app::{App, Level};

pub fn draw(f: &mut Frame, app: &mut App) {
    let area: Rect = f.area();
    // Clamp log height so header(3) + middle(min 8) + footer(1) always fit.
    let max_log = area.height.saturating_sub(12).max(6);
    let log_height = app.log_height.min(max_log).max(6);
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(log_height),
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
    draw_footer(f, app, vert[3]);
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
            i18n::ui(app.lang, "tagline"),
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    let block = Block::default().borders(Borders::ALL).title(title);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let installed = app.statuses.values().filter(|s| s.is_installed()).count();
    let total = app.infos.len();

    if app.running {
        let pct = if app.total_count > 0 {
            (app.done_count as u16 * 100) / app.total_count as u16
        } else {
            0
        };
        let label = i18n::ui(app.lang, "gauge_label")
            .replace("{current}", app.current.as_deref().unwrap_or("…"))
            .replace("{done}", &app.done_count.to_string())
            .replace("{total}", &app.total_count.to_string());
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(Color::Cyan))
            .percent(pct)
            .label(label);
        f.render_widget(gauge, inner);
    } else {
        let count = i18n::ui(app.lang, "hdr_count")
            .replace("{installed}", &installed.to_string())
            .replace("{total}", &total.to_string());
        let status_key = if app.finished { "hdr_finished" } else { "hdr_ready" };
        let line = Line::from(vec![
            Span::styled(count, Style::default().fg(Color::Green)),
            Span::raw("  "),
            Span::styled(
                i18n::ui(app.lang, status_key),
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
            i18n::ui(app.lang, "panel_tools"),
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
                tags.push_str(i18n::ui(app.lang, "foundation"));
            }
            if !info.requires.is_empty() {
                tags.push_str(
                    &i18n::ui(app.lang, "needs").replace("{list}", &info.requires.join(", ")),
                );
            }
            if !tags.is_empty() {
                spans.push(Span::styled(tags, Style::default().fg(Color::DarkGray)));
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
            i18n::ui(app.lang, "panel_details"),
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
            Span::styled(format!("{glyph}  "), Style::default().fg(gcolor)),
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
            Span::styled(
                i18n::ui(app.lang, "label_category"),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(i18n::category_label(app.lang, info.category)),
        ]),
        Line::from(vec![
            Span::styled(
                i18n::ui(app.lang, "label_homepage"),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(info.homepage.clone(), Style::default().fg(Color::Blue)),
        ]),
        Line::from(vec![
            Span::styled(
                i18n::ui(app.lang, "label_selected"),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                i18n::ui(app.lang, if selected { "sel_yes" } else { "sel_no" }),
                Style::default().fg(if selected {
                    Color::Green
                } else {
                    Color::DarkGray
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                i18n::ui(app.lang, "label_status"),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(status_text(app.lang, app.statuses.get(&info.id))),
        ]),
    ];

    let para = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(para, inner);
}

fn draw_log(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            i18n::ui(app.lang, "panel_log"),
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

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let help = i18n::ui(app.lang, "footer_help");
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

fn status_text(lang: Lang, status: Option<&Status>) -> String {
    let key = match status {
        Some(Status::Installed { version: Some(_) }) => "st_installed_v",
        Some(Status::Installed { version: None }) => "st_installed",
        Some(Status::NotInstalled) => "st_not_installed",
        Some(Status::Unknown) | None => "st_unknown",
    };
    let s = i18n::ui(lang, key);
    if let Some(Status::Installed { version: Some(v) }) = status {
        s.replace("{v}", v)
    } else {
        s.to_string()
    }
}
