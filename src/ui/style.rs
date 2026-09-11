use ratatui::style::{Color, Modifier, Style};

use crate::model::builds::BuildStatus;
use crate::model::prs::{CheckConclusion, CheckState};

pub(crate) const BAR: &str = "▌";

pub(crate) const INDENT: usize = 4;

pub(crate) fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

pub(crate) fn glyph(checks: CheckState) -> (char, Color) {
    match checks {
        CheckState::Success => ('✓', Color::Green),
        CheckState::Failure => ('✗', Color::Red),
        CheckState::Pending => ('●', Color::Yellow),
        CheckState::None => ('○', Color::DarkGray),
        CheckState::Unknown => ('?', Color::DarkGray),
    }
}

pub(crate) fn build_glyph(status: BuildStatus) -> (char, Color) {
    match status {
        BuildStatus::Success => ('✓', Color::Green),
        BuildStatus::Failure => ('✗', Color::Red),
        BuildStatus::Running => ('●', Color::Yellow),
        BuildStatus::Queued => ('○', Color::Yellow),
        BuildStatus::Cancelled => ('⊘', Color::DarkGray),
        BuildStatus::Unknown => ('?', Color::DarkGray),
    }
}

pub(crate) fn conclusion_glyph(conclusion: CheckConclusion) -> (char, Color) {
    match conclusion {
        CheckConclusion::Success => ('✓', Color::Green),
        CheckConclusion::Failure => ('✗', Color::Red),
        CheckConclusion::Pending => ('●', Color::Yellow),
        CheckConclusion::Skipped => ('-', Color::DarkGray),
        CheckConclusion::Unknown => ('?', Color::DarkGray),
    }
}

pub(crate) fn status_word(status: BuildStatus) -> &'static str {
    match status {
        BuildStatus::Success => "success",
        BuildStatus::Failure => "failure",
        BuildStatus::Running => "running",
        BuildStatus::Queued => "queued",
        BuildStatus::Cancelled => "cancelled",
        BuildStatus::Unknown => "?",
    }
}

pub(crate) fn check_word(checks: CheckState) -> &'static str {
    match checks {
        CheckState::Success => "success",
        CheckState::Failure => "failure",
        CheckState::Pending => "pending",
        CheckState::None => "none",
        CheckState::Unknown => "?",
    }
}

pub(crate) fn short_repo(repo: &str) -> &str {
    repo.rsplit('/').next().unwrap_or(repo)
}

pub(crate) fn sha8(sha: &str) -> String {
    sha.chars().take(8).collect()
}
