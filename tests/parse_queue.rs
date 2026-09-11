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

#[test]
fn a_merge_group_run_attaches_whole_to_the_entry_it_belongs_to() {
    use conveyor::model::builds::BuildStatus;
    use conveyor::model::queue::parse_merge_group_runs;

    let runs = parse_merge_group_runs(&serde_json::json!({"workflow_runs": [
        {"id": 4242, "run_number": 313, "status": "completed", "conclusion": "failure",
         "head_branch": "gh-readonly-queue/main/pr-4821-0000000000000000000000000000000000000000",
         "display_title": "CI/CD", "actor": {"login": "merge-bot"},
         "html_url": "https://github.com/acme/webapp/actions/runs/4242"},
        {"id": 7, "head_branch": "main", "status": "completed", "conclusion": "success"}
    ]}));

    assert_eq!(runs.len(), 1, "only merge-group branches count");
    assert_eq!(runs[0].number, 4821, "the pull request number");
    assert_eq!(runs[0].build.run_number, 313, "the run number");
    assert_eq!(runs[0].build.id, 4242);
    assert_eq!(runs[0].build.status, BuildStatus::Failure);
    assert_eq!(runs[0].build.actor, "merge-bot");

    let mut queue = parse(&fixture(include_str!("fixtures/queue.json"))).unwrap();

    queue.attach_runs(&runs);

    let attached = queue.entries[0].run.as_ref().expect("attached by number");
    assert_eq!(attached.id, 4242);
    assert_eq!(attached.status, BuildStatus::Failure);
    assert!(queue.entries[1].run.is_none(), "no run for this entry");
    assert_eq!(
        queue.entries[0].checks,
        CheckState::Pending,
        "the rollup is left alone: the card and the pane read the run itself"
    );
}

#[test]
fn a_cancelled_run_keeps_its_own_status_instead_of_becoming_a_failure() {
    use conveyor::model::builds::BuildStatus;
    use conveyor::model::queue::parse_merge_group_runs;

    let status_of = |conclusion: serde_json::Value| {
        parse_merge_group_runs(&serde_json::json!({"workflow_runs": [
            {"id": 1, "run_number": 1, "status": "completed", "conclusion": conclusion,
             "head_branch": "gh-readonly-queue/main/pr-4821-0000000000000000000000000000000000000000"}
        ]}))[0]
            .build
            .status
    };

    assert_eq!(
        status_of(serde_json::json!("cancelled")),
        BuildStatus::Cancelled,
        "a re-batched queue cancels runs; that is not a failure"
    );
    assert_eq!(
        status_of(serde_json::json!("failure")),
        BuildStatus::Failure
    );
    assert_eq!(
        status_of(serde_json::json!(null)),
        BuildStatus::Unknown,
        "completed with no conclusion is unknown, not failed"
    );
}
