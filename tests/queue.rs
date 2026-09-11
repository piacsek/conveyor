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
fn the_pane_shows_the_merge_group_run_of_the_selected_entry_and_its_jobs() {
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
fn an_entry_without_a_run_says_so_in_the_pane_and_asks_for_no_jobs() {
    let mut h = Harness::with_size(160, 24);

    h.run(vec![
        queue(vec![entry(1, 4821, "alice", "Retry webhooks")]),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('d')),
    ])
    .unwrap();

    assert!(h.requests.is_empty(), "{:?}", h.requests);
    let screen = h.screen();
    assert_eq!(
        screen.matches("no merge-group run yet").count(),
        2,
        "on the card and in the pane: {screen}"
    );
}

#[test]
fn selecting_a_queue_entry_asks_for_its_runs_jobs_pane_or_no_pane() {
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

#[test]
fn a_failing_merge_group_run_shows_its_failed_job_on_the_card() {
    let mut h = Harness::new();

    h.run(vec![
        queue(queued_run()),
        key(KeyCode::Char('l')),
        jobs(
            4242,
            vec![
                job(1, "audit", BuildStatus::Success, None),
                job(2, "check", BuildStatus::Failure, Some("Run cargo test")),
            ],
        ),
    ])
    .unwrap();

    let col = column(&h.screen(), 1);
    assert!(
        col[4].contains("✗ check · Run cargo test"),
        "the card, not just the pane: {}",
        h.screen()
    );
}

#[test]
fn a_queue_refresh_asks_again_for_an_unsettled_run_and_keeps_a_settled_one() {
    let mut h = Harness::new();
    let mut settled = queued_run();
    let mut running = build(313, BuildStatus::Running, None);
    running.id = 4343;
    settled[1].run = Some(running);

    h.run(vec![
        queue(settled.clone()),
        key(KeyCode::Char('l')),
        jobs(4242, vec![job(1, "audit", BuildStatus::Success, None)]),
        key(KeyCode::Char('j')),
        jobs(4343, vec![job(2, "check", BuildStatus::Success, None)]),
        queue(settled),
    ])
    .unwrap();

    assert_eq!(
        h.requests,
        vec![
            Request::Jobs { run_id: 4242 },
            Request::Jobs { run_id: 4343 },
            Request::Jobs { run_id: 4343 },
        ],
        "only the run still in flight is asked again"
    );
}

#[test]
fn a_refresh_while_the_jobs_are_in_flight_does_not_ask_twice() {
    let mut h = Harness::new();
    let mut entries = two();
    let mut running = build(313, BuildStatus::Running, None);
    running.id = 4343;
    entries[0].run = Some(running);

    h.run(vec![
        queue(entries.clone()),
        key(KeyCode::Char('l')),
        queue(entries.clone()),
        queue(entries),
    ])
    .unwrap();

    assert_eq!(
        h.requests,
        vec![Request::Jobs { run_id: 4343 }],
        "the answer is still on its way"
    );
}

fn with_run(status: BuildStatus) -> Vec<conveyor::model::queue::QueueEntry> {
    let mut entries = vec![entry(1, 4821, "alice", "Retry webhooks")];
    let mut run = build(312, status, None);
    run.id = 4242;
    entries[0].run = Some(run);
    entries
}

#[test]
fn a_cancelled_run_reads_as_cancelled_and_never_as_a_failure() {
    let mut h = Harness::new();

    h.run(vec![queue(with_run(BuildStatus::Cancelled))])
        .unwrap();

    let col = column(&h.screen(), 1);
    assert!(
        col[1].contains("⊘ #4821"),
        "its own glyph, not the skipped dash: {}",
        h.screen()
    );
    assert!(col[3].contains("run 312 · cancelled"), "{}", h.screen());
    assert!(!h.screen().contains("✗"), "nothing failed: {}", h.screen());
    let glyph = h
        .screen()
        .lines()
        .nth(1)
        .unwrap()
        .chars()
        .position(|c| c == '⊘')
        .unwrap() as u16;
    assert_eq!(
        h.cell(glyph, 1).fg,
        ratatui::style::Color::DarkGray,
        "grey, not red"
    );
}

#[test]
fn a_failed_run_still_reads_as_a_failure() {
    let mut h = Harness::new();

    h.run(vec![queue(with_run(BuildStatus::Failure))]).unwrap();

    let col = column(&h.screen(), 1);
    assert!(col[1].contains("✗ #4821"), "{}", h.screen());
    assert!(col[3].contains("run 312 · failure"), "{}", h.screen());
}

#[test]
fn an_entry_with_no_run_still_reads_from_the_rollup() {
    let mut h = Harness::new();
    let mut entries = vec![entry(1, 4821, "alice", "Retry webhooks")];
    entries[0].checks = CheckState::Success;

    h.run(vec![queue(entries)]).unwrap();

    let col = column(&h.screen(), 1);
    assert!(col[1].contains("✓ #4821"), "{}", h.screen());
    assert!(col[3].contains("no merge-group run yet"), "{}", h.screen());
}
