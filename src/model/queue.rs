use std::fmt;
use std::time::{Duration, SystemTime};

use crate::model::prs::{CheckState, parse_timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QueueState {
    AwaitingChecks,
    Locked,
    Mergeable,
    Queued,
    Unmergeable,
    #[default]
    Unknown,
}

impl fmt::Display for QueueState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AwaitingChecks => "checks",
            Self::Locked => "locked",
            Self::Mergeable => "mergeable",
            Self::Queued => "queued",
            Self::Unmergeable => "unmergeable",
            Self::Unknown => "?",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QueueEntry {
    pub position: u64,
    pub state: QueueState,
    pub number: u64,
    pub title: String,
    pub author: String,
    pub url: String,
    pub head_sha: String,
    pub checks: CheckState,
    pub eta: Option<Duration>,
    pub enqueued_at: Option<SystemTime>,
    pub solo: bool,
    pub jump: bool,
    pub run_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Queue {
    pub repo: String,
    pub url: String,
    pub entries: Vec<QueueEntry>,
}

pub fn parse(value: &serde_json::Value) -> Result<Queue, String> {
    let queue = value
        .pointer("/data/repository/mergeQueue")
        .ok_or_else(|| "gh response has no data.repository.mergeQueue".to_string())?;
    if queue.is_null() {
        return Err("no merge queue on this repository".to_string());
    }
    let mut entries: Vec<QueueEntry> = queue
        .pointer("/entries/nodes")
        .and_then(|v| v.as_array())
        .map(|nodes| nodes.iter().filter_map(parse_entry).collect())
        .unwrap_or_default();
    entries.sort_by_key(|entry| entry.position);
    Ok(Queue {
        repo: String::new(),
        url: text(queue, "url"),
        entries,
    })
}

fn parse_entry(node: &serde_json::Value) -> Option<QueueEntry> {
    let pr = node.get("pullRequest")?;
    let rollup = node
        .pointer("/headCommit/statusCheckRollup")
        .filter(|v| !v.is_null());
    Some(QueueEntry {
        position: node.get("position").and_then(|v| v.as_u64()).unwrap_or(0),
        state: state(node.get("state").and_then(|v| v.as_str())),
        number: pr.get("number")?.as_u64()?,
        title: text(pr, "title"),
        author: pr
            .pointer("/author/login")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        url: text(pr, "url"),
        head_sha: node
            .pointer("/headCommit/oid")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        checks: match rollup.and_then(|r| r.get("state")).and_then(|v| v.as_str()) {
            Some("SUCCESS") => CheckState::Success,
            Some("FAILURE" | "ERROR") => CheckState::Failure,
            Some("PENDING" | "EXPECTED") => CheckState::Pending,
            None => CheckState::None,
            Some(_) => CheckState::Unknown,
        },
        eta: node
            .get("estimatedTimeToMerge")
            .and_then(|v| v.as_u64())
            .map(Duration::from_secs),
        enqueued_at: node
            .get("enqueuedAt")
            .and_then(|v| v.as_str())
            .and_then(parse_timestamp),
        solo: node.get("solo").and_then(|v| v.as_bool()).unwrap_or(false),
        jump: node.get("jump").and_then(|v| v.as_bool()).unwrap_or(false),
        run_url: None,
    })
}

fn state(value: Option<&str>) -> QueueState {
    match value {
        Some("AWAITING_CHECKS") => QueueState::AwaitingChecks,
        Some("LOCKED") => QueueState::Locked,
        Some("MERGEABLE") => QueueState::Mergeable,
        Some("QUEUED") => QueueState::Queued,
        Some("UNMERGEABLE") => QueueState::Unmergeable,
        _ => QueueState::Unknown,
    }
}

fn text(node: &serde_json::Value, key: &str) -> String {
    node.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeGroupRun {
    pub number: u64,
    pub url: String,
    pub checks: CheckState,
}

pub fn parse_merge_group_runs(value: &serde_json::Value) -> Vec<MergeGroupRun> {
    value
        .get("workflow_runs")
        .and_then(|v| v.as_array())
        .map(|runs| runs.iter().filter_map(parse_run).collect())
        .unwrap_or_default()
}

fn parse_run(run: &serde_json::Value) -> Option<MergeGroupRun> {
    let branch = run.get("head_branch")?.as_str()?;
    let number = branch
        .rsplit('/')
        .next()?
        .strip_prefix("pr-")?
        .split('-')
        .next()?
        .parse()
        .ok()?;
    let checks = match (
        run.get("status").and_then(|v| v.as_str()),
        run.get("conclusion").and_then(|v| v.as_str()),
    ) {
        (Some("completed"), Some("success" | "neutral" | "skipped")) => CheckState::Success,
        (Some("completed"), Some(_)) => CheckState::Failure,
        (Some(_), _) => CheckState::Pending,
        _ => CheckState::Unknown,
    };
    Some(MergeGroupRun {
        number,
        url: text(run, "html_url"),
        checks,
    })
}

impl Queue {
    pub fn attach_runs(&mut self, runs: &[MergeGroupRun]) {
        for entry in &mut self.entries {
            if let Some(run) = runs.iter().find(|run| run.number == entry.number) {
                entry.run_url = Some(run.url.clone());
                entry.checks = run.checks;
            }
        }
    }
}
