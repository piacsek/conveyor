mod support;

use conveyor::app::{Input, Request, Stage, UpdateTarget};
use conveyor::config::{Config, UpdateMethod};
use conveyor::model::prs::{BranchUpdate, MergeState, PullRequest};
use ratatui::crossterm::event::KeyCode;
use support::{Harness, entry, failed, fetching, key, pr, prs, queue};

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
fn shift_u_asks_in_the_dialog_and_y_requests_the_update() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![behind(1, "Retry webhooks")]),
        key(KeyCode::Char('U')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("update branch of #1 Retry webhooks?"),
        "{screen}"
    );
    assert!(screen.contains("y confirms"), "{screen}");
    assert!(h.requests.is_empty());

    h.run(vec![key(KeyCode::Char('y'))]).unwrap();

    assert_eq!(
        h.requests,
        vec![target(1, "Retry webhooks", UpdateMethod::Merge)]
    );
    assert!(h.screen().contains("updating #1…"), "{}", h.screen());
}

#[test]
fn with_update_method_rebase_the_dialog_says_rebase_and_the_request_carries_it() {
    let mut config = Config::default();
    config.prs.update_method = UpdateMethod::Rebase;
    let mut h = Harness::with_config(160, 12, config);

    h.run(vec![prs(vec![behind(1, "a")]), key(KeyCode::Char('U'))])
        .unwrap();

    assert!(
        h.screen().contains("rebase branch of #1 a?"),
        "{}",
        h.screen()
    );

    h.run(vec![key(KeyCode::Char('y'))]).unwrap();

    assert_eq!(h.requests, vec![target(1, "a", UpdateMethod::Rebase)]);
}

#[test]
fn any_other_key_cancels_the_update() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('U')),
        key(KeyCode::Char('n')),
    ])
    .unwrap();

    assert!(h.screen().contains("update cancelled"), "{}", h.screen());
    assert!(h.requests.is_empty());
}

#[test]
fn shift_u_outside_my_prs_says_where_it_works() {
    let mut h = Harness::new();

    h.run(vec![
        queue(vec![entry(1, 7, "alice", "a")]),
        key(KeyCode::Char('2')),
        key(KeyCode::Char('U')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("U updates a branch from My PRs"),
        "{screen}"
    );
    assert!(!screen.contains("y confirms"), "{screen}");
}

#[test]
fn a_queued_pull_request_is_refused_because_a_push_would_drop_it() {
    let mut h = Harness::new();
    let mut queued = behind(1, "a");
    queued.queue_position = Some(2);

    h.run(vec![prs(vec![queued]), key(KeyCode::Char('U'))])
        .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("#1 is in the merge queue; updating it would drop it"),
        "{screen}"
    );
    assert!(!screen.contains("y confirms"), "{screen}");
}

#[test]
fn a_pull_request_github_marks_conflicting_says_so_without_a_prompt() {
    let mut h = Harness::new();
    let mut dirty = behind(1, "a");
    dirty.merge_state = MergeState::Dirty;

    h.run(vec![prs(vec![dirty]), key(KeyCode::Char('U'))])
        .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("#1 has conflicts with its base; resolve them locally"),
        "{screen}"
    );
    assert!(!screen.contains("y confirms"), "{screen}");
}

#[test]
fn a_draft_can_be_updated() {
    let mut h = Harness::new();
    let mut draft = behind(1, "a");
    draft.is_draft = true;

    h.run(vec![
        prs(vec![draft]),
        key(KeyCode::Char('U')),
        key(KeyCode::Char('y')),
    ])
    .unwrap();

    assert_eq!(h.requests, vec![target(1, "a", UpdateMethod::Merge)]);
}

#[test]
fn shift_u_refuses_while_the_last_my_prs_refresh_failed() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![behind(1, "a")]),
        failed(Stage::Prs, "gh: HTTP 502"),
        key(KeyCode::Char('U')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("the last refresh failed; press r and try again"),
        "{screen}"
    );
    assert!(!screen.contains("y confirms"), "{screen}");
}

#[test]
fn y_after_a_new_head_commit_asks_for_u_again() {
    let mut h = Harness::new();
    let mut pushed = behind(1, "a");
    pushed.head_oid = "b0b51365".to_string();

    h.run(vec![
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('U')),
        prs(vec![pushed]),
        key(KeyCode::Char('y')),
    ])
    .unwrap();

    assert!(
        h.screen()
            .contains("the pull request changed; press U again"),
        "{}",
        h.screen()
    );
    assert!(h.requests.is_empty());
}

#[test]
fn while_an_update_is_in_flight_shift_u_says_so_instead_of_asking_again() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('U')),
        key(KeyCode::Char('y')),
        key(KeyCode::Char('U')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("updating #1…"), "{screen}");
    assert!(!screen.contains("y confirms"), "{screen}");
    assert_eq!(h.requests.len(), 1);
}

fn updated(number: u64, result: Result<BranchUpdate, &str>) -> std::io::Result<Input> {
    Ok(Input::BranchUpdated(number, result.map_err(str::to_string)))
}

fn confirmed() -> Vec<std::io::Result<Input>> {
    vec![
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('U')),
        key(KeyCode::Char('y')),
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
fn a_refused_update_shows_what_gh_said_and_shift_u_asks_again() {
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

    h.run(vec![key(KeyCode::Char('U'))]).unwrap();

    assert!(
        h.screen().contains("update branch of #1 a?"),
        "{}",
        h.screen()
    );
}

#[test]
fn shift_u_waits_while_a_merge_of_the_same_pull_request_is_in_flight() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('M')),
        key(KeyCode::Char('y')),
        key(KeyCode::Char('U')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("merging #1…"), "{screen}");
    assert!(!screen.contains("y confirms"), "{screen}");
    assert_eq!(h.requests.len(), 1);
}

#[test]
fn after_a_merge_lands_shift_u_waits_for_a_refresh_that_started_after_it() {
    let mut h = Harness::new();
    let mut queued = behind(1, "a");
    queued.queue_position = Some(1);

    h.run(vec![
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('M')),
        key(KeyCode::Char('y')),
        Ok(Input::Merged(1, Ok(()))),
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('U')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("waiting for My PRs to refresh after merging #1"),
        "{screen}"
    );
    assert!(!screen.contains("y confirms"), "{screen}");

    h.run(vec![
        fetching(Stage::Prs),
        prs(vec![queued]),
        key(KeyCode::Char('U')),
    ])
    .unwrap();

    assert!(
        h.screen()
            .contains("#1 is in the merge queue; updating it would drop it"),
        "{}",
        h.screen()
    );
}

#[test]
fn after_a_refused_merge_shift_u_asks_right_away() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('M')),
        key(KeyCode::Char('y')),
        Ok(Input::Merged(1, Err("not mergeable".to_string()))),
        key(KeyCode::Char('U')),
    ])
    .unwrap();

    assert!(h.screen().contains("y confirms"), "{}", h.screen());
}

#[test]
fn shift_m_waits_while_an_update_of_the_branch_is_in_flight() {
    let mut h = Harness::new();

    h.run(vec![
        prs(vec![behind(1, "a")]),
        key(KeyCode::Char('U')),
        key(KeyCode::Char('y')),
        key(KeyCode::Char('M')),
    ])
    .unwrap();

    let screen = h.screen();
    assert!(screen.contains("updating #1…"), "{screen}");
    assert!(!screen.contains("y confirms"), "{screen}");
    assert_eq!(h.requests.len(), 1);
}
