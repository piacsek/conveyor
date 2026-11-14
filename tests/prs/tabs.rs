use conveyor::app::{Request, Stage};
use conveyor::config::{Config, PrsTab};
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{highlighted_row, three};
use crate::support::{Harness, key, pr, prs_in};

#[test]
fn tab_shows_the_second_tab_and_asks_for_its_rows() {
    let mut h = Harness::with_config(160, 12, two_tabs());
    h.run(vec![prs_in(0, three())]).unwrap();

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    assert_eq!(h.requests, vec![Request::Refresh(Stage::Prs, 1)]);
    assert!(!h.screen().contains("#1 a"), "{}", h.screen());
}

#[test]
fn the_second_tab_shows_the_rows_fetched_for_it() {
    let mut h = Harness::with_config(160, 12, two_tabs());
    h.run(vec![prs_in(0, three()), key(KeyCode::Tab)]).unwrap();

    h.run(vec![prs_in(1, vec![pr(7, "g")])]).unwrap();

    assert!(h.screen().contains("#7 g"), "{}", h.screen());
}

#[test]
fn shift_tab_returns_to_the_first_tab_with_its_selection_and_without_the_filter() {
    let mut h = Harness::with_config(160, 12, two_tabs());
    h.run(vec![prs_in(0, three())]).unwrap();
    h.run(filter("#2")).unwrap();
    h.run(vec![key(KeyCode::Tab), prs_in(1, vec![pr(7, "g")])])
        .unwrap();

    h.run(vec![key(KeyCode::BackTab)]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("My PRs (3)"), "{screen}");
    assert!(
        screen
            .lines()
            .nth(highlighted_row(&screen))
            .unwrap()
            .contains("#2 b"),
        "{screen}"
    );
    assert_eq!(h.app.query, None);
}

#[test]
fn switching_tab_clears_a_filter_kept_on_the_new_tab_too() {
    let mut h = Harness::with_config(160, 12, two_tabs());
    h.run(vec![
        prs_in(0, three()),
        key(KeyCode::Tab),
        prs_in(1, vec![pr(7, "g"), pr(8, "h")]),
    ])
    .unwrap();
    h.run(filter("#7")).unwrap();

    h.run(vec![key(KeyCode::Tab)]).unwrap();
    h.run(vec![key(KeyCode::Tab)]).unwrap();

    assert!(h.screen().contains("#8 h"), "{}", h.screen());
}

#[test]
fn tab_in_a_column_with_one_tab_does_nothing() {
    let mut h = Harness::with_config(160, 12, two_tabs());
    h.run(vec![
        prs_in(0, three()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
    ])
    .unwrap();

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    assert_eq!(h.app.focus, Stage::Builds);
    assert!(h.requests.is_empty());
}

#[test]
fn tab_with_one_my_prs_tab_does_nothing() {
    let mut h = Harness::new();
    h.run(vec![prs_in(0, three())]).unwrap();

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    assert_eq!(h.app.focus, Stage::Prs);
    assert!(h.requests.is_empty());
}

#[test]
fn tab_while_typing_a_filter_does_nothing() {
    let mut h = Harness::with_config(160, 12, two_tabs());
    h.run(vec![
        prs_in(0, three()),
        key(KeyCode::Char('/')),
        key(KeyCode::Char('b')),
    ])
    .unwrap();

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    assert!(h.requests.is_empty());
    assert_eq!(h.app.query.as_deref(), Some("b"));
}

#[test]
fn h_and_l_move_focus_and_keep_the_filter() {
    let mut h = Harness::with_config(160, 12, two_tabs());
    h.run(vec![prs_in(0, three())]).unwrap();
    h.run(filter("b")).unwrap();

    h.run(vec![key(KeyCode::Char('l'))]).unwrap();

    assert_eq!(h.app.focus, Stage::Queue);
    assert_eq!(h.app.query.as_deref(), Some("b"));
}

fn two_tabs() -> Config {
    let mut config = Config::default();
    config.prs.tab = vec![tab("mine"), tab("review")];
    config
}

fn tab(name: &str) -> PrsTab {
    PrsTab {
        name: name.to_string(),
        query: format!("is:pr {name}"),
        limit: None,
    }
}

fn filter(query: &str) -> Vec<std::io::Result<conveyor::app::Input>> {
    let mut inputs = vec![key(KeyCode::Char('/'))];
    inputs.extend(query.chars().map(|c| key(KeyCode::Char(c))));
    inputs.push(key(KeyCode::Enter));
    inputs
}
