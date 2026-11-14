mod support;

use std::time::{Duration, UNIX_EPOCH};

use conveyor::app::Stage;
use conveyor::model::builds::BuildStatus;
use conveyor::model::deployed::{Health, Pod, parse_deployment, parse_pods};
use ratatui::crossterm::event::KeyCode;
use ratatui::style::Color;
use support::{Harness, build, builds, condition, deployed, deployment, failed, key, waiting};

fn column(screen: &str, index: usize) -> Vec<String> {
    screen
        .lines()
        .map(|line| {
            let width = line.chars().count() / 4;
            line.chars().skip(index * width).take(width).collect()
        })
        .collect()
}

fn envs() -> Vec<conveyor::model::deployed::Deployment> {
    let mut prod = deployment("prod", 24, Some((4790, "dave", "Spike tests")));
    prod.fetched_at = None;
    let mut failing = deployment("uat", 0, None);
    failing.sha = None;
    failing.image = None;
    failing.error = Some("ERROR: Active profile expired.".to_string());
    vec![
        deployment("staging", 26, Some((4840, "bob", "Speed up CI"))),
        prod,
        failing,
    ]
}

fn main_builds() -> Vec<conveyor::model::builds::Build> {
    vec![
        build(26, BuildStatus::Success, Some((4840, "bob", "Speed up CI"))),
        build(
            25,
            BuildStatus::Failure,
            Some((4821, "alice", "Retry hooks")),
        ),
        build(
            24,
            BuildStatus::Success,
            Some((4790, "dave", "Spike tests")),
        ),
    ]
}

#[test]
fn deployed_cards_give_each_field_its_own_line_with_the_title_before_the_author() {
    let mut h = Harness::with_size(160, 20);

    h.run(vec![builds(main_builds()), deployed(envs())])
        .unwrap();

    let col = column(&h.screen(), 3);
    assert!(col[0].contains("Deployed api"), "{}", h.screen());
    assert!(col[1].contains("✓ staging"), "{}", h.screen());
    assert!(col[1].contains("at main"), "{}", h.screen());
    assert!(col[2].contains("#4840 Speed up CI"), "{}", h.screen());
    assert!(col[3].contains("bob"), "the author below: {}", h.screen());
    assert!(col[4].contains("00000000 · read 6d ago"), "{}", h.screen());
    assert!(col[5].contains("◐ prod"), "{}", h.screen());
    assert!(col[5].contains("↓2"), "two builds behind: {}", h.screen());
    assert!(col[6].contains("#4790 Spike tests"), "{}", h.screen());
    assert!(col[7].contains("dave"), "{}", h.screen());
    assert!(col[9].contains("✗ uat"), "{}", h.screen());
    assert!(
        col[10].contains("ERROR: Active profile expired."),
        "the error takes the place of a pull request: {}",
        h.screen()
    );
}

#[test]
fn a_crashlooping_pod_puts_a_yellow_warning_after_the_glyph_and_its_reason_on_the_card() {
    let mut h = Harness::with_size(160, 20);
    let mut staging = deployment("staging", 26, Some((4840, "bob", "Speed up CI")));
    staging.health = Health::Warning("1 of 3 pods CrashLoopBackOff".to_string());

    h.run(vec![builds(main_builds()), deployed(vec![staging])])
        .unwrap();

    let col = column(&h.screen(), 3);
    assert!(col[1].contains("✓ ⚠ staging"), "{}", h.screen());
    assert!(col[1].contains("at main"), "{}", h.screen());
    assert!(col[3].contains("bob"), "{}", h.screen());
    assert!(
        col[4].contains("1 of 3 pods CrashLoopBackOff"),
        "{}",
        h.screen()
    );
    assert!(col[5].contains("00000000 · read 6d ago"), "{}", h.screen());
    assert_eq!(h.cell(125, 1).fg, Color::Yellow);
    assert_eq!(h.cell(125, 4).fg, Color::Yellow);
}

#[test]
fn an_unavailable_deployment_gets_a_red_alert_and_unread_pods_a_grey_question_mark() {
    let mut h = Harness::with_size(160, 20);
    let mut staging = deployment("staging", 26, Some((4840, "bob", "Speed up CI")));
    staging.health = Health::Alert("0 of 2 available".to_string());
    let mut prod = deployment("prod", 24, Some((4790, "dave", "Spike tests")));
    prod.health = Health::Unread;
    prod.pods_error = Some("Error from server (Forbidden): pods is forbidden".to_string());

    h.run(vec![builds(main_builds()), deployed(vec![staging, prod])])
        .unwrap();

    let col = column(&h.screen(), 3);
    assert!(col[1].contains("✓ ‼ staging"), "{}", h.screen());
    assert!(col[4].contains("0 of 2 available"), "{}", h.screen());
    assert_eq!(h.cell(125, 1).fg, Color::Red);
    assert_eq!(h.cell(125, 4).fg, Color::Red);
    assert!(col[6].contains("◐ ? prod"), "{}", h.screen());
    assert!(
        col[9].contains("pods not read: Error from server"),
        "{}",
        h.screen()
    );
    assert_eq!(h.cell(125, 6).fg, Color::DarkGray);
}

#[test]
fn unread_pods_stay_on_the_card_under_an_alert() {
    let mut h = Harness::with_size(160, 20);
    let mut staging = deployment("staging", 26, Some((4840, "bob", "Speed up CI")));
    staging.health = Health::Alert("0 of 2 available".to_string());
    staging.pods_error = Some("Error from server (Forbidden): pods is forbidden".to_string());

    h.run(vec![builds(main_builds()), deployed(vec![staging])])
        .unwrap();

    let col = column(&h.screen(), 3);
    assert!(col[1].contains("✓ ‼ staging"), "{}", h.screen());
    assert!(col[4].contains("0 of 2 available"), "{}", h.screen());
    assert!(
        col[5].contains("pods not read: Error from server"),
        "{}",
        h.screen()
    );
}

#[test]
fn a_long_pull_request_title_wraps_onto_a_second_line() {
    let mut h = Harness::new();
    let rows = vec![deployment(
        "staging",
        26,
        Some((
            4840,
            "bob",
            "Cancel the pending notification job when a tour is rebooked",
        )),
    )];

    h.run(vec![deployed(rows)]).unwrap();

    let col = column(&h.screen(), 3);
    assert!(
        col[2].contains("#4840 Cancel the pending"),
        "{}",
        h.screen()
    );
    assert!(
        col[3].contains("notification job when a tour"),
        "{}",
        h.screen()
    );
    assert!(col[4].contains("bob"), "{}", h.screen());
}

#[test]
fn a_failed_env_keeps_its_last_known_sha_from_the_previous_fetch() {
    let mut h = Harness::new();
    let mut later = envs();
    later[0].sha = None;
    later[0].image = None;
    later[0].pull = None;
    later[0].error = Some("kubectl exited with exit status: 1".to_string());

    h.run(vec![
        deployed(envs()),
        deployed(later),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
    ])
    .unwrap();

    let col = column(&h.screen(), 3);
    assert!(col[1].contains("✗ staging"), "{}", h.screen());
    assert!(
        col[2].contains("#4840 Speed up CI"),
        "old row kept: {}",
        h.screen()
    );
    assert!(
        col[4].contains("kubectl exited with exit status: 1"),
        "the error is on the card: {}",
        h.screen()
    );
    assert!(
        h.screen().contains("kubectl exited with exit status: 1"),
        "{}",
        h.screen()
    );
}

#[test]
fn p_opens_the_pull_request_and_d_shows_the_image_and_target() {
    let mut h = Harness::with_size(160, 20);

    h.run(vec![
        deployed(envs()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('p')),
        key(KeyCode::Char('d')),
    ])
    .unwrap();

    assert_eq!(
        h.opener.opened(),
        vec!["https://github.com/acme/webapp/pull/4840".to_string()]
    );
    let screen = h.screen();
    assert!(
        screen.contains("staging  ghcr.io/acme/api:000000000000000000000000000000000000001a"),
        "{screen}"
    );
    assert!(screen.contains("#4840 bob  Speed up CI"), "{screen}");
    assert!(screen.contains("deployed 6d ago"), "{screen}");
    assert!(
        !screen.contains("https://"),
        "no URLs in the pane: {screen}"
    );
}

#[test]
fn d_lists_the_target_the_replicas_the_failing_condition_and_every_pod() {
    let mut h = Harness::with_size(160, 30);
    let rollout = parse_deployment(include_str!("fixtures/deployment.json")).unwrap();
    let mut pods = parse_pods(include_str!("fixtures/pods.json")).unwrap();
    pods[0].started_at = Some(UNIX_EPOCH + Duration::from_secs(support::NOW - 2 * 3600));
    pods[1].started_at = Some(UNIX_EPOCH + Duration::from_secs(support::NOW - 7 * 86_400));
    let mut staging = deployment("staging", 26, Some((4840, "bob", "Speed up CI")));
    staging.target = "ctx-staging / web / api".to_string();
    staging.health = Health::Warning("1 of 3 pods CrashLoopBackOff".to_string());
    staging.rollout = Some(rollout);
    staging.pods = pods;

    h.run(vec![
        deployed(vec![staging]),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('d')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("ctx-staging / web / api"), "{screen}");
    assert!(
        screen.contains("replicas: 2 desired · 1 updated · 2 ready · 2 available"),
        "{screen}"
    );
    assert!(
        screen.contains(
            "Progressing False · ProgressDeadlineExceeded · ReplicaSet \"api-7d9c6b5f48\" has timed out progressing."
        ),
        "{screen}"
    );
    assert!(
        !screen.contains("MinimumReplicasAvailable"),
        "a healthy condition is not listed: {screen}"
    );
    assert!(
        screen.contains("✗ api-7d9c6b5f48-x2k9p  CrashLoopBackOff  restarts 7  2h  8c1f2a7d"),
        "{screen}"
    );
    assert!(
        screen
            .contains("last exit: OOMKilled (137) · back-off 5m0s restarting failed container=api"),
        "{screen}"
    );
    assert!(
        screen.contains("✓ api-5b8f7c6d94-hq4zt  Running  restarts 0  7d  4e9b01c3"),
        "{screen}"
    );
    assert!(
        screen.contains("✓ api-5b8f7c6d94-m7wvn  Running"),
        "{screen}"
    );
}

#[test]
fn d_marks_a_pod_that_is_only_starting_as_pending_not_failed() {
    let mut h = Harness::with_size(160, 30);
    let mut staging = deployment("staging", 26, Some((4840, "bob", "Speed up CI")));
    staging.pods = vec![Pod {
        name: "api-1".to_string(),
        phase: "Pending".to_string(),
        waiting: Some(waiting("ContainerCreating")),
        ..Pod::default()
    }];

    h.run(vec![
        deployed(vec![staging]),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('d')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("│● api-1  ContainerCreating  restarts 0"),
        "{screen}"
    );
}

#[test]
fn d_lists_a_replica_failure_although_its_status_is_true() {
    let mut h = Harness::with_size(160, 30);
    let mut rollout = parse_deployment(include_str!("fixtures/deployment.json")).unwrap();
    rollout.conditions[1].status = "True".to_string();
    rollout.conditions.push(condition(
        "ReplicaFailure",
        "True",
        "FailedCreate",
        "exceeded quota",
    ));
    let mut staging = deployment("staging", 26, Some((4840, "bob", "Speed up CI")));
    staging.rollout = Some(rollout);

    h.run(vec![
        deployed(vec![staging]),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('d')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("│ReplicaFailure True · FailedCreate · exceeded quota"),
        "{screen}"
    );
    assert!(!screen.contains("│Progressing True"), "{screen}");
}

#[test]
fn d_draws_a_finished_pod_in_grey_below_the_running_ones() {
    let mut h = Harness::with_size(160, 30);
    let mut staging = deployment("staging", 26, Some((4840, "bob", "Speed up CI")));
    staging.pods = parse_pods(
        &serde_json::json!({"items": [
            {"metadata": {"name": "api-evicted"}, "status": {"phase": "Failed"}},
            {"metadata": {"name": "api-1"},
             "status": {"phase": "Running", "containerStatuses": [{"ready": true, "restartCount": 0}]}}
        ]})
        .to_string(),
    )
    .unwrap();

    h.run(vec![
        deployed(vec![staging]),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('d')),
    ])
    .unwrap();

    let screen = h.screen();
    let running = screen.find("│✓ api-1  Running").unwrap();
    let finished = screen.find("│○ api-evicted  Failed").unwrap();
    assert!(running < finished, "{screen}");
}

#[test]
fn d_paints_a_lost_available_condition_red_only_when_nothing_serves() {
    let mut h = Harness::with_size(160, 30);
    let mut rollout = parse_deployment(include_str!("fixtures/deployment.json")).unwrap();
    rollout.available = 1;
    rollout.conditions[0].status = "False".to_string();
    let mut staging = deployment("staging", 26, Some((4840, "bob", "Speed up CI")));
    staging.rollout = Some(rollout.clone());
    rollout.available = 0;
    let mut prod = deployment("prod", 24, Some((4790, "dave", "Spike tests")));
    prod.rollout = Some(rollout);

    h.run(vec![
        deployed(vec![staging, prod]),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('d')),
    ])
    .unwrap();
    let serving = condition_row(&h.screen(), "Available False");
    assert_eq!(h.cell(1, serving).fg, Color::Yellow);

    h.run(vec![key(KeyCode::Char('j'))]).unwrap();
    let down = condition_row(&h.screen(), "Available False");
    assert_eq!(h.cell(1, down).fg, Color::Red);
}

#[test]
fn d_says_why_the_pods_were_not_read() {
    let mut h = Harness::with_size(160, 30);
    let mut staging = deployment("staging", 26, Some((4840, "bob", "Speed up CI")));
    staging.health = Health::Unread;
    staging.pods_error = Some("Error from server (Forbidden): pods is forbidden".to_string());

    h.run(vec![
        deployed(vec![staging]),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('d')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("│pods: Error from server (Forbidden): pods is forbidden"),
        "{screen}"
    );
}

#[test]
fn without_deploy_config_the_column_says_so_and_a_failed_fetch_flags_it() {
    let mut h = Harness::new();
    h.run(vec![]).unwrap();
    let col = column(&h.screen(), 3);
    assert!(
        col[1].contains("no [[repo.deploy]] configured"),
        "{}",
        h.screen()
    );

    h.run(vec![failed(Stage::Deployed, "no repository to watch")])
        .unwrap();
    let col = column(&h.screen(), 3);
    assert!(col[0].contains("Deployed ⚠"), "{}", h.screen());
}

#[test]
fn b_opens_the_main_build_that_matches_the_deployed_sha_or_the_commit_page() {
    let mut h = Harness::new();
    let rows = vec![
        deployment("staging", 26, Some((4840, "bob", "Speed up CI"))),
        deployment("dev", 99, Some((4999, "erin", "Unbuilt"))),
    ];

    h.run(vec![
        builds(main_builds()),
        deployed(rows),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('b')),
    ])
    .unwrap();

    assert_eq!(
        h.opener.opened(),
        vec!["https://github.com/acme/webapp/actions/runs/1026".to_string()]
    );

    h.run(vec![
        key(KeyCode::Char('j')),
        key(KeyCode::Char('b')),
        key(KeyCode::Char('Y')),
    ])
    .unwrap();
    let commit = format!("https://github.com/acme/webapp/commit/{:040x}", 99);
    assert_eq!(
        h.opener.opened(),
        vec![
            "https://github.com/acme/webapp/actions/runs/1026".to_string(),
            commit.clone()
        ],
        "a sha older than the fetched builds falls back to the commit page"
    );
    assert_eq!(h.opener.copied(), vec![commit], "Y copies the same URL");
    assert!(
        !h.screen().contains("no main build found"),
        "{}",
        h.screen()
    );
}

#[test]
fn p_says_which_lookup_failed_and_y_says_nothing_is_selected() {
    let mut h = Harness::with_size(160, 20);

    h.run(vec![
        deployed(vec![deployment("dev", 0x1a, None)]),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('p')),
    ])
    .unwrap();

    assert!(
        h.screen().contains("no pull request for this commit"),
        "{}",
        h.screen()
    );
    assert!(h.opener.opened().is_empty());

    h.run(vec![deployed(vec![]), key(KeyCode::Char('y'))])
        .unwrap();

    assert!(h.screen().contains("nothing selected"), "{}", h.screen());
    assert!(h.opener.copied().is_empty());
}

fn condition_row(screen: &str, text: &str) -> u16 {
    screen
        .lines()
        .position(|line| line.starts_with(&format!("│{text}")))
        .unwrap_or_else(|| panic!("no `{text}` line: {screen}")) as u16
}
