use ratatui::style::Modifier;

use crate::helpers::two_repos;
use conveyor::app::Stage;

use crate::support::{Harness, entry, failed_in, no_queue_in, queue_in};

#[test]
fn a_repository_without_a_merge_queue_reads_no_merge_queue_in_dim() {
    let mut h = Harness::with_config(160, 12, two_repos());

    h.run(vec![no_queue_in(0, "acme/monorepo")]).unwrap();

    let screen = h.screen();
    let body = screen.lines().nth(2).unwrap();
    assert!(
        body[body.find("││").unwrap()..].starts_with("││no merge queue"),
        "{screen}"
    );
    assert!(h.cell(41, 2).modifier.contains(Modifier::DIM));
}

#[test]
fn a_repository_without_a_merge_queue_is_no_error_and_has_no_count() {
    let mut h = Harness::with_config(160, 12, two_repos());

    h.run(vec![no_queue_in(0, "acme/monorepo")]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("Queue monorepo─"), "{screen}");
    assert!(!screen.contains("⚠"), "{screen}");
}

#[test]
fn the_active_tab_of_a_repository_without_a_queue_is_dim_in_the_queue_bar_only() {
    let mut h = Harness::with_config(160, 12, two_repos());

    h.run(vec![no_queue_in(0, "acme/monorepo")]).unwrap();

    let queue_tab = h.cell(41, 1).modifier;
    let builds_tab = h.cell(81, 1).modifier;
    assert!(queue_tab.contains(Modifier::DIM), "{queue_tab:?}");
    assert!(queue_tab.contains(Modifier::REVERSED), "{queue_tab:?}");
    assert!(!builds_tab.contains(Modifier::DIM), "{builds_tab:?}");
    assert!(builds_tab.contains(Modifier::BOLD), "{builds_tab:?}");
}

#[test]
fn the_active_tab_of_a_repository_with_a_queue_is_not_dim() {
    let mut h = Harness::with_config(160, 12, two_repos());

    h.run(vec![queue_in(0, "acme/monorepo", vec![])]).unwrap();

    let queue_tab = h.cell(41, 1).modifier;
    assert!(!queue_tab.contains(Modifier::DIM), "{queue_tab:?}");
    assert!(queue_tab.contains(Modifier::BOLD), "{queue_tab:?}");
}

#[test]
fn a_queue_enabled_later_shows_its_cards_and_its_count() {
    let mut h = Harness::with_config(160, 12, two_repos());
    h.run(vec![no_queue_in(0, "acme/monorepo")]).unwrap();

    h.run(vec![queue_in(
        0,
        "acme/monorepo",
        vec![entry(1, 12, "alice", "Retry webhooks")],
    )])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("#12 Retry webhooks"), "{screen}");
    assert!(screen.contains("Queue monorepo (1)"), "{screen}");
    assert!(!screen.contains("no merge queue"), "{screen}");
}

#[test]
fn a_queue_switched_off_drops_its_cards_and_its_count() {
    let mut h = Harness::with_config(160, 12, two_repos());
    h.run(vec![queue_in(
        0,
        "acme/monorepo",
        vec![entry(1, 12, "alice", "Retry webhooks")],
    )])
    .unwrap();

    h.run(vec![no_queue_in(0, "acme/monorepo")]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("no merge queue"), "{screen}");
    assert!(!screen.contains("#12 Retry webhooks"), "{screen}");
    assert!(screen.contains("Queue monorepo─"), "{screen}");
}

#[test]
fn a_failed_call_after_no_queue_flags_the_title_and_keeps_no_merge_queue() {
    let mut h = Harness::with_config(160, 12, two_repos());
    h.run(vec![no_queue_in(0, "acme/monorepo")]).unwrap();

    h.run(vec![failed_in(Stage::Queue, 0, "API rate limit exceeded")])
        .unwrap();

    let screen = h.screen();
    assert!(screen.contains("Queue monorepo ⚠"), "{screen}");
    assert!(screen.contains("no merge queue"), "{screen}");
}

#[test]
fn a_switched_off_queue_keeps_its_cards_out_when_it_comes_back_empty() {
    let mut h = Harness::with_config(160, 12, two_repos());
    h.run(vec![
        queue_in(
            0,
            "acme/monorepo",
            vec![entry(1, 12, "alice", "Retry webhooks")],
        ),
        no_queue_in(0, "acme/monorepo"),
    ])
    .unwrap();

    h.run(vec![queue_in(0, "acme/monorepo", vec![])]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("queue empty"), "{screen}");
    assert!(screen.contains("Queue monorepo (0)"), "{screen}");
}
