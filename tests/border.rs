mod support;

use conveyor::app::Stage;
use conveyor::config::{Config, PrsTab};
use conveyor::model::prs::PullRequest;
use ratatui::crossterm::event::KeyCode;
use ratatui::style::Color;
use support::{Harness, failed, key, pr, prs};

#[test]
fn the_focused_column_has_its_border_in_the_colour_of_the_selection_bar() {
    let mut h = Harness::new();

    h.run(vec![prs(three())]).unwrap();

    assert_eq!(h.cell(1, 1).fg, Color::Cyan);
    assert_eq!(h.cell(0, 0).fg, Color::Cyan);
    assert_eq!(h.cell(30, 0).fg, Color::Cyan);
    assert_eq!(h.cell(39, 0).fg, Color::Cyan);
    assert_eq!(h.cell(0, 5).fg, Color::Cyan);
    assert_eq!(h.cell(39, 5).fg, Color::Cyan);
    assert_eq!(h.cell(0, 10).fg, Color::Cyan);
    assert_eq!(h.cell(20, 10).fg, Color::Cyan);
    assert_eq!(h.cell(39, 10).fg, Color::Cyan);
}

#[test]
fn the_focused_title_keeps_its_default_colour() {
    let mut h = Harness::new();

    h.run(vec![prs(three())]).unwrap();

    assert_eq!(h.cell(1, 0).symbol(), "M");
    assert_eq!(h.cell(1, 0).fg, Color::Reset);
}

#[test]
fn the_unfocused_columns_keep_their_border() {
    let mut h = Harness::new();

    h.run(vec![prs(three())]).unwrap();

    assert_eq!(h.cell(40, 0).fg, Color::Reset);
    assert_eq!(h.cell(70, 0).fg, Color::Reset);
    assert_eq!(h.cell(79, 5).fg, Color::Reset);
    assert_eq!(h.cell(80, 10).fg, Color::Reset);
    assert_eq!(h.cell(159, 10).fg, Color::Reset);
}

#[test]
fn the_border_colour_moves_with_the_focus() {
    let mut h = Harness::new();

    h.run(vec![prs(three()), key(KeyCode::Char('l'))]).unwrap();

    assert_eq!(h.cell(40, 0).fg, Color::Cyan);
    assert_eq!(h.cell(79, 5).fg, Color::Cyan);
    assert_eq!(h.cell(60, 10).fg, Color::Cyan);
    assert_eq!(h.cell(0, 0).fg, Color::Reset);
    assert_eq!(h.cell(39, 5).fg, Color::Reset);
    assert_eq!(h.cell(20, 10).fg, Color::Reset);
}

#[test]
fn a_focused_column_with_tabs_has_all_four_edges_in_the_colour_with_no_break_at_the_tab_bar() {
    let mut h = Harness::with_config(160, 12, two_tabs());

    h.run(vec![prs(three())]).unwrap();

    assert_eq!(h.cell(0, 0).fg, Color::Cyan);
    assert_eq!(h.cell(30, 0).fg, Color::Cyan);
    assert_eq!(h.cell(39, 0).fg, Color::Cyan);
    assert_eq!(h.cell(0, 1).fg, Color::Cyan);
    assert_eq!(h.cell(39, 1).fg, Color::Cyan);
    assert_eq!(h.cell(0, 2).fg, Color::Cyan);
    assert_eq!(h.cell(39, 2).fg, Color::Cyan);
    assert_eq!(h.cell(0, 6).fg, Color::Cyan);
    assert_eq!(h.cell(39, 6).fg, Color::Cyan);
    assert_eq!(h.cell(0, 10).fg, Color::Cyan);
    assert_eq!(h.cell(20, 10).fg, Color::Cyan);
    assert_eq!(h.cell(39, 10).fg, Color::Cyan);
}

#[test]
fn the_tab_bar_keeps_its_colours_inside_a_focused_column() {
    let mut h = Harness::with_config(160, 12, two_tabs());

    h.run(vec![prs(three())]).unwrap();

    assert_eq!(h.cell(1, 1).symbol(), "m");
    assert_eq!(h.cell(1, 1).fg, Color::Reset);
}

#[test]
fn an_unfocused_column_with_tabs_keeps_its_border() {
    let mut h = Harness::with_config(160, 12, two_tabs());

    h.run(vec![prs(three()), key(KeyCode::Char('l'))]).unwrap();

    assert_eq!(h.cell(0, 0).fg, Color::Reset);
    assert_eq!(h.cell(0, 1).fg, Color::Reset);
    assert_eq!(h.cell(0, 6).fg, Color::Reset);
    assert_eq!(h.cell(20, 10).fg, Color::Reset);
}

#[test]
fn a_loading_column_has_the_colour_when_focused() {
    let mut h = Harness::new();

    h.run(vec![]).unwrap();

    assert_eq!(h.cell(0, 5).fg, Color::Cyan);
    assert_eq!(h.cell(20, 10).fg, Color::Cyan);
}

#[test]
fn an_empty_column_has_the_colour_when_focused() {
    let mut h = Harness::new();

    h.run(vec![prs(vec![])]).unwrap();

    assert_eq!(h.cell(0, 5).fg, Color::Cyan);
    assert_eq!(h.cell(20, 10).fg, Color::Cyan);
}

#[test]
fn a_failed_column_has_the_colour_when_focused() {
    let mut h = Harness::new();

    h.run(vec![failed(Stage::Prs, "gh: boom")]).unwrap();

    assert_eq!(h.cell(0, 5).fg, Color::Cyan);
    assert_eq!(h.cell(20, 10).fg, Color::Cyan);
}

#[test]
fn a_zoomed_column_has_the_colour_across_the_whole_body() {
    let mut h = Harness::new();

    h.run(vec![
        prs(three()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('z')),
    ])
    .unwrap();

    assert_eq!(h.cell(0, 0).fg, Color::Cyan);
    assert_eq!(h.cell(100, 0).fg, Color::Cyan);
    assert_eq!(h.cell(159, 5).fg, Color::Cyan);
    assert_eq!(h.cell(120, 10).fg, Color::Cyan);
}

#[test]
fn the_narrow_layout_draws_its_one_column_in_the_colour() {
    let mut h = Harness::with_size(100, 12);

    h.run(vec![prs(three())]).unwrap();

    assert_eq!(h.cell(0, 1).fg, Color::Cyan);
    assert_eq!(h.cell(50, 1).fg, Color::Cyan);
    assert_eq!(h.cell(99, 5).fg, Color::Cyan);
    assert_eq!(h.cell(0, 10).fg, Color::Cyan);
}

#[test]
fn the_details_pane_keeps_its_border_under_a_focused_column() {
    let mut h = Harness::with_size(160, 24);

    h.run(vec![prs(three()), key(KeyCode::Char('d'))]).unwrap();

    let top = row_starting_with(&h, '┌', 1);
    assert_eq!(h.cell(0, top - 1).fg, Color::Cyan);
    assert_eq!(h.cell(0, top).fg, Color::Reset);
    assert_eq!(h.cell(0, top + 1).fg, Color::Reset);
    assert_eq!(h.cell(159, top + 1).fg, Color::Reset);
}

#[test]
fn the_s_popup_keeps_its_border_over_a_focused_column() {
    let mut h = Harness::with_size(160, 24);

    h.run(vec![
        prs(vec![mergeable(1, "a")]),
        key(KeyCode::Char('z')),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    let (x, y) = dialog_corner(&h);
    assert_eq!(h.cell(x, y).fg, Color::Reset);
    assert_eq!(h.cell(x, y + 1).fg, Color::Reset);
    assert_eq!(h.cell(0, y + 1).fg, Color::Cyan);
}

#[test]
fn the_confirm_dialog_keeps_its_border_over_a_focused_column() {
    let mut h = Harness::with_size(160, 24);

    h.run(vec![
        prs(vec![mergeable(1, "a")]),
        key(KeyCode::Char('z')),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
    ])
    .unwrap();

    assert!(h.screen().contains("Confirm"), "{}", h.screen());
    let (x, y) = dialog_corner(&h);
    assert_eq!(h.cell(x, y).fg, Color::Reset);
    assert_eq!(h.cell(x, y + 1).fg, Color::Reset);
    assert_eq!(h.cell(0, y + 1).fg, Color::Cyan);
}

#[test]
fn the_help_pane_keeps_its_border() {
    let mut h = Harness::new();

    h.run(vec![prs(three()), key(KeyCode::Char('?'))]).unwrap();

    assert_eq!(h.cell(0, 0).fg, Color::Reset);
    assert_eq!(h.cell(0, 5).fg, Color::Reset);
}

#[test]
fn the_changelog_pane_keeps_its_border() {
    let mut h = Harness::new();

    h.run(vec![prs(three()), key(KeyCode::Char('c'))]).unwrap();

    assert_eq!(h.cell(0, 0).fg, Color::Reset);
    assert_eq!(h.cell(0, 5).fg, Color::Reset);
}

fn three() -> Vec<PullRequest> {
    vec![pr(1, "a"), pr(2, "b"), pr(3, "c")]
}

fn mergeable(number: u64, title: &str) -> PullRequest {
    let mut pull = pr(number, title);
    pull.head_oid = format!("8c1f2a7d{number}");
    pull
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

fn row_starting_with(h: &Harness, corner: char, from: usize) -> u16 {
    let screen = h.screen();
    let row = screen
        .lines()
        .enumerate()
        .skip(from)
        .find(|(_, line)| line.starts_with(corner))
        .map(|(y, _)| y)
        .unwrap_or_else(|| panic!("no row starts with {corner}: {screen}"));
    u16::try_from(row).unwrap()
}

fn dialog_corner(h: &Harness) -> (u16, u16) {
    let screen = h.screen();
    screen
        .lines()
        .enumerate()
        .find_map(|(y, line)| {
            line.chars()
                .skip(1)
                .position(|c| c == '┌')
                .map(|x| (u16::try_from(x + 1).unwrap(), u16::try_from(y).unwrap()))
        })
        .unwrap_or_else(|| panic!("no dialog: {screen}"))
}
