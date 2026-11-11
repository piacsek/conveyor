use ratatui::crossterm::event::KeyCode;

use crate::helpers::three;
use crate::support::{Harness, entry, key, pr, prs, queue};

#[test]
fn no_pull_requests_shows_an_empty_column_and_q_quits() {
    let mut h = Harness::new();

    h.run(vec![prs(vec![]), key(KeyCode::Char('q'))]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("My PRs (0)"), "{screen}");
    assert!(screen.contains("no open pull requests"), "{screen}");
    assert!(h.opener.opened().is_empty());
}

#[test]
fn the_column_says_fetching_until_the_first_data_arrives() {
    let mut h = Harness::new();

    h.run(vec![key(KeyCode::Char('q'))]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("fetching…"), "{screen}");
    assert!(!screen.contains("no open pull requests"), "{screen}");
}

#[test]
fn cards_stack_a_title_a_dim_meta_line_and_the_unhappy_checks_under_a_selection_bar() {
    let mut h = Harness::new();
    let mut first = pr(4821, "Retry hooks");
    first.head_ref = "webhook-retry".to_string();
    first.additions = 12;
    first.deletions = 3;

    h.run(vec![
        prs(vec![first, pr(4830, "Rate limits")]),
        key(KeyCode::Char('q')),
    ])
    .unwrap();

    let screen = h.screen();
    let rows: Vec<&str> = screen.lines().collect();
    assert!(rows[0].contains("My PRs (2)"), "{screen}");
    assert!(
        rows[1].starts_with("│▌ ✓ \u{f407} #4821 Retry hooks"),
        "{screen}"
    );
    assert!(rows[1].contains("2h│"), "{screen}");
    assert!(
        rows[2].starts_with("│▌   webapp · webhook-retry · +12 −3"),
        "{screen}"
    );
    assert!(rows[3].starts_with("│▌   checks: success"), "{screen}");
    assert!(
        rows[4].starts_with("│  ✓ \u{f407} #4830 Rate limits"),
        "{screen}"
    );
    assert!(!screen.contains("1 ✓"), "no row numbers: {screen}");
}

#[test]
fn a_pull_request_icon_is_green_when_open_and_grey_when_draft() {
    use ratatui::style::Color;
    let mut h = Harness::new();
    let open = pr(1, "a");
    let mut draft = pr(2, "b");
    draft.is_draft = true;

    h.run(vec![prs(vec![open, draft])]).unwrap();

    let screen = h.screen();
    let rows: Vec<&str> = screen.lines().collect();
    assert!(rows[1].starts_with("│▌ ✓ \u{f407} #1 a"), "{screen}");
    assert!(rows[4].starts_with("│  ✓ \u{f4dd} #2 b"), "{screen}");
    assert!(!rows[5].contains("draft"), "{screen}");
    assert_eq!(h.cell(5, 1).fg, Color::Green);
    assert_eq!(h.cell(5, 4).fg, Color::DarkGray);
}

#[test]
fn a_pull_request_in_the_merge_queue_shows_the_merge_queue_icon_in_yellow() {
    use ratatui::style::Color;
    let mut h = Harness::new();
    let mut queued = pr(1, "a");
    queued.queue_position = Some(2);

    h.run(vec![prs(vec![queued])]).unwrap();

    let screen = h.screen();
    let rows: Vec<&str> = screen.lines().collect();
    assert!(rows[1].starts_with("│▌ ✓ \u{f4db} #1 a"), "{screen}");
    assert_eq!(h.cell(5, 1).fg, Color::Yellow);
}

#[test]
fn queued_pull_requests_come_first_then_open_then_drafts() {
    let mut h = Harness::with_size(160, 16);
    let mut draft = pr(1, "draft");
    draft.is_draft = true;
    let open = pr(2, "open");
    let mut second = pr(3, "second in queue");
    second.queue_position = Some(2);
    let mut first = pr(4, "first in queue");
    first.queue_position = Some(1);

    h.run(vec![prs(vec![draft, open, second, first])]).unwrap();

    let screen = h.screen();
    let rows: Vec<&str> = screen.lines().collect();
    assert!(rows[1].contains("#4 first in queue"), "{screen}");
    assert!(rows[4].contains("#3 second in queue"), "{screen}");
    assert!(rows[7].contains("#2 open"), "{screen}");
    assert!(rows[10].contains("#1 draft"), "{screen}");
}

#[test]
fn check_glyphs_are_colored_by_state_and_the_age_is_dim() {
    use conveyor::model::prs::CheckState;
    use ratatui::style::{Color, Modifier};
    let mut h = Harness::new();
    let mut failing = pr(1, "a");
    failing.checks = CheckState::Failure;
    let mut pending = pr(2, "b");
    pending.checks = CheckState::Pending;
    let ok = pr(3, "c");

    h.run(vec![prs(vec![failing, pending, ok])]).unwrap();

    assert_eq!(h.cell(3, 1).fg, Color::Red);
    assert_eq!(h.cell(3, 4).fg, Color::Yellow);
    assert_eq!(h.cell(3, 7).fg, Color::Green);
    assert!(h.cell(37, 1).modifier.contains(Modifier::DIM), "age is dim");
    assert!(h.cell(1, 1).fg == Color::Cyan, "the selection bar is cyan");
    assert!(
        h.cell(6, 2).modifier.contains(Modifier::DIM),
        "the meta line is dim"
    );
}

#[test]
fn a_pull_request_in_the_merge_queue_shows_its_position_on_the_row() {
    let mut h = Harness::new();
    let mut queued = pr(4821, "Retry hooks");
    queued.queue_position = Some(2);

    h.run(vec![prs(vec![queued, pr(4830, "Rate limits")])])
        .unwrap();

    let screen = h.screen();
    let rows: Vec<&str> = screen.lines().collect();
    assert!(rows[1].contains("#4821 Retry hooks"), "{screen}");
    assert!(rows[1].contains("⇥2 2h"), "{screen}");
    assert!(!rows[4].contains("⇥"), "{screen}");
}

#[test]
fn the_selection_bar_runs_down_every_line_of_the_selected_card() {
    use ratatui::style::Color;
    let mut h = Harness::new();

    h.run(vec![prs(three()), key(KeyCode::Char('j'))]).unwrap();

    let screen = h.screen();
    let rows: Vec<&str> = screen.lines().collect();
    for (y, row) in rows.iter().enumerate().take(7).skip(4) {
        assert!(
            row.starts_with("│▌"),
            "line {y} of the selected card: {screen}"
        );
        assert_eq!(h.cell(1, y as u16).fg, Color::Cyan, "line {y} is cyan");
    }
    for y in [1, 2, 3, 7, 8, 9] {
        assert!(
            !rows[y].starts_with("│▌"),
            "line {y} belongs to another card: {screen}"
        );
    }
    assert!(
        rows[5].starts_with("│▌   webapp"),
        "the text still starts at the same column: {screen}"
    );
}

#[test]
fn only_the_focused_column_highlights_its_selected_card() {
    let mut h = Harness::new();

    h.run(vec![
        prs(three()),
        queue(vec![entry(1, 4821, "alice", "Retry webhooks")]),
    ])
    .unwrap();

    let screen = h.screen();
    let rows: Vec<&str> = screen.lines().collect();
    assert!(rows[1].starts_with("│▌ ✓ \u{f407} #1 a"), "{screen}");
    assert_eq!(
        screen.matches('▌').count(),
        3,
        "one bar per line of the one selected card: {screen}"
    );

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    let screen = h.screen();
    assert!(
        !screen.lines().nth(1).unwrap().starts_with("│▌"),
        "the PR card lost its bar: {screen}"
    );
    assert!(
        screen.contains("▌ ○ #4821 Retry webhooks"),
        "the queue card took it: {screen}"
    );
    assert_eq!(screen.matches('▌').count(), 3, "still only one card");
}

#[test]
fn a_pull_request_with_auto_merge_on_says_so_on_its_meta_line() {
    let mut h = Harness::new();
    let mut auto = pr(1, "a");
    auto.head_ref = "retry".to_string();
    auto.auto_merge = true;

    h.run(vec![prs(vec![auto, pr(2, "b")])]).unwrap();

    let screen = h.screen();
    let rows: Vec<&str> = screen.lines().collect();
    assert!(
        rows[2].starts_with("│▌   webapp · auto-merge · retry"),
        "{screen}"
    );
    assert!(!rows[5].contains("auto-merge"), "{screen}");
}
