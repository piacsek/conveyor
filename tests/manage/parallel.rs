use conveyor::model::prs::{PullRequest, StatusChange};
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{open, status, status_changed, tall};
use crate::support::{key, prs};

#[test]
fn s_opens_on_another_pull_request_while_a_status_change_is_on_its_way() {
    let mut h = tall();

    h.run(vec![
        prs(vec![draft(1, "a"), draft(2, "b")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(h.screen().contains("Manage #2"), "{}", h.screen());

    h.run(vec![key(KeyCode::Char('t'))]).unwrap();

    assert_eq!(h.requests.len(), 2, "{:?}", h.requests);
    assert_eq!(h.requests[1], status(2, "b", StatusChange::Ready));
}

#[test]
fn s_still_waits_on_the_pull_request_whose_status_change_is_on_its_way() {
    let mut h = tall();

    h.run(vec![
        prs(vec![draft(1, "a"), draft(2, "b")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('k')),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(h.screen().contains("marking #1 ready…"), "{}", h.screen());
    assert!(!h.screen().contains("Manage #1"), "{}", h.screen());
}

#[test]
fn a_landed_status_change_frees_only_its_own_pull_request() {
    let mut h = tall();

    h.run(vec![
        prs(vec![draft(1, "a"), draft(2, "b")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        status_changed(1, StatusChange::Ready, Ok(())),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(h.screen().contains("marking #2 ready…"), "{}", h.screen());
    assert!(!h.screen().contains("Manage #2"), "{}", h.screen());

    h.run(vec![key(KeyCode::Char('k')), key(KeyCode::Char('S'))])
        .unwrap();

    assert!(h.screen().contains("Manage #1"), "{}", h.screen());
}

fn draft(number: u64, title: &str) -> PullRequest {
    let mut pull = open(number, title);
    pull.is_draft = true;
    pull
}
