mod support;

use conveyor::app::{Input, MergeTarget, Request, Stage};
use conveyor::model::prs::PullRequest;
use ratatui::crossterm::event::KeyCode;
use support::{Harness, ctrl, entry, key, pr, prs, queue};

fn mergeable(number: u64, title: &str) -> PullRequest {
    let mut pull = pr(number, title);
    pull.head_oid = format!("8c1f2a7d{number}");
    pull
}

fn target(number: u64, title: &str) -> Request {
    Request::Merge(MergeTarget {
        number,
        repo: "acme/webapp".to_string(),
        title: title.to_string(),
        head_oid: format!("8c1f2a7d{number}"),
    })
}

#[test]
fn shift_m_asks_in_the_footer_and_y_requests_the_merge() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![mergeable(1, "Retry webhooks")]),
        key(KeyCode::Char('M')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("merge #1 Retry webhooks? y/n"), "{screen}");
    assert!(h.requests.is_empty());

    h.run(vec![key(KeyCode::Char('y'))]).unwrap();

    assert_eq!(h.requests, vec![target(1, "Retry webhooks")]);
}

#[test]
fn any_other_key_cancels_and_leaves_the_prompt() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![mergeable(1, "a")]),
        key(KeyCode::Char('M')),
        key(KeyCode::Char('n')),
    ])
    .unwrap();

    assert!(h.screen().contains("merge cancelled"), "{}", h.screen());
    assert!(h.requests.is_empty());

    h.run(vec![key(KeyCode::Char('y'))]).unwrap();

    assert!(h.requests.is_empty());
    assert_eq!(
        h.opener.copied(),
        vec!["https://github.com/acme/webapp/pull/1".to_string()]
    );
}

#[test]
fn esc_and_ctrl_d_cancel_the_prompt_instead_of_closing_or_scrolling() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![mergeable(1, "a")]),
        key(KeyCode::Char('M')),
        key(KeyCode::Esc),
    ])
    .unwrap();

    assert!(h.screen().contains("merge cancelled"), "{}", h.screen());

    h.run(vec![key(KeyCode::Char('M')), ctrl('d')]).unwrap();

    assert!(h.screen().contains("merge cancelled"), "{}", h.screen());
    assert!(h.requests.is_empty());
}

#[test]
fn ctrl_c_still_quits_while_the_prompt_is_open() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![mergeable(1, "a")]),
        key(KeyCode::Char('M')),
        ctrl('c'),
        key(KeyCode::Char('y')),
    ])
    .unwrap();

    assert!(h.requests.is_empty());
}

#[test]
fn y_merges_the_pull_request_named_in_the_prompt_even_after_a_refresh() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![mergeable(1, "a"), mergeable(2, "b")]),
        key(KeyCode::Char('M')),
        prs(vec![mergeable(2, "b")]),
        key(KeyCode::Char('y')),
    ])
    .unwrap();

    assert_eq!(h.requests, vec![target(1, "a")]);
}

#[test]
fn a_draft_is_refused_without_a_prompt() {
    let mut h = Harness::new();
    let mut draft = mergeable(1, "a");
    draft.is_draft = true;

    h.run(vec![prs(vec![draft]), key(KeyCode::Char('M'))])
        .unwrap();

    let screen = h.screen();
    assert!(screen.contains("#1 is a draft"), "{screen}");
    assert!(!screen.contains("y/n"), "{screen}");
}

#[test]
fn a_queued_pull_request_is_refused_without_a_prompt() {
    let mut h = Harness::new();
    let mut queued = mergeable(1, "a");
    queued.queue_position = Some(1);

    h.run(vec![prs(vec![queued]), key(KeyCode::Char('M'))])
        .unwrap();

    let screen = h.screen();
    assert!(screen.contains("#1 is already queued"), "{screen}");
    assert!(!screen.contains("y/n"), "{screen}");
}

#[test]
fn shift_m_outside_my_prs_says_where_it_works() {
    let mut h = Harness::new();

    h.run(vec![
        queue(vec![entry(1, 7, "alice", "a")]),
        key(KeyCode::Char('2')),
        key(KeyCode::Char('M')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("M merges from My PRs"), "{screen}");
    assert!(!screen.contains("y/n"), "{screen}");
}

#[test]
fn shift_m_with_nothing_selected_says_so() {
    let mut h = Harness::new();

    h.run(vec![prs(vec![]), key(KeyCode::Char('M'))]).unwrap();

    assert!(h.screen().contains("nothing selected"), "{}", h.screen());
}

fn merged(number: u64, result: Result<(), &str>) -> std::io::Result<Input> {
    Ok(Input::Merged(number, result.map_err(str::to_string)))
}

#[test]
fn while_a_merge_is_in_flight_shift_m_says_so_instead_of_asking_again() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![mergeable(1, "a")]),
        key(KeyCode::Char('M')),
        key(KeyCode::Char('y')),
    ])
    .unwrap();

    assert!(h.screen().contains("merging #1…"), "{}", h.screen());

    h.run(vec![key(KeyCode::Char('M'))]).unwrap();

    let screen = h.screen();
    assert!(screen.contains("merging #1…"), "{screen}");
    assert!(!screen.contains("y/n"), "{screen}");
    assert_eq!(h.requests, vec![target(1, "a")]);
}

#[test]
fn an_accepted_merge_says_so_and_refreshes_my_prs_and_the_queue() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![mergeable(1, "a")]),
        key(KeyCode::Char('M')),
        key(KeyCode::Char('y')),
        merged(1, Ok(())),
    ])
    .unwrap();

    assert!(h.screen().contains("merge sent for #1"), "{}", h.screen());
    assert_eq!(
        h.requests,
        vec![
            target(1, "a"),
            Request::Refresh(Stage::Prs),
            Request::Refresh(Stage::Queue)
        ]
    );
}

#[test]
fn a_refused_merge_shows_what_gh_said_and_shift_m_asks_again() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![mergeable(1, "a")]),
        key(KeyCode::Char('M')),
        key(KeyCode::Char('y')),
        merged(1, Err("Pull request is not mergeable")),
    ])
    .unwrap();

    assert!(
        h.screen().contains("#1: Pull request is not mergeable"),
        "{}",
        h.screen()
    );

    h.run(vec![key(KeyCode::Char('M'))]).unwrap();

    assert!(h.screen().contains("merge #1 a? y/n"), "{}", h.screen());
}

#[test]
fn the_help_lists_shift_m_and_still_fits_seventeen_rows() {
    let mut h = Harness::with_size(160, 17);

    h.run(vec![prs(vec![mergeable(1, "a")]), key(KeyCode::Char('?'))])
        .unwrap();

    let screen = h.screen();
    assert!(screen.contains("merge the pull request (y/n)"), "{screen}");
    assert!(screen.contains("help"), "{screen}");
}
