mod support;

use std::time::Duration;

use conveyor::app::Request;
use conveyor::app::Stage;
use conveyor::model::builds::BuildStatus;
use conveyor::model::prs::CheckState;
use conveyor::model::queue::QueueState;
use ratatui::crossterm::event::KeyCode;
use support::{Harness, build, entry, failed, job, jobs, key, queue};

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
    assert!(col[1].contains("● #4821 Retry webhooks"), "{}", h.screen());
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
        key(KeyCode::Char('p')),
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
fn d_shows_queue_entry_details_for_the_focused_queue_column() {
    let mut h = Harness::with_size(160, 20);
    let mut entries = two();
    entries[0].jump = true;

    h.run(vec![
        queue(entries),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('d')),
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
        !screen.contains("https://"),
        "no URLs in the pane: {screen}"
    );
}

#[test]
fn b_opens_the_merge_group_run_of_the_selected_entry() {
    let mut h = Harness::new();
    let mut entries = two();
    let mut run = build(99, BuildStatus::Running, None);
    run.url = "https://github.com/acme/webapp/actions/runs/99".to_string();
    entries[0].run = Some(run);

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

fn queued_run() -> Vec<conveyor::model::queue::QueueEntry> {
    let mut entries = two();
    let mut run = build(312, BuildStatus::Failure, None);
    run.id = 4242;
    run.actor = "github-merge-queue[bot]".to_string();
    entries[0].run = Some(run);
    entries
}

#[test]
fn d_on_a_queue_entry_shows_its_merge_group_run_and_asks_for_the_jobs() {
    let mut h = Harness::with_size(160, 24);

    h.run(vec![
        queue(queued_run()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('d')),
        jobs(
            4242,
            vec![
                job(1, "audit", BuildStatus::Success, None),
                job(2, "check", BuildStatus::Failure, Some("Run cargo test")),
            ],
        ),
    ])
    .unwrap();

    assert_eq!(
        h.requests,
        vec![Request::Jobs { run_id: 4242 }],
        "the selected entry's run is asked for"
    );
    let screen = h.screen();
    assert!(
        screen.contains("run 312  failure"),
        "the run line: {screen}"
    );
    assert!(screen.contains("by github-merge-queue[bot]"), "{screen}");
    assert!(
        screen.contains("✗ check  19s  failed at: Run cargo test"),
        "the failed job and step: {screen}"
    );
    assert!(screen.contains("✓ audit  19s"), "{screen}");
    assert!(!screen.contains("https://"), "still no URLs: {screen}");
}

#[test]
fn an_entry_without_a_run_says_so_and_asks_for_no_jobs() {
    let mut h = Harness::with_size(160, 24);

    h.run(vec![
        queue(two()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('d')),
    ])
    .unwrap();

    assert!(h.requests.is_empty(), "{:?}", h.requests);
    assert!(
        h.screen().contains("no merge-group run yet"),
        "{}",
        h.screen()
    );
}

#[test]
fn moving_off_a_queue_entry_asks_for_the_next_runs_jobs() {
    let mut h = Harness::with_size(160, 24);
    let mut entries = queued_run();
    let mut second = build(313, BuildStatus::Running, None);
    second.id = 4343;
    entries[1].run = Some(second);

    h.run(vec![
        queue(entries),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('d')),
        key(KeyCode::Char('j')),
    ])
    .unwrap();

    assert_eq!(
        h.requests,
        vec![
            Request::Jobs { run_id: 4242 },
            Request::Jobs { run_id: 4343 },
        ]
    );
}
