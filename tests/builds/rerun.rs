use conveyor::app::{Request, Stage};
use conveyor::model::builds::BuildStatus;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{failing_newest, focus_builds, focus_failing_newest, line_after};
use crate::support::{Harness, NOW, build, builds, failed, key, rerun_done, tick_at};

#[test]
fn f_checks_the_target_again_when_a_refresh_landed_while_the_popup_was_open() {
    let mut h = Harness::with_size(160, 20);
    let mut inputs = focus_failing_newest();
    inputs.push(key(KeyCode::Char('S')));
    let mut newer = vec![build(27, BuildStatus::Running, None)];
    newer.extend(failing_newest());
    inputs.push(builds(newer));
    inputs.push(key(KeyCode::Char('f')));
    h.run(inputs).unwrap();

    assert!(reruns(&h.requests).is_empty(), "{:?}", h.requests);
    assert!(
        footer(&h.screen()).contains("only the newest main build can be rerun"),
        "{}",
        h.screen()
    );
}

#[test]
fn shift_s_refuses_while_the_last_main_builds_refresh_failed() {
    let mut h = Harness::with_size(160, 20);
    let mut inputs = focus_failing_newest();
    inputs.push(failed(Stage::Builds, "gh: HTTP 502"));
    inputs.push(key(KeyCode::Char('S')));
    h.run(inputs).unwrap();

    assert!(
        footer(&h.screen()).contains("the last refresh failed; press r and try again"),
        "{}",
        h.screen()
    );
    assert_eq!(h.app.mode, conveyor::app::Mode::Normal);

    h.run(vec![builds(failing_newest()), key(KeyCode::Char('S'))])
        .unwrap();

    assert!(h.screen().contains("Manage run 25"), "{}", h.screen());
}

#[test]
fn f_refuses_when_a_refresh_failed_while_the_popup_was_open() {
    let mut h = Harness::with_size(160, 20);
    let mut inputs = focus_failing_newest();
    inputs.push(key(KeyCode::Char('S')));
    inputs.push(failed(Stage::Builds, "gh: HTTP 502"));
    inputs.push(key(KeyCode::Char('f')));
    h.run(inputs).unwrap();

    assert!(reruns(&h.requests).is_empty(), "{:?}", h.requests);
    assert!(
        footer(&h.screen()).contains("the last refresh failed; press r and try again"),
        "{}",
        h.screen()
    );
}

#[test]
fn an_older_failed_build_lists_its_rerun_as_refused() {
    let mut h = Harness::with_size(160, 20);
    let mut inputs = focus_builds();
    inputs.push(key(KeyCode::Char('j')));
    inputs.push(key(KeyCode::Char('S')));
    h.run(inputs).unwrap();

    assert!(
        h.screen()
            .contains("f  rerun failed jobs · only the newest main build can be"),
        "{}",
        h.screen()
    );
    assert!(h.screen().contains("│    rerun "), "{}", h.screen());
}

#[test]
fn shift_s_offers_the_rerun_of_a_failed_run_and_f_requests_it_at_once() {
    let mut h = Harness::with_size(160, 20);
    let mut inputs = focus_failing_newest();
    inputs.push(key(KeyCode::Char('S')));
    h.run(inputs).unwrap();

    assert!(h.screen().contains("Manage run 25"), "{}", h.screen());
    assert!(
        h.screen().contains("f  rerun failed jobs"),
        "{}",
        h.screen()
    );
    assert!(reruns(&h.requests).is_empty(), "{:?}", h.requests);

    h.run(vec![key(KeyCode::Char('f'))]).unwrap();

    assert_eq!(
        reruns(&h.requests),
        vec![&Request::Rerun {
            repo: "acme/webapp".to_string(),
            run_ids: vec![1025],
        }]
    );
    assert!(
        footer(&h.screen()).contains("rerunning run 25…"),
        "{}",
        h.screen()
    );
}

#[test]
fn any_other_key_than_f_closes_the_popup_and_does_nothing_else() {
    let mut h = Harness::with_size(160, 20);
    let mut inputs = focus_failing_newest();
    inputs.push(key(KeyCode::Char('S')));
    inputs.push(key(KeyCode::Char('q')));
    h.run(inputs).unwrap();

    assert!(reruns(&h.requests).is_empty(), "{:?}", h.requests);
    assert!(
        footer(&h.screen()).contains("nothing changed"),
        "{}",
        h.screen()
    );

    h.run(vec![key(KeyCode::Char('S')), key(KeyCode::Esc)])
        .unwrap();

    assert!(reruns(&h.requests).is_empty(), "{:?}", h.requests);
    assert!(
        footer(&h.screen()).contains("nothing changed"),
        "{}",
        h.screen()
    );
}

#[test]
fn a_running_run_lists_its_rerun_as_still_running_and_f_does_nothing() {
    let mut h = Harness::with_size(160, 20);
    let mut inputs = focus_builds();
    inputs.push(key(KeyCode::Char('S')));
    inputs.push(key(KeyCode::Char('f')));
    h.run(inputs).unwrap();

    assert!(
        h.screen()
            .contains("f  rerun failed jobs · run 26 is still running"),
        "{}",
        h.screen()
    );
    assert!(reruns(&h.requests).is_empty(), "{:?}", h.requests);
}

#[test]
fn a_rerun_on_its_way_blocks_a_second_one_until_it_lands() {
    let mut h = Harness::with_size(160, 20);
    let mut inputs = focus_failing_newest();
    inputs.push(key(KeyCode::Char('S')));
    inputs.push(key(KeyCode::Char('f')));
    inputs.push(key(KeyCode::Char('S')));
    h.run(inputs).unwrap();

    assert_eq!(reruns(&h.requests).len(), 1, "{:?}", h.requests);
    assert!(
        footer(&h.screen()).contains("rerunning run 25…"),
        "{}",
        h.screen()
    );
    assert!(!h.screen().contains("Manage run 25"), "{}", h.screen());

    h.run(vec![
        rerun_done(vec![(1025, Ok(()))]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('f')),
    ])
    .unwrap();

    assert_eq!(reruns(&h.requests).len(), 2, "{:?}", h.requests);
}

#[test]
fn a_landed_rerun_says_so_and_refreshes_the_column_it_came_from() {
    let mut h = Harness::with_size(160, 20);
    let mut inputs = focus_failing_newest();
    inputs.push(key(KeyCode::Char('S')));
    inputs.push(key(KeyCode::Char('f')));
    inputs.push(key(KeyCode::Char('1')));
    inputs.push(rerun_done(vec![(1025, Ok(()))]));
    h.run(inputs).unwrap();

    assert!(
        footer(&h.screen()).contains("rerun requested: run 25"),
        "{}",
        h.screen()
    );
    assert_eq!(
        h.requests.last(),
        Some(&Request::Refresh(Stage::Builds)),
        "{:?}",
        h.requests
    );
}

#[test]
fn a_failed_rerun_shows_what_gh_said() {
    let mut h = Harness::with_size(160, 20);
    let mut inputs = focus_failing_newest();
    inputs.push(key(KeyCode::Char('S')));
    inputs.push(key(KeyCode::Char('f')));
    inputs.push(rerun_done(vec![(1025, Err("HTTP 403"))]));
    h.run(inputs).unwrap();

    assert!(
        footer(&h.screen()).contains("rerun failed: run 1025: HTTP 403"),
        "{}",
        h.screen()
    );
    assert!(
        !h.requests.contains(&Request::Refresh(Stage::Builds)),
        "{:?}",
        h.requests
    );
}

#[test]
fn a_passing_run_lists_its_rerun_as_nothing_failed() {
    let mut h = Harness::with_size(160, 20);
    let mut inputs = vec![
        builds(vec![build(24, BuildStatus::Success, None)]),
        key(KeyCode::Char('3')),
    ];
    inputs.push(key(KeyCode::Char('S')));
    h.run(inputs).unwrap();

    assert!(
        h.screen()
            .contains("f  rerun failed jobs · nothing failed to rerun"),
        "{}",
        h.screen()
    );
}

#[test]
fn a_main_builds_rerun_schedules_no_my_prs_reload() {
    let mut h = Harness::with_size(160, 20);
    let mut inputs = focus_failing_newest();
    inputs.push(key(KeyCode::Char('S')));
    inputs.push(key(KeyCode::Char('f')));
    inputs.push(rerun_done(vec![(1025, Ok(()))]));
    inputs.push(tick_at(NOW + 30));
    h.run(inputs).unwrap();

    assert!(
        !h.requests.contains(&Request::Refresh(Stage::Prs)),
        "{:?}",
        h.requests
    );
}

#[test]
fn a_failed_rerun_from_main_builds_marks_the_build_card_under_its_title() {
    let mut h = Harness::with_size(160, 20);
    let mut inputs = focus_failing_newest();
    inputs.push(key(KeyCode::Char('S')));
    inputs.push(key(KeyCode::Char('f')));
    inputs.push(rerun_done(vec![(1025, Err("HTTP 403"))]));
    h.run(inputs).unwrap();

    let screen = h.screen();
    assert!(
        line_after(&screen, "Retry hooks").contains("! rerun failed: run 1025:"),
        "{screen}"
    );
}

#[test]
fn a_rerun_from_main_builds_that_went_through_leaves_no_mark() {
    let mut h = Harness::with_size(160, 20);
    let mut inputs = focus_failing_newest();
    inputs.push(key(KeyCode::Char('S')));
    inputs.push(key(KeyCode::Char('f')));
    inputs.push(rerun_done(vec![(1025, Ok(()))]));
    h.run(inputs).unwrap();

    assert!(!h.screen().contains("! rerun failed"), "{}", h.screen());
}

fn footer(screen: &str) -> String {
    screen.lines().last().unwrap_or_default().to_string()
}

fn reruns(requests: &[Request]) -> Vec<&Request> {
    requests
        .iter()
        .filter(|request| matches!(request, Request::Rerun { .. }))
        .collect()
}
