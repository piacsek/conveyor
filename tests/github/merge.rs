use conveyor::config::MergeMethod;
use conveyor::sources::github::{CliGh, Github};

use crate::shims::{serialized, shim};

#[test]
fn merge_argv_names_the_method_and_pins_the_head_commit() {
    let _serialized = serialized();
    assert_eq!(
        CliGh::merge_args("acme/webapp", 42, MergeMethod::Squash, "8c1f2a7d"),
        vec![
            "pr",
            "merge",
            "42",
            "-R",
            "acme/webapp",
            "--squash",
            "--match-head-commit",
            "8c1f2a7d"
        ]
    );
}

#[test]
fn merge_argv_without_a_known_head_does_not_pin_it() {
    let _serialized = serialized();
    assert_eq!(
        CliGh::merge_args("acme/webapp", 42, MergeMethod::Rebase, ""),
        vec!["pr", "merge", "42", "-R", "acme/webapp", "--rebase"]
    );
}

#[test]
fn a_refused_merge_reports_what_gh_said() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "echo 'Pull request is not mergeable' >&2; exit 1");

    let err = gh
        .merge("acme/webapp", 42, MergeMethod::Merge, "8c1f2a7d")
        .unwrap_err();

    assert_eq!(err.to_string(), "Pull request is not mergeable");
}

#[test]
fn an_accepted_merge_is_ok() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "exit 0");

    let result = gh.merge("acme/webapp", 42, MergeMethod::Squash, "8c1f2a7d");

    assert!(result.is_ok());
}

#[test]
fn a_refused_merge_on_a_queue_branch_reports_the_error_not_the_queue_warning() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(
        &dir,
        "echo '! The merge strategy for main is set by the merge queue' >&2; \
         echo 'GraphQL: Required status checks have not passed' >&2; exit 1",
    );

    let err = gh
        .merge("acme/webapp", 42, MergeMethod::Squash, "8c1f2a7d")
        .unwrap_err();

    assert_eq!(
        err.to_string(),
        "GraphQL: Required status checks have not passed"
    );
}
