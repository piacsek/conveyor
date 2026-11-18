use conveyor::model::builds::BuildStatus;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::two_repos;
use crate::support::{Harness, build, builds_in, entry, key, queue_in};

#[test]
fn tab_on_main_builds_moves_both_columns_to_the_next_repository() {
    let mut h = on_main_builds();

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("Queue infra (1)"), "{screen}");
    assert!(screen.contains("#40 Bump nodes"), "{screen}");
    assert!(screen.contains("run 25"), "{screen}");
    assert!(!screen.contains("run 24"), "{screen}");
}

#[test]
fn a_digit_on_main_builds_moves_both_columns_to_that_repository() {
    let mut h = on_main_builds();

    h.run(vec![key(KeyCode::Char('2'))]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("Queue infra (1)"), "{screen}");
    assert!(screen.contains("run 25"), "{screen}");
}

#[test]
fn shift_tab_on_main_builds_wraps_both_columns_to_the_last_repository() {
    let mut h = on_main_builds();

    h.run(vec![key(KeyCode::BackTab)]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("Queue infra (1)"), "{screen}");
    assert!(screen.contains("run 25"), "{screen}");
}

#[test]
fn a_digit_past_the_last_repository_on_main_builds_does_nothing() {
    let mut h = on_main_builds();

    h.run(vec![key(KeyCode::Char('3'))]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("Queue monorepo (1)"), "{screen}");
    assert!(screen.contains("run 24"), "{screen}");
    assert!(h.requests.is_empty(), "{:?}", h.requests);
}

#[test]
fn the_digit_of_the_active_repository_on_main_builds_does_nothing() {
    let mut h = on_main_builds();

    h.run(vec![key(KeyCode::Char('1'))]).unwrap();

    assert!(h.screen().contains("run 24"), "{}", h.screen());
    assert!(h.requests.is_empty(), "{:?}", h.requests);
}

fn on_main_builds() -> Harness {
    let mut h = Harness::with_config(160, 16, two_repos());
    h.run(vec![
        queue_in(
            0,
            "acme/monorepo",
            vec![entry(1, 12, "alice", "Retry webhooks")],
        ),
        queue_in(1, "acme/infra", vec![entry(1, 40, "bob", "Bump nodes")]),
        builds_in(
            0,
            "acme/monorepo",
            vec![build(24, BuildStatus::Success, None)],
        ),
        builds_in(1, "acme/infra", vec![build(25, BuildStatus::Success, None)]),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
    ])
    .unwrap();
    h.requests.clear();
    h
}
