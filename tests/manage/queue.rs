use conveyor::app::{Input, Request, Stage, StatusTarget};
use conveyor::model::prs::StatusChange;
use conveyor::model::queue::QueueState;
use conveyor::sources::failure::Failure;
use ratatui::crossterm::event::KeyCode;

use crate::helpers::{on_the_queue, open, tall, two_repos};
use crate::support::{Harness, entry, key, prs, queue, queue_in};

#[test]
fn s_on_a_merge_queue_row_offers_to_remove_it() {
    let mut h = tall();

    h.run(on_the_queue(vec![entry(1, 7, "alice", "Bump nodes")]))
        .unwrap();

    let screen = h.screen();
    assert!(screen.contains("Manage #7"), "{screen}");
    assert!(
        screen.contains("e  remove from the merge queue"),
        "{screen}"
    );
    assert!(
        !screen.contains("e  remove from the merge queue ·"),
        "{screen}"
    );
}

#[test]
fn e_asks_before_removing_and_y_removes_the_selected_entry() {
    let mut h = tall();
    let mut inputs = on_the_queue(vec![
        entry(1, 7, "alice", "Bump nodes"),
        entry(2, 9, "bob", "Retry webhooks"),
    ]);
    inputs.insert(2, key(KeyCode::Char('j')));
    h.run(inputs).unwrap();

    h.run(vec![key(KeyCode::Char('e'))]).unwrap();

    assert!(
        h.screen()
            .contains("remove from the merge queue #9 Retry webhooks?"),
        "{}",
        h.screen()
    );
    assert!(h.requests.is_empty(), "{:?}", h.requests);

    h.run(vec![key(KeyCode::Char('y'))]).unwrap();

    assert_eq!(
        h.requests,
        vec![Request::ChangeStatus(removal(
            "acme/webapp",
            9,
            "Retry webhooks"
        ))]
    );
}

#[test]
fn a_key_other_than_y_in_the_dialog_removes_nothing() {
    let mut h = tall();
    let mut inputs = on_the_queue(vec![entry(1, 7, "alice", "Bump nodes")]);
    inputs.push(key(KeyCode::Char('e')));
    inputs.push(key(KeyCode::Char('n')));

    h.run(inputs).unwrap();

    assert!(h.requests.is_empty(), "{:?}", h.requests);
    assert!(h.screen().contains("nothing changed"), "{}", h.screen());
}

#[test]
fn an_entry_being_merged_cannot_be_removed() {
    let mut h = tall();
    let mut locked = entry(1, 7, "alice", "Bump nodes");
    locked.state = QueueState::Locked;

    h.run(on_the_queue(vec![locked])).unwrap();
    h.run(vec![key(KeyCode::Char('e'))]).unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("e  remove from the merge queue · #7 is being merged"),
        "{screen}"
    );
    assert!(h.requests.is_empty(), "{:?}", h.requests);
}

#[test]
fn an_entry_github_sent_no_id_for_cannot_be_removed() {
    let mut h = tall();
    let mut nameless = entry(1, 7, "alice", "Bump nodes");
    nameless.node_id = String::new();

    h.run(on_the_queue(vec![nameless])).unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("e  remove from the merge queue · GitHub sent no id for"),
        "{screen}"
    );
}

#[test]
fn an_entry_github_sent_no_head_for_cannot_be_removed() {
    let mut h = tall();
    let mut headless = entry(1, 7, "alice", "Bump nodes");
    headless.head_oid = String::new();

    h.run(on_the_queue(vec![headless])).unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("e  remove from the merge queue · GitHub sent no id for"),
        "{screen}"
    );
}

#[test]
fn y_refuses_once_the_entry_left_the_queue_while_the_dialog_was_open() {
    let mut h = tall();
    let mut inputs = on_the_queue(vec![
        entry(1, 7, "alice", "Bump nodes"),
        entry(2, 9, "bob", "Retry webhooks"),
    ]);
    inputs.push(key(KeyCode::Char('e')));
    inputs.push(queue(vec![entry(1, 9, "bob", "Retry webhooks")]));
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
fn y_refuses_once_the_head_moved_while_the_dialog_was_open() {
    let mut h = tall();
    let mut pushed = entry(1, 7, "alice", "Bump nodes");
    pushed.head_oid = "4b9e01c3".to_string();
    let mut inputs = on_the_queue(vec![entry(1, 7, "alice", "Bump nodes")]);
    inputs.push(key(KeyCode::Char('e')));
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
fn while_the_removal_runs_s_on_its_row_repeats_its_progress() {
    let mut h = tall();
    let mut inputs = on_the_queue(vec![entry(1, 7, "alice", "Bump nodes")]);
    inputs.push(key(KeyCode::Char('e')));
    inputs.push(key(KeyCode::Char('y')));
    inputs.push(key(KeyCode::Char('S')));

    h.run(inputs).unwrap();

    let footer = h.screen().lines().last().unwrap_or_default().to_string();
    assert!(
        footer.contains("taking #7 out of the merge queue…"),
        "{footer}"
    );
    assert!(!h.screen().contains("Manage #7"), "{}", h.screen());
}

#[test]
fn a_removal_that_went_through_refreshes_my_prs_and_its_queue() {
    let mut h = tall();
    let mut inputs = on_the_queue(vec![entry(1, 7, "alice", "Bump nodes")]);
    inputs.push(key(KeyCode::Char('e')));
    inputs.push(key(KeyCode::Char('y')));
    inputs.push(removed("acme/webapp", 7, Ok(())));

    h.run(inputs).unwrap();

    assert!(
        h.screen().contains("#7 left the merge queue"),
        "{}",
        h.screen()
    );
    assert_eq!(h.requests[1], Request::Refresh(Stage::Prs, 0));
    assert_eq!(h.requests[2], Request::RefreshOnce(Stage::Queue, 0));
    assert_eq!(h.requests.len(), 3, "{:?}", h.requests);
}

#[test]
fn a_removal_refreshes_the_tab_it_was_sent_from_after_a_switch() {
    let mut h = Harness::with_config(160, 20, two_repos());
    h.run(vec![
        queue_in(1, "acme/infra", vec![entry(1, 40, "bob", "Bump nodes")]),
        key(KeyCode::Char('l')),
        key(KeyCode::Tab),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('e')),
        key(KeyCode::Char('y')),
        key(KeyCode::BackTab),
    ])
    .unwrap();
    h.requests.clear();

    h.run(vec![removed("acme/infra", 40, Ok(()))]).unwrap();

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
fn a_removal_in_flight_leaves_the_same_number_in_another_repository_free() {
    let mut h = Harness::with_config(160, 20, two_repos());
    h.run(vec![
        queue_in(
            0,
            "acme/monorepo",
            vec![entry(1, 40, "alice", "Retry webhooks")],
        ),
        queue_in(1, "acme/infra", vec![entry(1, 40, "bob", "Bump nodes")]),
        key(KeyCode::Char('l')),
        key(KeyCode::Tab),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('e')),
        key(KeyCode::Char('y')),
        key(KeyCode::BackTab),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(h.screen().contains("Manage #40"), "{}", h.screen());
}

#[test]
fn a_removal_sent_from_my_prs_keeps_the_same_pull_request_busy_on_the_queue() {
    let mut h = tall();
    let mut queued = open(7, "Bump nodes");
    queued.queue_position = Some(2);
    h.run(vec![
        prs(vec![queued]),
        queue(vec![entry(2, 7, "alice", "Bump nodes")]),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('e')),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(!h.screen().contains("Manage #7"), "{}", h.screen());
    assert_eq!(h.requests.len(), 1, "{:?}", h.requests);
}

#[test]
fn a_removal_sent_from_the_queue_keeps_the_same_pull_request_busy_on_my_prs() {
    let mut h = tall();
    let mut queued = open(7, "Bump nodes");
    queued.queue_position = Some(2);
    h.run(vec![
        prs(vec![queued]),
        queue(vec![entry(2, 7, "alice", "Bump nodes")]),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('e')),
        key(KeyCode::Char('y')),
        key(KeyCode::Char('h')),
        key(KeyCode::Char('S')),
    ])
    .unwrap();

    assert!(!h.screen().contains("Manage #7"), "{}", h.screen());
    assert_eq!(h.requests.len(), 1, "{:?}", h.requests);
}

#[test]
fn a_removal_in_flight_shows_on_its_queue_card() {
    let mut h = tall();
    let mut inputs = on_the_queue(vec![entry(1, 7, "alice", "Bump nodes")]);
    inputs.push(key(KeyCode::Char('e')));
    inputs.push(key(KeyCode::Char('y')));

    h.run(inputs).unwrap();

    assert!(
        h.screen().contains("leaving the merge queue…"),
        "{}",
        h.screen()
    );
}

#[test]
fn a_refused_removal_marks_its_queue_card_with_githubs_reason() {
    let mut h = tall();
    let mut inputs = on_the_queue(vec![entry(1, 7, "alice", "Bump nodes")]);
    inputs.push(key(KeyCode::Char('e')));
    inputs.push(key(KeyCode::Char('y')));
    inputs.push(removed("acme/webapp", 7, Err("not allowed")));

    h.run(inputs).unwrap();

    let screen = h.screen();
    assert!(
        screen.contains("! leave the merge queue failed: n"),
        "{screen}"
    );
    assert!(!screen.contains("leaving the merge queue…"), "{screen}");
    assert!(screen.contains("Queue webapp (1) !1"), "{screen}");
}

#[test]
fn a_removal_that_timed_out_marks_its_queue_card_as_maybe_out_of_the_queue() {
    let mut h = tall();
    let mut inputs = on_the_queue(vec![entry(1, 7, "alice", "Bump nodes")]);
    inputs.push(key(KeyCode::Char('e')));
    inputs.push(key(KeyCode::Char('y')));
    inputs.push(Ok(Input::StatusChanged(
        removal("acme/webapp", 7, ""),
        Err(Failure::unanswered("may be out of the queue", 60)),
    )));

    h.run(inputs).unwrap();

    assert!(
        h.screen().contains("! may be out of the queue"),
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
fn a_refused_removal_marks_the_tab_it_was_sent_from_after_a_switch() {
    let mut h = Harness::with_config(160, 20, two_repos());
    h.run(vec![
        queue_in(
            0,
            "acme/monorepo",
            vec![entry(1, 40, "alice", "Retry webhooks")],
        ),
        queue_in(1, "acme/infra", vec![entry(1, 40, "bob", "Bump nodes")]),
        key(KeyCode::Char('l')),
        key(KeyCode::Tab),
        key(KeyCode::Char('S')),
        key(KeyCode::Char('e')),
        key(KeyCode::Char('y')),
        key(KeyCode::BackTab),
        removed("acme/infra", 40, Err("not allowed")),
    ])
    .unwrap();

    assert!(!h.screen().contains("! leave"), "{}", h.screen());

    h.run(vec![key(KeyCode::Tab)]).unwrap();

    assert!(
        h.screen().contains("! leave the merge queue failed: n"),
        "{}",
        h.screen()
    );
}

#[test]
fn s_on_an_empty_queue_says_nothing_is_selected() {
    let mut h = tall();

    h.run(on_the_queue(Vec::new())).unwrap();

    let footer = h.screen().lines().last().unwrap_or_default().to_string();
    assert!(footer.contains("nothing selected"), "{footer}");
    assert!(!h.screen().contains("Manage #"), "{}", h.screen());
}

fn removal(repo: &str, number: u64, title: &str) -> StatusTarget {
    let row = entry(1, number, "", title);
    StatusTarget {
        number,
        repo: repo.to_string(),
        title: title.to_string(),
        head_oid: row.head_oid,
        change: StatusChange::Dequeue {
            node_id: row.node_id,
        },
    }
}

fn removed(repo: &str, number: u64, result: Result<(), &str>) -> std::io::Result<Input> {
    Ok(Input::StatusChanged(
        removal(repo, number, ""),
        result.map_err(|err| err.to_string().into()),
    ))
}
