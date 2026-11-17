use conveyor::app::{Input, Mode};
use conveyor::model::prs::{PullRequest, StatusChange};
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{draft, line_after, open, status_changed, tall};
use crate::support::{key, merge_target, prs};

#[test]
fn a_refresh_keeps_the_selection_on_the_pull_request_of_the_other_repository() {
    let mut h = tall();

    h.run(vec![
        prs(same_number()),
        key(KeyCode::Char('j')),
        prs(same_number()),
    ])
    .unwrap();

    let selected = h.app.prs.column().selected().unwrap();
    assert_eq!(selected.repo, "acme/infra");
}

#[test]
fn a_write_in_flight_on_one_repository_leaves_the_popup_of_the_other_open() {
    let mut h = tall();

    h.run(vec![
        prs(same_draft()),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(matches!(h.app.mode, Mode::Manage(_)), "{}", h.screen());
}

#[test]
fn a_failed_write_marks_only_the_card_of_its_own_repository() {
    let mut h = tall();

    h.run(vec![
        prs(same_draft()),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        status_changed(12, StatusChange::Ready, Err("HTTP 422")),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        line_after(&screen, "#12 Retry webhooks").contains("! mark ready failed: HTTP 422"),
        "{screen}"
    );
    assert!(
        !line_after(&screen, "#12 Drop the users table").contains("failed"),
        "{screen}"
    );
}

#[test]
fn a_merge_sent_in_one_repository_leaves_the_other_free_to_change() {
    let mut h = tall();

    h.run(vec![
        prs(same_number()),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
        key(KeyCode::Char('y')),
        Ok(Input::Merged(merge_target(12, ""), Ok(()))),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("t  convert to draft"), "{screen}");
    assert!(!screen.contains("waiting for My PRs"), "{screen}");
}

fn same_draft() -> Vec<PullRequest> {
    vec![
        draft(12, "Retry webhooks"),
        in_infra(draft(12, "Drop the users table")),
    ]
}

fn same_number() -> Vec<PullRequest> {
    vec![
        open(12, "Bump the api client"),
        in_infra(open(12, "Drop the users table")),
    ]
}

fn in_infra(mut pull: PullRequest) -> PullRequest {
    pull.repo = "acme/infra".to_string();
    pull.url = format!("https://github.com/acme/infra/pull/{}", pull.number);
    pull
}
