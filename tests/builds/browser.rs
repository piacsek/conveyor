use conveyor::model::builds::BuildStatus;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::three;
use crate::support::{Harness, build, builds, key};

#[test]
fn p_opens_the_pull_request_b_opens_the_run_and_d_shows_its_details() {
    let mut h = Harness::with_size(160, 20);

    h.run(vec![
        builds(three()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('p')),
        key(KeyCode::Char('b')),
        key(KeyCode::Char('d')),
    ])
    .unwrap();

    assert_eq!(
        h.opener.opened(),
        vec![
            "https://github.com/acme/webapp/pull/4821".to_string(),
            "https://github.com/acme/webapp/actions/runs/1025".to_string(),
        ]
    );
    let screen = h.screen();
    assert!(screen.contains("run 25  failure  3m48s"), "{screen}");
    assert!(screen.contains("#4821 alice  Retry hooks"), "{screen}");
    assert!(
        !screen.contains("https://"),
        "no URLs in the pane: {screen}"
    );
    assert!(screen.contains("started at 06:00:00"), "{screen}");
}

#[test]
fn p_says_so_when_no_pull_request_is_known_and_b_still_opens_the_run() {
    let mut h = Harness::new();

    h.run(vec![
        builds(three()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('G')),
        key(KeyCode::Char('p')),
    ])
    .unwrap();

    assert!(
        h.screen().contains("no pull request for this run"),
        "{}",
        h.screen()
    );
    assert!(h.opener.opened().is_empty(), "p opened nothing");

    h.run(vec![key(KeyCode::Char('b'))]).unwrap();

    assert_eq!(
        h.opener.opened(),
        vec!["https://github.com/acme/webapp/actions/runs/1024".to_string()]
    );
}

#[test]
fn y_copies_the_pull_request_url_and_shift_y_the_build_url() {
    let mut h = Harness::new();

    h.run(vec![
        builds(three()),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('y')),
        key(KeyCode::Char('Y')),
    ])
    .unwrap();

    assert_eq!(
        h.opener.copied(),
        vec![
            "https://github.com/acme/webapp/pull/4821".to_string(),
            "https://github.com/acme/webapp/actions/runs/1025".to_string(),
        ]
    );
    assert!(
        h.screen().contains("copied the build URL"),
        "{}",
        h.screen()
    );
}

#[test]
fn p_falls_back_to_the_squash_number_while_the_pull_request_is_unassociated() {
    let mut h = Harness::new();
    let mut pending = build(26, BuildStatus::Success, Some((4840, "bob", "Speed up CI")));
    pending.pull = None;

    h.run(vec![
        builds(vec![pending]),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('p')),
    ])
    .unwrap();

    assert_eq!(
        h.opener.opened(),
        vec!["https://github.com/acme/webapp/pull/4840".to_string()],
        "the card shows #4840, so p opens #4840"
    );
}
