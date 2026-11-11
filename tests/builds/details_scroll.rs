use conveyor::model::builds::BuildStatus;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{focus_builds, line_after, many_jobs};
use crate::support::{Harness, build, builds, ctrl, jobs, key};

#[test]
fn ctrl_d_and_ctrl_u_scroll_the_details_pane_and_stop_at_both_ends() {
    let mut h = Harness::with_size(160, 30);

    h.run(long_details()).unwrap();

    let screen = h.screen();
    assert_eq!(h.app.details_page, 10);
    assert_eq!(h.app.details_scroll, 0);
    assert!(screen.contains("bob  Speed up CI"), "the head: {screen}");
    assert!(screen.contains("job 1  19s"), "{screen}");
    assert!(!screen.contains("job 24  19s"), "below the fold: {screen}");

    h.run(vec![ctrl('d')]).unwrap();
    assert_eq!(h.app.details_scroll, 5, "half a page");
    let screen = h.screen();
    assert!(
        !screen.contains("bob  Speed up CI"),
        "the head left: {screen}"
    );
    assert!(screen.contains("job 3  19s"), "{screen}");

    h.run(vec![ctrl('d')]).unwrap();
    assert_eq!(h.app.details_scroll, 10);

    h.run(vec![ctrl('d'), ctrl('d'), ctrl('d')]).unwrap();
    assert_eq!(h.app.details_scroll, 17, "the bottom holds");
    let screen = h.screen();
    assert!(screen.contains("job 24  19s"), "the last job: {screen}");
    assert!(
        line_after(&screen, "job 24  19s").starts_with("└"),
        "the last line sits on the last row, no blank tail: {screen}"
    );

    h.run(vec![ctrl('u'), ctrl('u'), ctrl('u')]).unwrap();
    assert_eq!(h.app.details_scroll, 2);

    h.run(vec![ctrl('u'), ctrl('u')]).unwrap();
    assert_eq!(h.app.details_scroll, 0, "the top holds");
    let screen = h.screen();
    assert!(
        screen.contains("bob  Speed up CI"),
        "back at the top: {screen}"
    );
}

#[test]
fn moving_the_selection_changing_focus_and_closing_the_pane_return_it_to_the_top() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = long_details();
    inputs.push(ctrl('d'));
    h.run(inputs).unwrap();
    assert_eq!(h.app.details_scroll, 5);

    h.run(vec![key(KeyCode::Char('j'))]).unwrap();
    assert_eq!(h.app.details_scroll, 0, "a selection move");

    h.run(vec![ctrl('d'), key(KeyCode::Char('h'))]).unwrap();
    assert_eq!(h.app.details_scroll, 0, "a focus change");

    h.run(vec![
        key(KeyCode::Char('l')),
        ctrl('d'),
        key(KeyCode::Char('d')),
    ])
    .unwrap();
    assert_eq!(h.app.details_scroll, 0, "closing the pane");
}

#[test]
fn a_refresh_that_drops_the_selected_run_returns_the_pane_to_the_top() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = long_details();
    inputs.push(ctrl('d'));
    h.run(inputs).unwrap();
    assert_eq!(h.app.details_scroll, 5);

    h.run(vec![builds(vec![build(
        27,
        BuildStatus::Success,
        Some((4850, "erin", "Cache the toolchain")),
    )])])
    .unwrap();

    assert_eq!(h.app.details_scroll, 0, "another run is selected now");
}

#[test]
fn the_scroll_keys_work_while_a_filter_is_being_typed_and_never_reach_the_query() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = long_details();
    inputs.push(key(KeyCode::Char('/')));
    inputs.push(key(KeyCode::Char('4')));
    inputs.push(ctrl('d'));
    inputs.push(ctrl('u'));
    inputs.push(ctrl('d'));

    h.run(inputs).unwrap();

    assert_eq!(h.app.details_scroll, 5);
    let screen = h.screen();
    assert!(screen.contains("/4▏  "), "the query is still /4: {screen}");
    assert!(!screen.contains("/4d"), "{screen}");
}

#[test]
fn the_scroll_keys_close_the_help_pane_like_any_other_key() {
    let mut h = Harness::with_size(160, 30);
    let mut inputs = long_details();
    inputs.push(key(KeyCode::Char('?')));
    h.run(inputs).unwrap();
    assert!(h.screen().contains("any key returns"), "{}", h.screen());

    h.run(vec![ctrl('d')]).unwrap();

    let screen = h.screen();
    assert!(!screen.contains("any key returns"), "{screen}");
    assert_eq!(h.app.details_scroll, 0, "the help key did not scroll");
}

#[test]
fn the_details_title_says_which_way_there_is_more_to_scroll() {
    let mut h = Harness::with_size(160, 30);

    h.run(long_details()).unwrap();
    let screen = h.screen();
    assert!(screen.contains("run 26 ↓"), "{screen}");
    assert!(!screen.contains("↑"), "nothing above yet: {screen}");

    h.run(vec![ctrl('d')]).unwrap();
    assert!(h.screen().contains("run 26 ↑↓"), "{}", h.screen());

    h.run(vec![ctrl('d'), ctrl('d'), ctrl('d'), ctrl('d')])
        .unwrap();
    let screen = h.screen();
    assert!(screen.contains("run 26 ↑"), "{screen}");
    assert!(!screen.contains("↑↓"), "nothing below any more: {screen}");
}

fn long_details() -> Vec<std::io::Result<conveyor::app::Input>> {
    let mut inputs = focus_builds();
    inputs.push(key(KeyCode::Char('d')));
    inputs.push(jobs(1026, many_jobs(24)));
    inputs.push(jobs(1025, many_jobs(24)));
    inputs
}
