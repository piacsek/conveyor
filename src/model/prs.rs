use std::fmt;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CheckState {
    Success,
    Failure,
    Pending,
    #[default]
    None,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CheckConclusion {
    Success,
    Failure,
    Pending,
    Skipped,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Check {
    pub name: String,
    pub conclusion: CheckConclusion,
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReviewDecision {
    Approved,
    ChangesRequested,
    ReviewRequired,
    #[default]
    None,
    Unknown,
}

impl fmt::Display for ReviewDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Approved => "approved",
            Self::ChangesRequested => "changes requested",
            Self::ReviewRequired => "review required",
            Self::None => "—",
            Self::Unknown => "?",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MergeState {
    Clean,
    Blocked,
    Behind,
    Dirty,
    Draft,
    HasHooks,
    Unstable,
    #[default]
    Unknown,
}

impl fmt::Display for MergeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Clean => "clean",
            Self::Blocked => "blocked",
            Self::Behind => "behind",
            Self::Dirty => "dirty",
            Self::Draft => "draft",
            Self::HasHooks => "has hooks",
            Self::Unstable => "unstable",
            Self::Unknown => "—",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub repo: String,
    pub head_ref: String,
    pub is_draft: bool,
    pub checks: CheckState,
    pub checks_detail: Vec<Check>,
    pub review: ReviewDecision,
    pub merge_state: MergeState,
    pub additions: u64,
    pub deletions: u64,
    pub updated_at: Option<SystemTime>,
}

impl PullRequest {
    pub fn checks_failures_first(&self) -> Vec<&Check> {
        let mut checks: Vec<&Check> = self.checks_detail.iter().collect();
        checks.sort_by_key(|check| match check.conclusion {
            CheckConclusion::Failure => 0,
            CheckConclusion::Pending => 1,
            CheckConclusion::Unknown => 2,
            CheckConclusion::Success => 3,
            CheckConclusion::Skipped => 4,
        });
        checks
    }
}

pub fn parse(value: &serde_json::Value) -> Result<Vec<PullRequest>, String> {
    let nodes = value
        .pointer("/data/search/nodes")
        .and_then(|nodes| nodes.as_array())
        .ok_or_else(|| "gh response has no data.search.nodes".to_string())?;
    Ok(nodes.iter().filter_map(parse_pull_request).collect())
}

fn parse_pull_request(node: &serde_json::Value) -> Option<PullRequest> {
    let number = node.get("number")?.as_u64()?;
    let rollup = node.get("statusCheckRollup").filter(|v| !v.is_null());
    Some(PullRequest {
        number,
        title: text(node, "title"),
        url: text(node, "url"),
        repo: node
            .pointer("/repository/nameWithOwner")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        head_ref: text(node, "headRefName"),
        is_draft: node
            .get("isDraft")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        checks: rollup.map_or(CheckState::None, |r| {
            check_state(r.get("state").and_then(|v| v.as_str()))
        }),
        checks_detail: rollup
            .and_then(|r| r.pointer("/contexts/nodes"))
            .and_then(|v| v.as_array())
            .map(|contexts| contexts.iter().filter_map(parse_check).collect())
            .unwrap_or_default(),
        review: review_decision(node.get("reviewDecision")),
        merge_state: merge_state(node.get("mergeStateStatus").and_then(|v| v.as_str())),
        additions: node.get("additions").and_then(|v| v.as_u64()).unwrap_or(0),
        deletions: node.get("deletions").and_then(|v| v.as_u64()).unwrap_or(0),
        updated_at: node
            .get("updatedAt")
            .and_then(|v| v.as_str())
            .and_then(parse_timestamp),
    })
}

fn text(node: &serde_json::Value, key: &str) -> String {
    node.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn check_state(state: Option<&str>) -> CheckState {
    match state {
        Some("SUCCESS") => CheckState::Success,
        Some("FAILURE" | "ERROR") => CheckState::Failure,
        Some("PENDING" | "EXPECTED") => CheckState::Pending,
        None => CheckState::None,
        Some(_) => CheckState::Unknown,
    }
}

fn parse_check(node: &serde_json::Value) -> Option<Check> {
    if let Some(name) = node.get("name").and_then(|v| v.as_str()) {
        let status = node.get("status").and_then(|v| v.as_str());
        let conclusion = node.get("conclusion").and_then(|v| v.as_str());
        return Some(Check {
            name: name.to_string(),
            conclusion: check_run_conclusion(status, conclusion),
            url: text(node, "detailsUrl"),
        });
    }
    let context = node.get("context").and_then(|v| v.as_str())?;
    Some(Check {
        name: context.to_string(),
        conclusion: match node.get("state").and_then(|v| v.as_str()) {
            Some("SUCCESS") => CheckConclusion::Success,
            Some("FAILURE" | "ERROR") => CheckConclusion::Failure,
            Some("PENDING" | "EXPECTED") => CheckConclusion::Pending,
            _ => CheckConclusion::Unknown,
        },
        url: text(node, "targetUrl"),
    })
}

fn check_run_conclusion(status: Option<&str>, conclusion: Option<&str>) -> CheckConclusion {
    match (status, conclusion) {
        (Some("COMPLETED"), Some("SUCCESS" | "NEUTRAL")) => CheckConclusion::Success,
        (Some("COMPLETED"), Some("SKIPPED")) => CheckConclusion::Skipped,
        (Some("COMPLETED"), Some(_)) => CheckConclusion::Failure,
        (Some("QUEUED" | "IN_PROGRESS" | "WAITING" | "PENDING" | "REQUESTED"), _) => {
            CheckConclusion::Pending
        }
        _ => CheckConclusion::Unknown,
    }
}

fn review_decision(value: Option<&serde_json::Value>) -> ReviewDecision {
    match value.and_then(|v| v.as_str()) {
        Some("APPROVED") => ReviewDecision::Approved,
        Some("CHANGES_REQUESTED") => ReviewDecision::ChangesRequested,
        Some("REVIEW_REQUIRED") => ReviewDecision::ReviewRequired,
        Some(_) => ReviewDecision::Unknown,
        None => ReviewDecision::None,
    }
}

fn merge_state(value: Option<&str>) -> MergeState {
    match value {
        Some("CLEAN") => MergeState::Clean,
        Some("BLOCKED") => MergeState::Blocked,
        Some("BEHIND") => MergeState::Behind,
        Some("DIRTY") => MergeState::Dirty,
        Some("DRAFT") => MergeState::Draft,
        Some("HAS_HOOKS") => MergeState::HasHooks,
        Some("UNSTABLE") => MergeState::Unstable,
        _ => MergeState::Unknown,
    }
}

pub fn parse_timestamp(text: &str) -> Option<SystemTime> {
    let (date, time) = text.split_once('T')?;
    let mut date = date.split('-').map(|part| part.parse::<i64>().ok());
    let (year, month, day) = (date.next()??, date.next()??, date.next()??);
    let time = time.trim_end_matches('Z');
    let mut time = time.split(':').map(|part| part.parse::<i64>().ok());
    let (hour, minute, second) = (time.next()??, time.next()??, time.next()??);
    let days = days_from_civil(year, month, day);
    let secs = days * 86_400 + hour * 3600 + minute * 60 + second;
    Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(u64::try_from(secs).ok()?))
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}
