use conveyor::app::{Mode, Request, Stage};
use conveyor::config::ViewKey;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::three;
use crate::support::{self, Harness, ctrl, key, pr, prs, prs_in};

#[test]
fn tab_goes_to_the_next_view_and_refreshes_it() {
    let mut h = two_filled();

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    assert_eq!(h.app.prs.active, 1);
    assert!(h.screen().contains("#7 Rotate keys"), "{}", h.screen());
    assert!(!h.screen().contains("#1 a"), "{}", h.screen());
    assert_eq!(h.requests.len(), 1);
    assert_eq!(h.requests[0], Request::Refresh(Stage::Prs, 1));
}

#[test]
fn tab_wraps_from_the_last_view_to_the_first() {
    let mut h = Harness::with_config(160, 12, support::three_views());

    h.run(vec![
        key(KeyCode::Tab),
        key(KeyCode::Tab),
        key(KeyCode::Tab),
    ])
    .unwrap();

    assert_eq!(h.app.prs.active, 0);
    assert_eq!(h.requests.last(), Some(&Request::Refresh(Stage::Prs, 0)));
}

#[test]
fn shift_tab_goes_to_the_previous_view_and_wraps() {
    let mut h = Harness::with_config(160, 12, support::three_views());

    h.run(vec![key(KeyCode::BackTab)]).unwrap();

    assert_eq!(h.app.prs.active, 2);
    assert_eq!(h.requests.len(), 1);
    assert_eq!(h.requests[0], Request::Refresh(Stage::Prs, 2));

    h.run(vec![key(KeyCode::BackTab)]).unwrap();

    assert_eq!(h.app.prs.active, 1);
}

#[test]
fn a_digit_jumps_to_that_view() {
    let mut h = Harness::with_config(160, 12, support::three_views());

    h.run(vec![key(KeyCode::Char('3'))]).unwrap();

    assert_eq!(h.app.prs.active, 2);
    assert_eq!(h.requests.len(), 1);
    assert_eq!(h.requests[0], Request::Refresh(Stage::Prs, 2));

    h.run(vec![key(KeyCode::Char('2'))]).unwrap();

    assert_eq!(h.app.prs.active, 1);
}

#[test]
fn a_digit_past_the_last_view_does_nothing() {
    let mut h = two_filled();

    h.run(vec![key(KeyCode::Char('3')), key(KeyCode::Char('9'))])
        .unwrap();

    assert_eq!(h.app.prs.active, 0);
    assert_eq!(h.app.focus, Stage::Prs);
    assert!(h.requests.is_empty(), "{:?}", h.requests);
}

#[test]
fn the_active_views_digit_does_nothing() {
    let mut h = two_filled();

    h.run(vec![key(KeyCode::Char('1'))]).unwrap();

    assert_eq!(h.app.prs.active, 0);
    assert!(h.requests.is_empty(), "{:?}", h.requests);
}

#[test]
fn with_one_view_the_view_keys_do_nothing() {
    let mut h = Harness::new();

    h.run(vec![
        prs(three()),
        key(KeyCode::Tab),
        key(KeyCode::BackTab),
        key(KeyCode::Char('1')),
        key(KeyCode::Char('2')),
    ])
    .unwrap();

    assert_eq!(h.app.prs.active, 0);
    assert_eq!(h.app.focus, Stage::Prs);
    assert!(h.requests.is_empty(), "{:?}", h.requests);
}

#[test]
fn on_another_column_the_view_keys_do_nothing() {
    let mut h = two_filled();

    h.run(vec![
        key(KeyCode::Char('l')),
        key(KeyCode::Tab),
        key(KeyCode::BackTab),
        key(KeyCode::Char('2')),
    ])
    .unwrap();

    assert_eq!(h.app.prs.active, 0);
    assert_eq!(h.app.focus, Stage::Queue);
    assert!(h.requests.is_empty(), "{:?}", h.requests);
}

#[test]
fn h_and_l_still_move_the_focus_with_several_views() {
    let mut h = two_filled();

    h.run(vec![key(KeyCode::Char('l')), key(KeyCode::Char('l'))])
        .unwrap();

    assert_eq!(h.app.focus, Stage::Builds);

    h.run(vec![key(KeyCode::Char('h'))]).unwrap();

    assert_eq!(h.app.focus, Stage::Queue);
}

#[test]
fn each_view_keeps_its_own_selection() {
    let mut h = two_filled();

    h.run(vec![
        key(KeyCode::Char('j')),
        key(KeyCode::Tab),
        key(KeyCode::Char('j')),
    ])
    .unwrap();

    assert_eq!(h.app.prs.column().selected().unwrap().number, 8);

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    assert_eq!(h.app.prs.column().selected().unwrap().number, 2);

    h.run(vec![key(KeyCode::Char('2'))]).unwrap();

    assert_eq!(h.app.prs.column().selected().unwrap().number, 8);
}

#[test]
fn each_view_keeps_its_own_filter() {
    let mut h = two_filled();

    h.run(vec![
        key(KeyCode::Char('/')),
        key(KeyCode::Char('2')),
        key(KeyCode::Enter),
        key(KeyCode::Tab),
    ])
    .unwrap();

    assert_eq!(h.app.filter(), None);
    assert!(h.screen().contains("#7 Rotate keys"), "{}", h.screen());
    assert!(h.screen().contains("#8 Bump deps"), "{}", h.screen());

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    assert_eq!(h.app.filter(), Some("2"));
    assert!(h.screen().contains("#2 b"), "{}", h.screen());
    assert!(!h.screen().contains("#1 a"), "{}", h.screen());
}

#[test]
fn switching_resets_the_scroll_and_closes_nothing() {
    let mut h = Harness::with_config(160, 30, support::two_views());
    h.run(vec![
        prs(three()),
        prs_in(1, rows()),
        key(KeyCode::Char('z')),
        key(KeyCode::Char('d')),
        ctrl('d'),
    ])
    .unwrap();
    h.app.details_scroll = 4;

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    assert_eq!(h.app.details_scroll, 0);
    assert!(h.app.details);
    assert!(h.app.zoom);
}

#[test]
fn the_details_pane_follows_the_selected_row_of_the_new_view() {
    let mut h = Harness::with_config(160, 30, support::two_views());
    h.run(vec![
        prs(three()),
        prs_in(1, rows()),
        key(KeyCode::Char('d')),
        key(KeyCode::Tab),
    ])
    .unwrap();

    let screen = h.screen();
    assert_eq!(screen.matches("Rotate keys").count(), 2, "{screen}");
}

#[test]
fn tab_does_nothing_while_the_filter_is_typed() {
    let mut h = two_filled();

    h.run(vec![key(KeyCode::Char('/')), key(KeyCode::Tab)])
        .unwrap();

    assert_eq!(h.app.prs.active, 0);
    assert_eq!(h.app.mode, Mode::Filter(String::new()));
    assert!(h.requests.is_empty(), "{:?}", h.requests);
}

#[test]
fn a_digit_is_typed_into_the_filter() {
    let mut h = two_filled();

    h.run(vec![key(KeyCode::Char('/')), key(KeyCode::Char('2'))])
        .unwrap();

    assert_eq!(h.app.prs.active, 0);
    assert_eq!(h.app.filter(), Some("2"));
}

#[test]
fn a_configured_character_switches_and_tab_then_does_nothing() {
    let mut config = support::two_views();
    config.keys.next_view = ViewKey::Char(']');
    let mut h = Harness::with_config(160, 12, config);

    h.run(vec![key(KeyCode::Tab), key(KeyCode::BackTab)])
        .unwrap();

    assert_eq!(h.app.prs.active, 0);
    assert!(h.requests.is_empty(), "{:?}", h.requests);

    h.run(vec![key(KeyCode::Char(']'))]).unwrap();

    assert_eq!(h.app.prs.active, 1);
    assert_eq!(h.requests.len(), 1);
    assert_eq!(h.requests[0], Request::Refresh(Stage::Prs, 1));
}

fn two_filled() -> Harness {
    let mut h = Harness::with_config(160, 12, support::two_views());
    h.run(vec![prs(three()), prs_in(1, rows())]).unwrap();
    h
}

fn rows() -> Vec<conveyor::model::prs::PullRequest> {
    vec![pr(7, "Rotate keys"), pr(8, "Bump deps")]
}
