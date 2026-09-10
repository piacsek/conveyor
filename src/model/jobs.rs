use std::time::{Duration, SystemTime};

use crate::model::builds::{BuildStatus, status};
use crate::model::prs::parse_timestamp;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Job {
    pub id: u64,
    pub name: String,
    pub status: BuildStatus,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub url: String,
    pub failed_step: Option<String>,
}

impl Job {
    pub fn duration(&self) -> Option<Duration> {
        self.completed_at?.duration_since(self.started_at?).ok()
    }
}

pub fn parse_jobs(value: &serde_json::Value) -> Vec<Job> {
    let mut jobs: Vec<Job> = value
        .get("jobs")
        .and_then(|v| v.as_array())
        .map(|jobs| jobs.iter().filter_map(parse_job).collect())
        .unwrap_or_default();
    jobs.sort_by_key(|job| match job.status {
        BuildStatus::Failure => 0,
        BuildStatus::Running | BuildStatus::Queued => 1,
        BuildStatus::Unknown => 2,
        BuildStatus::Success => 3,
        BuildStatus::Cancelled => 4,
    });
    jobs
}

fn parse_job(job: &serde_json::Value) -> Option<Job> {
    Some(Job {
        id: job.get("id")?.as_u64()?,
        name: text(job, "name"),
        status: status(
            job.get("status").and_then(|v| v.as_str()),
            job.get("conclusion").and_then(|v| v.as_str()),
        ),
        started_at: timestamp(job, "started_at"),
        completed_at: timestamp(job, "completed_at"),
        url: text(job, "html_url"),
        failed_step: failed_step(job),
    })
}

fn failed_step(job: &serde_json::Value) -> Option<String> {
    job.get("steps")?
        .as_array()?
        .iter()
        .find(|step| {
            matches!(
                step.get("conclusion").and_then(|v| v.as_str()),
                Some("failure" | "timed_out" | "action_required" | "startup_failure")
            )
        })
        .map(|step| text(step, "name"))
}

fn timestamp(node: &serde_json::Value, key: &str) -> Option<SystemTime> {
    node.get(key)
        .and_then(|v| v.as_str())
        .and_then(parse_timestamp)
}

fn text(node: &serde_json::Value, key: &str) -> String {
    node.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}
