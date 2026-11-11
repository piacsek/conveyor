use ratatui::crossterm::event::KeyCode;

use crate::helpers::three;
use crate::support::{Harness, entry, key, pr, prs, queue};

#[test]
fn slash_filters_rows_by_number_title_or_repo_and_shows_the_count() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![pr(1, "alpha"), pr(2, "beta"), pr(3, "gamma")]),
        key(KeyCode::Char('/')),
        key(KeyCode::Char('a')),
        key(KeyCode::Char('m')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("#3 gamma"), "{screen}");
    assert!(!screen.contains("alpha"), "{screen}");
    assert!(screen.contains("/am▏  1/3"), "{screen}");

    h.run(vec![key(KeyCode::Char('z'))]).unwrap();
    let screen = h.screen();
    assert!(screen.contains("no matches for /amz"), "{screen}");

    h.run(vec![key(KeyCode::Esc)]).unwrap();
    let screen = h.screen();
    assert!(
        screen.contains("alpha") && screen.contains("gamma"),
        "{screen}"
    );
    assert!(!screen.contains("/amz"), "{screen}");
}

#[test]
fn enter_commits_the_filter_so_keys_move_again_and_esc_then_clears_it() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![pr(1, "alpha"), pr(2, "beta"), pr(3, "delta")]),
        key(KeyCode::Char('/')),
        key(KeyCode::Char('t')),
        key(KeyCode::Char('a')),
    ])
    .unwrap();
    assert!(h.screen().contains("/ta▏  2/3"), "editing: {}", h.screen());

    h.run(vec![
        key(KeyCode::Enter),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('p')),
    ])
    .unwrap();
    let screen = h.screen();
    assert!(
        screen.contains("/ta  2/3"),
        "committed, no cursor: {screen}"
    );
    assert!(!screen.contains("alpha"), "still filtered: {screen}");
    assert_eq!(
        h.opener.opened(),
        vec![pr(3, "delta").url],
        "j moved to delta instead of typing"
    );

    h.run(vec![key(KeyCode::Esc)]).unwrap();
    let screen = h.screen();
    assert!(
        screen.contains("alpha"),
        "Esc clears the committed filter: {screen}"
    );
    assert!(!screen.contains("/ta"), "{screen}");
}

#[test]
fn a_live_filter_puts_visible_over_total_in_every_column_title() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![pr(1, "alpha"), pr(2, "beta"), pr(3, "delta")]),
        queue(vec![
            entry(1, 10, "alice", "beta rate limits"),
            entry(2, 11, "bob", "gamma"),
        ]),
        key(KeyCode::Char('/')),
        key(KeyCode::Char('t')),
        key(KeyCode::Char('a')),
    ])
    .unwrap();
    let screen = h.screen();
    assert!(screen.contains("My PRs (2/3)"), "{screen}");
    assert!(
        screen.contains("(1/2)"),
        "the unfocused queue says so too: {screen}"
    );

    h.run(vec![key(KeyCode::Enter)]).unwrap();
    assert!(
        h.screen().contains("My PRs (2/3)"),
        "committed too: {}",
        h.screen()
    );

    h.run(vec![key(KeyCode::Esc)]).unwrap();
    let screen = h.screen();
    assert!(screen.contains("My PRs (3)"), "{screen}");
    assert!(screen.contains("(2)"), "{screen}");
}

#[test]
fn slash_alone_shows_the_empty_prompt_at_once() {
    let mut h = Harness::new();
    h.run(vec![prs(three()), key(KeyCode::Char('/'))]).unwrap();
    let screen = h.screen();
    assert!(screen.contains("/▏  3/3"), "{screen}");
    assert!(screen.contains("My PRs (3/3)"), "{screen}");
}

#[test]
fn slash_reopens_a_committed_filter_for_editing() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![pr(1, "alpha"), pr(2, "beta"), pr(3, "delta")]),
        key(KeyCode::Char('/')),
        key(KeyCode::Char('t')),
        key(KeyCode::Enter),
        key(KeyCode::Char('/')),
        key(KeyCode::Char('a')),
    ])
    .unwrap();
    assert!(h.screen().contains("/ta▏  2/3"), "{}", h.screen());
}
