use conveyor::app::Stage;
use ratatui::style::Modifier;

use crate::helpers::two_repos;
use crate::support::{Harness, failed_in, no_queue_in, queue_in};

const QUEUE_MONOREPO: u16 = 41;
const QUEUE_INFRA: u16 = 52;
const BUILDS_MONOREPO: u16 = 81;
const BUILDS_INFRA: u16 = 92;

#[test]
fn an_inactive_tab_without_a_merge_queue_is_struck_and_stays_dim() {
    let mut h = Harness::with_config(160, 12, two_repos());

    h.run(vec![no_queue_in(1, "acme/infra")]).unwrap();

    let tab = h.cell(QUEUE_INFRA, 1).modifier;
    assert!(tab.contains(Modifier::CROSSED_OUT), "{tab:?}");
    assert!(tab.contains(Modifier::DIM), "{tab:?}");
}

#[test]
fn the_active_tab_without_a_merge_queue_is_struck_and_keeps_the_active_emphasis() {
    let mut h = Harness::with_config(160, 12, two_repos());

    h.run(vec![no_queue_in(0, "acme/monorepo")]).unwrap();

    let tab = h.cell(QUEUE_MONOREPO, 1).modifier;
    assert!(tab.contains(Modifier::CROSSED_OUT), "{tab:?}");
    assert!(tab.contains(Modifier::BOLD), "{tab:?}");
    assert!(tab.contains(Modifier::REVERSED), "{tab:?}");
    assert!(!tab.contains(Modifier::DIM), "{tab:?}");
}

#[test]
fn the_builds_bar_never_strikes_a_repository_without_a_merge_queue() {
    let mut h = Harness::with_config(160, 12, two_repos());

    h.run(vec![
        no_queue_in(0, "acme/monorepo"),
        no_queue_in(1, "acme/infra"),
    ])
    .unwrap();

    let active = h.cell(BUILDS_MONOREPO, 1).modifier;
    let inactive = h.cell(BUILDS_INFRA, 1).modifier;
    assert!(!active.contains(Modifier::CROSSED_OUT), "{active:?}");
    assert!(!inactive.contains(Modifier::CROSSED_OUT), "{inactive:?}");
}

#[test]
fn no_tab_is_struck_before_the_first_answer() {
    let mut h = Harness::with_config(160, 12, two_repos());

    h.run(vec![]).unwrap();

    let active = h.cell(QUEUE_MONOREPO, 1).modifier;
    let inactive = h.cell(QUEUE_INFRA, 1).modifier;
    assert!(!active.contains(Modifier::CROSSED_OUT), "{active:?}");
    assert!(!inactive.contains(Modifier::CROSSED_OUT), "{inactive:?}");
}

#[test]
fn a_failed_call_after_no_queue_keeps_the_strike() {
    let mut h = Harness::with_config(160, 12, two_repos());
    h.run(vec![no_queue_in(0, "acme/monorepo")]).unwrap();

    h.run(vec![failed_in(Stage::Queue, 0, "API rate limit exceeded")])
        .unwrap();

    let tab = h.cell(QUEUE_MONOREPO, 1).modifier;
    assert!(tab.contains(Modifier::CROSSED_OUT), "{tab:?}");
}

#[test]
fn a_queue_that_appears_loses_the_strike() {
    let mut h = Harness::with_config(160, 12, two_repos());
    h.run(vec![no_queue_in(0, "acme/monorepo")]).unwrap();

    h.run(vec![queue_in(0, "acme/monorepo", vec![])]).unwrap();

    let tab = h.cell(QUEUE_MONOREPO, 1).modifier;
    assert!(!tab.contains(Modifier::CROSSED_OUT), "{tab:?}");
    assert!(tab.contains(Modifier::BOLD), "{tab:?}");
}
