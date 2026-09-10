use std::time::SystemTime;

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::app::JobsState;
use crate::model::builds::{Build, BuildStatus};
use crate::model::deployed::Deployment;
use crate::model::prs::{Check, CheckConclusion, MergeState, PullRequest, ReviewDecision};
use crate::model::queue::QueueEntry;
use crate::text::{age, duration, truncate, wrap};
use crate::ui::card::{card, meta};
use crate::ui::style::{
    INDENT, build_glyph, check_word, conclusion_glyph, dim, glyph, sha8, short_repo, status_word,
};

const DEPLOYED_TITLE_LINES: usize = 2;

pub(crate) fn build_label(build: &Build) -> (String, String) {
    match (&build.pull, build.pr_number) {
        (Some(pull), _) => (format!("#{}", pull.number), pull.author.clone()),
        (None, Some(number)) => (format!("#{number}"), build.actor.clone()),
        (None, None) => (format!("run {}", build.run_number), build.actor.clone()),
    }
}

pub(crate) fn pr_row(
    pr: &PullRequest,
    now: SystemTime,
    selected: bool,
    width: usize,
) -> Vec<Line<'static>> {
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

pub(crate) fn queue_row(
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

pub(crate) fn build_row(
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

pub(crate) fn deployed_row(
    row: &Deployment,
    behind: Option<usize>,
    now: SystemTime,
    selected: bool,
    width: usize,
) -> Vec<Line<'static>> {
    let glyph = match (&row.error, behind) {
        (Some(_), _) => ('✗', Color::Red),
        (None, Some(0)) => ('✓', Color::Green),
        (None, Some(_)) => ('◐', Color::Yellow),
        (None, None) => ('○', Color::DarkGray),
    };
    let right = match behind {
        Some(0) => "at main".to_string(),
        Some(n) => format!("↓{n}"),
        None => String::new(),
    };
    let mut below: Vec<Vec<Span<'static>>> = Vec::new();
    if let Some(pull) = &row.pull {
        below.extend(
            wrap(
                &format!("#{} {}", pull.number, pull.title),
                width.saturating_sub(INDENT),
                DEPLOYED_TITLE_LINES,
            )
            .into_iter()
            .map(|line| vec![Span::styled(line, dim())]),
        );
        below.push(meta(std::slice::from_ref(&pull.author), width));
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
    card(selected, glyph, row.env.clone(), right, below, width)
}
