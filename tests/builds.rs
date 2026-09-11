mod support;

use conveyor::app::Request;
use conveyor::app::Stage;
use conveyor::model::builds::BuildStatus;
use ratatui::crossterm::event::KeyCode;
use support::{Harness, NOW, build, builds, failed, job, jobs, jobs_failed, key, tick_at};

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
    let mut h = Harness::new();

    h.run(vec![builds(three()), tick_at(NOW)]).unwrap();

    let col = column(&h.screen(), 2);
    assert!(col[0].contains("Main webapp (3)"), "{}", h.screen());
    assert!(col[1].contains("● #4840 Speed up CI"), "{}", h.screen());
    assert!(col[1].contains("1m"), "elapsed age: {}", h.screen());
    assert!(
        col[2].contains("bob · run 26 · 1m · running"),
        "{}",
        h.screen()
    );
    assert!(col[3].contains("00000000"), "the sha: {}", h.screen());
    assert!(col[4].contains("✗ #4821 Retry hooks"), "{}", h.screen());
    assert!(
        col[5].contains("alice · run 25 · 3m · failure"),
        "{}",
        h.screen()
    );
    assert!(col[7].contains("✓ run 24"), "no PR known: {}", h.screen());
    assert!(
        col[8].contains("bot · run 24 · 3m · success"),
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
    inputs.push(jobs(
        1026,
        vec![job(1, "check", BuildStatus::Running, None)],
    ));
    inputs.push(builds(three()));
    inputs.push(builds(three()));

    h.run(inputs).unwrap();

    let asked = |run_id| {
        h.requests
            .iter()
            .filter(|request| **request == Request::Jobs { run_id })
            .count()
    };
    assert_eq!(asked(1026), 3, "the running run is asked on every refresh");
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
        col[6].contains("✗ check · Run cargo test"),
        "{}",
        h.screen()
    );
}

#[test]
fn main_builds_stay_newest_first_across_refreshes() {
    let mut h = Harness::new();
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
        col[1].contains("#4840 Speed up"),
        "the new run leads, whatever order the API sent: {}",
        h.screen()
    );
    assert!(col[4].contains("#4821 Retry"), "{}", h.screen());
    assert!(col[7].contains("run 24"), "{}", h.screen());
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
