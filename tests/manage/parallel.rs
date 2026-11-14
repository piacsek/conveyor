use conveyor::model::prs::StatusChange;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{draft, line_after, status, status_changed, tall};
use crate::support::{Harness, key, prs, prs_in, two_views};

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

#[test]
fn a_write_in_flight_spins_on_its_pull_request_in_every_view_that_lists_it() {
    let mut h = Harness::with_config(160, 20, two_views());

    h.run(vec![
        prs(vec![draft(12, "Retry webhooks")]),
        prs_in(
            1,
            vec![draft(14, "Bump serde"), draft(12, "Retry webhooks")],
        ),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        key(KeyCode::Tab),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        line_after(&screen, "#12 Retry webhooks").contains("marking ready…"),
        "{screen}"
    );
}
