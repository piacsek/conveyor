use std::time::SystemTime;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::app::{App, JobsState, Stage};
use crate::model::builds::Build;
use crate::model::deployed::Deployment;
use crate::model::jobs::Job;
use crate::model::prs::PullRequest;
use crate::model::queue::QueueEntry;
use crate::text::{age, clock, duration};
use crate::ui::rows::build_label;
use crate::ui::style::{build_glyph, check_word, conclusion_glyph, dim, status_word};

pub(crate) fn draw_details(frame: &mut Frame, app: &mut App, area: Rect) {
    let (title, lines) = match app.focus {
        Stage::Prs => match app.prs.selected() {
            Some(pr) => (format!("#{} {}", pr.number, pr.title), pr_details(pr)),
            None => ("Details".to_string(), vec![Line::from("nothing selected")]),
        },
        Stage::Queue => match app.queue.selected() {
            Some(entry) => {
                let mut lines = queue_details(entry, app.now);
                match &entry.run {
                    Some(run) => {
                        lines.push(run_line(run, app.now, app.utc_offset_secs));
                        lines.extend(job_lines(app.selected_jobs()));
                    }
                    None => lines.push(Line::from("no merge-group run yet")),
                }
                (format!("#{} {}", entry.number, entry.title), lines)
            }
            None => ("Details".to_string(), vec![Line::from("nothing selected")]),
        },
        Stage::Builds => match app.builds.selected() {
            Some(build) => {
                let (label, _) = build_label(build);
                let mut lines = build_details(build, app.now, app.utc_offset_secs);
                lines.extend(job_lines(app.selected_jobs()));
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
    let page = area.height.saturating_sub(2);
    let overflow = match page {
        0 => 0,
        page => (lines.len() as u16).saturating_sub(page),
    };
    app.details_page = page;
    app.details_scroll = app.details_scroll.min(overflow);
    let scroll = app.details_scroll;
    let arrows = match (scroll > 0, scroll < overflow) {
        (false, false) => String::new(),
        (above, below) => format!(
            " {}{}",
            if above { "↑" } else { "" },
            if below { "↓" } else { "" }
        ),
    };
    frame.render_widget(
        Paragraph::new(lines)
            .scroll((scroll, 0))
            .block(Block::bordered().title(format!("{title}{arrows}"))),
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
            Span::raw(format!(" {}", check.name)),
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
        Line::from(match entry.run {
            Some(_) => enqueued,
            None => format!("{enqueued}  checks: {}", check_word(entry.checks)),
        }),
        Line::from(Span::styled(entry.head_sha.clone(), dim())),
    ]
}

fn run_line(build: &Build, now: SystemTime, utc_offset_secs: i32) -> Line<'static> {
    let took = build
        .duration()
        .or_else(|| build.elapsed(now))
        .map(duration)
        .unwrap_or_default();
    let started = build
        .started_at
        .map(|at| format!("started at {}", clock(at, utc_offset_secs)))
        .unwrap_or_default();
    Line::from(format!(
        "run {}  {}  {took}  {started}  by {}",
        build.run_number,
        status_word(build.status),
        build.actor
    ))
}

fn build_details(build: &Build, now: SystemTime, utc_offset_secs: i32) -> Vec<Line<'static>> {
    let mut lines = vec![run_line(build, now, utc_offset_secs)];
    match &build.pull {
        Some(pull) => lines.push(Line::from(format!(
            "#{} {}  {}",
            pull.number, pull.author, pull.title
        ))),
        None => lines.push(Line::from(build.title.clone())),
    }
    lines.push(Line::from(Span::styled(build.sha.clone(), dim())));
    lines
}

fn deployed_details(row: &Deployment, now: SystemTime) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(format!(
        "{}  {}",
        row.env,
        row.image.clone().unwrap_or_else(|| "no image".to_string())
    ))];
    match &row.pull {
        Some(pull) => lines.push(Line::from(format!(
            "#{} {}  {}",
            pull.number, pull.author, pull.title
        ))),
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
        Span::raw(format!(" {}  {took}{failed_at}", job.name)),
    ])
}

fn job_lines(jobs: Option<&JobsState>) -> Vec<Line<'static>> {
    match jobs {
        None => Vec::new(),
        Some(JobsState::Loading) => vec![Line::from(Span::styled("jobs: loading…", dim()))],
        Some(JobsState::Failed(message)) => vec![Line::from(Span::styled(
            format!("jobs: {message}"),
            Style::default().fg(Color::Red),
        ))],
        Some(JobsState::Ready(jobs)) => jobs.iter().map(job_line).collect(),
    }
}
