use conveyor::app::{Input, Request, Stage, pull_key};
use conveyor::config::UpdateMethod;
use conveyor::model::prs::{BranchUpdate, MergeState, PullRequest, StatusChange};
use conveyor::sources::failure::Failure;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{draft, line_after, open, queueable, status_changed, tall};
use crate::support::{
    Harness, NOW, fetching, key, merge_target, prs, status_target, tick_at, update_target,
};

#[test]
fn a_timed_out_enqueue_keeps_its_mark_and_holds_update_until_my_prs_refreshes() {
    let mut h = tall();

    h.run(vec![
        prs(vec![queueable(1, "Retry webhooks")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
        key(KeyCode::Char('y')),
        timed_out_status(1, enqueue(1), "may be in the queue"),
    ])
    .unwrap();

    assert!(
        line_after(&h.screen(), "#1 Retry webhooks").contains("! may be in the queue: no answer"),
        "{}",
        h.screen()
    );
    assert_eq!(h.requests[1], Request::Refresh(Stage::Prs, 0));
    assert_eq!(h.requests[2], Request::Refresh(Stage::Queue, 0));

    h.run(vec![
        prs(vec![queueable(1, "Retry webhooks")]),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(
        h.screen()
            .contains("u  update branch · waiting for My PRs to catch up with"),
        "{}",
        h.screen()
    );

    h.run(vec![key(KeyCode::Char('u'))]).unwrap();

    assert_eq!(h.requests.len(), 3, "{:?}", h.requests);

    h.run(vec![
        key(KeyCode::Esc),
        fetching(Stage::Prs),
        prs(vec![queueable(1, "Retry webhooks")]),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(
        !h.screen().contains("waiting for My PRs to catch up with"),
        "{}",
        h.screen()
    );
    assert_eq!(
        h.app
            .failure_on(Stage::Prs, pull_key("acme/webapp", 1))
            .map(|mark| mark.message.as_str()),
        Some("may be in the queue: no answer in 60s")
    );
}

#[test]
fn a_timed_out_merge_reloads_both_columns_and_holds_update() {
    let mut h = tall();

    h.run(vec![
        prs(vec![open(1, "Retry webhooks")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
        key(KeyCode::Char('y')),
        Ok(Input::Merged(
            merge_target(1, ""),
            Err(timed_out("may be merged")),
        )),
        prs(vec![open(1, "Retry webhooks")]),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert_eq!(h.requests[1], Request::Refresh(Stage::Prs, 0));
    assert_eq!(h.requests[2], Request::Refresh(Stage::Queue, 0));
    assert!(
        h.screen()
            .contains("u  update branch · waiting for My PRs to catch up with"),
        "{}",
        h.screen()
    );
}

#[test]
fn a_timed_out_status_change_reloads_my_prs_now_and_ten_seconds_later() {
    let mut h = tall();

    h.run(vec![
        prs(vec![draft(12, "Retry webhooks")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        timed_out_status(12, StatusChange::Ready, "may be marked ready"),
        tick_at(NOW + 9),
    ])
    .unwrap();

    assert_eq!(prs_refreshes(&h.requests), 1);

    h.run(vec![tick_at(NOW + 10), tick_at(NOW + 30)]).unwrap();

    assert_eq!(prs_refreshes(&h.requests), 2);
}

#[test]
fn a_timed_out_branch_update_reloads_my_prs() {
    let mut h = tall();
    let mut behind = open(1, "Retry webhooks");
    behind.merge_state = MergeState::Behind;

    h.run(vec![
        prs(vec![behind]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('u')),
        Ok(Input::BranchUpdated(
            update_target(1, "", UpdateMethod::Merge),
            Err::<BranchUpdate, _>(timed_out("branch may be updated")),
        )),
    ])
    .unwrap();

    assert_eq!(prs_refreshes(&h.requests), 1);
}

#[test]
fn an_enqueue_that_failed_in_time_neither_reloads_nor_holds_update() {
    let mut h = tall();

    h.run(vec![
        prs(vec![queueable(1, "Retry webhooks")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
        key(KeyCode::Char('y')),
        status_changed(1, enqueue(1), Err("HTTP 422")),
        tick_at(NOW + 30),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert_eq!(h.requests.len(), 1, "{:?}", h.requests);
    assert!(
        !h.screen().contains("waiting for My PRs to catch up with"),
        "{}",
        h.screen()
    );
}

#[test]
fn a_timed_out_enqueue_holds_update_only_on_the_pull_request_of_its_own_repository() {
    let mut h = tall();

    h.run(vec![
        prs(same_number()),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
        key(KeyCode::Char('y')),
        timed_out_in_infra(enqueue(12), "may be in the queue"),
        prs(same_number()),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(
        h.screen().contains("waiting for My PRs to catch up with"),
        "{}",
        h.screen()
    );

    h.run(vec![
        key(KeyCode::Esc),
        key(KeyCode::Char('k')),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(
        !h.screen().contains("waiting for My PRs to catch up with"),
        "{}",
        h.screen()
    );
}

#[test]
fn a_timed_out_write_leads_its_card_with_what_may_have_happened_at_four_columns() {
    let mut h = Harness::with_size(200, 20);
    let mut queued = open(1, "Retry webhooks");
    queued.queue_position = Some(2);

    h.run(vec![
        prs(vec![queued]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('e')),
        timed_out_status(1, dequeue(1), "may be out of the queue"),
    ])
    .unwrap();

    let card = line_after(&h.screen(), "#1 Retry webhooks");
    assert!(
        card.contains("! may be out of the queue: no answer in 60s"),
        "{}",
        h.screen()
    );
    assert!(!card.contains("failed"), "{}", h.screen());
}

#[test]
fn a_timed_out_update_tells_the_footer_and_the_details_what_may_have_happened() {
    let mut h = tall();
    let mut behind = open(1, "Retry webhooks");
    behind.merge_state = MergeState::Behind;

    h.run(vec![
        prs(vec![behind]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('u')),
        Ok(Input::BranchUpdated(
            update_target(1, "", UpdateMethod::Merge),
            Err::<BranchUpdate, _>(timed_out("branch may be updated")),
        )),
    ])
    .unwrap();

    assert!(
        h.screen()
            .contains("#1: branch may be updated: no answer in 60s"),
        "{}",
        h.screen()
    );

    h.run(vec![key(KeyCode::Char('d'))]).unwrap();

    assert!(
        h.screen()
            .contains("update sent at 08:00:00: branch may be"),
        "{}",
        h.screen()
    );
    assert!(!h.screen().contains("failed"), "{}", h.screen());
}

fn enqueue(number: u64) -> StatusChange {
    StatusChange::Enqueue {
        node_id: format!("PR_node{number}"),
        head_oid: format!("8c1f2a7d{number}"),
    }
}

fn dequeue(number: u64) -> StatusChange {
    StatusChange::Dequeue {
        node_id: format!("PR_node{number}"),
    }
}

fn timed_out(aftermath: &str) -> Failure {
    Failure::timed_out(format!("{aftermath}: no answer in 60s"))
}

fn timed_out_status(number: u64, change: StatusChange, aftermath: &str) -> std::io::Result<Input> {
    Ok(Input::StatusChanged(
        status_target(number, "", change),
        Err(timed_out(aftermath)),
    ))
}

fn prs_refreshes(requests: &[Request]) -> usize {
    requests
        .iter()
        .filter(|request| **request == Request::Refresh(Stage::Prs, 0))
        .count()
}

fn timed_out_in_infra(change: StatusChange, aftermath: &str) -> std::io::Result<Input> {
    let mut target = status_target(12, "", change);
    target.repo = "acme/infra".to_string();
    Ok(Input::StatusChanged(target, Err(timed_out(aftermath))))
}

fn same_number() -> Vec<PullRequest> {
    let mut infra = queueable(12, "Drop the users table");
    infra.repo = "acme/infra".to_string();
    infra.url = "https://github.com/acme/infra/pull/12".to_string();
    vec![queueable(12, "Bump the api client"), infra]
}
