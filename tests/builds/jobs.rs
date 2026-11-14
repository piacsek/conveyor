use conveyor::app::Request;
use conveyor::model::builds::BuildStatus;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{column, focus_builds, three};
use crate::support::{Harness, NOW, build, builds, job, jobs, jobs_failed, key, tick_at};

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
fn a_running_job_in_the_details_says_how_long_it_has_been_running() {
    let mut h = Harness::with_size(160, 20);
    let mut running = job(1, "check", BuildStatus::Running, None);
    running.started_at = Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(NOW - 75));
    running.completed_at = None;

    h.run(vec![
        builds(three()),
        jobs(1026, vec![running]),
        tick_at(NOW),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('d')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("● check  1m15s"), "{screen}");
}
