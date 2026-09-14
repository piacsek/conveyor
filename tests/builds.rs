mod support;

use conveyor::app::Request;
use conveyor::app::Stage;
use conveyor::model::builds::BuildStatus;
use ratatui::crossterm::event::KeyCode;
use support::{Harness, NOW, build, builds, ctrl, failed, job, jobs, jobs_failed, key, tick_at};

fn column(screen: &str, index: usize) -> Vec<String> {
    screen
        .lines()
        .map(|line| {
            let width = line.chars().count() / 4;
            line.chars().skip(index * width).take(width).collect()
        })
        .collect()
}

fn three() -> Vec<conveyor::model::builds::Build> {
    let mut running = build(26, BuildStatus::Running, Some((4840, "bob", "Speed up CI")));
    running.finished_at = None;
    running.started_at = Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(NOW - 90));
    vec![
        running,
        build(
            25,
            BuildStatus::Failure,
            Some((4821, "alice", "Retry hooks")),
        ),
        build(24, BuildStatus::Success, None),
    ]
}

#[test]
fn build_cards_show_the_pull_request_the_run_and_the_sha() {
    let mut h = Harness::with_size(160, 20);

    h.run(vec![builds(three()), tick_at(NOW)]).unwrap();

    let col = column(&h.screen(), 2);
    assert!(col[0].contains("Main webapp (3)"), "{}", h.screen());
    assert!(col[1].contains("● #4840"), "{}", h.screen());
    assert!(col[1].contains("1m"), "elapsed age: {}", h.screen());
    assert!(col[2].contains("Speed up CI"), "the title: {}", h.screen());
    assert!(
        col[3].contains("bob · run 26 · 1m · running"),
        "{}",
        h.screen()
    );
    assert!(col[4].contains("00000000"), "the sha: {}", h.screen());
    assert!(col[5].contains("✗ #4821"), "{}", h.screen());
    assert!(col[6].contains("Retry hooks"), "{}", h.screen());
    assert!(
        col[7].contains("alice · run 25 · 3m · failure"),
        "{}",
        h.screen()
    );
    assert!(col[9].contains("✓ run 24"), "no PR known: {}", h.screen());
    assert!(
        col[11].contains("bot · run 24 · 3m · success"),
        "{}",
        h.screen()
    );
}

#[test]
fn p_opens_the_pull_request_b_opens_the_run_and_d_shows_its_details() {
    let mut h = Harness::with_size(160, 20);

    h.run(vec![
        builds(three()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('p')),
        key(KeyCode::Char('b')),
        key(KeyCode::Char('d')),
    ])
    .unwrap();

    assert_eq!(
        h.opener.opened(),
        vec![
            "https://github.com/acme/webapp/pull/4821".to_string(),
            "https://github.com/acme/webapp/actions/runs/1025".to_string(),
        ]
    );
    let screen = h.screen();
    assert!(screen.contains("run 25  failure  3m"), "{screen}");
    assert!(screen.contains("#4821 alice  Retry hooks"), "{screen}");
    assert!(
        !screen.contains("https://"),
        "no URLs in the pane: {screen}"
    );
    assert!(screen.contains("started at 06:00:00"), "{screen}");
}

#[test]
fn a_failed_builds_fetch_flags_the_column() {
    let mut h = Harness::new();

    h.run(vec![failed(
        Stage::Builds,
        "main_workflow `Nope` not found",
    )])
    .unwrap();

    let col = column(&h.screen(), 2);
    assert!(col[0].contains("Main builds ⚠"), "{}", h.screen());
    assert!(
        col[1].contains("main_workflow `Nope` not found"),
        "{}",
        h.screen()
    );
}

#[test]
fn p_says_so_when_no_pull_request_is_known_and_b_still_opens_the_run() {
    let mut h = Harness::new();

    h.run(vec![
        builds(three()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('G')),
        key(KeyCode::Char('p')),
    ])
    .unwrap();

    assert!(
        h.screen().contains("no pull request for this run"),
        "{}",
        h.screen()
    );
    assert!(h.opener.opened().is_empty(), "p opened nothing");

    h.run(vec![key(KeyCode::Char('b'))]).unwrap();

    assert_eq!(
        h.opener.opened(),
        vec!["https://github.com/acme/webapp/actions/runs/1024".to_string()]
    );
}

fn focus_builds() -> Vec<std::io::Result<conveyor::app::Input>> {
    vec![
        builds(three()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
    ]
}

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

#[test]
fn d_on_a_build_asks_for_its_jobs_and_lists_them_failures_first() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(key(KeyCode::Char('d')));

    h.run(inputs).unwrap();

    assert_eq!(
        h.requests,
        vec![
            Request::Jobs { run_id: 1025 },
            Request::Jobs { run_id: 1026 },
        ],
        "the failing run is asked for without being selected, then the selected one"
    );
    assert!(h.screen().contains("jobs: loading…"), "{}", h.screen());

    h.run(vec![jobs(
        1026,
        vec![
            job(1, "check", BuildStatus::Failure, Some("Run cargo test")),
            job(2, "audit", BuildStatus::Success, None),
        ],
    )])
    .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("✗ check  19s  failed at: Run cargo test"),
        "{screen}"
    );
    assert!(screen.contains("✓ audit  19s"), "{screen}");
    assert!(!screen.contains("https://"), "no job URLs: {screen}");
    assert!(
        screen.contains("✗ check · Run cargo test"),
        "and on the card itself: {screen}"
    );
}

#[test]
fn moving_the_selection_asks_for_each_runs_jobs_once() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(key(KeyCode::Char('d')));
    inputs.push(key(KeyCode::Char('j')));
    inputs.push(key(KeyCode::Char('k')));
    inputs.push(key(KeyCode::Char('j')));

    h.run(inputs).unwrap();

    assert_eq!(
        h.requests,
        vec![
            Request::Jobs { run_id: 1025 },
            Request::Jobs { run_id: 1026 },
        ],
        "each run is asked for once"
    );
}

#[test]
fn a_failed_jobs_fetch_shows_the_message_in_the_details_pane() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = focus_builds();
    inputs.push(key(KeyCode::Char('d')));
    inputs.push(jobs_failed(1026, "gh: HTTP 404: Not Found"));

    h.run(inputs).unwrap();

    assert!(
        h.screen().contains("jobs: gh: HTTP 404: Not Found"),
        "{}",
        h.screen()
    );
}

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
fn a_failing_build_shows_its_failed_step_on_the_card() {
    let mut h = Harness::new();

    h.run(vec![
        builds(three()),
        jobs(
            1025,
            vec![job(
                1,
                "check",
                BuildStatus::Failure,
                Some("Run cargo test"),
            )],
        ),
    ])
    .unwrap();

    let col = column(&h.screen(), 2);
    assert!(
        col[8].contains("✗ check · Run cargo test"),
        "{}",
        h.screen()
    );
}

#[test]
fn main_builds_stay_newest_first_across_refreshes() {
    let mut h = Harness::with_size(160, 20);
    let older = vec![
        build(25, BuildStatus::Success, Some((4821, "alice", "Retry"))),
        build(24, BuildStatus::Success, None),
    ];
    let mut running = build(26, BuildStatus::Running, Some((4840, "bob", "Speed up")));
    running.finished_at = None;
    let with_new = vec![older[1].clone(), running.clone(), older[0].clone()];

    h.run(vec![builds(older), builds(with_new)]).unwrap();

    let col = column(&h.screen(), 2);
    assert!(
        col[1].contains("#4840"),
        "the new run leads, whatever order the API sent: {}",
        h.screen()
    );
    assert!(col[2].contains("Speed up"), "{}", h.screen());
    assert!(col[5].contains("#4821"), "{}", h.screen());
    assert!(col[6].contains("Retry"), "{}", h.screen());
    assert!(col[9].contains("run 24"), "{}", h.screen());
}

#[test]
fn y_copies_the_pull_request_url_and_shift_y_the_build_url() {
    let mut h = Harness::new();

    h.run(vec![
        builds(three()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('y')),
        key(KeyCode::Char('Y')),
    ])
    .unwrap();

    assert_eq!(
        h.opener.copied(),
        vec![
            "https://github.com/acme/webapp/pull/4821".to_string(),
            "https://github.com/acme/webapp/actions/runs/1025".to_string(),
        ]
    );
    assert!(
        h.screen().contains("copied the build URL"),
        "{}",
        h.screen()
    );
}

#[test]
fn p_falls_back_to_the_squash_number_while_the_pull_request_is_unassociated() {
    let mut h = Harness::new();
    let mut pending = build(26, BuildStatus::Success, Some((4840, "bob", "Speed up CI")));
    pending.pull = None;

    h.run(vec![
        builds(vec![pending]),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('p')),
    ])
    .unwrap();

    assert_eq!(
        h.opener.opened(),
        vec!["https://github.com/acme/webapp/pull/4840".to_string()],
        "the card shows #4840, so p opens #4840"
    );
}

fn many_jobs(count: u64) -> Vec<conveyor::model::jobs::Job> {
    (1..=count)
        .map(|i| job(i, &format!("job {i}"), BuildStatus::Success, None))
        .collect()
}

fn long_details() -> Vec<std::io::Result<conveyor::app::Input>> {
    let mut inputs = focus_builds();
    inputs.push(key(KeyCode::Char('d')));
    inputs.push(jobs(1026, many_jobs(24)));
    inputs.push(jobs(1025, many_jobs(24)));
    inputs
}

fn line_after(screen: &str, needle: &str) -> String {
    let lines: Vec<&str> = screen.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("{needle} is not on screen: {screen}"));
    lines[at + 1].to_string()
}

#[test]
fn ctrl_d_and_ctrl_u_scroll_the_details_pane_and_stop_at_both_ends() {
    let mut h = Harness::with_size(160, 30);

    h.run(long_details()).unwrap();

    let screen = h.screen();
    assert_eq!(h.app.details_page, 10);
    assert_eq!(h.app.details_scroll, 0);
    assert!(screen.contains("bob  Speed up CI"), "the head: {screen}");
    assert!(screen.contains("job 1  19s"), "{screen}");
    assert!(!screen.contains("job 24  19s"), "below the fold: {screen}");

    h.run(vec![ctrl('d')]).unwrap();
    assert_eq!(h.app.details_scroll, 5, "half a page");
    let screen = h.screen();
    assert!(
        !screen.contains("bob  Speed up CI"),
        "the head left: {screen}"
    );
    assert!(screen.contains("job 3  19s"), "{screen}");

    h.run(vec![ctrl('d')]).unwrap();
    assert_eq!(h.app.details_scroll, 10);

    h.run(vec![ctrl('d'), ctrl('d'), ctrl('d')]).unwrap();
    assert_eq!(h.app.details_scroll, 17, "the bottom holds");
    let screen = h.screen();
    assert!(screen.contains("job 24  19s"), "the last job: {screen}");
    assert!(
        line_after(&screen, "job 24  19s").starts_with("└"),
        "the last line sits on the last row, no blank tail: {screen}"
    );

    h.run(vec![ctrl('u'), ctrl('u'), ctrl('u')]).unwrap();
    assert_eq!(h.app.details_scroll, 2);

    h.run(vec![ctrl('u'), ctrl('u')]).unwrap();
    assert_eq!(h.app.details_scroll, 0, "the top holds");
    let screen = h.screen();
    assert!(
        screen.contains("bob  Speed up CI"),
        "back at the top: {screen}"
    );
}

#[test]
fn moving_the_selection_changing_focus_and_closing_the_pane_return_it_to_the_top() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = long_details();
    inputs.push(ctrl('d'));
    h.run(inputs).unwrap();
    assert_eq!(h.app.details_scroll, 5);

    h.run(vec![key(KeyCode::Char('j'))]).unwrap();
    assert_eq!(h.app.details_scroll, 0, "a selection move");

    h.run(vec![ctrl('d'), key(KeyCode::Char('h'))]).unwrap();
    assert_eq!(h.app.details_scroll, 0, "a focus change");

    h.run(vec![
        key(KeyCode::Char('l')),
        ctrl('d'),
        key(KeyCode::Char('d')),
    ])
    .unwrap();
    assert_eq!(h.app.details_scroll, 0, "closing the pane");
}

#[test]
fn a_refresh_that_drops_the_selected_run_returns_the_pane_to_the_top() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = long_details();
    inputs.push(ctrl('d'));
    h.run(inputs).unwrap();
    assert_eq!(h.app.details_scroll, 5);

    h.run(vec![builds(vec![build(
        27,
        BuildStatus::Success,
        Some((4850, "erin", "Cache the toolchain")),
    )])])
    .unwrap();

    assert_eq!(h.app.details_scroll, 0, "another run is selected now");
}

#[test]
fn the_scroll_keys_work_while_a_filter_is_being_typed_and_never_reach_the_query() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = long_details();
    inputs.push(key(KeyCode::Char('/')));
    inputs.push(key(KeyCode::Char('4')));
    inputs.push(ctrl('d'));
    inputs.push(ctrl('u'));
    inputs.push(ctrl('d'));

    h.run(inputs).unwrap();

    assert_eq!(h.app.details_scroll, 5);
    let screen = h.screen();
    assert!(screen.contains("/4▏  "), "the query is still /4: {screen}");
    assert!(!screen.contains("/4d"), "{screen}");
}

#[test]
fn the_scroll_keys_close_the_help_pane_like_any_other_key() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = long_details();
    inputs.push(key(KeyCode::Char('?')));
    h.run(inputs).unwrap();
    assert!(h.screen().contains("any key returns"), "{}", h.screen());

    h.run(vec![ctrl('d')]).unwrap();

    let screen = h.screen();
    assert!(!screen.contains("any key returns"), "{screen}");
    assert_eq!(h.app.details_scroll, 0, "the help key did not scroll");
}

#[test]
fn the_details_title_says_which_way_there_is_more_to_scroll() {
    let mut h = Harness::with_size(160, 30);

    h.run(long_details()).unwrap();
    let screen = h.screen();
    assert!(screen.contains("run 26 ↓"), "{screen}");
    assert!(!screen.contains("↑"), "nothing above yet: {screen}");

    h.run(vec![ctrl('d')]).unwrap();
    assert!(h.screen().contains("run 26 ↑↓"), "{}", h.screen());

    h.run(vec![ctrl('d'), ctrl('d'), ctrl('d'), ctrl('d')])
        .unwrap();
    let screen = h.screen();
    assert!(screen.contains("run 26 ↑"), "{screen}");
    assert!(!screen.contains("↑↓"), "nothing below any more: {screen}");
}

#[test]
fn a_cancelled_main_build_gets_its_own_glyph_not_the_skipped_dash() {
    let mut h = Harness::new();

    h.run(vec![builds(vec![build(
        26,
        BuildStatus::Cancelled,
        Some((4840, "bob", "Speed up CI")),
    )])])
    .unwrap();

    let col = column(&h.screen(), 2);
    assert!(col[1].contains("⊘ #4840"), "{}", h.screen());
    assert!(col[3].contains("cancelled"), "{}", h.screen());
}

#[test]
fn a_cancelled_build_does_not_blame_the_job_the_cancellation_killed() {
    let mut h = Harness::new();
    let mut cancelled = build(
        26,
        BuildStatus::Cancelled,
        Some((4840, "bob", "Speed up CI")),
    );
    cancelled.id = 1026;

    h.run(vec![
        builds(vec![cancelled]),
        jobs(
            1026,
            vec![job(
                1,
                "Nx / Status",
                BuildStatus::Failure,
                Some("Check Nx workflow status"),
            )],
        ),
    ])
    .unwrap();

    let col = column(&h.screen(), 2);
    assert!(col[1].contains("⊘ #4840"), "{}", h.screen());
    assert!(
        !col[4].contains("Nx / Status"),
        "the job died with the run, it did not fail on its own: {}",
        h.screen()
    );
    assert!(
        col[4].contains("00000000"),
        "the sha instead: {}",
        h.screen()
    );
}

#[test]
fn the_pane_still_lists_the_jobs_a_cancelled_card_stops_blaming() {
    let mut h = Harness::with_size(160, 24);
    let mut cancelled = build(
        26,
        BuildStatus::Cancelled,
        Some((4840, "bob", "Speed up CI")),
    );
    cancelled.id = 1026;

    h.run(vec![
        builds(vec![cancelled]),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('d')),
        jobs(
            1026,
            vec![job(
                1,
                "Nx / Status",
                BuildStatus::Failure,
                Some("Check Nx workflow status"),
            )],
        ),
    ])
    .unwrap();

    let screen = h.screen();
    let col = column(&screen, 2);
    assert!(!col[4].contains("Nx / Status"), "not on the card: {screen}");
    assert!(
        screen.contains("✗ Nx / Status  19s  failed at: Check Nx workflow status"),
        "but the pane still has it: {screen}"
    );
}

#[test]
fn a_build_card_gives_the_title_its_own_line_under_the_number() {
    let mut h = Harness::new();
    let long = build(
        26,
        BuildStatus::Success,
        Some((
            7108,
            "rvargas",
            "[AIE-65] Remove commit and push permission gate",
        )),
    );

    h.run(vec![builds(vec![long])]).unwrap();

    let col = column(&h.screen(), 2);
    assert!(col[1].contains("✓ #7108"), "{}", h.screen());
    assert!(
        !col[1].contains("AIE-65"),
        "the first line is the number and the age: {}",
        h.screen()
    );
    assert!(col[1].contains("2h"), "{}", h.screen());
    assert!(
        col[2].contains("[AIE-65] Remove commit and push"),
        "the title, on its own line: {}",
        h.screen()
    );
    assert!(
        col[3].contains("rvargas · run 26"),
        "then the meta line: four lines to a card: {}",
        h.screen()
    );
}
