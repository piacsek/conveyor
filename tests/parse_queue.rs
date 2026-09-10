use std::time::{Duration, UNIX_EPOCH};

use conveyor::model::prs::CheckState;
use conveyor::model::queue::{QueueState, parse};

fn fixture(text: &str) -> serde_json::Value {
    serde_json::from_str(text).unwrap()
}

#[test]
fn parses_the_queue_entries_in_position_order() {
    let queue = parse(&fixture(include_str!("fixtures/queue.json"))).unwrap();

    assert_eq!(queue.url, "https://github.com/acme/webapp/queue/main");
    assert_eq!(queue.entries.len(), 2);
    let first = &queue.entries[0];
    assert_eq!(first.position, 1);
    assert_eq!(first.state, QueueState::AwaitingChecks);
    assert_eq!(first.number, 4821);
    assert_eq!(first.title, "Retry webhook delivery with backoff");
    assert_eq!(first.author, "alice");
    assert_eq!(first.url, "https://github.com/acme/webapp/pull/4821");
    assert_eq!(first.head_sha, "6babeb79d61cce5208232c669505ea4b42300c6d");
    assert_eq!(first.checks, CheckState::Pending);
    assert_eq!(first.eta, Some(Duration::from_secs(720)));
    assert_eq!(
        first.enqueued_at,
        Some(UNIX_EPOCH + Duration::from_secs(1_789_052_400))
    );
    assert!(!first.jump);
    let second = &queue.entries[1];
    assert_eq!(second.state, QueueState::Queued);
    assert_eq!(second.checks, CheckState::None);
    assert_eq!(second.eta, None);
    assert!(second.jump);
}

#[test]
fn an_empty_queue_parses_to_no_entries() {
    let queue = parse(&fixture(include_str!("fixtures/queue-empty.json"))).unwrap();
    assert!(queue.entries.is_empty());
}

#[test]
fn a_repository_without_a_merge_queue_is_an_error_naming_it() {
    let err = parse(&fixture(r#"{"data":{"repository":{"mergeQueue":null}}}"#)).unwrap_err();
    assert!(err.contains("no merge queue"), "{err}");
}

#[test]
fn unknown_states_degrade_to_unknown() {
    let queue = parse(&fixture(
        r#"{"data":{"repository":{"mergeQueue":{"url":"u","entries":{"nodes":[
            {"position":1,"state":"SOMETHING_NEW","pullRequest":{"number":1,"title":"t","author":null}}]}}}}}"#,
    ))
    .unwrap();
    assert_eq!(queue.entries[0].state, QueueState::Unknown);
    assert_eq!(queue.entries[0].author, "");
}
