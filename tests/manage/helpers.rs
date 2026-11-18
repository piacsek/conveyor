use conveyor::app::{Input, Request};
use conveyor::config::{Config, Repo};
use conveyor::model::prs::{PullRequest, StatusChange};
use conveyor::model::queue::QueueEntry;
use ratatui::crossterm::event::KeyCode;

use crate::support::{Harness, key, pr, queue, status_target};

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
        status_target(number, "", change),
        result.map_err(|err| err.to_string().into()),
    ))
}

pub fn draft(number: u64, title: &str) -> PullRequest {
    let mut pull = open(number, title);
    pull.is_draft = true;
    pull
}

pub fn line_after(screen: &str, needle: &str) -> String {
    let lines: Vec<&str> = screen.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("{needle} is not on screen: {screen}"));
    lines[at + 1].to_string()
}

pub fn queueable(number: u64, title: &str) -> PullRequest {
    let mut pull = open(number, title);
    pull.merge_queue_enabled = true;
    pull
}

pub fn on_the_queue(entries: Vec<QueueEntry>) -> Vec<std::io::Result<Input>> {
    vec![
        queue(entries),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('S')),
    ]
}

pub fn two_repos() -> Config {
    Config {
        repo: ["acme/monorepo", "acme/infra"]
            .into_iter()
            .map(|name| Repo {
                name: name.to_string(),
                ..Repo::default()
            })
            .collect(),
        ..Config::default()
    }
}
