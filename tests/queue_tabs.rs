mod support;

use conveyor::app::{Request, Stage};
use conveyor::config::{Config, Repo};
use ratatui::crossterm::event::KeyCode;
use support::{Harness, entry, failed_in, key, queue_in};

#[test]
fn tab_on_merge_queue_asks_for_the_second_repository() {
    let mut h = Harness::with_config(160, 12, two_repos());
    h.run(vec![
        queue_in(
            0,
            "acme/monorepo",
            vec![entry(1, 12, "alice", "Retry webhooks")],
        ),
        key(KeyCode::Char('l')),
    ])
    .unwrap();

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    assert_eq!(h.requests, vec![Request::Refresh(Stage::Queue, 1)]);
}

#[test]
fn merge_queue_draws_a_tab_per_repository() {
    let mut h = Harness::with_config(160, 12, two_repos());

    h.run(vec![queue_in(0, "acme/monorepo", vec![])]).unwrap();

    assert!(h.screen().contains("monorepo │ infra"), "{}", h.screen());
}

#[test]
fn the_infra_tab_shows_the_infra_queue_and_names_it_in_the_title() {
    let mut h = Harness::with_config(160, 12, two_repos());
    h.run(vec![
        queue_in(
            0,
            "acme/monorepo",
            vec![entry(1, 12, "alice", "Retry webhooks")],
        ),
        key(KeyCode::Char('l')),
        key(KeyCode::Tab),
    ])
    .unwrap();

    h.run(vec![queue_in(
        1,
        "acme/infra",
        vec![entry(1, 40, "bob", "Bump nodes")],
    )])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("#40 Bump nodes"), "{screen}");
    assert!(!screen.contains("#12 Retry webhooks"), "{screen}");
    assert!(screen.contains("Queue infra (1)"), "{screen}");
}

#[test]
fn shift_tab_returns_to_the_monorepo_queue_with_its_selection_and_without_the_filter() {
    let mut h = Harness::with_config(160, 12, two_repos());
    h.run(vec![
        queue_in(
            0,
            "acme/monorepo",
            vec![
                entry(1, 12, "alice", "Retry webhooks"),
                entry(2, 13, "carol", "Rate limits"),
            ],
        ),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('/')),
        key(KeyCode::Char('1')),
        key(KeyCode::Char('3')),
        key(KeyCode::Enter),
        key(KeyCode::Tab),
    ])
    .unwrap();

    h.run(vec![key(KeyCode::BackTab)]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("#12 Retry webhooks"), "{screen}");
    assert_eq!(h.app.query, None);
    assert_eq!(h.app.queue.column().selected().unwrap().number, 13);
}

#[test]
fn one_repository_draws_no_tab_bar() {
    let mut h = Harness::with_config(
        160,
        12,
        Config {
            repo: vec![repo("acme/monorepo")],
            ..Config::default()
        },
    );

    h.run(vec![queue_in(0, "acme/monorepo", vec![])]).unwrap();

    assert!(!h.screen().contains("│ monorepo"), "{}", h.screen());
    assert!(h.screen().contains("Queue monorepo (0)"), "{}", h.screen());
}

#[test]
fn a_monorepo_result_landing_after_the_switch_stays_in_the_monorepo_tab() {
    let mut h = Harness::with_config(160, 12, two_repos());
    h.run(vec![key(KeyCode::Char('l')), key(KeyCode::Tab)])
        .unwrap();

    h.run(vec![queue_in(
        0,
        "acme/monorepo",
        vec![entry(1, 12, "alice", "Retry webhooks")],
    )])
    .unwrap();

    let screen = h.screen();
    assert!(!screen.contains("#12 Retry webhooks"), "{screen}");
    assert!(!screen.contains("Queue monorepo"), "{screen}");
    h.run(vec![key(KeyCode::BackTab)]).unwrap();
    assert!(h.screen().contains("#12 Retry webhooks"), "{}", h.screen());
}

#[test]
fn a_monorepo_error_landing_after_the_switch_shows_only_on_the_monorepo_tab() {
    let mut h = Harness::with_config(160, 12, two_repos());
    h.run(vec![key(KeyCode::Char('l')), key(KeyCode::Tab)])
        .unwrap();

    h.run(vec![failed_in(Stage::Queue, 0, "gh: HTTP 502")])
        .unwrap();

    assert!(!h.screen().contains("HTTP 502"), "{}", h.screen());
    h.run(vec![key(KeyCode::BackTab)]).unwrap();
    assert!(h.screen().contains("HTTP 502"), "{}", h.screen());
}

fn two_repos() -> Config {
    Config {
        repo: vec![repo("acme/monorepo"), repo("acme/infra")],
        ..Config::default()
    }
}

fn repo(name: &str) -> Repo {
    Repo {
        name: name.to_string(),
        ..Repo::default()
    }
}
