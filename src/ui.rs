use std::time::SystemTime;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, Paragraph};

use crate::app::{App, Column, Mode, Row, Stage};
use crate::model::builds::{Build, BuildStatus};
use crate::model::prs::{CheckConclusion, CheckState, PullRequest};
use crate::model::queue::QueueEntry;
use crate::text::{age, clock, duration, pad_right, refreshed};

const HIGHLIGHT: &str = "> ";

pub const KEYS: [(&str, &str); 10] = [
    ("j/k ↓/↑", "move"),
    ("1-9", "jump to row"),
    ("h/l Tab", "focus column"),
    ("Enter/o", "open in browser"),
    ("y", "copy URL"),
    ("p", "details"),
    ("/", "filter"),
    ("r", "refresh"),
    ("?", "this help"),
    ("q Esc", "quit"),
];

pub fn draw(frame: &mut Frame, app: &mut App) {
    let [body, footer] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());
    if app.mode == Mode::Help {
        draw_help(frame, body);
        frame.render_widget(Paragraph::new("any key returns"), footer);
        return;
    }
    let body = if app.details {
        let details_percent = app.config.ui.details_percent;
        let [columns, details] =
            Layout::vertical([Constraint::Fill(1), Constraint::Percentage(details_percent)])
                .areas(body);
        draw_details(frame, app, details);
        columns
    } else {
        body
    };
    draw_footer(frame, app, footer);
    if body.width < app.config.ui.min_column_width * Stage::ALL.len() as u16 {
        draw_tabs(frame, app, body);
    } else {
        let columns = Layout::horizontal([Constraint::Fill(1); 4]).split(body);
        for (stage, area) in Stage::ALL.into_iter().zip(columns.iter()) {
            draw_column(frame, app, stage, *area, true);
        }
    }
}

fn draw_tabs(frame: &mut Frame, app: &mut App, area: Rect) {
    let [bar, body] = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(area);
    let mut spans = Vec::new();
    for (i, stage) in Stage::ALL.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" │ "));
        }
        let style = if stage == app.focus {
            Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            dim()
        };
        spans.push(Span::styled(column_title(app, stage), style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), bar);
    draw_column(frame, app, app.focus, body, false);
}

fn titled<T: Row>(base: &str, column: &Column<T>, spinner: char) -> String {
    let warning = if column.error.is_some() { " ⚠" } else { "" };
    let spin = if column.refreshing {
        format!(" {spinner}")
    } else {
        String::new()
    };
    if column.is_loading() {
        format!("{base}{warning}{spin}")
    } else {
        format!("{base} ({}){warning}{spin}", column.all().len())
    }
}

fn column_title(app: &App, stage: Stage) -> String {
    match stage {
        Stage::Prs => titled("My PRs", &app.prs, app.spinner()),
        Stage::Queue => match &app.queue_repo {
            Some(repo) => titled(
                &format!("Queue {}", short_repo(repo)),
                &app.queue,
                app.spinner(),
            ),
            None => titled("Merge queue", &app.queue, app.spinner()),
        },
        Stage::Builds => match &app.builds_repo {
            Some(repo) => titled(
                &format!("Main {}", short_repo(repo)),
                &app.builds,
                app.spinner(),
            ),
            None => titled("Main builds", &app.builds, app.spinner()),
        },
        Stage::Deployed => "Deployed".to_string(),
    }
}

fn short_repo(repo: &str) -> &str {
    repo.rsplit('/').next().unwrap_or(repo)
}

fn column_block(app: &App, stage: Stage, titled: bool) -> Block<'static> {
    if !titled {
        return Block::bordered();
    }
    let style = if stage == app.focus {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        dim()
    };
    Block::bordered().title(Span::styled(column_title(app, stage), style))
}

fn draw_column(frame: &mut Frame, app: &mut App, stage: Stage, area: Rect, titled: bool) {
    let block = column_block(app, stage, titled);
    let now = app.now;
    let spinner = app.spinner();
    match stage {
        Stage::Prs => draw_list(
            frame,
            &mut app.prs,
            area,
            block,
            "no open pull requests",
            spinner,
            |i, pr, w| pr_row(i, pr, now, w),
        ),
        Stage::Queue => draw_list(
            frame,
            &mut app.queue,
            area,
            block,
            "queue empty",
            spinner,
            |_, entry, w| queue_row(entry, w),
        ),
        Stage::Builds => draw_list(
            frame,
            &mut app.builds,
            area,
            block,
            "no builds on main",
            spinner,
            |i, build, w| build_row(i, build, now, w),
        ),
        Stage::Deployed => frame.render_widget(Paragraph::new("not configured").block(block), area),
    }
}

fn build_label(build: &Build) -> (String, String) {
    match (&build.pull, build.pr_number) {
        (Some(pull), _) => (format!("#{}", pull.number), pull.author.clone()),
        (None, Some(number)) => (format!("#{number}"), build.actor.clone()),
        (None, None) => (format!("run {}", build.run_number), build.actor.clone()),
    }
}

fn build_row(index: usize, build: &Build, now: SystemTime, width: usize) -> Line<'static> {
    let (label, who) = build_label(build);
    let title = build
        .pull
        .as_ref()
        .map(|p| p.title.clone())
        .unwrap_or_else(|| build.title.clone());
    let took = build
        .duration()
        .or_else(|| build.elapsed(now))
        .map(duration)
        .unwrap_or_default();
    let started = build.started_at.map(|at| age(at, now)).unwrap_or_default();
    line(
        row_number(index),
        build_glyph(build.status),
        format!("{label} {who}  {title}"),
        format!("{took} {started}"),
        width,
    )
}

fn build_glyph(status: BuildStatus) -> (char, Color) {
    match status {
        BuildStatus::Success => ('✓', Color::Green),
        BuildStatus::Failure => ('✗', Color::Red),
        BuildStatus::Running => ('●', Color::Yellow),
        BuildStatus::Queued => ('○', Color::Yellow),
        BuildStatus::Cancelled => ('-', Color::DarkGray),
        BuildStatus::Unknown => ('?', Color::DarkGray),
    }
}

fn status_word(status: BuildStatus) -> &'static str {
    match status {
        BuildStatus::Success => "success",
        BuildStatus::Failure => "failure",
        BuildStatus::Running => "running",
        BuildStatus::Queued => "queued",
        BuildStatus::Cancelled => "cancelled",
        BuildStatus::Unknown => "?",
    }
}

fn build_details(build: &Build, now: SystemTime, utc_offset_secs: i32) -> Vec<Line<'static>> {
    let took = build
        .duration()
        .or_else(|| build.elapsed(now))
        .map(duration)
        .unwrap_or_default();
    let started = build
        .started_at
        .map(|at| format!("started at {}", clock(at, utc_offset_secs)))
        .unwrap_or_default();
    let mut lines = vec![Line::from(format!(
        "run {}  {}  {took}  {started}  by {}",
        build.run_number,
        status_word(build.status),
        build.actor
    ))];
    match &build.pull {
        Some(pull) => lines.push(Line::from(vec![
            Span::raw(format!(
                "#{} {}  {}  ",
                pull.number, pull.author, pull.title
            )),
            Span::styled(pull.url.clone(), dim()),
        ])),
        None => lines.push(Line::from(build.title.clone())),
    }
    lines.push(Line::from(vec![
        Span::styled(build.sha.clone(), dim()),
        Span::raw("  "),
        Span::styled(build.url.clone(), dim()),
    ]));
    lines
}

fn draw_list<T: Row>(
    frame: &mut Frame,
    column: &mut Column<T>,
    area: Rect,
    block: Block<'static>,
    empty: &str,
    spinner: char,
    row: impl Fn(usize, &T, usize) -> Line<'static>,
) {
    if column.is_loading() {
        let body = column
            .error
            .clone()
            .unwrap_or_else(|| format!("{spinner} fetching…"));
        frame.render_widget(Paragraph::new(body).block(block), area);
        return;
    }
    if column.all().is_empty() {
        frame.render_widget(Paragraph::new(empty.to_string()).block(block), area);
        return;
    }
    let visible = column.visible();
    if visible.is_empty() {
        let query = column.filter.clone().unwrap_or_default();
        frame.render_widget(
            Paragraph::new(format!("no matches for /{query}")).block(block),
            area,
        );
        return;
    }
    let width = usize::from(area.width.saturating_sub(2 + HIGHLIGHT.len() as u16));
    let items: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .map(|(i, item)| ListItem::new(row(i, item, width)))
        .collect();
    let list = List::new(items).block(block).highlight_symbol(HIGHLIGHT);
    frame.render_stateful_widget(list, area, &mut column.list);
}

fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

fn line(
    number: String,
    glyph: (char, Color),
    text: String,
    right: String,
    width: usize,
) -> Line<'static> {
    let text_width = width.saturating_sub(number.chars().count() + 3 + right.chars().count() + 1);
    Line::from(vec![
        Span::styled(number, dim()),
        Span::raw(" "),
        Span::styled(glyph.0.to_string(), Style::default().fg(glyph.1)),
        Span::raw(" "),
        Span::raw(pad_right(&text, text_width)),
        Span::raw(" "),
        Span::styled(right, dim()),
    ])
}

fn row_number(index: usize) -> String {
    if index < 9 {
        (index + 1).to_string()
    } else {
        " ".to_string()
    }
}

fn pr_row(index: usize, pr: &PullRequest, now: SystemTime, width: usize) -> Line<'static> {
    let age = pr.updated_at.map(|at| age(at, now)).unwrap_or_default();
    let right = match pr.queue_position {
        Some(position) => format!("⇥{position} {age}"),
        None => age,
    };
    let text = format!("#{} {}  {}", pr.number, short_repo(&pr.repo), pr.title);
    line(row_number(index), glyph(pr.checks), text, right, width)
}

fn queue_row(entry: &QueueEntry, width: usize) -> Line<'static> {
    let right = match entry.eta {
        Some(eta) => duration(eta),
        None => entry.state.to_string(),
    };
    let text = format!("#{} {}  {}", entry.number, entry.author, entry.title);
    line(
        entry.position.to_string(),
        glyph(entry.checks),
        text,
        right,
        width,
    )
}

fn glyph(checks: CheckState) -> (char, Color) {
    match checks {
        CheckState::Success => ('✓', Color::Green),
        CheckState::Failure => ('✗', Color::Red),
        CheckState::Pending => ('●', Color::Yellow),
        CheckState::None => ('○', Color::DarkGray),
        CheckState::Unknown => ('?', Color::DarkGray),
    }
}

fn draw_details(frame: &mut Frame, app: &App, area: Rect) {
    let (title, lines) = match app.focus {
        Stage::Prs => match app.prs.selected() {
            Some(pr) => (format!("#{} {}", pr.number, pr.title), pr_details(pr)),
            None => ("Details".to_string(), vec![Line::from("nothing selected")]),
        },
        Stage::Queue => match app.queue.selected() {
            Some(entry) => (
                format!("#{} {}", entry.number, entry.title),
                queue_details(entry, app.now),
            ),
            None => ("Details".to_string(), vec![Line::from("nothing selected")]),
        },
        Stage::Builds => match app.builds.selected() {
            Some(build) => {
                let (label, _) = build_label(build);
                (
                    format!("{label} run {}", build.run_number),
                    build_details(build, app.now, app.utc_offset_secs),
                )
            }
            None => ("Details".to_string(), vec![Line::from("nothing selected")]),
        },
        Stage::Deployed => ("Details".to_string(), vec![Line::from("not configured")]),
    };
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title(title)),
        area,
    );
}

fn pr_details(pr: &PullRequest) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(format!(
            "#{} {}  {}  +{} −{}",
            pr.number, pr.repo, pr.head_ref, pr.additions, pr.deletions
        )),
        Line::from(format!("review: {}  merge: {}", pr.review, pr.merge_state)),
    ];
    lines.extend(pr.checks_failures_first().into_iter().map(|check| {
        let (glyph, color) = conclusion_glyph(check.conclusion);
        Line::from(vec![
            Span::styled(glyph.to_string(), Style::default().fg(color)),
            Span::raw(format!(" {}  ", check.name)),
            Span::styled(check.url.clone(), dim()),
        ])
    }));
    lines
}

fn queue_details(entry: &QueueEntry, now: SystemTime) -> Vec<Line<'static>> {
    let enqueued = entry
        .enqueued_at
        .map(|at| format!("enqueued {} ago", age(at, now)))
        .unwrap_or_default();
    let eta = entry
        .eta
        .map(|eta| format!("  eta {}", duration(eta)))
        .unwrap_or_default();
    let mut flags = Vec::new();
    if entry.solo {
        flags.push("solo");
    }
    if entry.jump {
        flags.push("jump");
    }
    let flags = if flags.is_empty() {
        String::new()
    } else {
        format!("  {}", flags.join(" "))
    };
    vec![
        Line::from(format!(
            "#{} by {}  position {}  {}{eta}{flags}",
            entry.number, entry.author, entry.position, entry.state
        )),
        Line::from(format!("{enqueued}  checks: {}", check_word(entry.checks))),
        Line::from(vec![
            Span::styled(entry.head_sha.clone(), dim()),
            Span::raw("  "),
            Span::styled(entry.url.clone(), dim()),
        ]),
        Line::from(match &entry.run_url {
            Some(url) => format!("run: {url}"),
            None => "run: none yet".to_string(),
        }),
    ]
}

fn check_word(checks: CheckState) -> &'static str {
    match checks {
        CheckState::Success => "success",
        CheckState::Failure => "failure",
        CheckState::Pending => "pending",
        CheckState::None => "none",
        CheckState::Unknown => "?",
    }
}

fn conclusion_glyph(conclusion: CheckConclusion) -> (char, Color) {
    match conclusion {
        CheckConclusion::Success => ('✓', Color::Green),
        CheckConclusion::Failure => ('✗', Color::Red),
        CheckConclusion::Pending => ('●', Color::Yellow),
        CheckConclusion::Skipped => ('-', Color::DarkGray),
        CheckConclusion::Unknown => ('?', Color::DarkGray),
    }
}

fn footer_text(app: &App) -> String {
    if let Some(notice) = &app.notice {
        return notice.clone();
    }
    if let Some(error) = app.focused_error() {
        return error.to_string();
    }
    match (app.filter(), app.focused_fetched_at()) {
        (Some(query), _) => {
            let (visible, total) = app.focused_counts();
            format!("/{query}  {visible}/{total}")
        }
        (None, Some(at)) => refreshed(at, app.now, app.utc_offset_secs),
        (None, None) => String::new(),
    }
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let width = KEYS
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);
    let lines: Vec<Line> = KEYS
        .iter()
        .map(|(key, what)| {
            Line::from(vec![
                Span::styled(
                    format!("{key:<width$}"),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(*what, dim()),
            ])
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title("Keys")),
        area,
    );
}

const BELT_WIDTH: usize = 12;

pub fn belt(now: SystemTime) -> String {
    let millis = now
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let offset = (millis / 250 % 3) as usize;
    let rollers: String = (0..BELT_WIDTH)
        .map(|i| if (i + offset) % 3 == 2 { '●' } else { '━' })
        .collect();
    format!("{rollers}▸")
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let belt = belt(app.now);
    let belt_width = belt.chars().count() as u16 + 1;
    let [text, right] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(belt_width)]).areas(area);
    frame.render_widget(Paragraph::new(footer_text(app)), text);
    frame.render_widget(
        Paragraph::new(Span::styled(belt, dim())).right_aligned(),
        right,
    );
}
