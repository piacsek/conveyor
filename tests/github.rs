use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Mutex, MutexGuard};

use conveyor::config::MergeMethod;
use conveyor::sources::github::{CliGh, Github};

static SHIMS: Mutex<()> = Mutex::new(());

fn serialized() -> MutexGuard<'static, ()> {
    SHIMS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn shim(dir: &tempfile::TempDir, script: &str) -> CliGh {
    let bin = dir.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let gh = bin.join("gh");
    fs::write(&gh, format!("#!/bin/sh\n{script}\n")).unwrap();
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).unwrap();
    CliGh::with_program(gh)
}

#[test]
fn graphql_argv_passes_the_query_file_and_typed_variables() {
    let args = CliGh::graphql_args(
        "query($q: String!, $first: Int!) { x }",
        &[("q", "is:pr".into()), ("first", 20u64.into())],
    );
    assert_eq!(
        args,
        vec![
            "api",
            "graphql",
            "-f",
            "query=query($q: String!, $first: Int!) { x }",
            "-f",
            "q=is:pr",
            "-F",
            "first=20",
        ]
    );
}

#[test]
fn graphql_returns_the_parsed_json_from_stdout() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, r#"echo '{"data":{"ok":true}}'"#);

    let value = gh.graphql("query { x }", &[]).unwrap();

    assert_eq!(
        value.pointer("/data/ok"),
        Some(&serde_json::Value::Bool(true))
    );
}

#[test]
fn a_failing_gh_reports_its_stderr() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "echo 'gh: HTTP 401: Bad credentials' >&2; exit 1");

    let err = gh.graphql("query { x }", &[]).unwrap_err();

    assert_eq!(err.to_string(), "gh: HTTP 401: Bad credentials");
}

#[test]
fn a_missing_gh_binary_says_how_to_install_it() {
    let gh = CliGh::with_program("/nonexistent/gh".into());

    let err = gh.graphql("query { x }", &[]).unwrap_err();

    assert_eq!(
        err.to_string(),
        "gh not found on PATH; install gh (brew install gh) and run gh auth login"
    );
}

#[test]
fn current_repo_asks_gh_for_the_checked_out_repository() {
    let _serialized = serialized();
    assert_eq!(
        CliGh::current_repo_args(),
        vec![
            "repo",
            "view",
            "--json",
            "nameWithOwner",
            "--jq",
            ".nameWithOwner"
        ]
    );
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "echo acme/webapp");
    assert_eq!(gh.current_repo().unwrap(), "acme/webapp");
}

#[test]
fn rest_argv_and_json_parsing() {
    let _serialized = serialized();
    assert_eq!(
        CliGh::rest_args("repos/acme/webapp/actions/runs?event=merge_group&per_page=20"),
        vec![
            "api",
            "repos/acme/webapp/actions/runs?event=merge_group&per_page=20"
        ]
    );
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, r#"echo '{"total_count":0,"workflow_runs":[]}'"#);
    let value = gh.rest("repos/acme/webapp/actions/runs").unwrap();
    assert_eq!(value["total_count"], 0);
}

#[test]
fn log_failed_argv_views_the_job_with_gh_run_view_and_returns_the_raw_text() {
    let _serialized = serialized();
    assert_eq!(
        CliGh::log_failed_args("acme/webapp", 1026, 7),
        vec![
            "run",
            "view",
            "1026",
            "--job",
            "7",
            "--log-failed",
            "-R",
            "acme/webapp"
        ]
    );
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, r"printf 'check\tRun\tboom\n'");
    assert_eq!(
        gh.log_failed("acme/webapp", 1026, 7).unwrap(),
        "check\tRun\tboom\n"
    );
}

#[test]
fn rerun_failed_argv_reruns_only_the_failed_jobs_of_the_run() {
    let _serialized = serialized();
    assert_eq!(
        CliGh::rerun_failed_args("acme/webapp", 1026),
        vec!["run", "rerun", "1026", "--failed", "-R", "acme/webapp"]
    );
}

#[test]
fn rerun_failed_surfaces_what_gh_printed_on_stderr() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(
        &dir,
        "echo 'run 1026 cannot be rerun; it is in progress' >&2; exit 1",
    );
    let err = gh.rerun_failed("acme/webapp", 1026).unwrap_err();
    assert_eq!(
        err.to_string(),
        "run 1026 cannot be rerun; it is in progress"
    );
}

#[test]
fn rerun_failed_succeeds_when_gh_exits_zero() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "exit 0");
    assert!(gh.rerun_failed("acme/webapp", 1026).is_ok());
}

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
