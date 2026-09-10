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
fn queue_cards_show_the_entry_its_position_age_and_eta_or_state() {
    let mut h = Harness::new();

    h.run(vec![queue(two())]).unwrap();

    let col = column(&h.screen(), 1);
    assert!(col[0].contains("Queue webapp (2)"), "{}", h.screen());
    assert!(
        col[1].contains("▌ ● #4821 Retry webhooks"),
        "{}",
        h.screen()
    );
    assert!(col[1].contains("12m"), "{}", h.screen());
    assert!(
        col[2].contains("alice · position 1 · enqueued 20m"),
        "{}",
        h.screen()
    );
    assert!(col[3].contains("no merge-group run yet"), "{}", h.screen());
    assert!(col[4].contains("○ #4830 Rate limits"), "{}", h.screen());
    assert!(col[4].contains("queued"), "{}", h.screen());
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
    assert!(col[4].starts_with("│▌ ○ #4830"), "{}", h.screen());
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

#[test]
fn p_shows_queue_entry_details_for_the_focused_queue_column() {
    let mut h = Harness::with_size(160, 20);
    let mut entries = two();
    entries[0].jump = true;

    h.run(vec![
        queue(entries),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('p')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("#4821 by alice  position 1  checks  eta 12m  jump"),
        "{screen}"
    );
    assert!(
        screen.contains("enqueued 20m ago  checks: pending"),
        "{screen}"
    );
    assert!(
        screen.contains("https://github.com/acme/webapp/pull/4821"),
        "{screen}"
    );
}

#[test]
fn b_opens_the_merge_group_run_of_the_selected_entry() {
    let mut h = Harness::new();
    let mut entries = two();
    entries[0].run_url = Some("https://github.com/acme/webapp/actions/runs/99".to_string());

    h.run(vec![
        queue(entries),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('b')),
    ])
    .unwrap();

    assert_eq!(
        h.opener.opened(),
        vec!["https://github.com/acme/webapp/actions/runs/99".to_string()]
    );

    h.run(vec![key(KeyCode::Char('j')), key(KeyCode::Char('b'))])
        .unwrap();
    assert_eq!(h.opener.opened().len(), 1);
    assert!(
        h.screen().contains("no merge-group run yet"),
        "{}",
        h.screen()
    );
}

#[test]
fn queue_entries_stay_in_position_order_across_refreshes() {
    let mut h = Harness::new();
    let mut jumped = entry(1, 4830, "carol", "Rate limits");
    let mut demoted = entry(2, 4821, "alice", "Retry webhooks");
    jumped.jump = true;
    demoted.state = QueueState::Queued;

    h.run(vec![queue(two()), queue(vec![demoted, jumped])])
        .unwrap();

    let col = column(&h.screen(), 1);
    assert!(
        col[1].contains("#4830 Rate limits"),
        "the entry that jumped to position 1 leads: {}",
        h.screen()
    );
    assert!(col[4].contains("#4821 Retry webhooks"), "{}", h.screen());
}
