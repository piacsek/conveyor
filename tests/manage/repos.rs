use conveyor::app::{Input, Mode, Request};
use conveyor::config::{Config, MergeMethod, Repo};
use conveyor::model::prs::{PullRequest, StatusChange};
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{draft, line_after, open, status, status_changed, tall};
use crate::support::{Harness, key, merge_target, prs, status_target};

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

#[test]
fn the_popup_names_the_repository_when_the_tab_holds_more_than_one() {
    let mut h = tall();

    h.run(vec![
        prs(same_number()),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("Manage infra#12"), "{screen}");
}

#[test]
fn the_confirm_dialog_names_the_repository_when_the_tab_holds_more_than_one() {
    let mut h = tall();

    h.run(vec![
        prs(same_number()),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("merge infra#12 Drop the users table?"),
        "{screen}"
    );
}

#[test]
fn the_popup_names_the_owner_when_two_repositories_share_a_name() {
    let mut h = tall();

    h.run(vec![
        prs(same_name()),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("Manage me/webapp#12"), "{screen}");
}

#[test]
fn the_confirm_dialog_names_the_owner_when_two_repositories_share_a_name() {
    let mut h = tall();

    h.run(vec![
        prs(same_name()),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("merge me/webapp#12 Drop the users table?"),
        "{screen}"
    );
}

#[test]
fn a_refresh_that_brings_another_repository_leaves_the_open_popup_acting() {
    let mut h = tall();

    h.run(vec![
        prs(vec![open(12, "Bump the api client")]),
        key(KeyCode::Char('S')),
        prs(vec![
            open(12, "Bump the api client"),
            in_infra(open(7, "Drop the users table")),
        ]),
        key(KeyCode::Char('t')),
    ])
    .unwrap();

    assert_eq!(
        h.requests,
        vec![status(12, "Bump the api client", StatusChange::Draft)]
    );
}

#[test]
fn a_result_for_one_repository_leaves_the_same_write_on_the_other_busy() {
    let mut h = tall();

    h.run(vec![
        prs(same_draft()),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        infra_ready_failed(),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        line_after(&screen, "#12 Drop the users table").contains("! mark ready failed: HTTP 422"),
        "{screen}"
    );
    assert!(
        line_after(&screen, "#12 Retry webhooks").contains("marking ready…"),
        "{screen}"
    );
}

#[test]
fn a_confirmed_merge_goes_to_the_pull_request_of_the_other_repository() {
    let mut h = tall();

    h.run(vec![
        prs(same_number()),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
        key(KeyCode::Char('y')),
    ])
    .unwrap();

    let merged = h.requests.iter().find_map(|request| match request {
        Request::Merge(target) => Some(target.repo.clone()),
        _ => None,
    });
    assert_eq!(merged.as_deref(), Some("acme/infra"));
}

#[test]
fn a_confirmed_merge_uses_the_merge_method_of_its_repository() {
    let mut h = Harness::with_config(160, 20, infra_rebases());

    h.run(vec![
        prs(same_number()),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
        key(KeyCode::Char('y')),
    ])
    .unwrap();

    assert_eq!(merged_with(&h), Some(MergeMethod::Rebase));
}

#[test]
fn a_confirmed_merge_in_a_repository_without_its_own_method_uses_the_prs_one() {
    let mut h = Harness::with_config(160, 20, infra_rebases());

    h.run(vec![
        prs(same_number()),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
        key(KeyCode::Char('y')),
    ])
    .unwrap();

    assert_eq!(merged_with(&h), Some(MergeMethod::Squash));
}

fn infra_rebases() -> Config {
    Config {
        repo: vec![Repo {
            name: "acme/infra".to_string(),
            merge_method: Some(MergeMethod::Rebase),
            ..Repo::default()
        }],
        ..Config::default()
    }
}

fn merged_with(h: &Harness) -> Option<MergeMethod> {
    h.requests.iter().find_map(|request| match request {
        Request::Merge(target) => Some(target.method),
        _ => None,
    })
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

fn same_name() -> Vec<PullRequest> {
    vec![
        open(12, "Bump the api client"),
        in_fork(open(12, "Drop the users table")),
    ]
}

fn in_fork(mut pull: PullRequest) -> PullRequest {
    pull.repo = "me/webapp".to_string();
    pull.url = format!("https://github.com/me/webapp/pull/{}", pull.number);
    pull
}

fn infra_ready_failed() -> std::io::Result<Input> {
    let mut target = status_target(12, "", StatusChange::Ready);
    target.repo = "acme/infra".to_string();
    Ok(Input::StatusChanged(
        target,
        Err("HTTP 422".to_string().into()),
    ))
}
