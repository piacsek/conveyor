mod support;

use conveyor::app::{Request, Stage};
use conveyor::config::{Config, Deploy, Repo};
use conveyor::model::builds::BuildStatus;
use conveyor::model::queue::QueueEntry;
use ratatui::crossterm::event::KeyCode;
use support::{
    Harness, build, builds_in, deployed, deployment, entry, failed_in, job, jobs, key, queue_in,
    rerun_done,
};

#[test]
fn tab_on_merge_queue_asks_for_the_second_repository() {
    let mut h = Harness::with_config(160, 12, two_repos());
    h.run(vec![
        queue_in(
            0,
            "acme/monorepo",
            vec![entry(1, 12, "alice", "Retry webhooks")],
        ),
        key(KeyCode::Char('l')),
    ])
    .unwrap();

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    assert_eq!(h.requests, vec![Request::Refresh(Stage::Queue, 1)]);
}

#[test]
fn merge_queue_draws_a_tab_per_repository() {
    let mut h = Harness::with_config(160, 12, two_repos());

    h.run(vec![queue_in(0, "acme/monorepo", vec![])]).unwrap();

    assert!(h.screen().contains("monorepo │ infra"), "{}", h.screen());
}

#[test]
fn the_infra_tab_shows_the_infra_queue_and_names_it_in_the_title() {
    let mut h = Harness::with_config(160, 12, two_repos());
    h.run(vec![
        queue_in(
            0,
            "acme/monorepo",
            vec![entry(1, 12, "alice", "Retry webhooks")],
        ),
        key(KeyCode::Char('l')),
        key(KeyCode::Tab),
    ])
    .unwrap();

    h.run(vec![queue_in(
        1,
        "acme/infra",
        vec![entry(1, 40, "bob", "Bump nodes")],
    )])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("#40 Bump nodes"), "{screen}");
    assert!(!screen.contains("#12 Retry webhooks"), "{screen}");
    assert!(screen.contains("Queue infra (1)"), "{screen}");
}

#[test]
fn shift_tab_returns_to_the_monorepo_queue_with_its_selection_and_without_the_filter() {
    let mut h = Harness::with_config(160, 12, two_repos());
    h.run(vec![
        queue_in(
            0,
            "acme/monorepo",
            vec![
                entry(1, 12, "alice", "Retry webhooks"),
                entry(2, 13, "carol", "Rate limits"),
            ],
        ),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('/')),
        key(KeyCode::Char('1')),
        key(KeyCode::Char('3')),
        key(KeyCode::Enter),
        key(KeyCode::Tab),
    ])
    .unwrap();

    h.run(vec![key(KeyCode::BackTab)]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("#12 Retry webhooks"), "{screen}");
    assert_eq!(h.app.query, None);
    assert_eq!(h.app.queue.column().selected().unwrap().number, 13);
}

#[test]
fn one_repository_draws_no_tab_bar() {
    let mut h = Harness::with_config(
        160,
        12,
        Config {
            repo: vec![repo("acme/monorepo")],
            ..Config::default()
        },
    );

    h.run(vec![queue_in(0, "acme/monorepo", vec![])]).unwrap();

    assert!(!h.screen().contains("│ monorepo"), "{}", h.screen());
    assert!(h.screen().contains("Queue monorepo (0)"), "{}", h.screen());
}

#[test]
fn a_monorepo_result_landing_after_the_switch_stays_in_the_monorepo_tab() {
    let mut h = Harness::with_config(160, 12, two_repos());
    h.run(vec![key(KeyCode::Char('l')), key(KeyCode::Tab)])
        .unwrap();

    h.run(vec![queue_in(
        0,
        "acme/monorepo",
        vec![entry(1, 12, "alice", "Retry webhooks")],
    )])
    .unwrap();

    let screen = h.screen();
    assert!(!screen.contains("#12 Retry webhooks"), "{screen}");
    assert!(!screen.contains("Queue monorepo"), "{screen}");
    h.run(vec![key(KeyCode::BackTab)]).unwrap();
    assert!(h.screen().contains("#12 Retry webhooks"), "{}", h.screen());
}

#[test]
fn a_monorepo_error_landing_after_the_switch_shows_only_on_the_monorepo_tab() {
    let mut h = Harness::with_config(160, 12, two_repos());
    h.run(vec![key(KeyCode::Char('l')), key(KeyCode::Tab)])
        .unwrap();

    h.run(vec![failed_in(Stage::Queue, 0, "gh: HTTP 502")])
        .unwrap();

    assert!(!h.screen().contains("HTTP 502"), "{}", h.screen());
    h.run(vec![key(KeyCode::BackTab)]).unwrap();
    assert!(h.screen().contains("HTTP 502"), "{}", h.screen());
}

#[test]
fn an_infra_merge_group_run_asks_for_its_jobs_in_the_infra_repository() {
    let mut h = Harness::with_config(160, 24, two_repos());
    h.run(vec![key(KeyCode::Char('l')), key(KeyCode::Tab)])
        .unwrap();
    h.requests.clear();

    h.run(vec![queue_in(1, "acme/infra", infra_run())]).unwrap();

    assert_eq!(
        h.requests,
        vec![Request::Jobs {
            repo: "acme/infra".to_string(),
            run_id: 4242
        }]
    );
}

#[test]
fn l_on_an_infra_merge_group_run_asks_for_the_log_in_the_infra_repository() {
    let mut h = Harness::with_config(160, 24, two_repos());
    h.run(vec![
        key(KeyCode::Char('l')),
        key(KeyCode::Tab),
        queue_in(1, "acme/infra", infra_run()),
        jobs(
            4242,
            vec![job(
                7,
                "check",
                BuildStatus::Failure,
                Some("Run cargo test"),
            )],
        ),
    ])
    .unwrap();

    h.run(vec![key(KeyCode::Char('L'))]).unwrap();

    assert!(
        h.requests.contains(&Request::Log {
            repo: "acme/infra".to_string(),
            run_id: 4242,
            job_id: 7
        }),
        "{:?}",
        h.requests
    );
}

#[test]
fn a_monorepo_refresh_does_not_ask_again_for_the_jobs_of_a_hidden_infra_run() {
    let mut h = Harness::with_config(160, 24, two_repos());
    h.run(vec![
        key(KeyCode::Char('l')),
        key(KeyCode::Tab),
        queue_in(1, "acme/infra", running_infra_run()),
        jobs(4242, vec![job(7, "check", BuildStatus::Running, None)]),
        key(KeyCode::BackTab),
    ])
    .unwrap();
    h.requests.clear();

    h.run(vec![queue_in(0, "acme/monorepo", vec![])]).unwrap();

    assert!(h.requests.is_empty(), "{:?}", h.requests);
}

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

fn two_repos() -> Config {
    Config {
        repo: vec![repo("acme/monorepo"), repo("acme/infra")],
        ..Config::default()
    }
}

fn deploy_first() -> Config {
    let mut monorepo = repo("acme/monorepo");
    monorepo.deploy = vec![Deploy {
        system: "api".to_string(),
        ..Deploy::default()
    }];
    Config {
        repo: vec![monorepo, repo("acme/infra")],
        ..Config::default()
    }
}

fn repo(name: &str) -> Repo {
    Repo {
        name: name.to_string(),
        ..Repo::default()
    }
}

fn on_infra_builds() -> Vec<std::io::Result<conveyor::app::Input>> {
    vec![
        builds_in(
            1,
            "acme/infra",
            vec![build(
                25,
                BuildStatus::Failure,
                Some((40, "bob", "Bump nodes")),
            )],
        ),
        key(KeyCode::Char('l')),
        key(KeyCode::Tab),
        key(KeyCode::Char('l')),
    ]
}

fn running(run_number: u64) -> conveyor::model::builds::Build {
    let mut run = build(run_number, BuildStatus::Running, None);
    run.finished_at = None;
    run
}

fn infra_run() -> Vec<QueueEntry> {
    let mut run = build(312, BuildStatus::Failure, None);
    run.id = 4242;
    run.url = "https://github.com/acme/infra/actions/runs/4242".to_string();
    let mut first = entry(1, 40, "bob", "Bump nodes");
    first.run = Some(run);
    vec![first]
}

fn running_infra_run() -> Vec<QueueEntry> {
    let mut entries = infra_run();
    if let Some(run) = entries[0].run.as_mut() {
        run.status = BuildStatus::Running;
    }
    entries
}
