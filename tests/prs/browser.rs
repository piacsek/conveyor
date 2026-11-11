use ratatui::crossterm::event::KeyCode;

use crate::helpers::{highlighted_row, three};
use crate::support::{Harness, NOW, key, pr, prs, tick_at};

#[test]
fn p_opens_the_selected_pull_request_and_enter_and_o_do_nothing() {
    let mut h = Harness::new();

    h.run(vec![
        prs(three()),
        key(KeyCode::Char('j')),
        key(KeyCode::Enter),
        key(KeyCode::Char('o')),
        key(KeyCode::Char('p')),
        key(KeyCode::Char('q')),
    ])
    .unwrap();

    assert_eq!(
        h.opener.opened(),
        vec!["https://github.com/acme/webapp/pull/2".to_string()],
        "Enter and o opened nothing before p did"
    );
    assert_eq!(
        highlighted_row(&h.screen()),
        4,
        "still open on the same row"
    );
}

#[test]
fn y_copies_the_url_and_the_footer_confirms_it() {
    let mut h = Harness::new();

    h.run(vec![prs(three()), key(KeyCode::Char('y'))]).unwrap();

    assert_eq!(
        h.opener.copied(),
        vec!["https://github.com/acme/webapp/pull/1".to_string()]
    );
    let screen = h.screen();
    assert!(screen.contains("copied #1"), "{screen}");

    h.run(vec![key(KeyCode::Char('j'))]).unwrap();
    assert!(!h.screen().contains("copied #1"), "cleared by the next key");
}

#[test]
fn a_failed_open_shows_in_the_footer_and_the_app_stays_up() {
    let mut h = Harness::new();
    h.opener.fail_next("open: exec failed");

    h.run(vec![prs(three()), key(KeyCode::Char('p'))]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("open: exec failed"), "{screen}");
    assert!(screen.contains("#1 a"), "{screen}");
}

#[test]
fn a_held_p_opens_the_same_row_only_once_per_second() {
    let mut h = Harness::new();

    h.run(vec![
        prs(three()),
        key(KeyCode::Char('p')),
        key(KeyCode::Char('p')),
        key(KeyCode::Char('p')),
        tick_at(NOW + 2),
        key(KeyCode::Char('p')),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('p')),
    ])
    .unwrap();

    assert_eq!(
        h.opener.opened(),
        vec![
            "https://github.com/acme/webapp/pull/1".to_string(),
            "https://github.com/acme/webapp/pull/1".to_string(),
            "https://github.com/acme/webapp/pull/2".to_string(),
        ]
    );
}

#[test]
fn b_opens_the_check_run_behind_a_pull_request_and_says_so_when_there_is_none() {
    use conveyor::model::prs::{Check, CheckConclusion};
    let mut h = Harness::new();
    let mut checked = pr(4821, "Retry hooks");
    checked.checks_detail = vec![
        Check {
            name: "lint".to_string(),
            conclusion: CheckConclusion::Success,
            url: "https://ci/lint".to_string(),
            ..Default::default()
        },
        Check {
            name: "api / test".to_string(),
            conclusion: CheckConclusion::Failure,
            url: "https://ci/test".to_string(),
            ..Default::default()
        },
    ];

    h.run(vec![
        prs(vec![checked, pr(4830, "Rate limits")]),
        key(KeyCode::Char('b')),
    ])
    .unwrap();

    assert_eq!(
        h.opener.opened(),
        vec!["https://ci/test".to_string()],
        "the failing check wins"
    );

    h.run(vec![key(KeyCode::Char('j')), key(KeyCode::Char('b'))])
        .unwrap();
    assert_eq!(h.opener.opened().len(), 1, "no second open");
    assert!(h.screen().contains("no checks yet"), "{}", h.screen());
}

#[test]
fn b_refuses_a_check_url_that_is_not_a_web_url() {
    use conveyor::model::prs::{Check, CheckConclusion};
    let mut h = Harness::new();
    let mut checked = pr(4821, "Retry hooks");
    checked.checks_detail = vec![Check {
        name: "evil".to_string(),
        conclusion: CheckConclusion::Failure,
        url: "file:///etc/passwd".to_string(),
        ..Default::default()
    }];

    h.run(vec![prs(vec![checked]), key(KeyCode::Char('b'))])
        .unwrap();

    assert!(h.opener.opened().is_empty(), "nothing reaches the opener");
    assert!(
        h.screen().contains("refusing to open `file:///etc/passwd`"),
        "{}",
        h.screen()
    );
}
