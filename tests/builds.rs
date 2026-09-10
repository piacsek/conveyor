mod support;

use conveyor::app::Stage;
use conveyor::model::builds::BuildStatus;
use ratatui::crossterm::event::KeyCode;
use support::{Harness, NOW, build, builds, failed, key, tick_at};

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
fn build_rows_show_status_pull_request_author_title_duration_and_age() {
    let mut h = Harness::new();

    h.run(vec![builds(three()), tick_at(NOW)]).unwrap();

    let col = column(&h.screen(), 2);
    assert!(col[0].contains("Main webapp (3)"), "{}", h.screen());
    assert!(
        col[1].contains("1 ● #4840 bob  Speed up CI"),
        "{}",
        h.screen()
    );
    assert!(col[1].contains("1m 1m"), "elapsed then age: {}", h.screen());
    assert!(
        col[2].contains("2 ✗ #4821 alice  Retry hooks"),
        "{}",
        h.screen()
    );
    assert!(col[2].contains("3m 2h"), "{}", h.screen());
    assert!(
        col[3].contains("3 ✓ run 24 bot  run 24"),
        "no PR known: {}",
        h.screen()
    );
}

#[test]
fn enter_opens_the_run_and_p_shows_its_details() {
    let mut h = Harness::with_size(160, 20);

    h.run(vec![
        builds(three()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('j')),
        key(KeyCode::Enter),
        key(KeyCode::Char('p')),
    ])
    .unwrap();

    assert_eq!(
        h.opener.opened(),
        vec!["https://github.com/acme/webapp/actions/runs/1025".to_string()]
    );
    let screen = h.screen();
    assert!(screen.contains("run 25  failure  3m"), "{screen}");
    assert!(screen.contains("#4821 alice  Retry hooks"), "{screen}");
    assert!(
        screen.contains("https://github.com/acme/webapp/pull/4821"),
        "{screen}"
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
