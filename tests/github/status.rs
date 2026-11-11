use conveyor::config::MergeMethod;
use conveyor::model::prs::StatusChange;
use conveyor::sources::github::{CliGh, Github};

use crate::shims::{serialized, shim};

#[test]
fn status_argv_marks_a_draft_ready() {
    assert_eq!(
        CliGh::status_args("acme/webapp", 42, &StatusChange::Ready),
        vec!["pr", "ready", "42", "-R", "acme/webapp"]
    );
}

#[test]
fn status_argv_converts_a_pull_request_to_draft() {
    assert_eq!(
        CliGh::status_args("acme/webapp", 42, &StatusChange::Draft),
        vec!["pr", "ready", "42", "-R", "acme/webapp", "--undo"]
    );
}

#[test]
fn status_argv_enables_auto_merge_through_the_graphql_mutation_never_gh_pr_merge() {
    let change = StatusChange::EnableAutoMerge {
        node_id: "PR_kwDOabc".to_string(),
        method: MergeMethod::Squash,
        head_oid: "8c1f2a7d".to_string(),
    };
    let args = CliGh::status_args("acme/webapp", 42, &change);
    assert_eq!(args[0..3], ["api", "graphql", "-f"]);
    assert!(args[3].contains("enablePullRequestAutoMerge("), "{args:?}");
    assert!(args[3].contains("expectedHeadOid: $head"), "{args:?}");
    assert_eq!(
        args[4..],
        [
            "-f",
            "id=PR_kwDOabc",
            "-f",
            "method=SQUASH",
            "-f",
            "head=8c1f2a7d"
        ]
    );
}

#[test]
fn status_argv_disables_auto_merge_through_the_graphql_mutation() {
    let change = StatusChange::DisableAutoMerge {
        node_id: "PR_kwDOabc".to_string(),
    };
    let args = CliGh::status_args("acme/webapp", 42, &change);
    assert_eq!(args[0..3], ["api", "graphql", "-f"]);
    assert!(
        args[3].contains("disablePullRequestAutoMerge(input: {pullRequestId: $id})"),
        "{args:?}"
    );
    assert_eq!(args[4..], ["-f", "id=PR_kwDOabc"]);
}

#[test]
fn status_argv_closes_a_pull_request() {
    assert_eq!(
        CliGh::status_args("acme/webapp", 42, &StatusChange::Close),
        vec!["pr", "close", "42", "-R", "acme/webapp"]
    );
}

#[test]
fn status_argv_enqueues_through_the_graphql_mutation_pinned_to_the_head() {
    let change = StatusChange::Enqueue {
        node_id: "PR_kwDOabc".to_string(),
        head_oid: "8c1f2a7d".to_string(),
    };
    let args = CliGh::status_args("acme/webapp", 42, &change);
    assert_eq!(args[0..3], ["api", "graphql", "-f"]);
    assert!(
        args[3].contains("enqueuePullRequest(input: {pullRequestId: $id, expectedHeadOid: $head})"),
        "{args:?}"
    );
    assert_eq!(args[4..], ["-f", "id=PR_kwDOabc", "-f", "head=8c1f2a7d"]);
}

#[test]
fn status_argv_dequeues_through_the_graphql_mutation_by_node_id() {
    let change = StatusChange::Dequeue {
        node_id: "PR_kwDOabc".to_string(),
    };
    let args = CliGh::status_args("acme/webapp", 42, &change);
    assert_eq!(args[0..3], ["api", "graphql", "-f"]);
    assert!(
        args[3].contains("dequeuePullRequest(input: {id: $id})"),
        "{args:?}"
    );
    assert_eq!(args[4..], ["-f", "id=PR_kwDOabc"]);
}

#[test]
fn a_refused_status_change_reports_what_gh_said_without_its_warnings() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(
        &dir,
        "echo '! a warning' >&2; echo 'Pull request is in clean status' >&2; exit 1",
    );

    let err = gh
        .change_status("acme/webapp", 42, &StatusChange::Close)
        .unwrap_err();

    assert_eq!(err.to_string(), "Pull request is in clean status");
}

#[test]
fn an_accepted_status_change_is_ok() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "exit 0");

    let result = gh.change_status("acme/webapp", 42, &StatusChange::Close);

    assert!(result.is_ok());
}
