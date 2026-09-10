use std::time::SystemTime;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, List, ListItem, Paragraph};

use crate::app::{App, Column, JobsState, Mode, Row, Stage};
use crate::model::builds::{Build, BuildStatus};
use crate::model::deployed::Deployment;
use crate::model::jobs::Job;
use crate::model::prs::{
    Check, CheckConclusion, CheckState, MergeState, PullRequest, ReviewDecision,
};
use crate::model::queue::QueueEntry;
use crate::text::{age, clock, duration, pad_right, refreshed, truncate};

const BAR: &str = "▌";
const INDENT: usize = 4;

pub const KEYS: [(&str, &str); 11] = [
    ("j/k ↓/↑", "move"),
    ("h/l Tab", "focus column"),
    ("Enter/o", "open in browser"),
    ("b", "open build"),
    ("y", "copy URL"),
    ("p", "details"),
    ("/", "filter"),
    ("r", "refresh"),
    ("R", "refresh all"),
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

fn titled<T: Row>(base: &str, column: &Column<T>) -> String {
    let warning = if column.error.is_some() { " ⚠" } else { "" };
    if column.is_loading() {
        format!("{base}{warning}")
    } else {
        format!("{base} ({}){warning}", column.all().len())
    }
}

fn column_title(app: &App, stage: Stage) -> String {
    match stage {
        Stage::Prs => titled("My PRs", &app.prs),
        Stage::Queue => match &app.queue_repo {
            Some(repo) => titled(&format!("Queue {}", short_repo(repo)), &app.queue),
            None => titled("Merge queue", &app.queue),
        },
        Stage::Builds => match &app.builds_repo {
            Some(repo) => titled(&format!("Main {}", short_repo(repo)), &app.builds),
            None => titled("Main builds", &app.builds),
        },
        Stage::Deployed => match &app.deployed_system {
            Some(system) => titled(&format!("Deployed {system}"), &app.deployed),
            None => titled("Deployed", &app.deployed),
        },
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
    let jobs = std::mem::take(&mut app.jobs);
    match stage {
        Stage::Prs => draw_list(
            frame,
            &mut app.prs,
            area,
            block,
            "no open pull requests",
            |_, pr, on, w| pr_row(pr, now, on, w),
        ),
        Stage::Queue => draw_list(
            frame,
            &mut app.queue,
            area,
            block,
            "queue empty",
            |_, entry, on, w| queue_row(entry, now, on, w),
        ),
        Stage::Builds => draw_list(
            frame,
            &mut app.builds,
            area,
            block,
            "no builds on main",
            |_, build, on, w| build_row(build, jobs.get(&build.id), now, on, w),
        ),
        Stage::Deployed
            if app.deployed.is_loading()
                && app.deployed.error.is_none()
                && !app.deploy_configured() =>
        {
            frame.render_widget(
                Paragraph::new("no [[repo.deploy]] configured").block(block),
                area,
            )
        }
        Stage::Deployed => {
            let behind: Vec<Option<usize>> = app
                .deployed
                .visible()
                .iter()
                .map(|row| app.behind_main(row))
                .collect();
            draw_list(
                frame,
                &mut app.deployed,
                area,
                block,
                "no environments",
                |i, row, on, w| deployed_row(row, behind.get(i).copied().flatten(), now, on, w),
            )
        }
    }
    app.jobs = jobs;
}

fn deployed_row(
    row: &Deployment,
    behind: Option<usize>,
    now: SystemTime,
    selected: bool,
    width: usize,
) -> Vec<Line<'static>> {
    let glyph = match (&row.error, behind) {
        (Some(_), _) => ('✗', Color::Red),
        (None, Some(0)) => ('✓', Color::Green),
        (None, Some(_)) => ('●', Color::Yellow),
        (None, None) => ('○', Color::DarkGray),
    };
    let right = match behind {
        Some(0) => "at main".to_string(),
        Some(n) => format!("↓{n}"),
        None => String::new(),
    };
    let mut below = Vec::new();
    if row.pull.is_some() || row.sha.is_some() {
        below.push(meta(&[pull_text(row)], width));
    }
    if let Some(error) = &row.error {
        below.push(vec![Span::styled(
            truncate(error, width.saturating_sub(INDENT)),
            dim().fg(Color::Red),
        )]);
    }
    if let Some(sha) = &row.sha {
        below.push(meta(
            &[
                sha8(sha),
                row.fetched_at
                    .map(|at| format!("read {} ago", age(at, now)))
                    .unwrap_or_default(),
            ],
            width,
        ));
    }
    if below.is_empty() {
        below.push(meta(&["unknown".to_string()], width));
    }
    below.truncate(2);
    card(selected, glyph, row.env.clone(), right, below, width)
}

fn pull_text(row: &Deployment) -> String {
    match (&row.pull, &row.sha) {
        (Some(pull), _) => format!("#{} {} · {}", pull.number, pull.author, pull.title),
        (None, Some(sha)) => sha.chars().take(8).collect(),
        (None, None) => "unknown".to_string(),
    }
}

fn deployed_details(row: &Deployment, now: SystemTime) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(format!(
        "{}  {}",
        row.env,
        row.image.clone().unwrap_or_else(|| "no image".to_string())
    ))];
    match &row.pull {
        Some(pull) => lines.push(Line::from(vec![
            Span::raw(format!(
                "#{} {}  {}  ",
                pull.number, pull.author, pull.title
            )),
            Span::styled(pull.url.clone(), dim()),
        ])),
        None => lines.push(Line::from("no pull request found for this commit")),
    }
    let deployed = row
        .fetched_at
        .map(|at| format!("deployed {} ago", age(at, now)))
        .unwrap_or_default();
    lines.push(Line::from(vec![
        Span::styled(row.sha.clone().unwrap_or_default(), dim()),
        Span::raw(format!("  {deployed}")),
    ]));
    if let Some(error) = &row.error {
        lines.push(Line::from(Span::styled(
            error.clone(),
            Style::default().fg(Color::Red),
        )));
    }
    lines
}

fn build_label(build: &Build) -> (String, String) {
    match (&build.pull, build.pr_number) {
        (Some(pull), _) => (format!("#{}", pull.number), pull.author.clone()),
        (None, Some(number)) => (format!("#{number}"), build.actor.clone()),
        (None, None) => (format!("run {}", build.run_number), build.actor.clone()),
    }
}

fn build_row(
    build: &Build,
    jobs: Option<&JobsState>,
    now: SystemTime,
    selected: bool,
    width: usize,
) -> Vec<Line<'static>> {
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
    let parts = [
        who,
        format!("run {}", build.run_number),
        took,
        status_word(build.status).to_string(),
    ];
    let headline = match build.pull.is_some() || build.pr_number.is_some() {
        true => format!("{label} {title}"),
        false => title,
    };
    card(
        selected,
        build_glyph(build.status),
        headline,
        build.started_at.map(|at| age(at, now)).unwrap_or_default(),
        vec![meta(&parts, width), build_jobs(build, jobs, width)],
        width,
    )
}

fn build_jobs(build: &Build, jobs: Option<&JobsState>, width: usize) -> Vec<Span<'static>> {
    match jobs {
        Some(JobsState::Ready(jobs)) => {
            let worst = jobs
                .iter()
                .find(|job| job.status == BuildStatus::Failure)
                .or_else(|| jobs.iter().find(|job| !job.status.is_settled()));
            match worst {
                Some(job) => {
                    let (glyph, color) = build_glyph(job.status);
                    let step = job
                        .failed_step
                        .as_ref()
                        .map(|step| format!(" · {step}"))
                        .unwrap_or_default();
                    vec![Span::styled(
                        truncate(
                            &format!("{glyph} {}{step}", job.name),
                            width.saturating_sub(INDENT),
                        ),
                        Style::default().fg(color),
                    )]
                }
                None => meta(&[format!("{} jobs ok", jobs.len())], width),
            }
        }
        Some(JobsState::Failed(message)) => meta(&[format!("jobs: {message}")], width),
        Some(JobsState::Loading) => meta(&["jobs: loading…".to_string()], width),
        None => meta(&[sha8(&build.sha)], width),
    }
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

fn job_line(job: &Job) -> Line<'static> {
    let took = job.duration().map(duration).unwrap_or_default();
    let failed_at = job
        .failed_step
        .as_ref()
        .map(|step| format!("  failed at: {step}"))
        .unwrap_or_default();
    let (glyph, color) = build_glyph(job.status);
    Line::from(vec![
        Span::styled(glyph.to_string(), Style::default().fg(color)),
        Span::raw(format!(" {}  {took}{failed_at}  ", job.name)),
        Span::styled(job.url.clone(), dim()),
    ])
}

fn job_lines(jobs: Option<&JobsState>, room: usize) -> Vec<Line<'static>> {
    match jobs {
        None => Vec::new(),
        Some(JobsState::Loading) => vec![Line::from(Span::styled("jobs: loading…", dim()))],
        Some(JobsState::Failed(message)) => vec![Line::from(Span::styled(
            format!("jobs: {message}"),
            Style::default().fg(Color::Red),
        ))],
        Some(JobsState::Ready(jobs)) => jobs.iter().take(room).map(job_line).collect(),
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
    row: impl Fn(usize, &T, bool, usize) -> Vec<Line<'static>>,
) {
    if column.is_loading() {
        let body = column
            .error
            .clone()
            .unwrap_or_else(|| "fetching…".to_string());
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
    let width = usize::from(area.width.saturating_sub(2));
    let selected = column.list.selected();
    let items: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .map(|(i, item)| ListItem::new(Text::from(row(i, item, Some(i) == selected, width))))
        .collect();
    let list = List::new(items).block(block);
    frame.render_stateful_widget(list, area, &mut column.list);
}

fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

fn card(
    selected: bool,
    glyph: (char, Color),
    title: String,
    right: String,
    rest: Vec<Vec<Span<'static>>>,
    width: usize,
) -> Vec<Line<'static>> {
    let title_width = width.saturating_sub(INDENT + right.chars().count() + 1);
    let (bar, emphasis) = if selected {
        (
            Span::styled(BAR, Style::default().fg(Color::Cyan)),
            Style::default().add_modifier(Modifier::BOLD),
        )
    } else {
        (Span::raw(" "), Style::default())
    };
    let mut lines = vec![Line::from(vec![
        bar,
        Span::raw(" "),
        Span::styled(glyph.0.to_string(), Style::default().fg(glyph.1)),
        Span::raw(" "),
        Span::styled(pad_right(&title, title_width), emphasis),
        Span::raw(" "),
        Span::styled(right, dim()),
    ])];
    lines.extend(rest.into_iter().map(|spans| {
        let mut line = vec![Span::raw(" ".repeat(INDENT))];
        line.extend(spans);
        Line::from(line)
    }));
    lines
}

fn meta(parts: &[String], width: usize) -> Vec<Span<'static>> {
    let text: Vec<&str> = parts
        .iter()
        .map(String::as_str)
        .filter(|part| !part.is_empty())
        .collect();
    vec![Span::styled(
        truncate(&text.join(" · "), width.saturating_sub(INDENT)),
        dim(),
    )]
}

fn sha8(sha: &str) -> String {
    sha.chars().take(8).collect()
}

fn pr_row(pr: &PullRequest, now: SystemTime, selected: bool, width: usize) -> Vec<Line<'static>> {
    let age = pr.updated_at.map(|at| age(at, now)).unwrap_or_default();
    let right = match pr.queue_position {
        Some(position) => format!("⇥{position} {age}"),
        None => age,
    };
    let parts = [
        short_repo(&pr.repo).to_string(),
        pr.head_ref.clone(),
        if pr.is_draft {
            "draft".to_string()
        } else {
            String::new()
        },
        match pr.review {
            ReviewDecision::None => String::new(),
            review => review.to_string(),
        },
        if pr.additions + pr.deletions > 0 {
            format!("+{} −{}", pr.additions, pr.deletions)
        } else {
            String::new()
        },
    ];
    card(
        selected,
        glyph(pr.checks),
        format!("#{} {}", pr.number, pr.title),
        right,
        vec![meta(&parts, width), pr_checks(pr, width)],
        width,
    )
}

fn pr_checks(pr: &PullRequest, width: usize) -> Vec<Span<'static>> {
    let unhappy: Vec<&Check> = pr
        .checks_failures_first()
        .into_iter()
        .filter(|check| {
            !matches!(
                check.conclusion,
                CheckConclusion::Success | CheckConclusion::Skipped
            )
        })
        .collect();
    if unhappy.is_empty() {
        let text = match pr.merge_state {
            MergeState::Unknown => format!("checks: {}", check_word(pr.checks)),
            state => format!("merge: {state}"),
        };
        return meta(&[text], width);
    }
    let mut spans = Vec::new();
    let mut room = width.saturating_sub(INDENT);
    for check in unhappy {
        let (glyph, color) = conclusion_glyph(check.conclusion);
        let text = format!("{glyph} {}", truncate(&check.name, room.saturating_sub(2)));
        let used = text.chars().count();
        if used + 1 > room {
            break;
        }
        if !spans.is_empty() {
            spans.push(Span::styled(" ", dim()));
            room -= 1;
        }
        spans.push(Span::styled(text, Style::default().fg(color)));
        room -= used;
    }
    spans
}

fn queue_row(
    entry: &QueueEntry,
    now: SystemTime,
    selected: bool,
    width: usize,
) -> Vec<Line<'static>> {
    let right = match entry.eta {
        Some(eta) => duration(eta),
        None => entry.state.to_string(),
    };
    let parts = [
        entry.author.clone(),
        format!("position {}", entry.position),
        entry
            .enqueued_at
            .map(|at| format!("enqueued {} ago", age(at, now)))
            .unwrap_or_default(),
    ];
    let mut flags = Vec::new();
    if entry.solo {
        flags.push("solo".to_string());
    }
    if entry.jump {
        flags.push("jump".to_string());
    }
    let run = match entry.run_url {
        Some(_) => format!("checks: {}", check_word(entry.checks)),
        None => "no merge-group run yet".to_string(),
    };
    let below = [run, flags.join(" "), sha8(&entry.head_sha)];
    card(
        selected,
        glyph(entry.checks),
        format!("#{} {}", entry.number, entry.title),
        right,
        vec![meta(&parts, width), meta(&below, width)],
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
                let mut lines = build_details(build, app.now, app.utc_offset_secs);
                let room = usize::from(area.height).saturating_sub(2 + lines.len());
                lines.extend(job_lines(app.selected_jobs(), room));
                (format!("{label} run {}", build.run_number), lines)
            }
            None => ("Details".to_string(), vec![Line::from("nothing selected")]),
        },
        Stage::Deployed => match app.deployed.selected() {
            Some(row) => (
                format!(
                    "{} ({})",
                    row.env,
                    app.deployed_system.clone().unwrap_or_default()
                ),
                deployed_details(row, app.now),
            ),
            None => ("Details".to_string(), vec![Line::from("nothing selected")]),
        },
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

pub fn logo() -> String {
    format!(
        "\u{27e6}\u{25a3}\u{27e7}\u{2501}\u{27e6}\u{25a3}\u{27e7}\u{2501}\u{27e6}\u{25a3}\u{27e7}\u{2501}\u{25b8} conveyor v{}",
        env!("CARGO_PKG_VERSION")
    )
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let text = if app.any_refreshing() {
        format!("{} {}", app.spinner(), footer_text(app))
    } else {
        footer_text(app)
    };
    let logo = logo();
    let logo_width = logo.chars().count() as u16 + 1;
    let [left, right] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(logo_width)]).areas(area);
    frame.render_widget(Paragraph::new(text), left);
    frame.render_widget(
        Paragraph::new(Span::styled(logo, dim())).right_aligned(),
        right,
    );
}
