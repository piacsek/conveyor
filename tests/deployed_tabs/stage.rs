use conveyor::app::{Request, Stage};
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{to_deployed, two_systems, typed_filter};
use crate::support::{Harness, deployed_in, deployment, failed_in, key};

#[test]
fn a_result_lands_in_the_tab_it_was_fetched_for() {
    let mut h = Harness::with_config(160, 24, two_systems());
    h.run(vec![
        deployed_in(1, "worker", vec![deployment("canary", 31, None)]),
        deployed_in(0, "api", vec![deployment("staging", 24, None)]),
    ])
    .unwrap();
    h.run(to_deployed()).unwrap();

    let api = h.screen();
    h.run(vec![key(KeyCode::Tab)]).unwrap();
    let worker = h.screen();

    assert!(api.contains("staging"), "{api}");
    assert!(!api.contains("canary"), "{api}");
    assert!(worker.contains("canary"), "{worker}");
    assert!(!worker.contains("staging"), "{worker}");
}

#[test]
fn a_refresh_lands_in_its_tab_after_a_switch_away() {
    let mut h = Harness::with_config(160, 24, two_systems());
    h.run(to_deployed()).unwrap();
    h.run(vec![key(KeyCode::Char('r')), key(KeyCode::Tab)])
        .unwrap();

    h.run(vec![deployed_in(
        0,
        "api",
        vec![deployment("staging", 24, None)],
    )])
    .unwrap();
    let worker = h.screen();
    h.run(vec![key(KeyCode::BackTab)]).unwrap();
    let api = h.screen();

    assert!(h.requests.contains(&Request::Refresh(Stage::Deployed, 0)));
    assert!(!worker.contains("staging"), "{worker}");
    assert!(api.contains("staging"), "{api}");
}

#[test]
fn a_failed_fetch_marks_only_its_tab() {
    let mut h = Harness::with_config(160, 24, two_systems());
    h.run(vec![
        deployed_in(0, "api", vec![deployment("staging", 24, None)]),
        deployed_in(1, "worker", vec![deployment("canary", 31, None)]),
    ])
    .unwrap();
    h.run(to_deployed()).unwrap();
    h.run(vec![key(KeyCode::Tab)]).unwrap();

    h.run(vec![failed_in(Stage::Deployed, 0, "cluster unreachable")])
        .unwrap();

    assert!(h.app.deployed.active().column.error.is_none());
    assert!(h.app.deployed.tab(0).column.error.is_some());
    assert_eq!(h.app.deployed.tab(0).column.all().len(), 1);
}

#[test]
fn a_switch_clears_the_filter_of_the_tab_left_and_keeps_its_selected_row() {
    let mut h = Harness::with_config(160, 24, two_systems());
    h.run(vec![deployed_in(
        0,
        "api",
        vec![
            deployment("staging", 24, None),
            deployment("prod", 23, None),
        ],
    )])
    .unwrap();
    h.run(to_deployed()).unwrap();
    h.run(typed_filter("prod")).unwrap();

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    let column = &h.app.deployed.tab(0).column;
    assert_eq!(column.filter, None);
    assert_eq!(column.selected().unwrap().env, "prod");
}

#[test]
fn a_row_without_a_sha_keeps_the_last_one_its_own_tab_knew() {
    let mut h = Harness::with_config(160, 24, two_systems());
    h.run(vec![
        deployed_in(0, "api", vec![deployment("prod", 24, None)]),
        deployed_in(1, "worker", vec![deployment("prod", 31, None)]),
    ])
    .unwrap();
    let mut unknown = deployment("prod", 0, None);
    unknown.sha = None;

    h.run(vec![deployed_in(1, "worker", vec![unknown])])
        .unwrap();

    let worker = &h.app.deployed.tab(1).column.all()[0];
    assert_eq!(worker.sha.as_deref(), Some(format!("{:040x}", 31).as_str()));
}
