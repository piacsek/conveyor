use conveyor::config::UpdateMethod;
use conveyor::model::prs::BranchUpdate;
use conveyor::sources::github::{CliGh, Github};

use crate::shims::{serialized, shim};

#[test]
fn update_branch_argv_merges_the_base_in_by_default() {
    let _serialized = serialized();
    assert_eq!(
        CliGh::update_branch_args("acme/webapp", 42, UpdateMethod::Merge),
        vec!["pr", "update-branch", "42", "-R", "acme/webapp"]
    );
}

#[test]
fn update_branch_argv_rebases_when_asked() {
    let _serialized = serialized();
    assert_eq!(
        CliGh::update_branch_args("acme/webapp", 42, UpdateMethod::Rebase),
        vec!["pr", "update-branch", "42", "-R", "acme/webapp", "--rebase"]
    );
}

#[test]
fn an_updated_branch_is_updated() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "echo '✓ PR branch updated'");

    let result = gh.update_branch("acme/webapp", 42, UpdateMethod::Merge);

    assert_eq!(result.unwrap(), BranchUpdate::Updated);
}

#[test]
fn a_branch_with_nothing_to_bring_in_is_up_to_date() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "echo '✓ PR branch already up-to-date'");

    let result = gh.update_branch("acme/webapp", 42, UpdateMethod::Merge);

    assert_eq!(result.unwrap(), BranchUpdate::UpToDate);
}

#[test]
fn a_conflicting_branch_is_reported_as_conflicts_not_as_an_error() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(
        &dir,
        "echo 'X Cannot update PR branch due to conflicts' >&2; exit 1",
    );

    let result = gh.update_branch("acme/webapp", 42, UpdateMethod::Merge);

    assert_eq!(result.unwrap(), BranchUpdate::Conflicts);
}

#[test]
fn any_other_refusal_reports_what_gh_said() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(
        &dir,
        "echo 'GraphQL: Resource not accessible by integration' >&2; exit 1",
    );

    let err = gh
        .update_branch("acme/webapp", 42, UpdateMethod::Merge)
        .unwrap_err();

    assert_eq!(
        err.to_string(),
        "GraphQL: Resource not accessible by integration"
    );
}
