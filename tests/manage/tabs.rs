use conveyor::app::{Input, Stage, pull_key};
use conveyor::config::{Config, PrsTab};
use conveyor::model::prs::{MergeState, PullRequest, StatusChange};
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{draft, line_after, open, status_changed};
use crate::support::{Harness, key, merge_target, prs_in};

#[test]
fn a_mark_survives_a_switch_and_a_refresh_of_a_tab_without_its_row() {
    let mut h = two_tabs();
    h.run(marked_on_the_first_tab()).unwrap();

    h.run(vec![key(KeyCode::Tab), prs_in(1, vec![open(20, "Other")])])
        .unwrap();
    h.run(vec![key(KeyCode::BackTab)]).unwrap();

    let screen = h.screen();
    assert!(
        line_after(&screen, "#12 Retry webhooks").contains("! mark ready failed: HTTP 422"),
        "{screen}"
    );
}

#[test]
fn the_title_counts_the_marks_of_the_active_tab_only() {
    let mut h = two_tabs();
    h.run(marked_on_the_first_tab()).unwrap();

    h.run(vec![key(KeyCode::Tab), prs_in(1, vec![open(20, "Other")])])
        .unwrap();

    assert!(h.screen().contains("My PRs (1)─"), "{}", h.screen());
}

#[test]
fn a_mark_stays_while_another_tab_still_holds_its_row() {
    let mut h = two_tabs();
    h.run(marked_on_the_first_tab()).unwrap();
    h.run(vec![
        key(KeyCode::Tab),
        prs_in(1, vec![draft(12, "Retry webhooks")]),
        key(KeyCode::BackTab),
    ])
    .unwrap();

    h.run(vec![prs_in(0, vec![open(14, "Bump serde")])])
        .unwrap();

    assert!(
        h.app
            .failure_on(Stage::Prs, pull_key("acme/webapp", 12))
            .is_some()
    );
}

#[test]
fn a_mark_drops_once_no_tab_holds_its_row() {
    let mut h = two_tabs();
    h.run(marked_on_the_first_tab()).unwrap();
    h.run(vec![key(KeyCode::Tab), prs_in(1, vec![open(20, "Other")])])
        .unwrap();

    h.run(vec![prs_in(0, vec![open(14, "Bump serde")])])
        .unwrap();

    assert!(
        h.app
            .failure_on(Stage::Prs, pull_key("acme/webapp", 12))
            .is_none()
    );
}

#[test]
fn a_write_that_fails_after_a_switch_still_marks_its_row() {
    let mut h = two_tabs();
    h.run(vec![
        prs_in(0, vec![draft(12, "Retry webhooks")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        key(KeyCode::Tab),
        prs_in(1, vec![open(20, "Other")]),
    ])
    .unwrap();

    h.run(vec![status_changed(
        12,
        StatusChange::Ready,
        Err("HTTP 422"),
    )])
    .unwrap();

    assert!(
        h.app
            .failure_on(Stage::Prs, pull_key("acme/webapp", 12))
            .is_some()
    );
}

#[test]
fn a_busy_row_spins_on_every_tab_that_lists_it() {
    let mut h = two_tabs();
    h.run(vec![
        prs_in(0, vec![draft(12, "Retry webhooks")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
    ])
    .unwrap();

    h.run(vec![
        key(KeyCode::Tab),
        prs_in(1, vec![draft(12, "Retry webhooks")]),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        line_after(&screen, "#12 Retry webhooks").contains("marking ready…"),
        "{screen}"
    );
}

#[test]
fn the_merge_wait_holds_through_a_fetch_of_another_tab() {
    let mut h = two_tabs();
    h.run(merged_then_switched()).unwrap();

    h.run(vec![
        Ok(Input::Fetching(Stage::Prs, 1)),
        prs_in(1, vec![open(20, "Other")]),
        key(KeyCode::BackTab),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(h.screen().contains("waiting for My PRs"), "{}", h.screen());
}

#[test]
fn the_merge_wait_ends_on_a_fetch_of_the_tab_it_was_merged_from() {
    let mut h = two_tabs();
    h.run(merged_then_switched()).unwrap();

    h.run(vec![
        key(KeyCode::BackTab),
        Ok(Input::Fetching(Stage::Prs, 0)),
        prs_in(0, vec![behind(1, "a")]),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(!h.screen().contains("waiting for My PRs"), "{}", h.screen());
}

fn two_tabs() -> Harness {
    let mut config = Config::default();
    config.prs.tab = ["mine", "review"]
        .map(|name| PrsTab {
            name: name.to_string(),
            query: format!("is:pr {name}"),
            limit: None,
        })
        .to_vec();
    Harness::with_config(160, 20, config)
}

fn marked_on_the_first_tab() -> Vec<std::io::Result<Input>> {
    vec![
        prs_in(
            0,
            vec![draft(12, "Retry webhooks"), draft(14, "Bump serde")],
        ),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        status_changed(12, StatusChange::Ready, Err("HTTP 422")),
    ]
}

fn merged_then_switched() -> Vec<std::io::Result<Input>> {
    vec![
        prs_in(0, vec![behind(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
        key(KeyCode::Char('y')),
        Ok(Input::Merged(merge_target(1, ""), Ok(()))),
        key(KeyCode::Tab),
    ]
}

fn behind(number: u64, title: &str) -> PullRequest {
    let mut pull = open(number, title);
    pull.merge_state = MergeState::Behind;
    pull
}
