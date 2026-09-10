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
