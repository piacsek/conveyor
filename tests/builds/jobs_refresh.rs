use conveyor::app::Request;
use conveyor::model::builds::BuildStatus;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{failing_newest, focus_builds, focus_failing_newest, three};
use crate::support::{self, Harness, builds, job, jobs, jobs_failed, key, rerun_done};

#[test]
fn a_refresh_asks_again_for_an_unsettled_runs_jobs_and_keeps_the_settled_ones() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    let running = || jobs(1026, vec![job(1, "check", BuildStatus::Running, None)]);
    inputs.push(running());
    inputs.push(builds(three()));
    inputs.push(running());
    inputs.push(builds(three()));
    inputs.push(builds(three()));

    h.run(inputs).unwrap();

    let asked = |run_id| {
        h.requests
            .iter()
            .filter(|request| **request == Request::Jobs { run_id })
            .count()
    };
    assert_eq!(
        asked(1026),
        3,
        "asked again after each answer, but never while one is in flight"
    );
    assert_eq!(asked(1025), 1, "the settled run keeps its jobs");
}

#[test]
fn a_refresh_keeps_a_running_runs_jobs_on_screen_while_it_asks_again() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(key(KeyCode::Char('d')));
    inputs.push(jobs(1025, vec![job(2, "lint", BuildStatus::Failure, None)]));
    inputs.push(jobs(
        1026,
        vec![job(1, "check", BuildStatus::Running, None)],
    ));
    inputs.push(builds(three()));

    h.run(inputs).unwrap();

    let screen = h.screen();
    assert!(!screen.contains("jobs: loading…"), "{screen}");
    assert!(screen.contains("● check  19s"), "the pane: {screen}");
    assert_eq!(
        h.requests.last(),
        Some(&Request::Jobs { run_id: 1026 }),
        "still asked again"
    );
}

#[test]
fn a_run_that_settles_while_not_selected_ends_on_its_final_jobs() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(jobs(1025, vec![job(2, "lint", BuildStatus::Failure, None)]));
    inputs.push(jobs(
        1026,
        vec![job(1, "check", BuildStatus::Running, None)],
    ));
    inputs.push(key(KeyCode::Char('j')));
    inputs.push(builds(three()));
    let mut settled = three();
    settled[0].status = BuildStatus::Failure;
    inputs.push(builds(settled.clone()));
    inputs.push(jobs(
        1026,
        vec![job(1, "check", BuildStatus::Running, None)],
    ));
    inputs.push(builds(settled));
    inputs.push(jobs(
        1026,
        vec![job(1, "check", BuildStatus::Failure, Some("Run tests"))],
    ));

    h.run(inputs).unwrap();

    assert!(!h.screen().contains("● check"), "{}", h.screen());
    assert!(h.screen().contains("✗ check · Run tests"), "{}", h.screen());
}

#[test]
fn a_failed_refetch_keeps_the_jobs_already_on_screen() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(key(KeyCode::Char('d')));
    inputs.push(jobs(
        1026,
        vec![job(1, "check", BuildStatus::Running, None)],
    ));
    inputs.push(builds(three()));
    inputs.push(jobs_failed(1026, "gh: HTTP 502"));

    h.run(inputs).unwrap();

    assert!(h.screen().contains("● check  19s"), "{}", h.screen());
    assert!(!h.screen().contains("HTTP 502"), "{}", h.screen());
}

#[test]
fn a_selected_run_that_settles_is_asked_for_its_final_jobs() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(jobs(1025, vec![job(2, "lint", BuildStatus::Failure, None)]));
    inputs.push(jobs(
        1026,
        vec![job(1, "check", BuildStatus::Running, None)],
    ));
    let mut settled = three();
    settled[0].status = BuildStatus::Failure;
    inputs.push(builds(settled));

    h.run(inputs).unwrap();

    let asked = h
        .requests
        .iter()
        .filter(|request| **request == Request::Jobs { run_id: 1026 })
        .count();
    assert_eq!(
        asked, 2,
        "asked once more after it finished: {:?}",
        h.requests
    );

    h.run(vec![jobs(
        1026,
        vec![job(1, "check", BuildStatus::Failure, Some("Run tests"))],
    )])
    .unwrap();

    assert!(!h.screen().contains("● check"), "{}", h.screen());
    assert!(h.screen().contains("✗ check · Run tests"), "{}", h.screen());
}

#[test]
fn a_re_run_that_passes_while_not_selected_drops_the_first_attempts_failure() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(jobs(
        1025,
        vec![job(2, "lint", BuildStatus::Failure, Some("Run clippy"))],
    ));
    inputs.push(jobs(
        1026,
        vec![job(1, "check", BuildStatus::Success, None)],
    ));
    let mut rerunning = three();
    rerunning[1].status = BuildStatus::Running;
    inputs.push(builds(rerunning));
    let mut passed = three();
    passed[1].status = BuildStatus::Success;
    inputs.push(builds(passed));

    h.run(inputs).unwrap();

    let asked = h
        .requests
        .iter()
        .filter(|request| **request == Request::Jobs { run_id: 1025 })
        .count();
    assert_eq!(asked, 2, "{:?}", h.requests);

    h.run(vec![jobs(
        1025,
        vec![
            job(2, "lint", BuildStatus::Success, None),
            job(3, "test", BuildStatus::Success, None),
        ],
    )])
    .unwrap();

    assert!(!h.screen().contains("✗ lint"), "{}", h.screen());
    assert!(h.screen().contains("2 jobs ok"), "{}", h.screen());
}

#[test]
fn a_rerun_from_github_that_passes_between_two_polls_drops_the_old_failure() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(jobs(
        1025,
        vec![job(2, "lint", BuildStatus::Failure, Some("Run clippy"))],
    ));
    inputs.push(jobs(
        1026,
        vec![job(1, "check", BuildStatus::Success, None)],
    ));
    let mut passed = three();
    passed[1].status = BuildStatus::Success;
    passed[1].attempt = 2;
    passed[0].status = BuildStatus::Success;
    inputs.push(builds(passed));
    inputs.push(jobs(
        1026,
        vec![job(1, "check", BuildStatus::Success, None)],
    ));

    h.run(inputs).unwrap();

    let asked = h
        .requests
        .iter()
        .filter(|request| **request == Request::Jobs { run_id: 1025 })
        .count();
    assert_eq!(asked, 2, "{:?}", h.requests);
    assert!(h.screen().contains("✗ lint"), "{}", h.screen());

    let mut rerun = job(3, "lint", BuildStatus::Success, None);
    rerun.attempt = 2;
    h.run(vec![jobs(1025, vec![rerun])]).unwrap();

    assert!(!h.screen().contains("✗ lint"), "{}", h.screen());
    assert!(h.screen().contains("1 jobs ok"), "{}", h.screen());
}

#[test]
fn a_settled_run_whose_jobs_are_from_its_current_attempt_is_not_asked_again() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    let mut lint = job(2, "lint", BuildStatus::Failure, Some("Run clippy"));
    lint.attempt = 2;
    inputs.push(jobs(1025, vec![lint]));
    let mut rerun = three();
    rerun[1].attempt = 2;
    inputs.push(builds(rerun.clone()));
    inputs.push(builds(rerun));

    h.run(inputs).unwrap();

    let asked = h
        .requests
        .iter()
        .filter(|request| **request == Request::Jobs { run_id: 1025 })
        .count();
    assert_eq!(asked, 1, "{:?}", h.requests);
}

#[test]
fn a_rerun_whose_jobs_came_back_empty_is_not_asked_on_every_refresh() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(jobs(1025, vec![]));
    let mut rerun = three();
    rerun[1].attempt = 2;
    inputs.push(builds(rerun.clone()));
    inputs.push(builds(rerun));

    h.run(inputs).unwrap();

    let asked = h
        .requests
        .iter()
        .filter(|request| **request == Request::Jobs { run_id: 1025 })
        .count();
    assert_eq!(asked, 1, "{:?}", h.requests);
}

#[test]
fn a_refresh_asks_again_for_a_running_run_that_is_not_selected() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(jobs(1025, vec![job(2, "lint", BuildStatus::Failure, None)]));
    inputs.push(jobs(
        1026,
        vec![job(1, "check", BuildStatus::Running, None)],
    ));
    inputs.push(key(KeyCode::Char('j')));
    inputs.push(builds(three()));

    h.run(inputs).unwrap();

    let asked = h
        .requests
        .iter()
        .filter(|request| **request == Request::Jobs { run_id: 1026 })
        .count();
    assert_eq!(asked, 2, "{:?}", h.requests);
}

#[test]
fn a_run_that_passes_while_not_selected_is_asked_once_more_and_ends_on_its_jobs() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(jobs(1025, vec![job(2, "lint", BuildStatus::Failure, None)]));
    inputs.push(jobs(
        1026,
        vec![job(1, "check", BuildStatus::Running, None)],
    ));
    inputs.push(key(KeyCode::Char('j')));
    let mut passed = three();
    passed[0].status = BuildStatus::Success;
    inputs.push(builds(passed));

    h.run(inputs).unwrap();

    let asked = h
        .requests
        .iter()
        .filter(|request| **request == Request::Jobs { run_id: 1026 })
        .count();
    assert_eq!(asked, 2, "{:?}", h.requests);

    h.run(vec![jobs(
        1026,
        vec![job(1, "check", BuildStatus::Success, None)],
    )])
    .unwrap();

    assert!(h.screen().contains("1 jobs ok"), "{}", h.screen());
}

#[test]
fn a_queue_refresh_leaves_the_main_builds_jobs_alone() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(jobs(1025, vec![job(2, "lint", BuildStatus::Failure, None)]));
    inputs.push(jobs(
        1026,
        vec![job(1, "check", BuildStatus::Running, None)],
    ));
    inputs.push(support::queue(Vec::new()));

    h.run(inputs).unwrap();

    let asked = h
        .requests
        .iter()
        .filter(|request| **request == Request::Jobs { run_id: 1026 })
        .count();
    assert_eq!(asked, 1, "{:?}", h.requests);
}

#[test]
fn a_failed_refetch_of_a_finished_run_shows_the_error_instead_of_old_jobs() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(key(KeyCode::Char('d')));
    inputs.push(jobs(
        1026,
        vec![job(1, "check", BuildStatus::Running, None)],
    ));
    inputs.push(builds(three()));
    let mut settled = three();
    settled[0].status = BuildStatus::Success;
    inputs.push(builds(settled));
    inputs.push(jobs_failed(1026, "gh: HTTP 502"));

    h.run(inputs).unwrap();

    assert!(!h.screen().contains("● check"), "{}", h.screen());
    assert!(h.screen().contains("jobs: gh: HTTP 502"), "{}", h.screen());
}

#[test]
fn a_rerun_run_keeps_its_old_jobs_on_screen_until_the_new_attempt_lands() {
    let mut h = Harness::with_size(160, 20);
    let mut inputs = focus_failing_newest();
    inputs.push(jobs(
        1025,
        vec![job(
            1,
            "check",
            BuildStatus::Failure,
            Some("Run cargo test"),
        )],
    ));
    inputs.push(key(KeyCode::Char('S')));
    inputs.push(key(KeyCode::Char('f')));
    inputs.push(rerun_done(vec![(1025, Ok(()))]));
    let mut requeued = failing_newest();
    requeued[0].status = BuildStatus::Queued;
    inputs.push(builds(requeued));
    h.run(inputs).unwrap();

    assert!(
        h.requests
            .iter()
            .filter(|request| **request == Request::Jobs { run_id: 1025 })
            .count()
            == 2,
        "asked again for the new attempt: {:?}",
        h.requests
    );
    assert!(h.screen().contains("check"), "{}", h.screen());

    h.run(vec![jobs(
        1025,
        vec![job(1, "check", BuildStatus::Running, None)],
    )])
    .unwrap();

    assert!(h.screen().contains("● check"), "{}", h.screen());
}
