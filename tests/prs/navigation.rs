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

#[test]
fn esc_closes_the_details_pane_before_it_clears_the_mark_of_the_selected_card() {
    let mut h = Harness::with_size(160, 24);
    h.run(marked_first(vec![key(KeyCode::Char('d'))])).unwrap();

    h.run(vec![key(KeyCode::Esc)]).unwrap();

    assert!(!h.app.details, "first Esc closes the details pane");
    assert!(h.screen().contains("! mark ready failed"), "{}", h.screen());

    h.run(vec![key(KeyCode::Esc)]).unwrap();

    assert!(
        !h.screen().contains("! mark ready failed"),
        "{}",
        h.screen()
    );
    assert!(h.screen().contains("┌My PRs (2)─"), "{}", h.screen());
}

#[test]
fn esc_clears_the_mark_of_the_selected_card_before_the_live_filter() {
    let mut h = Harness::with_size(160, 24);
    h.run(marked_first(vec![
        key(KeyCode::Char('/')),
        key(KeyCode::Char('r')),
        key(KeyCode::Enter),
    ]))
    .unwrap();

    h.run(vec![key(KeyCode::Esc)]).unwrap();

    assert!(
        !h.screen().contains("! mark ready failed"),
        "{}",
        h.screen()
    );
    assert_eq!(h.app.filter(), Some("r"));

    h.run(vec![key(KeyCode::Esc)]).unwrap();

    assert_eq!(h.app.filter(), None);
}

#[test]
fn esc_on_an_unmarked_card_leaves_the_other_marks_alone() {
    let mut h = Harness::with_size(160, 24);
    h.run(marked_first(vec![
        key(KeyCode::Char('/')),
        key(KeyCode::Char('z')),
        key(KeyCode::Enter),
    ]))
    .unwrap();

    h.run(vec![key(KeyCode::Esc)]).unwrap();

    assert_eq!(h.app.filter(), None);
    assert!(h.screen().contains("! mark ready failed"), "{}", h.screen());
}

fn marked_first(
    then: Vec<std::io::Result<conveyor::app::Input>>,
) -> Vec<std::io::Result<conveyor::app::Input>> {
    let mut drafts = vec![pr(1, "Retry"), pr(2, "Zap")];
    for draft in &mut drafts {
        draft.is_draft = true;
    }
    let mut inputs = vec![
        prs(drafts),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        Ok(conveyor::app::Input::StatusChanged(
            1,
            conveyor::model::prs::StatusChange::Ready,
            Err("HTTP 422".to_string()),
        )),
    ];
    inputs.extend(then);
    inputs
}
