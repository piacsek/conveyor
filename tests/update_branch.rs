mod support;

use conveyor::app::{Input, Request, Stage, UpdateTarget};
use conveyor::config::{Config, UpdateMethod};
use conveyor::model::prs::{BranchUpdate, MergeState, PullRequest};
use ratatui::crossterm::event::KeyCode;
use support::{Harness, NOW, entry, failed, fetching, key, pr, prs, queue, tick_at};

fn behind(number: u64, title: &str) -> PullRequest {
    let mut pull = pr(number, title);
    pull.head_oid = format!("8c1f2a7d{number}");
    pull.merge_state = MergeState::Behind;
    pull
}

fn target(number: u64, title: &str, method: UpdateMethod) -> Request {
    Request::UpdateBranch(UpdateTarget {
        number,
        repo: "acme/webapp".to_string(),
        title: title.to_string(),
        head_oid: format!("8c1f2a7d{number}"),
        method,
    })
}

#[test]
fn s_then_u_updates_the_branch_at_once() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![behind(1, "Retry webhooks")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('u')),
    ])
    .unwrap();

    assert_eq!(
        h.requests,
        vec![target(1, "Retry webhooks", UpdateMethod::Merge)]
    );
    assert!(h.screen().contains("updating #1…"), "{}", h.screen());
    assert!(!h.screen().contains("y confirms"), "{}", h.screen());
}

#[test]
fn with_update_method_rebase_u_asks_first_and_the_request_carries_it() {
    let mut h = Harness::with_config(160, 12, rebasing());

    h.run(vec![
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('u')),
    ])
    .unwrap();

    assert!(
        h.screen().contains("rebase branch of #1 a?"),
        "{}",
        h.screen()
    );
    assert!(h.requests.is_empty());

    h.run(vec![key(KeyCode::Char('y'))]).unwrap();

    assert_eq!(h.requests, vec![target(1, "a", UpdateMethod::Rebase)]);
}

fn rebasing() -> Config {
    let mut config = Config::default();
    config.prs.update_method = UpdateMethod::Rebase;
    config
}

#[test]
fn any_other_key_cancels_a_rebase() {
    let mut h = Harness::with_config(160, 12, rebasing());

    h.run(vec![
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('u')),
        key(KeyCode::Char('n')),
    ])
    .unwrap();

    assert!(h.screen().contains("update cancelled"), "{}", h.screen());
    assert!(h.requests.is_empty());
}

#[test]
fn s_in_the_merge_queue_says_where_it_works() {
    let mut h = Harness::new();

    h.run(vec![
        queue(vec![entry(1, 7, "alice", "a")]),
        key(KeyCode::Char('2')),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("S manages My PRs and Main builds"),
        "{screen}"
    );
}

#[test]
fn a_queued_pull_request_lists_the_update_as_refused_because_a_push_would_drop_it() {
    let mut h = Harness::new();
    let mut queued = behind(1, "a");
    queued.queue_position = Some(2);

    h.run(vec![
        prs(vec![queued]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('u')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("u  update branch · #1 is in the merge queue; updating it"),
        "{screen}"
    );
    assert!(screen.contains("would drop it"), "{screen}");
    assert!(h.requests.is_empty());
}

#[test]
fn a_pull_request_github_marks_conflicting_lists_the_update_as_refused() {
    let mut h = Harness::new();
    let mut dirty = behind(1, "a");
    dirty.merge_state = MergeState::Dirty;

    h.run(vec![
        prs(vec![dirty]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('u')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("u  update branch · #1 has conflicts with its base;"),
        "{screen}"
    );
    assert!(screen.contains("   resolve them locally"), "{screen}");
    assert!(h.requests.is_empty());
}

#[test]
fn a_draft_can_be_updated() {
    let mut h = Harness::new();
    let mut draft = behind(1, "a");
    draft.is_draft = true;

    h.run(vec![
        prs(vec![draft]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('u')),
    ])
    .unwrap();

    assert_eq!(h.requests, vec![target(1, "a", UpdateMethod::Merge)]);
}

#[test]
fn s_refuses_while_the_last_my_prs_refresh_failed() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![behind(1, "a")]),
        failed(Stage::Prs, "gh: HTTP 502"),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("the last refresh failed; press r and try again"),
        "{screen}"
    );
    assert!(!screen.contains("Manage #1"), "{screen}");
}

#[test]
fn u_after_a_new_head_commit_asks_for_s_again() {
    let mut h = Harness::new();
    let mut pushed = behind(1, "a");
    pushed.head_oid = "b0b51365".to_string();

    h.run(vec![
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('S')),
        prs(vec![pushed]),
        key(KeyCode::Char('u')),
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
fn while_an_update_is_in_flight_s_says_so_instead_of_opening() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('u')),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("updating #1…"), "{screen}");
    assert!(!screen.contains("Manage #1"), "{screen}");
    assert_eq!(h.requests.len(), 1);
}

fn updated(number: u64, result: Result<BranchUpdate, &str>) -> std::io::Result<Input> {
    Ok(Input::BranchUpdated(number, result.map_err(str::to_string)))
}

fn confirmed() -> Vec<std::io::Result<Input>> {
    vec![
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('u')),
    ]
}

#[test]
fn an_updated_branch_says_so_and_refreshes_my_prs() {
    let mut h = Harness::new();
    let mut inputs = confirmed();
    inputs.push(updated(1, Ok(BranchUpdate::Updated)));

    h.run(inputs).unwrap();

    assert!(
        h.screen().contains("branch of #1 updated"),
        "{}",
        h.screen()
    );
    assert_eq!(h.requests.len(), 2);
    assert_eq!(h.requests[1], Request::Refresh(Stage::Prs));
}

#[test]
fn an_up_to_date_branch_says_so_and_refreshes_nothing() {
    let mut h = Harness::new();
    let mut inputs = confirmed();
    inputs.push(updated(1, Ok(BranchUpdate::UpToDate)));

    h.run(inputs).unwrap();

    assert!(
        h.screen().contains("#1 is already up to date"),
        "{}",
        h.screen()
    );
    assert_eq!(h.requests.len(), 1);
}

#[test]
fn conflicts_found_by_gh_are_a_plain_message() {
    let mut h = Harness::new();
    let mut inputs = confirmed();
    inputs.push(updated(1, Ok(BranchUpdate::Conflicts)));

    h.run(inputs).unwrap();

    assert!(
        h.screen()
            .contains("#1 has conflicts with its base; resolve them locally"),
        "{}",
        h.screen()
    );
    assert_eq!(h.requests.len(), 1);
}

#[test]
fn a_refused_update_shows_what_gh_said_and_s_offers_it_again() {
    let mut h = Harness::new();
    let mut inputs = confirmed();
    inputs.push(updated(1, Err("GraphQL: Resource not accessible")));

    h.run(inputs).unwrap();

    assert!(
        h.screen()
            .contains("update failed for #1: GraphQL: Resource not accessible"),
        "{}",
        h.screen()
    );

    h.run(vec![key(KeyCode::Char('S'))]).unwrap();

    assert!(h.screen().contains("u  update branch"), "{}", h.screen());
    assert!(!h.screen().contains("u  update branch ·"), "{}", h.screen());
}

#[test]
fn s_waits_while_a_merge_is_in_flight() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
        key(KeyCode::Char('y')),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("merging #1…"), "{screen}");
    assert!(!screen.contains("Manage #1"), "{screen}");
    assert_eq!(h.requests.len(), 1);
}

#[test]
fn after_a_merge_lands_the_update_waits_for_a_refresh_that_started_after_it() {
    let mut h = Harness::new();
    let mut queued = behind(1, "a");
    queued.queue_position = Some(1);

    h.run(vec![
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
        key(KeyCode::Char('y')),
        Ok(Input::Merged(1, Ok(()))),
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('u')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("u  update branch · waiting for My PRs to refresh after"),
        "{screen}"
    );
    assert!(
        !h.requests
            .iter()
            .any(|request| matches!(request, Request::UpdateBranch(_))),
        "{:?}",
        h.requests
    );

    h.run(vec![
        key(KeyCode::Esc),
        fetching(Stage::Prs),
        prs(vec![queued]),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(
        h.screen()
            .contains("u  update branch · #1 is in the merge queue; updating it"),
        "{}",
        h.screen()
    );
}

#[test]
fn after_a_refused_merge_u_updates_right_away() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
        key(KeyCode::Char('y')),
        Ok(Input::Merged(1, Err("not mergeable".to_string()))),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('u')),
    ])
    .unwrap();

    assert_eq!(h.requests.len(), 2);
    assert_eq!(h.requests[1], target(1, "a", UpdateMethod::Merge));
}

#[test]
fn s_waits_while_an_update_of_the_branch_is_in_flight() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('u')),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('m')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(!screen.contains("y confirms"), "{screen}");
    assert_eq!(h.requests.len(), 1);
}

fn prs_refreshes(requests: &[Request]) -> usize {
    requests
        .iter()
        .filter(|request| **request == Request::Refresh(Stage::Prs))
        .count()
}

#[test]
fn an_updated_branch_reloads_my_prs_once_more_ten_seconds_later() {
    let mut h = Harness::new();
    let mut inputs = confirmed();
    inputs.push(updated(1, Ok(BranchUpdate::Updated)));
    inputs.push(tick_at(NOW + 9));

    h.run(inputs).unwrap();

    assert_eq!(prs_refreshes(&h.requests), 1);

    h.run(vec![
        tick_at(NOW + 10),
        tick_at(NOW + 11),
        tick_at(NOW + 30),
    ])
    .unwrap();

    assert_eq!(prs_refreshes(&h.requests), 2);
    assert_eq!(h.requests.last(), Some(&Request::Refresh(Stage::Prs)));
}

#[test]
fn an_up_to_date_branch_schedules_no_second_reload() {
    let mut h = Harness::new();
    let mut inputs = confirmed();
    inputs.push(updated(1, Ok(BranchUpdate::UpToDate)));
    inputs.push(tick_at(NOW + 30));

    h.run(inputs).unwrap();

    assert_eq!(prs_refreshes(&h.requests), 0);
}

#[test]
fn conflicts_and_a_refused_update_schedule_no_reload() {
    let mut h = Harness::new();
    let mut inputs = confirmed();
    inputs.push(updated(1, Ok(BranchUpdate::Conflicts)));
    inputs.push(key(KeyCode::Char('S')));
    inputs.push(key(KeyCode::Char('u')));
    inputs.push(updated(1, Err("HTTP 403")));
    inputs.push(tick_at(NOW + 30));

    h.run(inputs).unwrap();

    assert_eq!(prs_refreshes(&h.requests), 0);
}

#[test]
fn a_second_update_before_the_follow_up_moves_it_to_ten_seconds_after_the_second() {
    let mut h = Harness::new();
    let mut inputs = confirmed();
    inputs.push(updated(1, Ok(BranchUpdate::Updated)));
    inputs.push(tick_at(NOW + 5));
    inputs.push(key(KeyCode::Char('S')));
    inputs.push(key(KeyCode::Char('u')));
    inputs.push(updated(1, Ok(BranchUpdate::Updated)));
    inputs.push(tick_at(NOW + 14));

    h.run(inputs).unwrap();

    assert_eq!(prs_refreshes(&h.requests), 2);

    h.run(vec![tick_at(NOW + 15), tick_at(NOW + 40)]).unwrap();

    assert_eq!(prs_refreshes(&h.requests), 3);
}

#[test]
fn y_refuses_a_rebase_when_a_refresh_of_my_prs_failed_while_the_dialog_was_open() {
    let mut h = Harness::with_config(160, 12, rebasing());

    h.run(vec![
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('u')),
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
