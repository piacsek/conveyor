use conveyor::app::{Request, Stage};
use conveyor::model::prs::StatusChange;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{line_after, open, status, status_changed, tall};
use crate::support::{key, prs};

#[test]
fn the_popup_offers_to_approve_someone_elses_pull_request() {
    let mut h = tall();

    h.run(vec![
        prs(vec![open(1, "Retry webhooks")]),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("v  approve"), "{screen}");
    assert!(!screen.contains("v  approve ·"), "{screen}");
}

#[test]
fn v_pressed_outside_the_popup_sends_nothing_and_opens_nothing() {
    let mut h = tall();
    h.run(vec![prs(vec![open(1, "Retry webhooks")])]).unwrap();
    let before = h.screen();

    h.run(vec![key(KeyCode::Char('v'))]).unwrap();

    assert!(h.requests.is_empty(), "{:?}", h.requests);
    assert_eq!(h.screen(), before);
}

#[test]
fn on_the_viewers_own_pull_request_approve_is_dim_with_the_reason() {
    let mut h = tall();
    let mut mine = open(1, "Retry webhooks");
    mine.viewer_did_author = true;

    h.run(vec![prs(vec![mine]), key(KeyCode::Char('S'))])
        .unwrap();

    let screen = h.screen();
    assert!(screen.contains("v  approve · you opened #1"), "{screen}");
}

#[test]
fn approve_is_dim_once_the_viewer_approved_the_current_head() {
    let mut h = tall();
    let mut approved = open(1, "Retry webhooks");
    approved.viewer_approved_oid = Some(approved.head_oid.clone());

    h.run(vec![prs(vec![approved]), key(KeyCode::Char('S'))])
        .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("v  approve · you already approved #1"),
        "{screen}"
    );
}

#[test]
fn approve_is_offered_again_once_the_head_moved_past_the_viewers_approval() {
    let mut h = tall();
    let mut approved = open(1, "Retry webhooks");
    approved.viewer_approved_oid = Some("4b9e01c3".to_string());

    h.run(vec![prs(vec![approved]), key(KeyCode::Char('S'))])
        .unwrap();

    let screen = h.screen();
    assert!(screen.contains("v  approve"), "{screen}");
    assert!(!screen.contains("v  approve ·"), "{screen}");
}

#[test]
fn v_asks_before_approving_and_y_approves() {
    let mut h = tall();

    h.run(vec![
        prs(vec![open(1, "Retry webhooks")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('v')),
    ])
    .unwrap();

    assert!(
        h.screen().contains("approve #1 Retry webhooks?"),
        "{}",
        h.screen()
    );
    assert!(h.requests.is_empty());

    h.run(vec![key(KeyCode::Char('y'))]).unwrap();

    assert_eq!(h.requests, vec![status(1, "Retry webhooks", approve(1))]);
}

#[test]
fn while_the_approve_runs_the_footer_and_the_card_say_so() {
    let mut h = tall();

    h.run(vec![
        prs(vec![open(1, "Retry webhooks")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('v')),
        key(KeyCode::Char('y')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("approving #1…"), "{screen}");
    assert!(
        line_after(&screen, "#1 Retry webhooks").contains("approving…"),
        "{screen}"
    );
}

#[test]
fn an_approve_that_went_through_says_so_and_refreshes_only_my_prs() {
    let mut h = tall();

    h.run(vec![
        prs(vec![open(1, "Retry webhooks")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('v')),
        key(KeyCode::Char('y')),
        status_changed(1, approve(1), Ok(())),
    ])
    .unwrap();

    assert!(h.screen().contains("#1 approved"), "{}", h.screen());
    assert_eq!(h.requests[1], Request::Refresh(Stage::Prs, 0));
    assert_eq!(h.requests.len(), 2, "{:?}", h.requests);
}

#[test]
fn a_failed_approve_marks_the_card() {
    let mut h = tall();

    h.run(vec![
        prs(vec![open(1, "Retry webhooks")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('v')),
        key(KeyCode::Char('y')),
        status_changed(1, approve(1), Err("HTTP 422")),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        line_after(&screen, "#1 Retry webhooks").contains("! approve failed: HTTP 422"),
        "{screen}"
    );
}

#[test]
fn y_refuses_to_approve_once_the_head_moved_while_the_dialog_was_open() {
    let mut h = tall();
    let mut pushed = open(1, "Retry webhooks");
    pushed.head_oid = "4b9e01c3".to_string();

    h.run(vec![
        prs(vec![open(1, "Retry webhooks")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('v')),
        prs(vec![pushed]),
        key(KeyCode::Char('y')),
    ])
    .unwrap();

    assert!(
        h.screen()
            .contains("the pull request changed; press S again"),
        "{}",
        h.screen()
    );
    assert!(h.requests.is_empty(), "{:?}", h.requests);
}

fn approve(number: u64) -> StatusChange {
    StatusChange::Approve {
        node_id: format!("PR_node{number}"),
        head_oid: format!("8c1f2a7d{number}"),
    }
}
