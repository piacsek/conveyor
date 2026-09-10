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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub repo: String,
    pub checks: CheckState,
    pub updated_at: Option<SystemTime>,
}
