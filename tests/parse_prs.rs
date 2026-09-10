use conveyor::model::prs::{CheckConclusion, CheckState, MergeState, ReviewDecision, parse};
use std::time::{Duration, UNIX_EPOCH};

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/prs.json")).unwrap()
}

#[test]
fn parses_the_verbatim_search_response() {
    let prs = parse(&fixture()).unwrap();

    assert_eq!(prs.len(), 1);
    let pr = &prs[0];
    assert_eq!(pr.number, 1);
    assert_eq!(pr.title, "Phase 0: setup");
    assert_eq!(pr.url, "https://github.com/piacsek/conveyor/pull/1");
    assert_eq!(pr.repo, "piacsek/conveyor");
    assert_eq!(pr.head_ref, "phase-0-setup");
    assert!(!pr.is_draft);
    assert_eq!(pr.checks, CheckState::Success);
    assert_eq!(pr.review, ReviewDecision::None);
    assert_eq!(pr.merge_state, MergeState::Unknown);
    assert_eq!((pr.additions, pr.deletions), (1007, 1));
    assert_eq!(
        pr.updated_at,
        Some(UNIX_EPOCH + Duration::from_secs(1_789_053_943))
    );
    assert_eq!(pr.checks_detail.len(), 3);
    assert_eq!(pr.checks_detail[0].name, "check");
    assert_eq!(pr.checks_detail[0].conclusion, CheckConclusion::Success);
    assert!(pr.checks_detail[0].url.contains("/job/102931850230"));
}

fn one(node: &str) -> conveyor::model::prs::PullRequest {
    let value: serde_json::Value =
        serde_json::from_str(&format!(r#"{{"data":{{"search":{{"nodes":[{node}]}}}}}}"#)).unwrap();
    parse(&value).unwrap().remove(0)
}

#[test]
fn unknown_enum_values_and_null_rollups_degrade_instead_of_failing() {
    let pr = one(
        r#"{"number": 7, "title": "x", "reviewDecision": "SOMETHING_NEW",
            "mergeStateStatus": "BEHIND", "statusCheckRollup": null, "updatedAt": "garbage"}"#,
    );
    assert_eq!(pr.review, ReviewDecision::Unknown);
    assert_eq!(pr.merge_state, MergeState::Behind);
    assert_eq!(pr.checks, CheckState::None);
    assert!(pr.checks_detail.is_empty());
    assert_eq!(pr.updated_at, None);
}

#[test]
fn status_contexts_and_running_check_runs_are_read() {
    let pr = one(
        r#"{"number": 8, "statusCheckRollup": {"state": "PENDING", "contexts": {"nodes": [
            {"context": "ci/circleci", "state": "FAILURE", "targetUrl": "https://ci/1"},
            {"name": "build", "status": "IN_PROGRESS", "conclusion": null, "detailsUrl": "https://ci/2"},
            {"name": "docs", "status": "COMPLETED", "conclusion": "SKIPPED", "detailsUrl": ""},
            {"name": "e2e", "status": "COMPLETED", "conclusion": "CANCELLED", "detailsUrl": ""}
        ]}}}"#,
    );
    assert_eq!(pr.checks, CheckState::Pending);
    let conclusions: Vec<_> = pr
        .checks_detail
        .iter()
        .map(|c| (c.name.as_str(), c.conclusion))
        .collect();
    assert_eq!(
        conclusions,
        vec![
            ("ci/circleci", CheckConclusion::Failure),
            ("build", CheckConclusion::Pending),
            ("docs", CheckConclusion::Skipped),
            ("e2e", CheckConclusion::Failure),
        ]
    );
    assert_eq!(pr.checks_detail[0].url, "https://ci/1");
}

#[test]
fn a_response_without_search_nodes_is_an_error() {
    let value: serde_json::Value =
        serde_json::from_str(r#"{"errors":[{"message":"boom"}]}"#).unwrap();
    let err = parse(&value).unwrap_err();
    assert!(err.contains("data.search.nodes"), "{err}");
}

#[test]
fn the_merge_queue_position_is_read_when_present() {
    let queued = one(r#"{"number": 9, "mergeQueueEntry": {"position": 3, "state": "QUEUED"}}"#);
    assert_eq!(queued.queue_position, Some(3));
    let plain = one(r#"{"number": 9, "mergeQueueEntry": null}"#);
    assert_eq!(plain.queue_position, None);
}
