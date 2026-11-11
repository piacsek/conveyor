use conveyor::app::Request;
use conveyor::model::builds::BuildStatus;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{focus_builds, line_after, many_jobs};
use crate::support::{self, Harness, ctrl, job, jobs, key};

#[test]
fn shift_l_shows_the_failed_steps_log_in_the_pane_and_again_hides_it() {
    use support::log;
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(key(KeyCode::Char('j')));
    inputs.push(jobs(
        1025,
        vec![
            job(1, "check", BuildStatus::Failure, Some("Run cargo test")),
            job(2, "audit", BuildStatus::Success, None),
        ],
    ));
    inputs.push(key(KeyCode::Char('L')));
    h.run(inputs).unwrap();

    assert!(
        h.requests.contains(&Request::Log {
            run_id: 1025,
            job_id: 1
        }),
        "{:?}",
        h.requests
    );
    assert!(h.app.details, "L opens the pane");
    let screen = h.screen();
    assert!(screen.contains("log: check · Run cargo test"), "{screen}");
    assert!(
        line_after(&screen, "log: check · Run cargo test").contains("loading…"),
        "{screen}"
    );

    h.run(vec![log(1, vec!["line one", "line two"])]).unwrap();
    let screen = h.screen();
    assert!(screen.contains("log: check · Run cargo test"), "{screen}");
    assert!(
        screen.contains("line one") && screen.contains("line two"),
        "{screen}"
    );

    h.run(vec![key(KeyCode::Char('L'))]).unwrap();
    let screen = h.screen();
    assert!(
        !screen.contains("line two"),
        "L again hides the log: {screen}"
    );
    assert!(h.app.details, "the pane stays open");

    h.run(vec![key(KeyCode::Char('L'))]).unwrap();
    assert!(
        h.screen().contains("line two"),
        "and it comes back without a second fetch"
    );
    assert_eq!(
        h.requests
            .iter()
            .filter(|request| matches!(request, Request::Log { .. }))
            .count(),
        1
    );
}

#[test]
fn shift_l_opens_on_the_first_press_after_the_selection_moves_or_the_pane_closes() {
    use support::log;
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(key(KeyCode::Char('j')));
    inputs.push(jobs(
        1025,
        vec![job(
            1,
            "check",
            BuildStatus::Failure,
            Some("Run cargo test"),
        )],
    ));
    inputs.push(jobs(
        1024,
        vec![job(4, "lint", BuildStatus::Failure, Some("Run cargo fmt"))],
    ));
    inputs.push(key(KeyCode::Char('L')));
    inputs.push(log(1, vec!["check said no"]));
    h.run(inputs).unwrap();
    assert!(h.screen().contains("check said no"), "{}", h.screen());

    h.run(vec![key(KeyCode::Char('j')), key(KeyCode::Char('L'))])
        .unwrap();
    assert!(
        h.requests.contains(&Request::Log {
            run_id: 1024,
            job_id: 4
        }),
        "one L on the next failed run fetches its log: {:?}",
        h.requests
    );

    h.run(vec![key(KeyCode::Esc), key(KeyCode::Char('L'))])
        .unwrap();
    assert!(
        h.app.details,
        "Esc closed the pane; one L reopens it with the log"
    );
    assert!(h.screen().contains("loading…"), "{}", h.screen());
}

#[test]
fn a_failed_log_fetch_is_retried_on_the_next_shift_l() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(key(KeyCode::Char('j')));
    inputs.push(jobs(
        1025,
        vec![job(
            1,
            "check",
            BuildStatus::Failure,
            Some("Run cargo test"),
        )],
    ));
    inputs.push(key(KeyCode::Char('L')));
    inputs.push(Ok(conveyor::app::Input::Log(
        1,
        Err("gh: HTTP 429".to_string()),
    )));
    h.run(inputs).unwrap();
    assert!(h.screen().contains("gh: HTTP 429"), "{}", h.screen());

    h.run(vec![key(KeyCode::Char('L')), key(KeyCode::Char('L'))])
        .unwrap();
    assert_eq!(
        h.requests
            .iter()
            .filter(|request| matches!(request, Request::Log { .. }))
            .count(),
        2,
        "off and on again asks once more: {:?}",
        h.requests
    );
}

#[test]
fn the_log_opens_in_its_own_pane_beside_the_details_scrolled_to_its_last_line() {
    use support::log;
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(key(KeyCode::Char('j')));
    let mut jobs_list = vec![job(
        1,
        "check",
        BuildStatus::Failure,
        Some("Run cargo test"),
    )];
    jobs_list.extend(many_jobs(24));
    inputs.push(jobs(1025, jobs_list));
    inputs.push(key(KeyCode::Char('L')));
    let lines: Vec<String> = (1..=15).map(|i| format!("line {i}")).collect();
    inputs.push(log(1, lines.iter().map(String::as_str).collect()));
    h.run(inputs).unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("log: check · Run cargo test"),
        "own title: {screen}"
    );
    assert!(
        screen.contains("line 15"),
        "the end of the log is on screen: {screen}"
    );
    let (details_row, log_row) = (
        screen
            .lines()
            .find(|l| l.contains("job 1 "))
            .unwrap_or_default()
            .to_string(),
        screen
            .lines()
            .find(|l| l.contains("line 15"))
            .unwrap_or_default()
            .to_string(),
    );
    assert!(
        !details_row.is_empty(),
        "the job list is still there: {screen}"
    );
    let details_x = screen
        .lines()
        .map(|l| l.find("run 25  failure").unwrap_or(usize::MAX))
        .min()
        .unwrap();
    let log_x = log_row.find("line 15").unwrap();
    assert!(
        log_x > 70 && details_x < 10,
        "side by side, log on the right: {screen}"
    );

    h.run(vec![ctrl('u')]).unwrap();
    let screen = h.screen();
    assert!(
        screen.contains("line 1 ") || screen.contains("line 1\n"),
        "Ctrl-u scrolls the log pane: {screen}"
    );
    assert!(!screen.contains("line 15"), "{screen}");

    h.run(vec![key(KeyCode::Char('L'))]).unwrap();
    let screen = h.screen();
    assert!(
        !screen.contains("log: check"),
        "L closes the log pane: {screen}"
    );
    assert!(screen.contains("job 1 "), "and the details stay: {screen}");
}

#[test]
fn coming_back_to_a_run_shows_its_log_at_the_end_again() {
    use support::log;
    let mut h = two_failed_runs_with_logs();
    assert!(h.screen().contains("line 40"), "{}", h.screen());

    h.run(vec![
        key(KeyCode::Char('j')),
        key(KeyCode::Char('L')),
        log(4, vec!["only line"]),
        key(KeyCode::Char('k')),
    ])
    .unwrap();
    let screen = h.screen();
    assert!(
        screen.contains("line 40"),
        "back on run 25 the log is at its end: {screen}"
    );
    assert!(!screen.contains("line 1 "), "{screen}");
}

#[test]
fn a_late_log_for_another_job_does_not_move_the_log_on_screen() {
    use support::log;
    let mut h = two_failed_runs_with_logs();
    h.run(vec![
        key(KeyCode::Char('j')),
        key(KeyCode::Char('L')),
        log(
            4,
            (1..=20)
                .map(|i| format!("lint {i}"))
                .collect::<Vec<_>>()
                .iter()
                .map(String::as_str)
                .collect(),
        ),
        ctrl('u'),
        ctrl('u'),
    ])
    .unwrap();
    assert!(h.screen().contains("lint 1 "), "{}", h.screen());

    h.run(vec![log(1, vec!["fresh check log"])]).unwrap();
    let screen = h.screen();
    assert!(
        screen.contains("lint 1 "),
        "run 24's view did not jump: {screen}"
    );
    assert!(!screen.contains("lint 20"), "{screen}");
}

#[test]
fn scroll_keys_with_the_pane_closed_are_forgotten_by_the_log_pane_too() {
    let mut h = two_failed_runs_with_logs();
    h.run(vec![
        key(KeyCode::Esc),
        ctrl('u'),
        ctrl('u'),
        key(KeyCode::Char('d')),
    ])
    .unwrap();
    let screen = h.screen();
    assert!(
        screen.contains("line 40"),
        "the log reopens at its end: {screen}"
    );
}

#[test]
fn shift_l_says_so_when_the_run_has_no_failed_job() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(jobs(
        1026,
        vec![job(3, "check", BuildStatus::Running, None)],
    ));
    inputs.push(key(KeyCode::Char('L')));
    h.run(inputs).unwrap();

    assert!(
        h.screen().contains("no failed job to show"),
        "{}",
        h.screen()
    );
    assert!(
        !h.requests
            .iter()
            .any(|request| matches!(request, Request::Log { .. })),
        "{:?}",
        h.requests
    );
}

fn two_failed_runs_with_logs() -> Harness {
    use support::log;
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(key(KeyCode::Char('j')));
    inputs.push(jobs(
        1025,
        vec![job(
            1,
            "check",
            BuildStatus::Failure,
            Some("Run cargo test"),
        )],
    ));
    inputs.push(jobs(
        1024,
        vec![job(4, "lint", BuildStatus::Failure, Some("Run cargo fmt"))],
    ));
    inputs.push(key(KeyCode::Char('L')));
    let long: Vec<String> = (1..=40).map(|i| format!("line {i}")).collect();
    inputs.push(log(1, long.iter().map(String::as_str).collect()));
    h.run(inputs).unwrap();
    h
}
