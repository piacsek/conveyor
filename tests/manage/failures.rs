use conveyor::model::prs::StatusChange;
use ratatui::crossterm::event::KeyCode;
use ratatui::style::Color;

use crate::helpers::{draft, line_after, open, status_changed, tall};
use crate::support::{key, prs};

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
