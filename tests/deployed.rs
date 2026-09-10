mod support;

use conveyor::app::Stage;
use conveyor::model::builds::BuildStatus;
use ratatui::crossterm::event::KeyCode;
use support::{Harness, build, builds, deployed, deployment, failed, key};

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
fn deployed_cards_show_env_pull_request_sha_and_how_far_behind_main() {
    let mut h = Harness::new();

    h.run(vec![builds(main_builds()), deployed(envs())])
        .unwrap();

    let col = column(&h.screen(), 3);
    assert!(col[0].contains("Deployed api"), "{}", h.screen());
    assert!(col[1].contains("▌ ✓ staging"), "{}", h.screen());
    assert!(col[1].contains("at main"), "{}", h.screen());
    assert!(col[2].contains("#4840 bob · Speed up CI"), "{}", h.screen());
    assert!(col[3].contains("00000000 · read 6d ago"), "{}", h.screen());
    assert!(col[4].contains("● prod"), "{}", h.screen());
    assert!(col[4].contains("↓2"), "two builds behind: {}", h.screen());
    assert!(
        col[5].contains("#4790 dave · Spike tests"),
        "{}",
        h.screen()
    );
    assert!(col[7].contains("✗ uat"), "{}", h.screen());
    assert!(
        col[8].contains("ERROR: Active profile expired."),
        "the error takes the place of a pull request: {}",
        h.screen()
    );
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
        col[2].contains("#4840 bob · Speed up CI"),
        "old row kept: {}",
        h.screen()
    );
    assert!(
        col[3].contains("kubectl exited with exit status: 1"),
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
fn enter_opens_the_pull_request_and_p_shows_the_image_and_target() {
    let mut h = Harness::with_size(160, 20);

    h.run(vec![
        deployed(envs()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Enter),
        key(KeyCode::Char('p')),
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
fn b_opens_the_main_build_that_matches_the_deployed_sha() {
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

    h.run(vec![key(KeyCode::Char('j')), key(KeyCode::Char('b'))])
        .unwrap();
    assert_eq!(h.opener.opened().len(), 1);
    assert!(
        h.screen().contains("no main build found for 00000000"),
        "{}",
        h.screen()
    );
}
