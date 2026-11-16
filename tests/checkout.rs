mod support;

use std::path::PathBuf;

use conveyor::app::{Input, Request, Stage};
use conveyor::config::{Config, Repo};
use conveyor::sources::failure::FailureKind;
use conveyor::sources::github::{CliGh, Github};
use ratatui::crossterm::event::KeyCode;
use support::{Harness, checkout_target, key, pr, prs};

#[test]
fn s_then_c_checks_the_pull_request_out_where_conveyor_runs_at_once() {
    let mut h = tall(Config::default());

    h.run(vec![
        prs(vec![pr(1, "Retry webhooks")]),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(
        h.screen().contains("c  check out locally"),
        "{}",
        h.screen()
    );

    h.run(vec![key(KeyCode::Char('c'))]).unwrap();

    assert_eq!(h.requests, vec![checkout(1, None)]);
    assert!(h.screen().contains("checking out #1…"), "{}", h.screen());
    assert!(!h.screen().contains("y confirms"), "{}", h.screen());
    assert!(!h.screen().contains("Manage #1"), "{}", h.screen());
}

#[test]
fn the_checkout_runs_in_the_path_configured_for_the_repository() {
    let mut h = tall(with_path("Acme/WebApp", "~/projects/webapp"));

    h.run(vec![
        prs(vec![pr(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('c')),
    ])
    .unwrap();

    assert_eq!(h.requests, vec![checkout(1, Some("~/projects/webapp"))]);
}

#[test]
fn a_path_configured_for_another_repository_is_not_used() {
    let mut h = tall(with_path("acme/billing", "~/projects/billing"));

    h.run(vec![
        prs(vec![pr(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('c')),
    ])
    .unwrap();

    assert_eq!(h.requests, vec![checkout(1, None)]);
}

#[test]
fn a_repository_listed_twice_uses_the_entry_that_has_a_path() {
    let mut config = with_path("acme/webapp", "~/projects/webapp");
    config.repo.insert(
        0,
        Repo {
            name: "acme/webapp".to_string(),
            ..Repo::default()
        },
    );
    let mut h = tall(config);

    h.run(vec![
        prs(vec![pr(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('c')),
    ])
    .unwrap();

    assert_eq!(h.requests, vec![checkout(1, Some("~/projects/webapp"))]);
}

#[test]
fn s_waits_while_a_checkout_is_on_its_way() {
    let mut h = tall(Config::default());

    h.run(vec![
        prs(vec![pr(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('c')),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(h.screen().contains("checking out #1…"), "{}", h.screen());
    assert!(!h.screen().contains("Manage #1"), "{}", h.screen());
}

#[test]
fn while_a_checkout_is_on_its_way_s_opens_on_another_pull_request() {
    let mut h = tall(Config::default());

    h.run(vec![
        prs(vec![pr(1, "a"), pr(2, "b")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('c')),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('c')),
    ])
    .unwrap();

    assert_eq!(h.requests.len(), 2, "{:?}", h.requests);
    assert_eq!(h.requests[1], checkout(2, None));

    h.run(vec![key(KeyCode::Char('k')), key(KeyCode::Char('S'))])
        .unwrap();

    assert!(h.screen().contains("checking out #1…"), "{}", h.screen());
}

#[test]
fn a_landed_checkout_names_the_directory_and_refreshes_nothing() {
    let mut h = tall(with_path("acme/webapp", "~/projects/webapp"));

    h.run(vec![
        prs(vec![pr(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('c')),
        checked_out(1, Some("~/projects/webapp"), Ok(())),
    ])
    .unwrap();

    assert!(
        h.screen().contains("#1 checked out in ~/projects/webapp"),
        "{}",
        h.screen()
    );
    assert_eq!(h.requests.len(), 1);
    assert!(h.app.follow_up.is_none());

    h.run(vec![key(KeyCode::Char('S'))]).unwrap();

    assert!(h.screen().contains("Manage #1"), "{}", h.screen());
}

#[test]
fn a_checkout_where_conveyor_runs_says_so() {
    let mut h = tall(Config::default());

    h.run(vec![
        prs(vec![pr(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('c')),
        checked_out(1, None, Ok(())),
    ])
    .unwrap();

    assert!(
        h.screen()
            .contains("#1 checked out in the current directory"),
        "{}",
        h.screen()
    );
}

#[test]
fn a_refused_checkout_shows_why_and_lets_s_open_again() {
    let mut h = tall(Config::default());

    h.run(vec![
        prs(vec![pr(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('c')),
        checked_out(1, None, Err("fatal: 'retry' is already checked out")),
    ])
    .unwrap();

    assert!(
        h.screen()
            .contains("#1: fatal: 'retry' is already checked out"),
        "{}",
        h.screen()
    );
    assert_eq!(h.requests.len(), 1);

    h.run(vec![key(KeyCode::Char('S'))]).unwrap();

    assert!(h.screen().contains("Manage #1"), "{}", h.screen());
}

#[test]
fn a_checkout_without_git_marks_the_card_as_a_missing_program_in_gits_words() {
    let clone = tempfile::tempdir().unwrap();
    let gh = CliGh::with_program("/nonexistent/gh".into()).with_git("/nonexistent/git".into());
    let mut h = tall(with_path("acme/webapp", clone.path().to_str().unwrap()));
    h.run(vec![
        prs(vec![pr(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('c')),
    ])
    .unwrap();

    let result = gh.checkout("acme/webapp", 1, Some(clone.path()));
    h.run(vec![Ok(Input::CheckedOut(
        checkout_target(1, clone.path().to_str()),
        result,
    ))])
    .unwrap();

    let mark = h.app.failure_on(Stage::Prs, 1).unwrap();
    assert_eq!(mark.kind, FailureKind::ProgramMissing);
    assert_eq!(mark.line(), "! check out failed: git not found on PATH");
}

fn tall(config: Config) -> Harness {
    Harness::with_config(160, 20, config)
}

fn checkout(number: u64, dir: Option<&str>) -> Request {
    Request::Checkout(checkout_target(number, dir))
}

fn with_path(name: &str, path: &str) -> Config {
    Config {
        repo: vec![Repo {
            name: name.to_string(),
            path: Some(PathBuf::from(path)),
            ..Repo::default()
        }],
        ..Config::default()
    }
}

fn checked_out(number: u64, dir: Option<&str>, result: Result<(), &str>) -> std::io::Result<Input> {
    Ok(Input::CheckedOut(
        checkout_target(number, dir),
        result.map_err(|err| err.to_string().into()),
    ))
}
