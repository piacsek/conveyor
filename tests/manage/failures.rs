use std::io;

use conveyor::app::{Input, Stage};
use conveyor::model::prs::{BranchUpdate, PullRequest, StatusChange};
use ratatui::crossterm::event::KeyCode;
use ratatui::style::Color;

use crate::helpers::{draft, line_after, open, status_changed, tall};
use crate::support::{Harness, checkout_target, failed, key, prs};

#[test]
fn a_failed_status_change_stays_on_its_card_after_another_one_lands_and_a_refresh() {
    let mut h = tall();

    h.run(vec![
        prs(vec![draft(12, "Retry webhooks"), draft(14, "Bump serde")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        status_changed(12, StatusChange::Ready, Err("HTTP 422")),
        status_changed(14, StatusChange::Ready, Ok(())),
        prs(vec![draft(12, "Retry webhooks"), open(14, "Bump serde")]),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        line_after(&screen, "#12 Retry webhooks").contains("! mark ready failed: HTTP 422"),
        "{screen}"
    );
    assert!(
        !line_after(&screen, "#14 Bump serde").contains("failed"),
        "{screen}"
    );
}

#[test]
fn the_mark_is_drawn_in_red() {
    let mut h = tall();

    h.run(vec![
        prs(vec![draft(12, "Retry webhooks")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        status_changed(12, StatusChange::Ready, Err("HTTP 422")),
    ])
    .unwrap();

    let (x, y) = position(&h.screen(), "! mark ready failed");
    assert_eq!(h.cell(x, y).fg, Color::Red);
}

#[test]
fn a_failure_that_lands_while_the_popup_is_open_on_another_card_stays_on_its_own() {
    let mut h = tall();

    h.run(vec![
        prs(vec![draft(12, "Retry webhooks"), draft(14, "Bump serde")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('S')),
        status_changed(12, StatusChange::Ready, Err("HTTP 422")),
        key(KeyCode::Esc),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(!screen.contains("Manage #14"), "{screen}");
    assert!(
        line_after(&screen, "#12 Retry webhooks").contains("! mark ready failed: HTTP 422"),
        "{screen}"
    );
}

#[test]
fn a_failed_merge_marks_its_card() {
    let screen = after(
        &['m', 'y'],
        Ok(Input::Merged(12, Err("HTTP 405".to_string()))),
    );

    assert!(
        line_after(&screen, "#12 Retry webhooks").contains("! merge failed: HTTP 405"),
        "{screen}"
    );
}

#[test]
fn a_merge_that_was_sent_leaves_no_mark() {
    let screen = after(&['m', 'y'], Ok(Input::Merged(12, Ok(()))));

    assert!(!screen.contains("failed"), "{screen}");
}

#[test]
fn a_failed_update_marks_its_card() {
    let screen = after(&['u'], updated(Err("HTTP 502")));

    assert!(
        line_after(&screen, "#12 Retry webhooks").contains("! update failed: HTTP 502"),
        "{screen}"
    );
}

#[test]
fn an_update_that_hit_conflicts_marks_its_card() {
    let screen = after(&['u'], updated(Ok(BranchUpdate::Conflicts)));

    assert!(
        line_after(&screen, "#12 Retry webhooks").contains("! update failed: has conflicts"),
        "{screen}"
    );
}

#[test]
fn an_update_already_up_to_date_leaves_no_mark() {
    let screen = after(&['u'], updated(Ok(BranchUpdate::UpToDate)));

    assert!(!screen.contains("failed"), "{screen}");
}

#[test]
fn a_failed_checkout_marks_its_card() {
    let screen = after(&['c'], checked_out(Err("not a clone")));

    assert!(
        line_after(&screen, "#12 Retry webhooks").contains("! check out failed: not a clone"),
        "{screen}"
    );
}

#[test]
fn a_checkout_that_worked_leaves_no_mark() {
    let screen = after(&['c'], checked_out(Ok(())));

    assert!(!screen.contains("failed"), "{screen}");
}

#[test]
fn a_new_write_on_the_marked_card_takes_the_mark_away() {
    let mut h = marked();

    h.run(vec![key(KeyCode::Char('S')), key(KeyCode::Char('t'))])
        .unwrap();

    let screen = h.screen();
    assert!(
        line_after(&screen, "#12 Retry webhooks").contains("marking ready…"),
        "{screen}"
    );
    assert!(!screen.contains("! mark ready failed"), "{screen}");
}

#[test]
fn a_new_write_that_works_leaves_no_mark() {
    let mut h = marked();

    h.run(vec![
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        status_changed(12, StatusChange::Ready, Ok(())),
    ])
    .unwrap();

    assert!(
        !h.screen().contains("! mark ready failed"),
        "{}",
        h.screen()
    );
}

#[test]
fn a_new_write_that_fails_shows_the_new_reason_alone() {
    let mut h = marked();

    h.run(vec![
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        status_changed(12, StatusChange::Ready, Err("HTTP 503")),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        line_after(&screen, "#12 Retry webhooks").contains("! mark ready failed: HTTP 503"),
        "{screen}"
    );
    assert!(!screen.contains("HTTP 422"), "{screen}");
}

#[test]
fn the_column_title_counts_the_marks() {
    let mut h = tall();

    h.run(two_marked()).unwrap();

    assert!(h.screen().contains("┌My PRs (5) !2─"), "{}", h.screen());
}

#[test]
fn the_tab_of_the_narrow_layout_counts_the_marks() {
    let mut h = Harness::with_size(80, 24);

    h.run(two_marked()).unwrap();

    assert!(
        h.screen()
            .lines()
            .next()
            .unwrap_or_default()
            .contains("My PRs (5) !2"),
        "{}",
        h.screen()
    );
}

#[test]
fn a_column_without_marks_has_no_count() {
    let mut h = tall();

    h.run(vec![prs(five())]).unwrap();

    assert!(h.screen().contains("┌My PRs (5)─"), "{}", h.screen());
}

#[test]
fn a_refresh_without_the_marked_row_takes_its_mark_along() {
    let mut h = marked_next_to_another();

    h.run(vec![prs(vec![draft(14, "Bump serde")])]).unwrap();

    assert!(h.screen().contains("┌My PRs (1)─"), "{}", h.screen());

    h.run(vec![prs(vec![
        draft(12, "Retry webhooks"),
        draft(14, "Bump serde"),
    ])])
    .unwrap();

    assert!(
        !h.screen().contains("! mark ready failed"),
        "{}",
        h.screen()
    );
}

#[test]
fn a_failed_refresh_keeps_the_mark() {
    let mut h = marked_next_to_another();

    h.run(vec![failed(Stage::Prs, "HTTP 502")]).unwrap();

    assert!(h.screen().contains("┌My PRs (2) !1 ⚠─"), "{}", h.screen());
}

#[test]
fn a_refresh_that_still_holds_the_marked_row_keeps_the_mark() {
    let mut h = marked_next_to_another();

    h.run(vec![prs(vec![
        draft(12, "Retry webhooks"),
        draft(14, "Bump serde"),
    ])])
    .unwrap();

    assert!(h.screen().contains("┌My PRs (2) !1─"), "{}", h.screen());
}

fn marked_next_to_another() -> Harness {
    let mut h = tall();
    h.run(vec![
        prs(vec![draft(12, "Retry webhooks"), draft(14, "Bump serde")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        status_changed(12, StatusChange::Ready, Err("HTTP 422")),
    ])
    .unwrap();
    h
}

fn two_marked() -> Vec<io::Result<Input>> {
    vec![
        prs(five()),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        status_changed(1, StatusChange::Ready, Err("HTTP 422")),
        status_changed(2, StatusChange::Ready, Err("HTTP 422")),
    ]
}

fn five() -> Vec<PullRequest> {
    (1..=5).map(|number| draft(number, "a")).collect()
}

fn marked() -> Harness {
    let mut h = tall();
    h.run(vec![
        prs(vec![draft(12, "Retry webhooks")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        status_changed(12, StatusChange::Ready, Err("HTTP 422")),
    ])
    .unwrap();
    h
}

fn after(letters: &[char], result: io::Result<Input>) -> String {
    let mut h = tall();
    let mut inputs = vec![
        prs(vec![open(12, "Retry webhooks")]),
        key(KeyCode::Char('S')),
    ];
    inputs.extend(letters.iter().map(|letter| key(KeyCode::Char(*letter))));
    inputs.push(result);
    h.run(inputs).unwrap();
    h.screen()
}

fn updated(result: Result<BranchUpdate, &str>) -> io::Result<Input> {
    Ok(Input::BranchUpdated(12, result.map_err(str::to_string)))
}

fn checked_out(result: Result<(), &str>) -> io::Result<Input> {
    Ok(Input::CheckedOut(
        checkout_target(12, None),
        result.map_err(str::to_string),
    ))
}

fn position(screen: &str, needle: &str) -> (u16, u16) {
    screen
        .lines()
        .enumerate()
        .find_map(|(y, line)| {
            line.find(needle)
                .map(|at| (line[..at].chars().count() as u16, y as u16))
        })
        .unwrap_or_else(|| panic!("{needle} is not on screen: {screen}"))
}
