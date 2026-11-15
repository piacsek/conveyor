use conveyor::model::builds::BuildStatus;
use ratatui::crossterm::event::KeyCode;

use crate::support::{NOW, build, builds, job, key};

pub fn column(screen: &str, index: usize) -> Vec<String> {
    screen
        .lines()
        .map(|line| {
            let width = line.chars().count() / 4;
            line.chars().skip(index * width).take(width).collect()
        })
        .collect()
}

pub fn three() -> Vec<conveyor::model::builds::Build> {
    let mut running = build(26, BuildStatus::Running, Some((4840, "bob", "Speed up CI")));
    running.finished_at = None;
    running.started_at = Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(NOW - 90));
    vec![
        running,
        build(
            25,
            BuildStatus::Failure,
            Some((4821, "alice", "Retry hooks")),
        ),
        build(24, BuildStatus::Success, None),
    ]
}

pub fn focus_builds() -> Vec<std::io::Result<conveyor::app::Input>> {
    vec![
        builds(three()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
    ]
}

pub fn many_jobs(count: u64) -> Vec<conveyor::model::jobs::Job> {
    (1..=count)
        .map(|i| job(i, &format!("job {i}"), BuildStatus::Success, None))
        .collect()
}

pub fn line_after(screen: &str, needle: &str) -> String {
    let lines: Vec<&str> = screen.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("{needle} is not on screen: {screen}"));
    lines[at + 1].to_string()
}

pub fn failing_newest() -> Vec<conveyor::model::builds::Build> {
    three().into_iter().skip(1).collect()
}

pub fn focus_failing_newest() -> Vec<std::io::Result<conveyor::app::Input>> {
    vec![
        builds(failing_newest()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
    ]
}
