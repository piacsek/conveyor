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

fn three() -> Vec<conveyor::model::prs::PullRequest> {
    vec![pr(1, "a"), pr(2, "b"), pr(3, "c")]
}

fn highlighted_row(screen: &str) -> usize {
    screen
        .lines()
        .position(|line| line.starts_with("│> "))
        .expect("a highlighted row")
}

#[test]
fn j_k_arrows_gg_and_g_move_the_highlight_and_clamp() {
    let mut h = Harness::new();
    h.run(vec![
        prs(three()),
        key(KeyCode::Char('j')),
        key(KeyCode::Down),
    ])
    .unwrap();
    assert_eq!(highlighted_row(&h.screen()), 3);

    h.run(vec![key(KeyCode::Char('j')), key(KeyCode::Char('j'))])
        .unwrap();
    assert_eq!(highlighted_row(&h.screen()), 3, "clamps at the end");

    h.run(vec![
        key(KeyCode::Char('k')),
        key(KeyCode::Up),
        key(KeyCode::Up),
    ])
    .unwrap();
    assert_eq!(highlighted_row(&h.screen()), 1, "clamps at the start");

    h.run(vec![key(KeyCode::Char('G'))]).unwrap();
    assert_eq!(highlighted_row(&h.screen()), 3);

    h.run(vec![key(KeyCode::Char('g')), key(KeyCode::Char('g'))])
        .unwrap();
    assert_eq!(highlighted_row(&h.screen()), 1);
}
