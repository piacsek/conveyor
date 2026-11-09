use std::time::SystemTime;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::app::{App, JobsState, LogState, Stage};
use crate::model::builds::Build;
use crate::model::deployed::{Deployment, Pod, Rollout};
use crate::model::jobs::Job;
use crate::model::prs::PullRequest;
use crate::model::queue::QueueEntry;
use crate::text::{age, clock, duration, elapsed};
use crate::ui::rows::build_label;
use crate::ui::style::{build_glyph, check_word, conclusion_glyph, dim, sha8, status_word};

pub(crate) fn draw_details(frame: &mut Frame, app: &mut App, area: Rect) {
    let (title, lines) = match app.focus {
        Stage::Prs => match app.prs.selected() {
            Some(pr) => (
                format!("#{} {}", pr.number, pr.title),
                pr_details(pr, app.now),
            ),
            None => ("Details".to_string(), vec![Line::from("nothing selected")]),
        },
        Stage::Queue => match app.queue.selected() {
            Some(entry) => {
                let mut lines = queue_details(entry, app.now);
                match &entry.run {
                    Some(run) => {
                        lines.push(run_line(run, app.now, app.utc_offset_secs));
                        lines.extend(job_lines(app.selected_jobs(), app.now));
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
                lines.extend(job_lines(app.selected_jobs(), app.now));
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
    // The log gets a pane of its own beside the details while `L` has it open.
    let log = app.selected_log().map(|(job, state)| {
        let step = job.failed_step.clone().unwrap_or_default();
        (format!("log: {} · {step}", job.name), log_body(state))
    });
    let (details_area, log_area) = match &log {
        Some(_) => {
            let [left, right] = Layout::horizontal([Constraint::Fill(1); 2]).areas(area);
            (left, Some(right))
        }
        None => (area, None),
    };
    let (page, scroll) = scrolled(details_area, lines.len(), app.details_scroll);
    app.details_page = page;
    app.details_scroll = scroll.0;
    frame.render_widget(
        Paragraph::new(lines)
            .scroll((scroll.0, 0))
            .block(Block::bordered().title(format!("{title}{}", scroll.1))),
        details_area,
    );
    if let (Some((log_title, body)), Some(log_area)) = (log, log_area) {
        let (page, scroll) = scrolled(log_area, body.len(), app.log_scroll);
        app.log_page = page;
        app.log_scroll = scroll.0;
        frame.render_widget(
            Paragraph::new(body)
                .scroll((scroll.0, 0))
                .block(Block::bordered().title(format!("{log_title}{}", scroll.1))),
            log_area,
        );
    }
}

/// Clamp a pane's scroll to its content and say which way there is more: (page, (scroll, arrows)).
pub(crate) fn scrolled(area: Rect, lines: usize, wanted: u16) -> (u16, (u16, String)) {
    let page = area.height.saturating_sub(2);
    let overflow = match page {
        0 => 0,
        page => (lines as u16).saturating_sub(page),
    };
    let scroll = wanted.min(overflow);
    let arrows = match (scroll > 0, scroll < overflow) {
        (false, false) => String::new(),
        (above, below) => format!(
            " {}{}",
            if above { "↑" } else { "" },
            if below { "↓" } else { "" }
        ),
    };
    (page, (scroll, arrows))
}

fn pr_details(pr: &PullRequest, now: SystemTime) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(format!(
            "#{} {}  {}  +{} −{}",
            pr.number, pr.repo, pr.head_ref, pr.additions, pr.deletions
        )),
        Line::from(format!("review: {}  merge: {}", pr.review, pr.merge_state)),
    ];
    lines.extend(pr.checks_failures_first().into_iter().map(|check| {
        let (glyph, color) = conclusion_glyph(check.conclusion);
        let took = check
            .took(now)
            .map(|took| format!("  {}", elapsed(took)))
            .unwrap_or_default();
        Line::from(vec![
            Span::styled(glyph.to_string(), Style::default().fg(color)),
            Span::raw(format!(" {}{took}", check.name)),
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
        .map(elapsed)
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
    if !row.target.is_empty() {
        lines.push(Line::from(Span::styled(row.target.clone(), dim())));
    }
    if let Some(rollout) = &row.rollout {
        lines.extend(rollout_lines(rollout));
    }
    for pod in &row.pods {
        lines.extend(pod_lines(pod, now));
    }
    if let Some(error) = &row.pods_error {
        lines.push(Line::from(Span::styled(
            format!("pods: {error}"),
            Style::default().fg(Color::Red),
        )));
    }
    lines
}

fn rollout_lines(rollout: &Rollout) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(format!(
        "replicas: {} desired · {} updated · {} ready · {} available",
        rollout.desired, rollout.updated, rollout.ready, rollout.available
    ))];
    lines.extend(
        rollout
            .conditions
            .iter()
            .filter(|condition| {
                (condition.kind == "ReplicaFailure") == (condition.status == "True")
            })
            .map(|condition| {
                let color = if condition.kind == "Available" {
                    Color::Red
                } else {
                    Color::Yellow
                };
                Line::from(Span::styled(
                    [
                        format!("{} {}", condition.kind, condition.status),
                        condition.reason.clone(),
                        condition.message.clone(),
                    ]
                    .into_iter()
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
                    .join(" · "),
                    Style::default().fg(color),
                ))
            }),
    );
    lines
}

fn pod_lines(pod: &Pod, now: SystemTime) -> Vec<Line<'static>> {
    let waiting = pod.waiting.as_ref().map(|waiting| waiting.reason.clone());
    let (glyph, color, status) = if pod.terminating {
        ('⊘', Color::DarkGray, "Terminating".to_string())
    } else if pod.finished() {
        ('○', Color::DarkGray, pod.phase.clone())
    } else if let Some(stuck) = &pod.stuck {
        ('✗', Color::Red, waiting.unwrap_or_else(|| stuck.clone()))
    } else if pod.ready {
        ('✓', Color::Green, pod.phase.clone())
    } else {
        (
            '●',
            Color::Yellow,
            waiting.unwrap_or_else(|| pod.phase.clone()),
        )
    };
    let mut parts = vec![
        pod.name.clone(),
        status,
        format!("restarts {}", pod.restarts),
    ];
    parts.extend(pod.started_at.map(|at| age(at, now)));
    parts.extend(pod.sha.as_deref().map(sha8));
    let mut lines = vec![Line::from(vec![
        Span::styled(glyph.to_string(), Style::default().fg(color)),
        Span::raw(format!(" {}", parts.join("  "))),
    ])];
    let mut why: Vec<String> = pod
        .last_exit
        .iter()
        .map(|exit| format!("last exit: {} ({})", exit.reason, exit.exit_code))
        .collect();
    why.extend(
        pod.waiting
            .iter()
            .map(|waiting| waiting.message.clone())
            .filter(|message| !message.is_empty()),
    );
    if !why.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("    {}", why.join(" · ")),
            dim(),
        )));
    }
    lines
}

fn job_line(job: &Job, now: SystemTime) -> Line<'static> {
    let took = job.took(now).map(elapsed).unwrap_or_default();
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

fn job_lines(jobs: Option<&JobsState>, now: SystemTime) -> Vec<Line<'static>> {
    match jobs {
        None => Vec::new(),
        Some(JobsState::Loading) => vec![Line::from(Span::styled("jobs: loading…", dim()))],
        Some(JobsState::Failed(message)) => vec![Line::from(Span::styled(
            format!("jobs: {message}"),
            Style::default().fg(Color::Red),
        ))],
        Some(JobsState::Ready(jobs) | JobsState::Stale(jobs) | JobsState::Refreshing(jobs)) => {
            jobs.iter().map(|job| job_line(job, now)).collect()
        }
    }
}

/// The body of the log pane.
fn log_body(state: &LogState) -> Vec<Line<'static>> {
    match state {
        LogState::Loading => vec![Line::from(Span::styled("loading…", dim()))],
        LogState::Failed(message) => vec![Line::from(Span::styled(
            message.clone(),
            Style::default().fg(Color::Red),
        ))],
        LogState::Ready(lines) => lines.iter().map(|line| Line::from(line.clone())).collect(),
    }
}
