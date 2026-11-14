use conveyor::app::{Request, Stage};
use conveyor::config::{Config, Key, PrsTab};
use ratatui::crossterm::event::KeyCode;
use ratatui::style::Modifier;

use crate::helpers::{highlighted_row, three};
use crate::support::{Harness, ctrl, failed_in, key, pr, prs_in};

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

#[test]
fn a_digit_shows_that_tab_and_asks_for_its_rows() {
    let mut h = Harness::with_config(160, 12, three_tabs());
    h.run(vec![prs_in(0, three())]).unwrap();

    h.run(vec![key(KeyCode::Char('3'))]).unwrap();

    assert_eq!(h.requests, vec![Request::Refresh(Stage::Prs, 2)]);
    assert!(!h.screen().contains("#1 a"), "{}", h.screen());
}

#[test]
fn one_returns_to_the_first_tab() {
    let mut h = Harness::with_config(160, 12, three_tabs());
    h.run(vec![prs_in(0, three()), key(KeyCode::Char('3'))])
        .unwrap();

    h.run(vec![key(KeyCode::Char('1'))]).unwrap();

    assert!(h.screen().contains("#1 a"), "{}", h.screen());
    assert_eq!(h.requests.last(), Some(&Request::Refresh(Stage::Prs, 0)));
}

#[test]
fn a_digit_past_the_last_tab_does_nothing() {
    let mut h = Harness::with_config(160, 12, three_tabs());
    h.run(vec![prs_in(0, three())]).unwrap();

    h.run(vec![key(KeyCode::Char('4'))]).unwrap();

    assert!(h.requests.is_empty());
    assert_eq!(h.app.focus, Stage::Prs);
    assert!(h.screen().contains("#1 a"), "{}", h.screen());
}

#[test]
fn the_active_tabs_digit_does_nothing() {
    let mut h = Harness::with_config(160, 12, three_tabs());
    h.run(vec![prs_in(0, three())]).unwrap();

    h.run(vec![key(KeyCode::Char('1'))]).unwrap();

    assert!(h.requests.is_empty());
}

#[test]
fn a_digit_in_a_column_with_one_tab_keeps_the_focus() {
    let mut h = Harness::with_config(160, 12, three_tabs());
    h.run(vec![
        prs_in(0, three()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
    ])
    .unwrap();

    h.run(vec![key(KeyCode::Char('2'))]).unwrap();

    assert_eq!(h.app.focus, Stage::Builds);
    assert!(h.requests.is_empty());
}

#[test]
fn a_digit_while_typing_a_filter_is_typed_into_it() {
    let mut h = Harness::with_config(160, 12, three_tabs());
    h.run(vec![prs_in(0, three()), key(KeyCode::Char('/'))])
        .unwrap();

    h.run(vec![key(KeyCode::Char('2'))]).unwrap();

    assert_eq!(h.app.query.as_deref(), Some("2"));
    assert!(h.requests.is_empty());
}

#[test]
fn rows_fetched_for_a_tab_left_behind_land_in_that_tab() {
    let mut h = Harness::with_config(160, 12, two_tabs());
    h.run(vec![
        prs_in(0, three()),
        key(KeyCode::Tab),
        prs_in(1, vec![pr(7, "g")]),
    ])
    .unwrap();

    h.run(vec![prs_in(0, vec![pr(9, "z")])]).unwrap();

    assert!(!h.screen().contains("#9 z"), "{}", h.screen());
    assert!(h.screen().contains("#7 g"), "{}", h.screen());
    h.run(vec![key(KeyCode::BackTab)]).unwrap();
    assert!(h.screen().contains("#9 z"), "{}", h.screen());
}

#[test]
fn an_error_for_a_tab_left_behind_shows_once_you_switch_to_it() {
    let mut h = Harness::with_config(160, 12, two_tabs());
    h.run(vec![
        prs_in(0, three()),
        key(KeyCode::Tab),
        prs_in(1, vec![pr(7, "g")]),
    ])
    .unwrap();

    h.run(vec![failed_in(Stage::Prs, 0, "gh: HTTP 502")])
        .unwrap();

    assert!(!h.screen().contains("⚠"), "{}", h.screen());
    h.run(vec![key(KeyCode::BackTab)]).unwrap();
    assert!(h.screen().contains("My PRs (3) ⚠"), "{}", h.screen());
}

#[test]
fn a_fetch_started_for_a_tab_left_behind_does_not_mark_the_active_one_refreshing() {
    let mut h = Harness::with_config(160, 12, two_tabs());
    h.run(vec![
        prs_in(0, three()),
        key(KeyCode::Tab),
        prs_in(1, vec![pr(7, "g")]),
    ])
    .unwrap();

    h.run(vec![Ok(conveyor::app::Input::Fetching(Stage::Prs, 0))])
        .unwrap();

    assert!(!h.app.prs.column().refreshing);
}

#[test]
fn the_bar_lists_the_tabs_inside_the_column_above_the_cards() {
    let mut h = Harness::with_config(160, 12, three_tabs());

    h.run(vec![prs_in(0, three())]).unwrap();

    let screen = h.screen();
    let lines: Vec<&str> = screen.lines().collect();
    assert!(lines[1].starts_with("│mine │ review │ team"), "{screen}");
    assert!(lines[2].contains("#1 a"), "{screen}");
}

#[test]
fn the_active_tab_is_bold_and_reversed() {
    let mut h = Harness::with_config(160, 12, three_tabs());

    h.run(vec![prs_in(0, three()), key(KeyCode::Char('2'))])
        .unwrap();

    let review = h.cell(8, 1).modifier;
    assert!(review.contains(Modifier::REVERSED));
    assert!(review.contains(Modifier::BOLD));
}

#[test]
fn the_other_tabs_are_dim() {
    let mut h = Harness::with_config(160, 12, three_tabs());

    h.run(vec![prs_in(0, three()), key(KeyCode::Char('2'))])
        .unwrap();

    let mine = h.cell(1, 1).modifier;
    assert!(mine.contains(Modifier::DIM));
    assert!(!mine.contains(Modifier::REVERSED));
}

#[test]
fn a_bar_too_wide_for_the_column_is_cut_after_the_active_tab() {
    let mut h = Harness::with_config(160, 12, long_tabs());

    h.run(vec![prs_in(0, three()), key(KeyCode::Char('2'))])
        .unwrap();

    let screen = h.screen();
    let bar = screen.lines().nth(1).unwrap();
    assert!(
        bar.starts_with("│alpha-long-name │ bravo-long-name │ c…│"),
        "{screen}"
    );
}

#[test]
fn a_bar_too_wide_for_the_column_drops_tabs_before_the_active_one() {
    let mut h = Harness::with_config(160, 12, long_tabs());

    h.run(vec![prs_in(0, three()), key(KeyCode::Char('3'))])
        .unwrap();

    let screen = h.screen();
    let bar = screen.lines().nth(1).unwrap();
    assert!(bar.starts_with("│… │ charlie-long-name "), "{screen}");
}

#[test]
fn the_narrow_layout_draws_the_column_bar_then_the_tab_bar() {
    let mut h = Harness::with_config(80, 12, two_tabs());

    h.run(vec![prs_in(0, three())]).unwrap();

    let screen = h.screen();
    let lines: Vec<&str> = screen.lines().collect();
    assert!(lines[0].starts_with("My PRs (3) │ Merge queue"), "{screen}");
    assert!(lines[2].starts_with("│mine │ review"), "{screen}");
}

#[test]
fn one_tab_draws_no_bar() {
    let mut h = Harness::new();

    h.run(vec![prs_in(0, three())]).unwrap();

    assert!(h.screen().lines().nth(1).unwrap().contains("#1 a"));
}

#[test]
fn a_configured_next_tab_key_switches_tab() {
    let mut h = Harness::with_config(160, 12, bracket_keys());
    h.run(vec![prs_in(0, three())]).unwrap();

    h.run(vec![key(KeyCode::Char(']'))]).unwrap();

    assert_eq!(h.requests, vec![Request::Refresh(Stage::Prs, 1)]);
}

#[test]
fn a_configured_previous_tab_key_switches_back() {
    let mut h = Harness::with_config(160, 12, bracket_keys());
    h.run(vec![prs_in(0, three()), key(KeyCode::Char(']'))])
        .unwrap();

    h.run(vec![key(KeyCode::Char('['))]).unwrap();

    assert_eq!(h.requests.last(), Some(&Request::Refresh(Stage::Prs, 0)));
}

#[test]
fn tab_does_nothing_once_another_key_switches_tabs() {
    let mut h = Harness::with_config(160, 12, bracket_keys());
    h.run(vec![prs_in(0, three())]).unwrap();

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    assert!(h.requests.is_empty());
}

#[test]
fn a_configured_control_chord_switches_tab() {
    let mut config = two_tabs();
    config.ui.keys.next_tab = Key::Ctrl('n');
    let mut h = Harness::with_config(160, 12, config);
    h.run(vec![prs_in(0, three())]).unwrap();

    h.run(vec![ctrl('n')]).unwrap();

    assert_eq!(h.requests, vec![Request::Refresh(Stage::Prs, 1)]);
}

#[test]
fn the_help_shows_the_configured_tab_keys() {
    let mut h = Harness::with_config(160, 20, bracket_keys());

    h.run(vec![key(KeyCode::Char('?'))]).unwrap();

    assert!(h.screen().contains("] [ 1-9"), "{}", h.screen());
}

#[test]
fn the_help_shows_the_default_tab_keys() {
    let mut h = Harness::with_size(160, 20);

    h.run(vec![key(KeyCode::Char('?'))]).unwrap();

    assert!(h.screen().contains("Tab S-Tab 1-9"), "{}", h.screen());
}

fn three_tabs() -> Config {
    let mut config = Config::default();
    config.prs.tab = vec![tab("mine"), tab("review"), tab("team")];
    config
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

fn long_tabs() -> Config {
    let mut config = Config::default();
    config.prs.tab = vec![
        tab("alpha-long-name"),
        tab("bravo-long-name"),
        tab("charlie-long-name"),
    ];
    config
}

fn bracket_keys() -> Config {
    let mut config = two_tabs();
    config.ui.keys.next_tab = Key::Char(']');
    config.ui.keys.previous_tab = Key::Char('[');
    config
}
