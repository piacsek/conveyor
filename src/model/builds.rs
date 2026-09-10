use std::time::{Duration, SystemTime};

use crate::model::prs::parse_timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuildStatus {
    Success,
    Failure,
    Running,
    Queued,
    Cancelled,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PullRef {
    pub number: u64,
    pub title: String,
    pub author: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Build {
    pub id: u64,
    pub run_number: u64,
    pub status: BuildStatus,
    pub sha: String,
    pub title: String,
    pub actor: String,
    pub url: String,
    pub started_at: Option<SystemTime>,
    pub finished_at: Option<SystemTime>,
    pub pr_number: Option<u64>,
    pub pull: Option<PullRef>,
}

impl Build {
    pub fn duration(&self) -> Option<Duration> {
        match self.status {
            BuildStatus::Running | BuildStatus::Queued => None,
            _ => self.finished_at?.duration_since(self.started_at?).ok(),
        }
    }

    pub fn elapsed(&self, now: SystemTime) -> Option<Duration> {
        match self.status {
            BuildStatus::Running | BuildStatus::Queued => now.duration_since(self.started_at?).ok(),
            _ => None,
        }
    }

    pub fn is_settled(&self) -> bool {
        !matches!(self.status, BuildStatus::Running | BuildStatus::Queued)
    }
}

pub fn parse_runs(value: &serde_json::Value) -> Vec<Build> {
    value
        .get("workflow_runs")
        .and_then(|v| v.as_array())
        .map(|runs| runs.iter().filter_map(parse_run).collect())
        .unwrap_or_default()
}

fn parse_run(run: &serde_json::Value) -> Option<Build> {
    let title = text(run, "display_title");
    Some(Build {
        id: run.get("id")?.as_u64()?,
        run_number: run.get("run_number").and_then(|v| v.as_u64()).unwrap_or(0),
        status: status(
            run.get("status").and_then(|v| v.as_str()),
            run.get("conclusion").and_then(|v| v.as_str()),
        ),
        sha: text(run, "head_sha"),
        actor: run
            .pointer("/actor/login")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        url: text(run, "html_url"),
        started_at: run
            .get("run_started_at")
            .and_then(|v| v.as_str())
            .and_then(parse_timestamp),
        finished_at: run
            .get("updated_at")
            .and_then(|v| v.as_str())
            .and_then(parse_timestamp),
        pr_number: pr_number_from_title(&title),
        pull: None,
        title,
    })
}

fn status(status: Option<&str>, conclusion: Option<&str>) -> BuildStatus {
    match (status, conclusion) {
        (Some("completed"), Some("success" | "neutral" | "skipped")) => BuildStatus::Success,
        (
            Some("completed"),
            Some("failure" | "timed_out" | "action_required" | "startup_failure"),
        ) => BuildStatus::Failure,
        (Some("completed"), Some("cancelled")) => BuildStatus::Cancelled,
        (Some("in_progress"), _) => BuildStatus::Running,
        (Some("queued" | "waiting" | "pending" | "requested"), _) => BuildStatus::Queued,
        _ => BuildStatus::Unknown,
    }
}

pub fn pr_number_from_title(title: &str) -> Option<u64> {
    title
        .trim_end()
        .strip_suffix(')')?
        .rsplit_once("(#")?
        .1
        .parse()
        .ok()
}

pub fn parse_pull_numbers(value: &serde_json::Value) -> Option<PullRef> {
    let pr = value.as_array()?.first()?;
    Some(PullRef {
        number: pr.get("number")?.as_u64()?,
        title: text(pr, "title"),
        author: pr
            .pointer("/user/login")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        url: text(pr, "html_url"),
    })
}

fn text(node: &serde_json::Value, key: &str) -> String {
    node.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}
