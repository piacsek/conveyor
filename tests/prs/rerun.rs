use ratatui::crossterm::event::KeyCode;

use crate::support::{self, Harness, NOW, key, pr, prs, tick_at};

#[test]
fn f_on_a_pull_request_reruns_every_finished_run_behind_its_failing_checks() {
    use conveyor::app::Request;
    let mut h = Harness::with_size(160, 20);
    let mut failing = pr(4821, "Retry hooks");
    failing.repo = "acme/api".to_string();
    failing.checks_detail = vec![failing_in("test", 70), failing_in("e2e", 80)];

    h.run(vec![prs(vec![failing]), key(KeyCode::Char('S'))])
        .unwrap();

    let screen = h.screen();
    assert!(screen.contains("f  rerun failed jobs (2 runs)"), "{screen}");

    h.run(vec![key(KeyCode::Char('f'))]).unwrap();

    assert_eq!(
        h.requests.last(),
        Some(&Request::Rerun {
            repo: "acme/api".to_string(),
            run_ids: vec![70, 80],
        })
    );
}

#[test]
fn a_rerun_that_went_through_for_some_runs_says_which_failed_and_still_refreshes() {
    use conveyor::app::{Request, Stage};
    let mut h = Harness::with_size(160, 20);
    let mut failing = pr(4821, "Retry hooks");
    failing.checks_detail = vec![failing_in("test", 70), failing_in("e2e", 80)];

    h.run(vec![
        prs(vec![failing]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('f')),
        support::rerun_done(vec![(70, Ok(())), (80, Err("HTTP 403"))]),
    ])
    .unwrap();

    let footer = h.screen().lines().last().unwrap_or_default().to_string();
    assert!(
        footer.contains("rerun requested for 1 of 2 runs; run 80: HTTP 403"),
        "{footer}"
    );
    assert_eq!(h.requests.last(), Some(&Request::Refresh(Stage::Prs, 0)));
}

#[test]
fn f_refuses_when_the_runs_behind_the_pull_request_changed_while_the_popup_was_open() {
    use conveyor::app::Request;
    let mut h = Harness::with_size(160, 20);
    let mut failing = pr(4821, "Retry hooks");
    failing.checks_detail = vec![failing_in("test", 70), failing_in("e2e", 80)];
    let mut one_left = failing.clone();
    one_left.checks_detail[1].run_finished = false;

    h.run(vec![
        prs(vec![failing]),
        key(KeyCode::Char('S')),
        prs(vec![one_left]),
        key(KeyCode::Char('f')),
    ])
    .unwrap();

    let footer = h.screen().lines().last().unwrap_or_default().to_string();
    assert!(
        footer.contains("the pull request changed; press S again"),
        "{footer}"
    );
    assert!(
        !h.requests
            .iter()
            .any(|request| matches!(request, Request::Rerun { .. })),
        "{:?}",
        h.requests
    );
}

#[test]
fn s_on_a_pull_request_refuses_while_the_last_refresh_failed() {
    let mut h = Harness::with_size(160, 20);
    let mut failing = pr(4821, "Retry hooks");
    failing.checks_detail = vec![failing_in("test", 70)];

    h.run(vec![
        prs(vec![failing]),
        support::failed(conveyor::app::Stage::Prs, "gh: HTTP 502"),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    let footer = h.screen().lines().last().unwrap_or_default().to_string();
    assert!(
        footer.contains("the last refresh failed; press r and try again"),
        "{footer}"
    );
    assert!(h.requests.is_empty(), "{:?}", h.requests);
}

#[test]
fn a_pull_request_with_nothing_finished_and_failing_lists_the_rerun_as_refused() {
    let mut h = Harness::with_size(160, 20);

    h.run(vec![
        prs(vec![pr(4821, "Retry hooks")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('f')),
    ])
    .unwrap();

    assert!(
        h.screen()
            .contains("f  rerun failed jobs · no finished failing run to rerun"),
        "{}",
        h.screen()
    );
    assert!(h.requests.is_empty(), "{:?}", h.requests);
}

#[test]
fn a_rerun_from_my_prs_reloads_them_once_more_ten_seconds_later() {
    use conveyor::app::{Request, Stage};
    let mut h = Harness::with_size(160, 20);
    let mut failing = pr(4821, "Retry hooks");
    failing.checks_detail = vec![failing_in("test", 70)];

    h.run(vec![
        prs(vec![failing]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('f')),
        support::rerun_done(vec![(70, Ok(()))]),
        tick_at(NOW + 9),
    ])
    .unwrap();

    let refreshes = |requests: &[Request]| {
        requests
            .iter()
            .filter(|request| **request == Request::Refresh(Stage::Prs, 0))
            .count()
    };
    assert_eq!(refreshes(&h.requests), 1);

    h.run(vec![tick_at(NOW + 10), tick_at(NOW + 30)]).unwrap();

    assert_eq!(refreshes(&h.requests), 2);
}

#[test]
fn a_rerun_that_failed_for_every_run_schedules_no_reload() {
    use conveyor::app::{Request, Stage};
    let mut h = Harness::with_size(160, 20);
    let mut failing = pr(4821, "Retry hooks");
    failing.checks_detail = vec![failing_in("test", 70)];

    h.run(vec![
        prs(vec![failing]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('f')),
        support::rerun_done(vec![(70, Err("HTTP 403"))]),
        tick_at(NOW + 30),
    ])
    .unwrap();

    assert!(
        !h.requests.contains(&Request::Refresh(Stage::Prs, 0)),
        "{:?}",
        h.requests
    );
}

#[test]
fn two_reruns_on_their_way_each_land_with_their_own_label_and_column() {
    use conveyor::app::{Request, Stage};
    use conveyor::model::builds::BuildStatus;
    let mut h = Harness::with_size(160, 20);
    let mut failing = pr(4821, "Retry hooks");
    failing.checks_detail = vec![failing_in("test", 70), failing_in("e2e", 80)];

    h.run(vec![
        prs(vec![failing]),
        support::builds(vec![support::build(25, BuildStatus::Failure, None)]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('f')),
        key(KeyCode::Char('3')),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('f')),
    ])
    .unwrap();

    assert_eq!(
        h.requests.last(),
        Some(&Request::Rerun {
            repo: "acme/webapp".to_string(),
            run_ids: vec![1025],
        })
    );

    h.run(vec![support::rerun_done(vec![(1025, Ok(()))])])
        .unwrap();

    let footer = h.screen().lines().last().unwrap_or_default().to_string();
    assert!(footer.contains("rerun requested: run 25"), "{footer}");
    assert_eq!(h.requests.last(), Some(&Request::Refresh(Stage::Builds, 0)));

    h.run(vec![support::rerun_done(vec![(70, Ok(())), (80, Ok(()))])])
        .unwrap();

    let footer = h.screen().lines().last().unwrap_or_default().to_string();
    assert!(
        footer.contains("rerun requested: #4821 (2 runs)"),
        "{footer}"
    );
    assert_eq!(h.requests.last(), Some(&Request::Refresh(Stage::Prs, 0)));
}

#[test]
fn a_failed_rerun_from_my_prs_marks_the_pull_request_card() {
    let screen = rerun_from_my_prs(vec![(70, Err("HTTP 403")), (80, Err("HTTP 403"))]);

    assert!(
        line_after(&screen, "#4821 Retry hooks").contains("! rerun failed: run 70: HTTP 403"),
        "{screen}"
    );
}

#[test]
fn a_rerun_that_failed_for_some_runs_says_how_many_on_the_card() {
    let screen = rerun_from_my_prs(vec![(70, Ok(())), (80, Err("HTTP 403"))]);

    assert!(
        line_after(&screen, "#4821 Retry hooks").contains("! rerun failed: 1 of 2 runs"),
        "{screen}"
    );
}

fn rerun_from_my_prs(results: Vec<(u64, Result<(), &str>)>) -> String {
    let mut h = Harness::with_size(160, 20);
    let mut failing = pr(4821, "Retry hooks");
    failing.checks_detail = vec![failing_in("test", 70), failing_in("e2e", 80)];
    h.run(vec![
        prs(vec![failing]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('f')),
        support::rerun_done(results),
    ])
    .unwrap();
    h.screen()
}

fn line_after(screen: &str, needle: &str) -> String {
    let lines: Vec<&str> = screen.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("{needle} is not on screen: {screen}"));
    lines[at + 1].to_string()
}

fn failing_in(name: &str, run_id: u64) -> conveyor::model::prs::Check {
    conveyor::model::prs::Check {
        name: name.to_string(),
        conclusion: conveyor::model::prs::CheckConclusion::Failure,
        run_id: Some(run_id),
        run_finished: true,
        ..Default::default()
    }
}
