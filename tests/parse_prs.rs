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
    assert_eq!(
        pr.checks_detail[0].took(UNIX_EPOCH),
        Some(Duration::from_secs(23))
    );
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
    assert_eq!(pr.checks, CheckState::Failure);
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

#[test]
fn a_check_from_a_newer_run_of_the_same_workflow_replaces_the_older_one() {
    let pr = one(
        r#"{"number": 9, "statusCheckRollup": {"state": "PENDING", "contexts": {"nodes": [
            {"name": "test", "status": "COMPLETED", "conclusion": "SUCCESS", "detailsUrl": "https://ci/1",
             "databaseId": 100, "checkSuite": {"workflowRun": {"event": "pull_request", "workflow": {"name": "CI"}}}},
            {"name": "test", "status": "IN_PROGRESS", "conclusion": null, "detailsUrl": "https://ci/2",
             "databaseId": 200, "checkSuite": {"workflowRun": {"event": "pull_request", "workflow": {"name": "CI"}}}}
        ]}}}"#,
    );

    assert_eq!(pr.checks_detail.len(), 1);
    assert_eq!(pr.checks_detail[0].conclusion, CheckConclusion::Pending);
    assert_eq!(pr.checks_detail[0].url, "https://ci/2");
}

#[test]
fn checks_sharing_a_name_across_workflows_are_told_apart_by_the_workflow() {
    let pr = one(
        r#"{"number": 10, "statusCheckRollup": {"state": "SUCCESS", "contexts": {"nodes": [
            {"name": "gate", "status": "COMPLETED", "conclusion": "SUCCESS", "detailsUrl": "",
             "databaseId": 1, "checkSuite": {"workflowRun": {"event": "pull_request", "workflow": {"name": "Review A"}}}},
            {"name": "gate", "status": "COMPLETED", "conclusion": "SUCCESS", "detailsUrl": "",
             "databaseId": 2, "checkSuite": {"workflowRun": {"event": "pull_request", "workflow": {"name": "Review B"}}}},
            {"name": "lint", "status": "COMPLETED", "conclusion": "SUCCESS", "detailsUrl": "",
             "databaseId": 3, "checkSuite": {"workflowRun": {"event": "pull_request", "workflow": {"name": "Review A"}}}}
        ]}}}"#,
    );

    assert_eq!(pr.checks_detail.len(), 3);
    assert_eq!(pr.checks_detail[0].name, "Review A / gate");
    assert_eq!(pr.checks_detail[1].name, "Review B / gate");
    assert_eq!(pr.checks_detail[2].name, "lint");
}

#[test]
fn one_workflow_run_by_two_events_is_told_apart_by_the_event() {
    let pr = one(
        r#"{"number": 11, "statusCheckRollup": {"state": "SUCCESS", "contexts": {"nodes": [
            {"name": "test", "status": "COMPLETED", "conclusion": "SUCCESS", "detailsUrl": "",
             "databaseId": 1, "checkSuite": {"workflowRun": {"event": "push", "workflow": {"name": "CI"}}}},
            {"name": "test", "status": "COMPLETED", "conclusion": "SUCCESS", "detailsUrl": "",
             "databaseId": 2, "checkSuite": {"workflowRun": {"event": "pull_request", "workflow": {"name": "CI"}}}}
        ]}}}"#,
    );

    assert_eq!(pr.checks_detail.len(), 2);
    assert_eq!(pr.checks_detail[0].name, "CI / test (push)");
    assert_eq!(pr.checks_detail[1].name, "CI / test (pull_request)");
}

#[test]
fn a_status_and_a_check_run_sharing_a_name_are_both_kept() {
    let pr = one(
        r#"{"number": 12, "statusCheckRollup": {"state": "FAILURE", "contexts": {"nodes": [
            {"context": "deploy", "state": "FAILURE", "targetUrl": "https://ci/1"},
            {"name": "deploy", "status": "COMPLETED", "conclusion": "SUCCESS", "detailsUrl": "https://ci/2",
             "databaseId": 5, "checkSuite": {"workflowRun": null}}
        ]}}}"#,
    );

    assert_eq!(pr.checks_detail.len(), 2);
    assert_eq!(pr.checks_detail[0].conclusion, CheckConclusion::Failure);
    assert_eq!(pr.checks_detail[1].conclusion, CheckConclusion::Success);
}

#[test]
fn a_cancelled_check_from_a_superseded_run_does_not_fail_the_pull_request() {
    let pr = one(
        r#"{"number": 13, "statusCheckRollup": {"state": "FAILURE", "contexts": {"nodes": [
            {"name": "gate", "status": "COMPLETED", "conclusion": "CANCELLED", "detailsUrl": "",
             "databaseId": 1, "checkSuite": {"workflowRun": {"event": "pull_request", "workflow": {"name": "Review"}}}},
            {"name": "gate", "status": "COMPLETED", "conclusion": "SUCCESS", "detailsUrl": "",
             "databaseId": 2, "checkSuite": {"workflowRun": {"event": "pull_request", "workflow": {"name": "Review"}}}},
            {"name": "build", "status": "IN_PROGRESS", "conclusion": null, "detailsUrl": "",
             "databaseId": 3, "checkSuite": {"workflowRun": {"event": "pull_request", "workflow": {"name": "CI"}}}}
        ]}}}"#,
    );

    assert_eq!(pr.checks, CheckState::Pending);
}

#[test]
fn two_apps_posting_a_check_of_the_same_name_are_both_kept() {
    let pr = one(
        r#"{"number": 14, "statusCheckRollup": {"state": "FAILURE", "contexts": {"nodes": [
            {"name": "test", "status": "COMPLETED", "conclusion": "FAILURE", "detailsUrl": "",
             "databaseId": 1, "checkSuite": {"app": {"slug": "app-a"}, "workflowRun": null}},
            {"name": "test", "status": "COMPLETED", "conclusion": "SUCCESS", "detailsUrl": "",
             "databaseId": 2, "checkSuite": {"app": {"slug": "app-b"}, "workflowRun": null}}
        ]}}}"#,
    );

    assert_eq!(pr.checks, CheckState::Failure);
    assert_eq!(pr.checks_detail.len(), 2);
    assert_eq!(pr.checks_detail[0].name, "app-a / test");
    assert_eq!(pr.checks_detail[1].name, "app-b / test");
}

#[test]
fn a_job_only_a_cancelled_older_run_has_is_dropped_once_a_newer_run_exists() {
    let pr = one(
        r#"{"number": 15, "statusCheckRollup": {"state": "FAILURE", "contexts": {"nodes": [
            {"name": "status", "status": "COMPLETED", "conclusion": "FAILURE", "detailsUrl": "",
             "databaseId": 1, "checkSuite": {"conclusion": "CANCELLED",
               "workflowRun": {"databaseId": 70, "event": "pull_request", "workflow": {"name": "CI"}}}},
            {"name": "test", "status": "COMPLETED", "conclusion": "CANCELLED", "detailsUrl": "",
             "databaseId": 2, "checkSuite": {"conclusion": "CANCELLED",
               "workflowRun": {"databaseId": 70, "event": "pull_request", "workflow": {"name": "CI"}}}},
            {"name": "test", "status": "IN_PROGRESS", "conclusion": null, "detailsUrl": "",
             "databaseId": 3, "checkSuite": {"conclusion": null,
               "workflowRun": {"databaseId": 80, "event": "pull_request", "workflow": {"name": "CI"}}}}
        ]}}}"#,
    );

    assert_eq!(pr.checks, CheckState::Pending);
    assert_eq!(pr.checks_detail.len(), 1);
    assert_eq!(pr.checks_detail[0].name, "test");
}

#[test]
fn a_job_only_a_finished_older_run_has_stays_when_a_newer_run_exists() {
    let pr = one(
        r#"{"number": 16, "statusCheckRollup": {"state": "FAILURE", "contexts": {"nodes": [
            {"name": "test", "status": "COMPLETED", "conclusion": "FAILURE", "detailsUrl": "",
             "databaseId": 1, "checkSuite": {"conclusion": "FAILURE",
               "workflowRun": {"databaseId": 70, "event": "pull_request", "workflow": {"name": "CI"}}}},
            {"name": "label", "status": "COMPLETED", "conclusion": "SUCCESS", "detailsUrl": "",
             "databaseId": 2, "checkSuite": {"conclusion": "SUCCESS",
               "workflowRun": {"databaseId": 80, "event": "pull_request", "workflow": {"name": "CI"}}}}
        ]}}}"#,
    );

    assert_eq!(pr.checks, CheckState::Failure);
    assert_eq!(pr.checks_detail.len(), 2);
}

#[test]
fn a_cancelled_run_with_nothing_newer_keeps_its_jobs() {
    let pr = one(
        r#"{"number": 17, "statusCheckRollup": {"state": "FAILURE", "contexts": {"nodes": [
            {"name": "status", "status": "COMPLETED", "conclusion": "FAILURE", "detailsUrl": "",
             "databaseId": 1, "checkSuite": {"conclusion": "CANCELLED",
               "workflowRun": {"databaseId": 70, "event": "pull_request", "workflow": {"name": "CI"}}}},
            {"name": "deploy", "status": "COMPLETED", "conclusion": "SUCCESS", "detailsUrl": "",
             "databaseId": 2, "checkSuite": {"conclusion": "SUCCESS",
               "workflowRun": {"databaseId": 90, "event": "pull_request", "workflow": {"name": "Deploy"}}}}
        ]}}}"#,
    );

    assert_eq!(pr.checks, CheckState::Failure);
    assert_eq!(pr.checks_detail.len(), 2);
}

#[test]
fn a_cancelled_run_is_not_superseded_by_a_newer_run_of_another_event() {
    let pr = one(
        r#"{"number": 18, "statusCheckRollup": {"state": "FAILURE", "contexts": {"nodes": [
            {"name": "status", "status": "COMPLETED", "conclusion": "FAILURE", "detailsUrl": "",
             "databaseId": 1, "checkSuite": {"conclusion": "CANCELLED",
               "workflowRun": {"databaseId": 70, "event": "pull_request", "workflow": {"name": "CI"}}}},
            {"name": "test", "status": "COMPLETED", "conclusion": "SUCCESS", "detailsUrl": "",
             "databaseId": 2, "checkSuite": {"conclusion": "SUCCESS",
               "workflowRun": {"databaseId": 80, "event": "push", "workflow": {"name": "CI"}}}}
        ]}}}"#,
    );

    assert_eq!(pr.checks, CheckState::Failure);
    assert_eq!(pr.checks_detail.len(), 2);
}

#[test]
fn a_running_check_has_taken_the_time_since_it_started() {
    let pr = one(
        r#"{"number": 9, "statusCheckRollup": {"state": "PENDING", "contexts": {"nodes": [
            {"name": "build", "status": "IN_PROGRESS", "conclusion": null, "detailsUrl": "",
             "startedAt": "2026-09-10T15:22:40Z", "completedAt": null}
        ]}}}"#,
    );
    let started = UNIX_EPOCH + Duration::from_secs(1_789_053_760);

    assert_eq!(
        pr.checks_detail[0].took(started + Duration::from_secs(192)),
        Some(Duration::from_secs(192))
    );
}

#[test]
fn a_check_that_never_started_and_a_status_context_have_no_time() {
    let pr = one(
        r#"{"number": 10, "statusCheckRollup": {"state": "PENDING", "contexts": {"nodes": [
            {"name": "queued", "status": "QUEUED", "conclusion": null, "detailsUrl": "",
             "startedAt": null, "completedAt": null},
            {"context": "ci/circleci", "state": "PENDING", "targetUrl": "https://ci/1"}
        ]}}}"#,
    );
    let now = UNIX_EPOCH + Duration::from_secs(1_789_053_760);

    assert_eq!(pr.checks_detail[0].took(now), None);
    assert_eq!(pr.checks_detail[1].took(now), None);
}

#[test]
fn a_skipped_check_has_no_time_although_github_stamps_it() {
    let pr = one(
        r#"{"number": 11, "statusCheckRollup": {"state": "SUCCESS", "contexts": {"nodes": [
            {"name": "deploy", "status": "COMPLETED", "conclusion": "SKIPPED", "detailsUrl": "",
             "startedAt": "2026-09-10T15:22:40Z", "completedAt": "2026-09-10T15:22:41Z"}
        ]}}}"#,
    );
    let now = UNIX_EPOCH + Duration::from_secs(1_789_053_900);

    assert_eq!(pr.checks_detail[0].took(now), None);
}

fn check_run(name: &str, status: &str, conclusion: &str, id: u64, run: u64) -> String {
    format!(
        r#"{{"name": "{name}", "status": "{status}", "conclusion": {conclusion}, "detailsUrl": "",
            "databaseId": {id}, "checkSuite": {{"conclusion": null,
              "workflowRun": {{"databaseId": {run}, "event": "pull_request",
                "workflow": {{"name": "wf{run}"}}}}}}}}"#
    )
}

fn ci_check(name: &str, status: &str, conclusion: &str, id: u64, run: u64) -> String {
    format!(
        r#"{{"name": "{name}", "status": "{status}", "conclusion": {conclusion}, "detailsUrl": "",
            "databaseId": {id}, "checkSuite": {{"conclusion": null,
              "workflowRun": {{"databaseId": {run}, "event": "pull_request",
                "workflow": {{"name": "CI"}}}}}}}}"#
    )
}

fn with_checks(checks: &[String]) -> conveyor::model::prs::PullRequest {
    one(&format!(
        r#"{{"number": 30, "statusCheckRollup": {{"state": "FAILURE", "contexts": {{"nodes": [{}]}}}}}}"#,
        checks.join(",")
    ))
}

#[test]
fn a_failed_run_is_not_offered_once_a_newer_run_of_its_workflow_has_started() {
    let pr = with_checks(&[
        ci_check("test", "COMPLETED", r#""FAILURE""#, 1, 70),
        ci_check("lint", "IN_PROGRESS", "null", 2, 80),
    ]);

    assert!(pr.rerunnable_runs().is_empty());
}

#[test]
fn a_running_older_run_whose_pending_job_a_newer_run_hides_is_not_offered() {
    let pr = with_checks(&[
        ci_check("test", "COMPLETED", r#""FAILURE""#, 1, 70),
        ci_check("e2e", "IN_PROGRESS", "null", 2, 70),
        ci_check("e2e", "COMPLETED", r#""SUCCESS""#, 3, 80),
    ]);

    assert!(pr.rerunnable_runs().is_empty());
}

#[test]
fn a_newer_run_of_another_workflow_leaves_the_failed_run_rerunnable() {
    let pr = with_checks(&[
        ci_check("test", "COMPLETED", r#""FAILURE""#, 1, 70),
        check_run("docs", "IN_PROGRESS", "null", 2, 80),
    ]);

    assert_eq!(pr.rerunnable_runs(), vec![70]);
}

#[test]
fn a_failure_in_a_finished_run_is_rerunnable() {
    let pr = with_checks(&[
        check_run("test", "COMPLETED", r#""FAILURE""#, 1, 70),
        check_run("lint", "COMPLETED", r#""SUCCESS""#, 2, 70),
    ]);

    assert_eq!(pr.rerunnable_runs(), vec![70]);
}

#[test]
fn a_run_with_a_check_still_going_is_not_rerunnable_yet() {
    let pr = with_checks(&[
        check_run("test", "COMPLETED", r#""FAILURE""#, 1, 70),
        check_run("e2e", "IN_PROGRESS", "null", 2, 70),
    ]);

    assert!(pr.rerunnable_runs().is_empty());
}

#[test]
fn a_failing_status_context_has_no_run_to_rerun() {
    let pr = one(
        r#"{"number": 31, "statusCheckRollup": {"state": "FAILURE", "contexts": {"nodes": [
            {"context": "ci/external", "state": "FAILURE", "targetUrl": ""}
        ]}}}"#,
    );

    assert!(pr.rerunnable_runs().is_empty());
}

#[test]
fn two_failures_in_one_run_rerun_it_once_and_each_failing_run_is_listed() {
    let pr = with_checks(&[
        check_run("test", "COMPLETED", r#""FAILURE""#, 1, 70),
        check_run("lint", "COMPLETED", r#""FAILURE""#, 2, 70),
        check_run("e2e", "COMPLETED", r#""CANCELLED""#, 3, 80),
        check_run("docs", "COMPLETED", r#""SUCCESS""#, 4, 90),
    ]);

    assert_eq!(pr.rerunnable_runs(), vec![70, 80]);
}
