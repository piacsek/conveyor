use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Mutex, MutexGuard};

use conveyor::config::{MergeMethod, UpdateMethod};
use conveyor::log::Log;
use conveyor::model::prs::{BranchUpdate, StatusChange};
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
fn a_call_made_while_another_hangs_is_not_held_up_by_it() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = std::sync::Arc::new(shim(
        &dir,
        r#"case "$*" in *hung*) sleep 2;; *) echo '{}';; esac"#,
    ));
    let mut held_up = 0;

    for _ in 0..150 {
        let start = std::sync::Arc::new(std::sync::Barrier::new(4));
        std::thread::spawn({
            let (gh, start) = (gh.clone(), start.clone());
            move || {
                start.wait();
                let _ = gh.rest("hung");
            }
        });
        let healthy: Vec<_> = (0..3)
            .map(|_| {
                let (gh, start) = (gh.clone(), start.clone());
                std::thread::spawn(move || {
                    start.wait();
                    let started = std::time::Instant::now();
                    let answer = gh.rest("repos/acme/webapp");
                    answer.is_err() || started.elapsed() > std::time::Duration::from_secs(1)
                })
            })
            .collect();
        held_up += healthy
            .into_iter()
            .map(|call| call.join().unwrap())
            .filter(|held_up| *held_up)
            .count();
    }

    assert_eq!(held_up, 0);
}

#[test]
fn a_failing_call_is_logged_with_its_argv_duration_and_stderr() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("conveyor.log");
    let gh = shim(
        &dir,
        "echo 'gh: HTTP 502' >&2; echo 'retry later' >&2; exit 1",
    )
    .with_log(Log::open(&file));

    let _ = gh.graphql("query { x }", &[("q", "is:pr".into())]);

    let text = fs::read_to_string(&file).unwrap();
    assert!(
        text.contains(" gh api graphql -f query=<11 chars> -f q=is:pr "),
        "{text}"
    );
    assert!(
        text.ends_with("ms exit 1: gh: HTTP 502 | retry later\n"),
        "{text}"
    );
}

#[test]
fn a_successful_call_is_logged_as_ok_without_its_stdout() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("conveyor.log");
    let gh = shim(&dir, r#"echo '{"secret":"payload"}'"#).with_log(Log::open(&file));

    let _ = gh.rest("repos/acme/webapp/actions/runs");

    let text = fs::read_to_string(&file).unwrap();
    assert!(
        text.contains(" gh api repos/acme/webapp/actions/runs "),
        "{text}"
    );
    assert!(text.ends_with("ms ok\n"), "{text}");
    assert!(!text.contains("payload"), "{text}");
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

#[test]
fn checkout_argv_names_the_pull_request_and_its_repository() {
    let _serialized = serialized();
    assert_eq!(
        CliGh::checkout_args("acme/webapp", 42),
        vec!["pr", "checkout", "42", "-R", "acme/webapp"]
    );
}

fn logging_shim(dir: &tempfile::TempDir, script: &str) -> (CliGh, std::path::PathBuf) {
    let calls = dir.path().join("calls.log");
    let gh = shim(
        dir,
        &format!("echo \"$PWD: $*\" >> '{}'\n{script}", calls.display()),
    );
    (gh, calls)
}

fn clone_dir(dir: &tempfile::TempDir) -> std::path::PathBuf {
    let clone = dir.path().join("webapp");
    fs::create_dir_all(&clone).unwrap();
    clone.canonicalize().unwrap()
}

#[test]
fn checkout_runs_in_the_clone_after_asking_which_repository_it_is() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_dir(&dir);
    let (gh, calls) = logging_shim(
        &dir,
        r#"case "$*" in 'repo view'*) echo acme/webapp;; esac"#,
    );

    gh.checkout("acme/webapp", 42, Some(&clone)).unwrap();

    let log = fs::read_to_string(calls).unwrap();
    let lines: Vec<&str> = log.lines().collect();
    let here = clone.display();
    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines[0],
        format!("{here}: repo view --json nameWithOwner --jq .nameWithOwner")
    );
    assert_eq!(lines[1], format!("{here}: pr checkout 42 -R acme/webapp"));
}

#[test]
fn checkout_refuses_a_clone_of_another_repository_without_touching_it() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_dir(&dir);
    let (gh, calls) = logging_shim(
        &dir,
        r#"case "$*" in 'repo view'*) echo acme/billing;; esac"#,
    );

    let err = gh.checkout("acme/webapp", 42, Some(&clone)).unwrap_err();

    assert_eq!(
        err.to_string(),
        format!(
            "{} is a clone of acme/billing, not acme/webapp",
            clone.display()
        )
    );
    assert_eq!(fs::read_to_string(calls).unwrap().lines().count(), 1);
}

#[test]
fn checkout_from_the_working_directory_says_which_key_to_set_when_it_is_another_clone() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let (gh, calls) = logging_shim(
        &dir,
        r#"case "$*" in 'repo view'*) echo acme/billing;; esac"#,
    );

    let err = gh.checkout("acme/webapp", 42, None).unwrap_err();

    assert_eq!(
        err.to_string(),
        "conveyor runs in a clone of acme/billing; set `path` under the [[repo]] of acme/webapp"
    );
    assert_eq!(fs::read_to_string(calls).unwrap().lines().count(), 1);
}

#[test]
fn checkout_accepts_a_repository_name_in_another_case() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_dir(&dir);
    let (gh, _calls) = logging_shim(
        &dir,
        r#"case "$*" in 'repo view'*) echo Acme/WebApp;; esac"#,
    );

    assert!(gh.checkout("acme/webapp", 42, Some(&clone)).is_ok());
}

#[test]
fn checkout_where_there_is_no_clone_says_so() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_dir(&dir);
    let (gh, calls) = logging_shim(
        &dir,
        "echo 'failed to run git: fatal: not a git repository (or any of the parent directories): .git' >&2; exit 1",
    );

    let configured = gh.checkout("acme/webapp", 42, Some(&clone)).unwrap_err();
    let started_in = gh.checkout("acme/webapp", 42, None).unwrap_err();

    assert_eq!(
        configured.to_string(),
        format!("{} is not a clone of acme/webapp", clone.display())
    );
    assert_eq!(
        started_in.to_string(),
        "conveyor does not run in a clone of acme/webapp; set `path` under its [[repo]]"
    );
    assert_eq!(fs::read_to_string(calls).unwrap().lines().count(), 2);
}

#[test]
fn checkout_into_a_directory_that_is_missing_names_it() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let gh = shim(&dir, "echo acme/webapp");
    let missing = dir.path().join("gone");

    let err = gh.checkout("acme/webapp", 42, Some(&missing)).unwrap_err();

    assert_eq!(
        err.to_string(),
        format!("{} is not a directory", missing.display())
    );
}

#[test]
fn a_refused_checkout_reports_gits_reason_not_its_progress() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_dir(&dir);
    let gh = shim(
        &dir,
        r#"case "$*" in 'repo view'*) echo acme/webapp;; *) printf '%s\n' 'From https://github.com/acme/webapp' ' * [new ref] refs/pull/42/head -> retry' "fatal: 'retry' is already checked out at '/work/trees/retry'" 'failed to run git: exit status 128' >&2; exit 1;; esac"#,
    );

    let err = gh.checkout("acme/webapp", 42, Some(&clone)).unwrap_err();

    assert_eq!(
        err.to_string(),
        "fatal: 'retry' is already checked out at '/work/trees/retry'"
    );
}
