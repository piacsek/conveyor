use conveyor::app::{Input, Request, Stage};
use conveyor::model::prs::{BranchUpdate, MergeState, StatusChange};
use conveyor::sources::failure::Failure;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{draft, line_after, open, queueable, status_changed, tall};
use crate::support::{NOW, fetching, key, prs, tick_at};

#[test]
fn a_timed_out_enqueue_keeps_its_mark_and_holds_update_until_my_prs_refreshes() {
    let mut h = tall();

    h.run(vec![
        prs(vec![queueable(1, "Retry webhooks")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
        key(KeyCode::Char('y')),
        timed_out_status(1, enqueue(1), "it may already be in the merge queue"),
    ])
    .unwrap();

    assert!(
        line_after(&h.screen(), "#1 Retry webhooks").contains("! add to the merge queue failed"),
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
            .failure_on(Stage::Prs, 1)
            .map(|mark| mark.message.as_str()),
        Some("gh did not answer within 60s; it may already be in the merge queue")
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
        Ok(Input::Merged(1, Err(timed_out("it may already be merged")))),
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
        timed_out_status(12, StatusChange::Ready, "it may already be marked ready"),
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
            1,
            Err::<BranchUpdate, _>(timed_out("the branch may already be updated")),
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

fn enqueue(number: u64) -> StatusChange {
    StatusChange::Enqueue {
        node_id: format!("PR_node{number}"),
        head_oid: format!("8c1f2a7d{number}"),
    }
}

fn timed_out(aftermath: &str) -> Failure {
    Failure::timed_out(format!("gh did not answer within 60s; {aftermath}"))
}

fn timed_out_status(number: u64, change: StatusChange, aftermath: &str) -> std::io::Result<Input> {
    Ok(Input::StatusChanged(
        number,
        change,
        Err(timed_out(aftermath)),
    ))
}

fn prs_refreshes(requests: &[Request]) -> usize {
    requests
        .iter()
        .filter(|request| **request == Request::Refresh(Stage::Prs, 0))
        .count()
}
