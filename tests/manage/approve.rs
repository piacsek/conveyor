use ratatui::crossterm::event::KeyCode;

use crate::helpers::{open, tall};
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
