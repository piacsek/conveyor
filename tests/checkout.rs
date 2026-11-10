mod support;

use std::path::PathBuf;

use conveyor::app::{CheckoutTarget, Input, Request};
use conveyor::config::{Config, Repo};
use ratatui::crossterm::event::KeyCode;
use support::{Harness, key, pr, prs};

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

fn tall(config: Config) -> Harness {
    Harness::with_config(160, 20, config)
}

fn checkout(number: u64, dir: Option<&str>) -> Request {
    Request::Checkout(target(number, dir))
}

fn target(number: u64, dir: Option<&str>) -> CheckoutTarget {
    CheckoutTarget {
        number,
        repo: "acme/webapp".to_string(),
        dir: dir.map(PathBuf::from),
    }
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
        target(number, dir),
        result.map_err(str::to_string),
    ))
}
