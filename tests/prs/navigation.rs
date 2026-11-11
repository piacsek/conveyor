use ratatui::crossterm::event::KeyCode;

use crate::helpers::{highlighted_row, three};
use crate::support::{Harness, key, pr, prs};

#[test]
fn j_k_arrows_gg_and_g_move_the_highlight_and_clamp() {
    let mut h = Harness::new();
    h.run(vec![
        prs(three()),
        key(KeyCode::Char('j')),
        key(KeyCode::Down),
    ])
    .unwrap();
    assert_eq!(highlighted_row(&h.screen()), 7);

    h.run(vec![key(KeyCode::Char('j')), key(KeyCode::Char('j'))])
        .unwrap();
    assert_eq!(highlighted_row(&h.screen()), 7, "clamps at the end");

    h.run(vec![
        key(KeyCode::Char('k')),
        key(KeyCode::Up),
        key(KeyCode::Up),
    ])
    .unwrap();
    assert_eq!(highlighted_row(&h.screen()), 1, "clamps at the start");

    h.run(vec![key(KeyCode::Char('G'))]).unwrap();
    assert_eq!(highlighted_row(&h.screen()), 7);

    h.run(vec![key(KeyCode::Char('g')), key(KeyCode::Char('g'))])
        .unwrap();
    assert_eq!(highlighted_row(&h.screen()), 1);
}

#[test]
fn esc_closes_the_topmost_thing_details_then_zoom_then_the_filter_and_never_quits() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![pr(1, "alpha"), pr(2, "beta"), pr(3, "delta")]),
        key(KeyCode::Char('/')),
        key(KeyCode::Char('t')),
        key(KeyCode::Enter),
        key(KeyCode::Char('z')),
        key(KeyCode::Char('d')),
    ])
    .unwrap();
    assert!(h.app.details && h.app.zoom && h.app.filter() == Some("t"));

    h.run(vec![key(KeyCode::Esc)]).unwrap();
    assert!(!h.app.details, "first Esc closes the details pane");
    assert!(
        h.app.zoom && h.app.filter() == Some("t"),
        "and nothing else"
    );

    h.run(vec![key(KeyCode::Esc)]).unwrap();
    assert!(!h.app.zoom, "second Esc leaves zoom");
    assert_eq!(h.app.filter(), Some("t"), "the filter survives");

    h.run(vec![key(KeyCode::Esc)]).unwrap();
    assert_eq!(h.app.filter(), None, "third Esc clears the filter");

    h.run(vec![key(KeyCode::Esc), prs(vec![pr(9, "still here")])])
        .unwrap();
    assert!(
        h.screen().contains("still here"),
        "a fourth Esc is a no-op: {}",
        h.screen()
    );
}

#[test]
fn digits_1_to_4_focus_a_column_in_both_layouts() {
    use conveyor::app::Stage;
    let mut wide = Harness::new();
    wide.run(vec![prs(three()), key(KeyCode::Char('4'))])
        .unwrap();
    assert_eq!(wide.app.focus, Stage::Deployed);
    wide.run(vec![key(KeyCode::Char('2'))]).unwrap();
    assert_eq!(wide.app.focus, Stage::Queue);

    let mut tabs = Harness::with_size(80, 18);
    tabs.run(vec![prs(three()), key(KeyCode::Char('3'))])
        .unwrap();
    assert_eq!(tabs.app.focus, Stage::Builds);
    assert!(tabs.screen().contains("Main builds"), "{}", tabs.screen());
    tabs.run(vec![key(KeyCode::Char('1'))]).unwrap();
    assert_eq!(tabs.app.focus, Stage::Prs);
}

#[test]
fn narrow_terminals_collapse_the_columns_into_tabs_and_h_l_switch_them() {
    let mut h = Harness::with_size(80, 12);

    h.run(vec![prs(three())]).unwrap();
    let screen = h.screen();
    assert!(
        screen.contains("My PRs (3) │ Merge queue │ Main builds │ Deployed"),
        "{screen}"
    );
    assert!(screen.contains("#1 a"), "{screen}");
    assert!(!screen.contains("fetching…"), "{screen}");

    h.run(vec![key(KeyCode::Char('l'))]).unwrap();
    let screen = h.screen();
    assert!(screen.contains("fetching…"), "the queue column: {screen}");
    assert!(!screen.contains("#1 a"), "{screen}");

    h.run(vec![key(KeyCode::Char('h')), key(KeyCode::Char('h'))])
        .unwrap();
    assert!(h.screen().contains("#1 a"), "clamps at the first column");

    h.run(vec![
        key(KeyCode::Tab),
        key(KeyCode::Tab),
        key(KeyCode::Tab),
        key(KeyCode::Tab),
    ])
    .unwrap();
    assert!(h.screen().contains("#1 a"), "Tab wraps around");
}

#[test]
fn the_tab_layout_does_not_repeat_the_column_title() {
    let mut h = Harness::with_size(80, 12);

    h.run(vec![prs(three())]).unwrap();

    let screen = h.screen();
    assert_eq!(screen.matches("My PRs (3)").count(), 1, "{screen}");
}

#[test]
fn q_quits_from_every_state_and_is_ordinary_text_in_a_filter() {
    let mut zoomed = Harness::new();
    zoomed
        .run(vec![
            prs(three()),
            key(KeyCode::Char('z')),
            key(KeyCode::Char('q')),
            prs(vec![]),
        ])
        .unwrap();
    assert!(
        !zoomed.screen().contains("no open pull requests"),
        "q quit while zoomed, leaving the input unread: {}",
        zoomed.screen()
    );

    let mut helped = Harness::with_size(160, 16);
    helped
        .run(vec![
            prs(three()),
            key(KeyCode::Char('?')),
            key(KeyCode::Char('q')),
            key(KeyCode::Char('q')),
            prs(vec![]),
        ])
        .unwrap();
    assert!(
        !helped.screen().contains("no open pull requests"),
        "the first q closes the help, the second quits: {}",
        helped.screen()
    );

    let mut filtering = Harness::new();
    filtering
        .run(vec![
            prs(three()),
            key(KeyCode::Char('/')),
            key(KeyCode::Char('q')),
            prs(vec![]),
        ])
        .unwrap();
    let screen = filtering.screen();
    assert!(
        screen.contains("/q▏  0/0"),
        "q is text in a filter: {screen}"
    );
    assert!(
        screen.contains("no open pull requests"),
        "and the app kept running, so the input after it was read: {screen}"
    );
}
