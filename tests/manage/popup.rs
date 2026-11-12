use conveyor::app::{Input, Request, Stage};
use conveyor::config::MergeMethod;
use conveyor::model::builds::BuildStatus;
use conveyor::model::prs::StatusChange;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{open, queueable, status, status_changed, tall};
use crate::support::{build, builds, failed, key, prs};

#[test]
fn shift_s_opens_a_manage_popup_listing_every_action_for_the_pull_request() {
    let mut h = tall();

    h.run(vec![
        prs(vec![open(1, "Retry webhooks")]),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("Manage #1"), "{screen}");
    assert!(screen.contains("m  merge"), "{screen}");
    assert!(screen.contains("u  update branch"), "{screen}");
    assert!(screen.contains("t  convert to draft"), "{screen}");
    assert!(screen.contains("x  close PR"), "{screen}");
    assert!(h.requests.is_empty());
}

#[test]
fn an_action_that_does_not_apply_is_listed_with_its_reason() {
    let mut h = tall();
    let mut draft = open(1, "a");
    draft.is_draft = true;

    h.run(vec![prs(vec![draft]), key(KeyCode::Char('S'))])
        .unwrap();

    let screen = h.screen();
    assert!(screen.contains("m  merge · #1 is a draft"), "{screen}");
    assert!(screen.contains("t  mark ready"), "{screen}");
    assert!(
        screen.contains("e  leave the merge queue · #1 is not queued"),
        "{screen}"
    );
}

#[test]
fn the_letter_of_an_action_that_does_not_apply_does_nothing() {
    let mut h = tall();
    let mut draft = open(1, "a");
    draft.is_draft = true;

    h.run(vec![
        prs(vec![draft]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
    ])
    .unwrap();

    assert!(h.screen().contains("Manage #1"), "{}", h.screen());
    assert!(h.requests.is_empty());
}

#[test]
fn t_on_a_draft_marks_it_ready_without_asking_again() {
    let mut h = tall();
    let mut draft = open(1, "a");
    draft.is_draft = true;

    h.run(vec![
        prs(vec![draft]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
    ])
    .unwrap();

    assert_eq!(h.requests, vec![status(1, "a", StatusChange::Ready)]);
    assert!(h.screen().contains("marking #1 ready…"), "{}", h.screen());
}

#[test]
fn t_on_an_open_pull_request_converts_it_to_draft() {
    let mut h = tall();

    h.run(vec![
        prs(vec![open(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
    ])
    .unwrap();

    assert_eq!(h.requests, vec![status(1, "a", StatusChange::Draft)]);
}

#[test]
fn a_queued_pull_request_cannot_become_a_draft_but_can_leave_the_queue() {
    let mut h = tall();
    let mut queued = open(1, "a");
    queued.queue_position = Some(2);

    h.run(vec![prs(vec![queued]), key(KeyCode::Char('S'))])
        .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("t  convert to draft · #1 is in the merge queue"),
        "{screen}"
    );

    h.run(vec![key(KeyCode::Char('e'))]).unwrap();

    assert_eq!(
        h.requests,
        vec![status(
            1,
            "a",
            StatusChange::Dequeue {
                node_id: "PR_node1".to_string()
            }
        )]
    );
}

#[test]
fn a_asks_before_enabling_auto_merge_because_it_merges_later_and_y_enables_it() {
    let mut h = tall();
    let mut pull = open(1, "Retry webhooks");
    pull.can_enable_auto_merge = true;

    h.run(vec![
        prs(vec![pull]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('a')),
    ])
    .unwrap();

    assert!(
        h.screen()
            .contains("enable auto-merge for #1 Retry webhooks?"),
        "{}",
        h.screen()
    );
    assert!(h.requests.is_empty(), "{:?}", h.requests);

    h.run(vec![key(KeyCode::Char('y'))]).unwrap();

    assert_eq!(
        h.requests,
        vec![status(
            1,
            "Retry webhooks",
            StatusChange::EnableAutoMerge {
                node_id: "PR_node1".to_string(),
                method: MergeMethod::Squash,
                head_oid: "8c1f2a7d1".to_string()
            }
        )]
    );
}

#[test]
fn a_disables_auto_merge_once_it_is_on() {
    let mut h = tall();
    let mut pull = open(1, "a");
    pull.auto_merge = true;
    pull.can_disable_auto_merge = true;

    h.run(vec![prs(vec![pull]), key(KeyCode::Char('S'))])
        .unwrap();

    assert!(
        h.screen().contains("a  disable auto-merge"),
        "{}",
        h.screen()
    );

    h.run(vec![key(KeyCode::Char('a'))]).unwrap();

    assert_eq!(
        h.requests,
        vec![status(
            1,
            "a",
            StatusChange::DisableAutoMerge {
                node_id: "PR_node1".to_string()
            }
        )]
    );
}

#[test]
fn auto_merge_github_does_not_allow_is_listed_with_the_reason() {
    let mut h = tall();

    h.run(vec![prs(vec![open(1, "a")]), key(KeyCode::Char('S'))])
        .unwrap();

    assert!(
        h.screen()
            .contains("a  enable auto-merge · GitHub does not allow it for #1"),
        "{}",
        h.screen()
    );
}

#[test]
fn x_asks_before_closing_and_y_closes() {
    let mut h = tall();

    h.run(vec![
        prs(vec![open(1, "Retry webhooks")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('x')),
    ])
    .unwrap();

    assert!(
        h.screen().contains("close #1 Retry webhooks?"),
        "{}",
        h.screen()
    );
    assert!(h.requests.is_empty());

    h.run(vec![key(KeyCode::Char('y'))]).unwrap();

    assert_eq!(
        h.requests,
        vec![status(1, "Retry webhooks", StatusChange::Close)]
    );
}

#[test]
fn a_habit_key_in_the_popup_cancels_instead_of_acting() {
    let mut h = tall();

    h.run(vec![
        prs(vec![open(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('q')),
    ])
    .unwrap();

    assert!(h.screen().contains("nothing changed"), "{}", h.screen());
    assert!(!h.screen().contains("Manage #1"), "{}", h.screen());

    h.run(vec![key(KeyCode::Char('S')), key(KeyCode::Char('d'))])
        .unwrap();

    assert!(!h.app.details, "d in the popup does not open the details");
    assert!(h.requests.is_empty());
}

#[test]
fn a_refresh_that_changes_the_action_under_the_letter_asks_for_s_again() {
    let mut h = tall();
    let mut draft = open(1, "a");
    draft.is_draft = true;

    h.run(vec![
        prs(vec![draft]),
        key(KeyCode::Char('S')),
        prs(vec![open(1, "a")]),
        key(KeyCode::Char('t')),
    ])
    .unwrap();

    assert!(
        h.screen()
            .contains("the pull request changed; press S again"),
        "{}",
        h.screen()
    );
    assert!(h.requests.is_empty());
}

#[test]
fn s_refuses_while_the_last_refresh_of_my_prs_failed() {
    let mut h = tall();

    h.run(vec![
        prs(vec![open(1, "a")]),
        failed(Stage::Prs, "gh: HTTP 502"),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(
        h.screen()
            .contains("the last refresh failed; press r and try again"),
        "{}",
        h.screen()
    );
    assert!(!h.screen().contains("Manage #1"), "{}", h.screen());
}

#[test]
fn s_waits_while_a_status_change_is_on_its_way() {
    let mut h = tall();
    let mut draft = open(1, "a");
    draft.is_draft = true;

    h.run(vec![
        prs(vec![draft]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(h.screen().contains("marking #1 ready…"), "{}", h.screen());
    assert!(!h.screen().contains("Manage #1"), "{}", h.screen());
}

#[test]
fn a_landed_status_change_says_so_and_refreshes_my_prs() {
    let mut h = tall();

    h.run(vec![
        prs(vec![open(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        status_changed(1, StatusChange::Draft, Ok(())),
    ])
    .unwrap();

    assert!(
        h.screen().contains("#1 converted to draft"),
        "{}",
        h.screen()
    );
    assert_eq!(h.requests[1], Request::Refresh(Stage::Prs));
    assert!(h.app.follow_up.is_some());
}

#[test]
fn leaving_the_queue_also_refreshes_the_merge_queue() {
    let mut h = tall();
    let mut queued = open(1, "a");
    queued.queue_position = Some(1);
    let change = StatusChange::Dequeue {
        node_id: "PR_node1".to_string(),
    };

    h.run(vec![
        prs(vec![queued]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('e')),
        status_changed(1, change, Ok(())),
    ])
    .unwrap();

    assert!(
        h.screen().contains("#1 left the merge queue"),
        "{}",
        h.screen()
    );
    assert_eq!(h.requests[1], Request::Refresh(Stage::Prs));
    assert_eq!(h.requests[2], Request::Refresh(Stage::Queue));
}

#[test]
fn a_refused_status_change_shows_what_gh_said_and_refreshes_nothing() {
    let mut h = tall();

    h.run(vec![
        prs(vec![open(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        status_changed(1, StatusChange::Draft, Err("not permitted")),
    ])
    .unwrap();

    assert!(h.screen().contains("#1: not permitted"), "{}", h.screen());
    assert_eq!(h.requests.len(), 1);
}

#[test]
fn s_on_main_builds_offers_only_the_rerun() {
    let mut h = tall();

    h.run(vec![
        builds(vec![build(3, BuildStatus::Failure, None)]),
        key(KeyCode::Char('3')),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("Manage run 3"), "{screen}");
    assert!(screen.contains("f  rerun failed jobs"), "{screen}");
    assert!(!screen.contains("m  merge"), "{screen}");
}

#[test]
fn s_in_the_merge_queue_and_deployed_says_there_is_nothing_to_manage() {
    let mut h = tall();

    h.run(vec![key(KeyCode::Char('2')), key(KeyCode::Char('S'))])
        .unwrap();

    assert!(
        h.screen().contains("S manages My PRs and Main builds"),
        "{}",
        h.screen()
    );
}

#[test]
fn the_old_write_keys_do_nothing() {
    let mut h = tall();

    h.run(vec![
        prs(vec![open(1, "a")]),
        key(KeyCode::Char('M')),
        key(KeyCode::Char('U')),
        key(KeyCode::Char('X')),
    ])
    .unwrap();

    assert!(!h.screen().contains("Confirm"), "{}", h.screen());
    assert!(h.requests.is_empty());
}

#[test]
fn y_refuses_a_close_when_a_refresh_of_my_prs_failed_while_the_dialog_was_open() {
    let mut h = tall();

    h.run(vec![
        prs(vec![open(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('x')),
        failed(Stage::Prs, "gh: HTTP 502"),
        key(KeyCode::Char('y')),
    ])
    .unwrap();

    assert!(
        h.screen()
            .contains("the last refresh failed; press r and try again"),
        "{}",
        h.screen()
    );
    assert!(h.requests.is_empty(), "{:?}", h.requests);
}

#[test]
fn right_after_a_merge_lands_converting_to_draft_waits_for_a_refresh() {
    let mut h = tall();

    h.run(vec![
        prs(vec![open(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
        key(KeyCode::Char('y')),
        Ok(Input::Merged(1, Ok(()))),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
    ])
    .unwrap();

    assert!(
        h.screen()
            .contains("t  convert to draft · waiting for My PRs to catch up"),
        "{}",
        h.screen()
    );
    assert!(
        !h.requests
            .iter()
            .any(|request| matches!(request, Request::ChangeStatus(_))),
        "{:?}",
        h.requests
    );
}

#[test]
fn closing_also_refreshes_the_merge_queue() {
    let mut h = tall();
    let mut queued = open(1, "a");
    queued.queue_position = Some(1);

    h.run(vec![
        prs(vec![queued]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('x')),
        key(KeyCode::Char('y')),
        status_changed(1, StatusChange::Close, Ok(())),
    ])
    .unwrap();

    assert!(h.screen().contains("#1 closed"), "{}", h.screen());
    assert_eq!(h.requests[1], Request::Refresh(Stage::Prs));
    assert_eq!(h.requests[2], Request::Refresh(Stage::Queue));
}

#[test]
fn after_auto_merge_is_enabled_update_and_draft_wait_for_my_prs_to_catch_up() {
    let mut h = tall();
    let mut pull = open(1, "a");
    pull.can_enable_auto_merge = true;
    let change = StatusChange::EnableAutoMerge {
        node_id: "PR_node1".to_string(),
        method: MergeMethod::Squash,
        head_oid: "8c1f2a7d1".to_string(),
    };

    h.run(vec![
        prs(vec![pull]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('a')),
        key(KeyCode::Char('y')),
        status_changed(1, change, Ok(())),
    ])
    .unwrap();

    assert!(
        h.screen().contains("auto-merge enabled for #1"),
        "{}",
        h.screen()
    );

    h.run(vec![key(KeyCode::Char('S'))]).unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("u  update branch · waiting for My PRs to catch up with"),
        "{screen}"
    );
    assert!(
        screen.contains("t  convert to draft · waiting for My PRs to catch up"),
        "{screen}"
    );
}

#[test]
fn marking_ready_refreshes_my_prs_but_not_the_merge_queue() {
    let mut h = tall();
    let mut draft = open(1, "a");
    draft.is_draft = true;

    h.run(vec![
        prs(vec![draft]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('t')),
        status_changed(1, StatusChange::Ready, Ok(())),
    ])
    .unwrap();

    assert_eq!(h.requests.len(), 2, "{:?}", h.requests);
    assert_eq!(h.requests[1], Request::Refresh(Stage::Prs));
}

#[test]
fn the_popup_lists_only_the_actions_without_a_hint_line() {
    let mut h = tall();

    h.run(vec![prs(vec![open(1, "a")]), key(KeyCode::Char('S'))])
        .unwrap();

    assert!(h.screen().contains("x  close PR"), "{}", h.screen());
    assert!(!h.screen().contains("a letter acts"), "{}", h.screen());
}

#[test]
fn m_on_a_merge_queue_pull_request_asks_and_y_adds_it_to_the_queue() {
    let mut h = tall();

    h.run(vec![
        prs(vec![queueable(1, "Retry webhooks")]),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(
        h.screen().contains("m  add to merge queue"),
        "{}",
        h.screen()
    );

    h.run(vec![key(KeyCode::Char('m'))]).unwrap();

    assert!(
        h.screen()
            .contains("add to the merge queue #1 Retry webhooks?"),
        "{}",
        h.screen()
    );
    assert!(h.requests.is_empty(), "{:?}", h.requests);

    h.run(vec![key(KeyCode::Char('y'))]).unwrap();

    assert_eq!(
        h.requests,
        vec![status(
            1,
            "Retry webhooks",
            StatusChange::Enqueue {
                node_id: "PR_node1".to_string(),
                head_oid: "8c1f2a7d1".to_string()
            }
        )]
    );
}

#[test]
fn a_landed_enqueue_refreshes_both_columns_and_update_waits_for_my_prs() {
    let mut h = tall();
    let change = StatusChange::Enqueue {
        node_id: "PR_node1".to_string(),
        head_oid: "8c1f2a7d1".to_string(),
    };

    h.run(vec![
        prs(vec![queueable(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
        key(KeyCode::Char('y')),
        status_changed(1, change, Ok(())),
    ])
    .unwrap();

    assert!(
        h.screen().contains("#1 added to the merge queue"),
        "{}",
        h.screen()
    );
    assert_eq!(h.requests[1], Request::Refresh(Stage::Prs));
    assert_eq!(h.requests[2], Request::Refresh(Stage::Queue));

    h.run(vec![key(KeyCode::Char('S'))]).unwrap();

    assert!(
        h.screen()
            .contains("u  update branch · waiting for My PRs to catch up with"),
        "{}",
        h.screen()
    );
}

#[test]
fn a_draft_on_a_merge_queue_repository_is_not_offered_to_the_queue() {
    let mut h = tall();
    let mut draft = queueable(1, "a");
    draft.is_draft = true;

    h.run(vec![prs(vec![draft]), key(KeyCode::Char('S'))])
        .unwrap();

    assert!(
        h.screen().contains("m  add to merge queue · #1 is a draft"),
        "{}",
        h.screen()
    );
}
