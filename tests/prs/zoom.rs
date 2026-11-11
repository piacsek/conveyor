use ratatui::crossterm::event::KeyCode;

use crate::helpers::three;
use crate::support::{Harness, key, prs};

#[test]
fn z_zooms_the_focused_column_and_toggles_back() {
    let mut h = Harness::new();

    h.run(vec![prs(three()), key(KeyCode::Char('z'))]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("My PRs (3)"), "{screen}");
    for gone in ["Merge queue", "Main builds", "Deployed"] {
        assert!(!screen.contains(gone), "{gone} is hidden: {screen}");
    }
    assert!(
        screen.contains("#1 a"),
        "the rows are still there: {screen}"
    );
    let title = screen.lines().next().unwrap();
    assert_eq!(title.chars().count(), 160, "full width: {title}");

    h.run(vec![key(KeyCode::Char('z'))]).unwrap();
    assert!(h.screen().contains("Merge queue"), "{}", h.screen());
}

#[test]
fn a_zoomed_column_follows_the_focus_and_keeps_the_details_pane() {
    let mut h = Harness::with_size(160, 24);

    h.run(vec![
        prs(three()),
        key(KeyCode::Char('z')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('d')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("Merge queue"), "{screen}");
    assert!(!screen.contains("My PRs"), "{screen}");
    assert!(screen.contains("nothing selected"), "details: {screen}");
}

#[test]
fn esc_leaves_zoom_and_never_quits() {
    let mut h = Harness::new();

    h.run(vec![
        prs(three()),
        key(KeyCode::Char('z')),
        key(KeyCode::Esc),
    ])
    .unwrap();
    assert!(
        h.screen().contains("Merge queue"),
        "esc left zoom: {}",
        h.screen()
    );

    h.run(vec![key(KeyCode::Esc), key(KeyCode::Esc), prs(vec![])])
        .unwrap();
    assert!(
        h.screen().contains("no open pull requests"),
        "esc outside zoom is swallowed and the next input is still read: {}",
        h.screen()
    );

    h.run(vec![
        prs(three()),
        key(KeyCode::Char('z')),
        key(KeyCode::Char('z')),
    ])
    .unwrap();
    assert!(
        h.screen().contains("Merge queue"),
        "z still toggles zoom off: {}",
        h.screen()
    );
}

#[test]
fn leaving_a_filter_or_the_help_does_not_disturb_zoom() {
    let mut h = Harness::with_size(160, 16);

    h.run(vec![
        prs(three()),
        key(KeyCode::Char('z')),
        key(KeyCode::Char('/')),
        key(KeyCode::Char('a')),
        key(KeyCode::Esc),
    ])
    .unwrap();
    assert!(
        !h.screen().contains("Merge queue"),
        "esc dropped the filter and kept zoom: {}",
        h.screen()
    );

    h.run(vec![key(KeyCode::Char('?')), key(KeyCode::Esc)])
        .unwrap();
    assert!(
        !h.screen().contains("Merge queue"),
        "esc closed the help and kept zoom: {}",
        h.screen()
    );

    h.run(vec![key(KeyCode::Esc)]).unwrap();
    assert!(
        h.screen().contains("Merge queue"),
        "and now esc leaves zoom: {}",
        h.screen()
    );
}
