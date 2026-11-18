use conveyor::app::{Request, Stage};
use conveyor::model::builds::BuildStatus;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{deploy_first, on_infra_builds, running, running_infra_run, two_repos};
use crate::support::{
    Harness, build, builds_in, deployed, deployment, job, jobs, key, queue_in, rerun_done,
};

#[test]
fn main_builds_shows_the_repository_of_the_active_queue_tab() {
    let mut h = Harness::with_config(160, 24, two_repos());
    h.run(vec![
        builds_in(
            1,
            "acme/infra",
            vec![build(
                9,
                BuildStatus::Success,
                Some((40, "bob", "Bump nodes")),
            )],
        ),
        builds_in(
            0,
            "acme/monorepo",
            vec![build(
                7,
                BuildStatus::Success,
                Some((12, "alice", "Retry webhooks")),
            )],
        ),
        key(KeyCode::Char('l')),
    ])
    .unwrap();

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("Main infra (1)"), "{screen}");
    assert!(screen.contains("Bump nodes"), "{screen}");
    assert!(!screen.contains("Retry webhooks"), "{screen}");
}

#[test]
fn deployed_counts_the_builds_of_the_deploy_repository_while_the_queue_shows_another() {
    let mut h = Harness::with_config(160, 24, deploy_first());
    h.run(vec![
        builds_in(
            0,
            "acme/monorepo",
            vec![
                build(26, BuildStatus::Success, Some((14, "bob", "Speed up CI"))),
                build(25, BuildStatus::Success, Some((13, "carol", "Rate limits"))),
                build(
                    24,
                    BuildStatus::Success,
                    Some((12, "alice", "Retry webhooks")),
                ),
            ],
        ),
        builds_in(
            1,
            "acme/infra",
            vec![build(
                9,
                BuildStatus::Success,
                Some((40, "bob", "Bump nodes")),
            )],
        ),
        deployed(vec![deployment(
            "prod",
            24,
            Some((12, "alice", "Retry webhooks")),
        )]),
        key(KeyCode::Char('l')),
    ])
    .unwrap();

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("Main infra (1)"), "{screen}");
    assert!(screen.contains("↓2"), "{screen}");
}

#[test]
fn r_on_main_builds_refreshes_the_repository_it_shows() {
    let mut h = Harness::with_config(160, 24, two_repos());
    h.run(on_infra_builds()).unwrap();
    h.requests.clear();

    h.run(vec![key(KeyCode::Char('r'))]).unwrap();

    assert_eq!(h.requests, vec![Request::Refresh(Stage::Builds, 1)]);
}

#[test]
fn shift_r_refreshes_the_main_builds_repository_shown() {
    let mut h = Harness::with_config(160, 24, two_repos());
    h.run(on_infra_builds()).unwrap();
    h.requests.clear();

    h.run(vec![key(KeyCode::Char('R'))]).unwrap();

    assert!(
        h.requests.contains(&Request::Refresh(Stage::Builds, 1)),
        "{:?}",
        h.requests
    );
}

#[test]
fn a_landed_rerun_refreshes_the_main_builds_repository_shown() {
    let mut h = Harness::with_config(160, 24, two_repos());
    h.run(on_infra_builds()).unwrap();

    h.run(vec![
        key(KeyCode::Char('S')),
        key(KeyCode::Char('f')),
        rerun_done(vec![(1025, Ok(()))]),
    ])
    .unwrap();

    assert_eq!(h.requests.last(), Some(&Request::Refresh(Stage::Builds, 1)));
}

#[test]
fn a_rerun_landing_after_a_queue_switch_refreshes_the_repository_it_started_on() {
    let mut h = Harness::with_config(160, 24, two_repos());
    h.run(on_infra_builds()).unwrap();

    h.run(vec![
        key(KeyCode::Char('S')),
        key(KeyCode::Char('f')),
        key(KeyCode::Char('h')),
        key(KeyCode::BackTab),
        rerun_done(vec![(1025, Ok(()))]),
    ])
    .unwrap();

    assert_eq!(h.requests.last(), Some(&Request::Refresh(Stage::Builds, 1)));
}

#[test]
fn a_hidden_repository_result_asks_nothing_for_the_shown_running_build() {
    let mut h = Harness::with_config(160, 24, two_repos());
    h.run(vec![
        builds_in(0, "acme/monorepo", vec![running(26)]),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        jobs(1026, vec![job(7, "check", BuildStatus::Running, None)]),
    ])
    .unwrap();
    h.requests.clear();

    h.run(vec![builds_in(1, "acme/infra", vec![running(9)])])
        .unwrap();

    assert!(h.requests.is_empty(), "{:?}", h.requests);
}

#[test]
fn a_hidden_repository_result_ages_its_own_running_build_for_when_it_shows_again() {
    let mut h = Harness::with_config(160, 24, two_repos());
    h.run(vec![
        builds_in(1, "acme/infra", vec![running(9)]),
        key(KeyCode::Char('l')),
        key(KeyCode::Tab),
        key(KeyCode::Char('l')),
        jobs(1009, vec![job(7, "check", BuildStatus::Running, None)]),
        key(KeyCode::Char('h')),
        key(KeyCode::BackTab),
        builds_in(1, "acme/infra", vec![running(9)]),
    ])
    .unwrap();
    h.requests.clear();

    h.run(vec![key(KeyCode::Tab), key(KeyCode::Char('l'))])
        .unwrap();

    assert!(
        h.requests
            .iter()
            .any(|request| matches!(request, Request::Jobs { run_id: 1009, .. })),
        "{:?}",
        h.requests
    );
}

#[test]
fn a_late_monorepo_queue_result_asks_nothing_for_the_shown_infra_run() {
    let mut h = Harness::with_config(160, 24, two_repos());
    h.run(vec![
        key(KeyCode::Char('l')),
        key(KeyCode::Tab),
        queue_in(1, "acme/infra", running_infra_run()),
        jobs(4242, vec![job(7, "check", BuildStatus::Running, None)]),
    ])
    .unwrap();
    h.requests.clear();

    h.run(vec![queue_in(0, "acme/monorepo", vec![])]).unwrap();

    assert!(h.requests.is_empty(), "{:?}", h.requests);
}

#[test]
fn main_builds_keeps_its_filtered_selection_across_a_queue_round_trip() {
    let mut h = Harness::with_config(160, 24, two_repos());
    h.run(vec![
        builds_in(
            0,
            "acme/monorepo",
            vec![
                build(
                    7,
                    BuildStatus::Success,
                    Some((12, "alice", "Retry webhooks")),
                ),
                build(8, BuildStatus::Success, Some((13, "carol", "Rate limits"))),
            ],
        ),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('/')),
        key(KeyCode::Char('R')),
        key(KeyCode::Char('e')),
        key(KeyCode::Char('t')),
        key(KeyCode::Char('r')),
        key(KeyCode::Enter),
        key(KeyCode::Char('h')),
        key(KeyCode::Tab),
    ])
    .unwrap();

    h.run(vec![key(KeyCode::BackTab)]).unwrap();

    assert_eq!(
        h.app.builds.active().column.selected().unwrap().run_number,
        7
    );
}
