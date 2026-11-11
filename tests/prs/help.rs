use ratatui::crossterm::event::KeyCode;

use crate::helpers::three;
use crate::support::{Harness, key, prs};

#[test]
fn the_help_names_the_focused_column_and_says_what_p_and_b_do_there() {
    let mut h = Harness::new();

    h.run(vec![prs(three()), key(KeyCode::Char('?'))]).unwrap();
    let screen = h.screen();
    assert!(screen.contains("Keys — My PRs"), "{screen}");
    assert!(
        screen.contains("open the failing check, else the first"),
        "{screen}"
    );

    h.run(vec![
        key(KeyCode::Esc),
        key(KeyCode::Char('3')),
        key(KeyCode::Char('?')),
    ])
    .unwrap();
    let screen = h.screen();
    assert!(screen.contains("Keys — Main builds"), "{screen}");
    assert!(screen.contains("open the merged pull request"), "{screen}");
    assert!(screen.contains("open the run"), "{screen}");

    h.run(vec![
        key(KeyCode::Esc),
        key(KeyCode::Char('2')),
        key(KeyCode::Char('?')),
    ])
    .unwrap();
    assert!(
        h.screen().contains("open the merge-group run"),
        "{}",
        h.screen()
    );

    h.run(vec![
        key(KeyCode::Esc),
        key(KeyCode::Char('4')),
        key(KeyCode::Char('?')),
    ])
    .unwrap();
    let screen = h.screen();
    assert!(screen.contains("Keys — Deployed"), "{screen}");
    assert!(
        screen.contains("open the main build for this sha"),
        "{screen}"
    );
}

#[test]
fn question_mark_shows_the_key_help_and_any_key_returns() {
    let mut h = Harness::with_size(160, 16);

    h.run(vec![prs(three()), key(KeyCode::Char('?'))]).unwrap();

    let screen = h.screen();
    for needle in [
        "j/k",
        "p",
        "open pull request",
        "b",
        "open the failing check, else the first",
        "r/R",
        "refresh focused / all",
        "z",
        "zoom",
        "Esc",
        "close details / zoom / filter",
        "log of the failed step",
        "S",
        "manage: merge, update, rerun, status",
        "y/Y",
        "copy pull request / build URL",
        "d",
        "details",
        "/",
        "filter",
        "C-d/C-u",
        "details / scroll them",
        "h/l",
        "column",
        "q",
        "quit",
    ] {
        assert!(screen.contains(needle), "{needle}: {screen}");
    }
    assert!(!screen.contains("#1 a"), "{screen}");

    h.run(vec![key(KeyCode::Char('x'))]).unwrap();
    assert!(h.screen().contains("#1 a"), "{}", h.screen());
}
