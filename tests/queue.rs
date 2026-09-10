mod support;

use std::time::Duration;

use conveyor::app::Stage;
use conveyor::model::prs::CheckState;
use conveyor::model::queue::QueueState;
use ratatui::crossterm::event::KeyCode;
use support::{Harness, entry, failed, key, queue};

fn two() -> Vec<conveyor::model::queue::QueueEntry> {
    let mut first = entry(1, 4821, "alice", "Retry webhooks");
    first.state = QueueState::AwaitingChecks;
    first.checks = CheckState::Pending;
    first.eta = Some(Duration::from_secs(12 * 60));
    let second = entry(2, 4830, "carol", "Rate limits");
    vec![first, second]
}

fn column(screen: &str, index: usize) -> Vec<String> {
    screen
        .lines()
        .map(|line| {
            let width = line.chars().count() / 4;
            line.chars().skip(index * width).take(width).collect()
        })
        .collect()
}

#[test]
fn queue_rows_show_position_glyph_number_author_title_and_eta_or_state() {
    let mut h = Harness::new();

    h.run(vec![queue(two())]).unwrap();

    let col = column(&h.screen(), 1);
    assert!(col[0].contains("Queue webapp (2)"), "{}", h.screen());
    assert!(
        col[1].contains("1 ● #4821 alice  Retry webhooks"),
        "{}",
        h.screen()
    );
    assert!(col[1].contains("12m"), "{}", h.screen());
    assert!(
        col[2].contains("2 ○ #4830 carol  Rate limits"),
        "{}",
        h.screen()
    );
    assert!(col[2].contains("queued"), "{}", h.screen());
}

#[test]
fn an_empty_queue_says_so() {
    let mut h = Harness::new();

    h.run(vec![queue(vec![])]).unwrap();

    let col = column(&h.screen(), 1);
    assert!(col[0].contains("Queue webapp (0)"), "{}", h.screen());
    assert!(col[1].contains("queue empty"), "{}", h.screen());
}

#[test]
fn keys_act_on_the_focused_queue_column() {
    let mut h = Harness::new();

    h.run(vec![
        queue(two()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('j')),
        key(KeyCode::Enter),
        key(KeyCode::Char('y')),
    ])
    .unwrap();

    assert_eq!(
        h.opener.opened(),
        vec!["https://github.com/acme/webapp/pull/4830".to_string()]
    );
    assert_eq!(
        h.opener.copied(),
        vec!["https://github.com/acme/webapp/pull/4830".to_string()]
    );
    let col = column(&h.screen(), 1);
    assert!(col[2].starts_with("│> 2"), "{}", h.screen());
    assert!(h.screen().contains("copied #4830"), "{}", h.screen());
}

#[test]
fn a_failed_queue_fetch_flags_the_column_and_shows_the_message() {
    let mut h = Harness::new();

    h.run(vec![failed(
        Stage::Queue,
        "no merge queue on this repository",
    )])
    .unwrap();

    let col = column(&h.screen(), 1);
    assert!(col[0].contains("Merge queue ⚠"), "{}", h.screen());
    assert!(
        col[1].contains("no merge queue on this repository"),
        "{}",
        h.screen()
    );
}
