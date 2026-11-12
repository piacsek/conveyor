use ratatui::crossterm::event::KeyCode;

use crate::support::{Harness, NOW, key, pr, prs, tick_at};

const LONG_REASON: &str = "Pull request #4821 cannot be marked ready for review because the \
    repository requires every commit to be signed and the head commit 8c1f2a7d of retry-hooks \
    carries no signature that GitHub could verify against the key of its author";

#[test]
fn d_toggles_a_details_pane_with_failed_checks_first() {
    use conveyor::model::prs::{Check, CheckConclusion, MergeState, ReviewDecision};
    let mut h = Harness::with_size(160, 24);
    let mut detailed = pr(4821, "Retry hooks");
    detailed.head_ref = "webhook-retry".to_string();
    detailed.additions = 12;
    detailed.deletions = 3;
    detailed.review = ReviewDecision::Approved;
    detailed.merge_state = MergeState::Blocked;
    detailed.checks_detail = vec![
        Check {
            name: "lint".to_string(),
            conclusion: CheckConclusion::Success,
            url: "https://ci/lint".to_string(),
            ..Default::default()
        },
        Check {
            name: "api / test".to_string(),
            conclusion: CheckConclusion::Failure,
            url: "https://ci/test".to_string(),
            ..Default::default()
        },
    ];

    h.run(vec![prs(vec![detailed]), key(KeyCode::Char('d'))])
        .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("#4821 acme/webapp  webhook-retry  +12 −3"),
        "{screen}"
    );
    assert!(screen.contains("review: approved"), "{screen}");
    assert!(screen.contains("merge: blocked"), "{screen}");
    let test_line = screen
        .lines()
        .position(|l| l.contains("✗ api / test"))
        .unwrap();
    let lint_line = screen.lines().position(|l| l.contains("✓ lint")).unwrap();
    assert!(test_line < lint_line, "failures first: {screen}");
    assert!(!screen.contains("https://"), "no check URLs: {screen}");

    h.run(vec![key(KeyCode::Char('d'))]).unwrap();
    assert!(!h.screen().contains("review: approved"), "{}", h.screen());
    assert!(!h.screen().contains("✓ lint"), "{}", h.screen());
}

#[test]
fn a_skipped_check_keeps_the_dash_that_never_meant_cancelled() {
    use conveyor::model::prs::{Check, CheckConclusion};
    let mut h = Harness::with_size(160, 20);
    let mut skipping = pr(4821, "Retry hooks");
    skipping.checks_detail = vec![Check {
        name: "e2e".to_string(),
        conclusion: CheckConclusion::Skipped,
        url: String::new(),
        ..Default::default()
    }];

    h.run(vec![prs(vec![skipping]), key(KeyCode::Char('d'))])
        .unwrap();

    let screen = h.screen();
    assert!(screen.contains("- e2e"), "{screen}");
    assert!(!screen.contains("⊘"), "⊘ is only ever cancelled: {screen}");
}

#[test]
fn each_check_in_the_details_says_how_long_it_took_or_has_been_running() {
    use conveyor::model::prs::{Check, CheckConclusion};
    use std::time::{Duration, UNIX_EPOCH};
    let at = |secs: u64| Some(UNIX_EPOCH + Duration::from_secs(secs));
    let mut h = Harness::with_size(160, 20);
    let mut checked = pr(4821, "Retry hooks");
    checked.checks_detail = vec![
        Check {
            name: "lint".to_string(),
            conclusion: CheckConclusion::Success,
            started_at: at(NOW - 300),
            completed_at: at(NOW - 258),
            ..Default::default()
        },
        Check {
            name: "api / test".to_string(),
            conclusion: CheckConclusion::Pending,
            started_at: at(NOW - 192),
            ..Default::default()
        },
        Check {
            name: "ci/circleci".to_string(),
            conclusion: CheckConclusion::Pending,
            ..Default::default()
        },
    ];

    h.run(vec![
        prs(vec![checked]),
        tick_at(NOW),
        key(KeyCode::Char('d')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("✓ lint  42s"), "{screen}");
    assert!(screen.contains(" api / test  3m12s"), "{screen}");
    let after_status = screen.split("ci/circleci").nth(1).unwrap_or_default();
    let rest_of_line = after_status.split('│').next().unwrap_or_default();
    assert_eq!(rest_of_line.trim(), "", "{screen}");
}

#[test]
fn the_details_of_a_marked_card_start_with_the_whole_reason_and_its_time() {
    let mut h = Harness::with_size(160, 24);

    h.run(vec![
        prs(vec![draft(4821, "Retry hooks")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        tick_at(NOW + 3723),
        status_changed(Err(LONG_REASON)),
        key(KeyCode::Char('d')),
    ])
    .unwrap();

    let screen = h.screen();
    let first = pane_line(&screen, 0);
    assert!(
        first.starts_with("│mark ready failed at 09:02:03: Pull request #4821"),
        "{screen}"
    );
    assert!(pane_line(&screen, 1).contains("of its author"), "{screen}");
    assert!(
        pane_line(&screen, 2).contains("#4821 acme/webapp"),
        "{screen}"
    );
    let y = pane_top(&screen) as u16 + 1;
    assert_eq!(h.cell(1, y).fg, ratatui::style::Color::Red);
}

#[test]
fn the_details_of_an_unmarked_card_start_as_before() {
    let mut h = Harness::with_size(160, 24);

    h.run(vec![
        prs(vec![draft(4821, "Retry hooks")]),
        key(KeyCode::Char('d')),
    ])
    .unwrap();

    assert!(
        pane_line(&h.screen(), 0).contains("#4821 acme/webapp"),
        "{}",
        h.screen()
    );
}

fn draft(number: u64, title: &str) -> conveyor::model::prs::PullRequest {
    let mut pull = pr(number, title);
    pull.is_draft = true;
    pull
}

fn status_changed(result: Result<(), &str>) -> std::io::Result<conveyor::app::Input> {
    Ok(conveyor::app::Input::StatusChanged(
        4821,
        conveyor::model::prs::StatusChange::Ready,
        result.map_err(str::to_string),
    ))
}

fn pane_top(screen: &str) -> usize {
    screen
        .lines()
        .position(|line| line.starts_with("┌#4821"))
        .unwrap_or_else(|| panic!("no details pane: {screen}"))
}

fn pane_line(screen: &str, index: usize) -> String {
    let top = pane_top(screen);
    screen
        .lines()
        .nth(top + 1 + index)
        .unwrap_or_default()
        .to_string()
}
