use ratatui::crossterm::event::KeyCode;

use crate::helpers::{highlighted_row, three};
use crate::support::{self, Harness, NOW, key, pr, prs, tick_at};

#[test]
fn r_requests_a_refresh_and_the_footer_shows_the_data_age() {
    use conveyor::app::Stage;
    let mut h = Harness::new();

    h.run(vec![prs(three()), tick_at(NOW + 12)]).unwrap();
    assert!(
        h.screen().contains("refreshed at 08:00:00"),
        "{}",
        h.screen()
    );

    h.run(vec![key(KeyCode::Char('r'))]).unwrap();
    assert_eq!(
        h.requests,
        vec![conveyor::app::Request::Refresh(Stage::Prs, 0)]
    );
}

#[test]
fn a_failed_fetch_keeps_the_old_rows_and_flags_the_column_until_the_next_success() {
    use conveyor::app::Stage;
    let mut h = Harness::new();

    h.run(vec![
        prs(three()),
        support::failed(Stage::Prs, "gh: HTTP 401: Bad credentials"),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("My PRs (3) ⚠"), "{screen}");
    assert!(screen.contains("#1 a"), "rows kept: {screen}");
    assert!(screen.contains("gh: HTTP 401: Bad credentials"), "{screen}");

    h.run(vec![prs(three())]).unwrap();
    let screen = h.screen();
    assert!(!screen.contains("⚠"), "{screen}");
    assert!(!screen.contains("Bad credentials"), "{screen}");
}

#[test]
fn a_refresh_keeps_the_row_order_and_the_selection_follows_the_number() {
    let mut h = Harness::new();

    h.run(vec![prs(three()), key(KeyCode::Char('j'))]).unwrap();
    h.run(vec![prs(vec![pr(4, "d"), pr(3, "c"), pr(2, "b")])])
        .unwrap();

    let screen = h.screen();
    let rows: Vec<&str> = screen.lines().collect();
    assert!(rows[1].contains("#2 b"), "kept: {screen}");
    assert!(rows[4].contains("#3 c"), "kept: {screen}");
    assert!(rows[7].contains("#4 d"), "appended: {screen}");
    assert!(!screen.contains("#1 a"), "gone: {screen}");
    assert_eq!(highlighted_row(&screen), 1, "selection follows #2");

    h.run(vec![prs(vec![pr(4, "d")])]).unwrap();
    assert_eq!(
        highlighted_row(&h.screen()),
        1,
        "falls back to the first row"
    );
}

#[test]
fn a_failure_before_any_data_flags_the_column_and_shows_the_message_in_the_body() {
    use conveyor::app::Stage;
    let mut h = Harness::new();

    h.run(vec![support::failed(
        Stage::Prs,
        "gh: HTTP 401: Bad credentials",
    )])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("My PRs ⚠"), "{screen}");
    assert!(
        !screen.lines().nth(1).unwrap().starts_with("│fetching…"),
        "{screen}"
    );
    assert!(
        screen
            .lines()
            .nth(1)
            .unwrap()
            .contains("gh: HTTP 401: Bad credentials"),
        "{screen}"
    );
}

#[test]
fn shift_r_refreshes_every_column() {
    use conveyor::app::{Request, Stage};
    let mut h = Harness::new();

    h.run(vec![prs(three()), key(KeyCode::Char('R'))]).unwrap();

    assert_eq!(
        h.requests,
        Stage::ALL.map(|stage| Request::Refresh(stage, 0)).to_vec(),
        "one request per column"
    );
}
