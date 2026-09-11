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

pub(crate) fn draw_details(frame: &mut Frame, app: &App, area: Rect) {
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
        Line::from(format!("{enqueued}  checks: {}", check_word(entry.checks))),
        Line::from(Span::styled(entry.head_sha.clone(), dim())),
        Line::from(match entry.run_url {
            Some(_) => "run: b opens it",
            None => "run: none yet",
        }),
    ]
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
