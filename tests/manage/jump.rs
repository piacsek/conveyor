use conveyor::app::{Input, Request, Stage, StatusTarget};
use conveyor::model::prs::StatusChange;
use conveyor::model::queue::QueueState;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{on_the_queue, open, tall, two_repos};
use crate::support::{Harness, entry, key, prs, queue, queue_in};

#[test]
fn s_on_a_merge_queue_row_behind_the_first_offers_to_move_it_to_the_front() {
    let mut h = tall();

    h.run(on_the_queue(vec![entry(2, 7, "alice", "Bump nodes")]))
        .unwrap();

    let screen = h.screen();
    assert!(screen.contains("i  move to the front"), "{screen}");
    assert!(!screen.contains("i  move to the front ·"), "{screen}");
}

#[test]
fn i_asks_before_jumping_and_says_the_groups_behind_restart() {
    let mut h = tall();
    let mut inputs = on_the_queue(vec![entry(2, 7, "alice", "Bump nodes")]);
    inputs.push(key(KeyCode::Char('i')));

    h.run(inputs).unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("move to the front #7 Bump nodes?"),
        "{screen}"
    );
    assert!(
        screen.contains("The merge groups behind it are rebuilt and their checks"),
        "{screen}"
    );
    assert!(screen.contains("restart."), "{screen}");
    assert!(h.requests.is_empty(), "{:?}", h.requests);
}

#[test]
fn a_narrow_jump_dialog_keeps_the_number_and_the_whole_warning() {
    let mut h = Harness::with_size(35, 30);
    let mut inputs = on_the_queue(vec![entry(2, 7, "alice", "Bump nodes")]);
    inputs.push(key(KeyCode::Char('i')));

    h.run(inputs).unwrap();

    let screen = h.screen();
    assert!(screen.contains("move to the front #7"), "{screen}");
    assert!(screen.contains("The merge groups behind it"), "{screen}");
    assert!(screen.contains("are rebuilt and their"), "{screen}");
    assert!(screen.contains("checks restart."), "{screen}");
}

#[test]
fn a_long_title_is_cut_before_the_number_or_the_warning() {
    let mut h = tall();
    let long = "Retry webhook delivery with exponential backoff and jitter across every regional worker pool we run";
    let mut inputs = on_the_queue(vec![entry(2, 7, "alice", long)]);
    inputs.push(key(KeyCode::Char('i')));

    h.run(inputs).unwrap();

    let screen = h.screen();
    assert!(screen.contains("move to the front #7 Retry"), "{screen}");
    assert!(
        screen.contains("The merge groups behind it are rebuilt and their checks"),
        "{screen}"
    );
    assert!(screen.contains("restart."), "{screen}");
}

#[test]
fn y_after_i_sends_the_jump_pinned_to_the_head() {
    let mut h = tall();
    let mut inputs = on_the_queue(vec![entry(2, 7, "alice", "Bump nodes")]);
    inputs.push(key(KeyCode::Char('i')));
    inputs.push(key(KeyCode::Char('y')));

    h.run(inputs).unwrap();

    assert_eq!(
        h.requests,
        vec![Request::ChangeStatus(jump("acme/webapp", 7, "Bump nodes"))]
    );
    assert!(
        h.screen()
            .contains("asking GitHub to move #7 to the front…"),
        "{}",
        h.screen()
    );
}

#[test]
fn the_first_entry_cannot_move_to_the_front() {
    let mut h = tall();

    h.run(on_the_queue(vec![entry(1, 7, "alice", "Bump nodes")]))
        .unwrap();
    h.run(vec![key(KeyCode::Char('i'))]).unwrap();

    assert!(
        h.screen()
            .contains("i  move to the front · already at the front"),
        "{}",
        h.screen()
    );
    assert!(h.requests.is_empty(), "{:?}", h.requests);
}

#[test]
fn an_entry_already_jumping_cannot_jump_again() {
    let mut h = tall();
    let mut jumping = entry(2, 7, "alice", "Bump nodes");
    jumping.jump = true;

    h.run(on_the_queue(vec![jumping])).unwrap();

    assert!(
        h.screen()
            .contains("i  move to the front · already jumping the queue"),
        "{}",
        h.screen()
    );
}

#[test]
fn an_entry_being_merged_cannot_move_to_the_front() {
    let mut h = tall();
    let mut locked = entry(2, 7, "alice", "Bump nodes");
    locked.state = QueueState::Locked;

    h.run(on_the_queue(vec![locked])).unwrap();

    assert!(
        h.screen()
            .contains("i  move to the front · #7 is being merged"),
        "{}",
        h.screen()
    );
}

#[test]
fn an_entry_github_sent_no_id_for_cannot_move_to_the_front() {
    let mut h = tall();
    let mut nameless = entry(2, 7, "alice", "Bump nodes");
    nameless.node_id = String::new();

    h.run(on_the_queue(vec![nameless])).unwrap();

    assert!(
        h.screen()
            .contains("i  move to the front · GitHub sent no id for"),
        "{}",
        h.screen()
    );
}

#[test]
fn an_unmergeable_entry_may_still_try_to_move_to_the_front() {
    let mut h = tall();
    let mut unmergeable = entry(2, 7, "alice", "Bump nodes");
    unmergeable.state = QueueState::Unmergeable;

    h.run(on_the_queue(vec![unmergeable])).unwrap();

    let screen = h.screen();
    assert!(screen.contains("i  move to the front"), "{screen}");
    assert!(!screen.contains("i  move to the front ·"), "{screen}");
}

#[test]
fn y_after_i_refuses_once_the_head_moved_while_the_dialog_was_open() {
    let mut h = tall();
    let mut pushed = entry(2, 7, "alice", "Bump nodes");
    pushed.head_oid = "4b9e01c3".to_string();
    let mut inputs = on_the_queue(vec![entry(2, 7, "alice", "Bump nodes")]);
    inputs.push(key(KeyCode::Char('i')));
    inputs.push(queue(vec![pushed]));
    inputs.push(key(KeyCode::Char('y')));

    h.run(inputs).unwrap();

    assert!(h.requests.is_empty(), "{:?}", h.requests);
    assert!(
        h.screen()
            .contains("the pull request changed; press S again"),
        "{}",
        h.screen()
    );
}

#[test]
fn y_after_i_refuses_once_the_entry_reached_the_front_while_the_dialog_was_open() {
    let mut h = tall();
    let mut inputs = on_the_queue(vec![entry(2, 7, "alice", "Bump nodes")]);
    inputs.push(key(KeyCode::Char('i')));
    inputs.push(queue(vec![entry(1, 7, "alice", "Bump nodes")]));
    inputs.push(key(KeyCode::Char('y')));

    h.run(inputs).unwrap();

    assert!(h.requests.is_empty(), "{:?}", h.requests);
    let footer = h.screen().lines().last().unwrap_or_default().to_string();
    assert!(footer.contains("already at the front"), "{footer}");
}

#[test]
fn a_jump_in_flight_shows_on_its_queue_card() {
    let mut h = tall();
    let mut inputs = on_the_queue(vec![entry(2, 7, "alice", "Bump nodes")]);
    inputs.push(key(KeyCode::Char('i')));
    inputs.push(key(KeyCode::Char('y')));

    h.run(inputs).unwrap();

    assert!(
        h.screen().contains("moving to the front…"),
        "{}",
        h.screen()
    );
}

#[test]
fn a_jump_that_went_through_refreshes_its_queue() {
    let mut h = tall();
    let mut inputs = on_the_queue(vec![entry(2, 7, "alice", "Bump nodes")]);
    inputs.push(key(KeyCode::Char('i')));
    inputs.push(key(KeyCode::Char('y')));
    inputs.push(jumped("acme/webapp", 7, Ok(())));

    h.run(inputs).unwrap();

    assert!(
        h.screen().contains("asked GitHub to move #7 to the front"),
        "{}",
        h.screen()
    );
    assert!(
        h.requests.contains(&Request::RefreshOnce(Stage::Queue, 0)),
        "{:?}",
        h.requests
    );
}

#[test]
fn a_jump_that_went_through_does_not_wait_for_my_prs_to_catch_up() {
    let mut h = tall();
    let mut queued = open(7, "Bump nodes");
    queued.queue_position = Some(2);
    h.run(vec![
        prs(vec![queued]),
        queue(vec![entry(2, 7, "alice", "Bump nodes")]),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('i')),
        key(KeyCode::Char('y')),
        jumped("acme/webapp", 7, Ok(())),
        key(KeyCode::Char('h')),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(!h.screen().contains("waiting for My PRs"), "{}", h.screen());
}

#[test]
fn a_refused_jump_marks_its_queue_card() {
    let mut h = tall();
    let mut inputs = on_the_queue(vec![entry(2, 7, "alice", "Bump nodes")]);
    inputs.push(key(KeyCode::Char('i')));
    inputs.push(key(KeyCode::Char('y')));
    inputs.push(jumped("acme/webapp", 7, Err("no")));

    h.run(inputs).unwrap();

    assert!(
        h.screen().contains("! move to the front failed: no"),
        "{}",
        h.screen()
    );
    assert!(
        !h.requests.contains(&Request::RefreshOnce(Stage::Queue, 0)),
        "{:?}",
        h.requests
    );
}

#[test]
fn a_jump_refreshes_the_tab_it_was_sent_from_after_a_switch() {
    let mut h = Harness::with_config(160, 20, two_repos());
    h.run(vec![
        queue_in(1, "acme/infra", vec![entry(2, 40, "bob", "Bump nodes")]),
        key(KeyCode::Char('l')),
        key(KeyCode::Tab),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('i')),
        key(KeyCode::Char('y')),
        key(KeyCode::BackTab),
    ])
    .unwrap();
    h.requests.clear();

    h.run(vec![jumped("acme/infra", 40, Ok(()))]).unwrap();

    assert!(
        h.requests.contains(&Request::RefreshOnce(Stage::Queue, 1)),
        "{:?}",
        h.requests
    );
    assert!(
        !h.requests.contains(&Request::Refresh(Stage::Queue, 1)),
        "{:?}",
        h.requests
    );
    assert!(
        !h.requests.contains(&Request::RefreshOnce(Stage::Queue, 0)),
        "{:?}",
        h.requests
    );
}

#[test]
fn a_queued_pull_request_on_my_prs_offers_no_move_to_the_front() {
    let mut h = tall();
    let mut queued = open(1, "Bump nodes");
    queued.queue_position = Some(2);

    h.run(vec![prs(vec![queued]), key(KeyCode::Char('S'))])
        .unwrap();

    assert!(
        h.screen().contains("e  leave the merge queue"),
        "{}",
        h.screen()
    );
    assert!(!h.screen().contains("move to the front"), "{}", h.screen());
}

fn jump(repo: &str, number: u64, title: &str) -> StatusTarget {
    let row = entry(2, number, "", title);
    StatusTarget {
        number,
        repo: repo.to_string(),
        title: title.to_string(),
        head_oid: row.head_oid.clone(),
        change: StatusChange::Enqueue {
            node_id: row.node_id,
            head_oid: row.head_oid,
            jump: true,
        },
    }
}

fn jumped(repo: &str, number: u64, result: Result<(), &str>) -> std::io::Result<Input> {
    Ok(Input::StatusChanged(
        jump(repo, number, ""),
        result.map_err(|err| err.to_string().into()),
    ))
}
