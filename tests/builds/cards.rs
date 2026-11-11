use conveyor::app::Stage;
use conveyor::model::builds::BuildStatus;

use crate::helpers::{column, three};
use crate::support::{Harness, NOW, build, builds, failed, job, jobs, tick_at};

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

#[test]
fn a_run_whose_jobs_come_back_empty_shows_its_sha_not_zero_jobs_ok() {
    let mut h = Harness::with_size(160, 20);

    h.run(vec![builds(three()), jobs(1026, vec![])]).unwrap();

    let col = column(&h.screen(), 2);
    assert!(!col[4].contains("jobs ok"), "{}", h.screen());
    assert!(col[4].contains("00000000"), "{}", h.screen());
}
