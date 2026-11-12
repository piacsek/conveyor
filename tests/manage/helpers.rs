use conveyor::app::{Input, Request};
use conveyor::model::prs::{PullRequest, StatusChange};

use crate::support::{Harness, pr, status_target};

pub fn tall() -> Harness {
    Harness::with_size(160, 20)
}

pub fn open(number: u64, title: &str) -> PullRequest {
    let mut pull = pr(number, title);
    pull.head_oid = format!("8c1f2a7d{number}");
    pull.node_id = format!("PR_node{number}");
    pull
}

pub fn status(number: u64, title: &str, change: StatusChange) -> Request {
    Request::ChangeStatus(status_target(number, title, change))
}

pub fn status_changed(
    number: u64,
    change: StatusChange,
    result: Result<(), &str>,
) -> std::io::Result<Input> {
    Ok(Input::StatusChanged(
        number,
        change,
        result.map_err(str::to_string),
    ))
}

pub fn queueable(number: u64, title: &str) -> PullRequest {
    let mut pull = open(number, title);
    pull.merge_queue_enabled = true;
    pull
}
