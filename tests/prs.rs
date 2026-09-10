mod support;

use ratatui::crossterm::event::KeyCode;
use support::{Harness, key, pr, prs};

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
fn rows_show_number_glyph_repo_title_and_age_with_the_first_highlighted() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![pr(4821, "Retry hooks"), pr(4830, "Rate limits")]),
        key(KeyCode::Char('q')),
    ])
    .unwrap();

    let screen = h.screen();
    let rows: Vec<&str> = screen.lines().collect();
    assert!(rows[0].contains("My PRs (2)"), "{screen}");
    assert!(
        rows[1].starts_with("│> 1 ✓ #4821 webapp  Retry hooks"),
        "{screen}"
    );
    assert!(rows[1].contains("2h│"), "{screen}");
    assert!(
        rows[2].starts_with("│  2 ✓ #4830 webapp  Rate limits"),
        "{screen}"
    );
}
