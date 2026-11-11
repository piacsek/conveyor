use std::fs;
use std::os::unix::fs::PermissionsExt;

use conveyor::sources::github::{CliGh, Github};

use crate::shims::{serialized, shim};

#[test]
fn checkout_argv_names_the_pull_request_and_its_repository() {
    let _serialized = serialized();
    assert_eq!(
        CliGh::checkout_args("acme/webapp", 42),
        vec!["pr", "checkout", "42", "-R", "acme/webapp"]
    );
}

#[test]
fn checkout_runs_in_the_clone_after_reading_its_remotes() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_dir(&dir);
    let (gh, calls) = shims(&dir, ORIGIN, "");

    gh.checkout("acme/webapp", 42, Some(&clone)).unwrap();

    let log = fs::read_to_string(calls).unwrap();
    let lines: Vec<&str> = log.lines().collect();
    let here = clone.display();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], format!("{here}: git remote -v"));
    assert_eq!(
        lines[1],
        format!("{here}: gh pr checkout 42 -R acme/webapp")
    );
}

#[test]
fn checkout_accepts_a_fork_whose_other_remote_is_the_repository() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_dir(&dir);
    let (gh, _calls) = shims(
        &dir,
        r"printf 'origin\tgit@github.com:dev/webapp.git (fetch)\nupstream\thttps://github.com/Acme/WebApp/ (fetch)\n'",
        "",
    );

    assert!(gh.checkout("acme/webapp", 42, Some(&clone)).is_ok());
}

#[test]
fn checkout_refuses_a_clone_of_another_repository_without_touching_it() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_dir(&dir);
    let (gh, calls) = shims(
        &dir,
        r"printf 'origin\tgit@github.com:acme/webapp-docs.git (fetch)\n'",
        "",
    );

    let err = gh.checkout("acme/webapp", 42, Some(&clone)).unwrap_err();

    assert_eq!(
        err.to_string(),
        format!("{} is not a clone of acme/webapp", clone.display())
    );
    assert_eq!(fs::read_to_string(calls).unwrap().lines().count(), 1);
}

#[test]
fn checkout_from_the_working_directory_says_which_key_to_set_when_it_is_no_clone_of_it() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let (gh, calls) = shims(&dir, "", "");

    let err = gh.checkout("acme/webapp", 42, None).unwrap_err();

    assert_eq!(
        err.to_string(),
        "conveyor does not run in a clone of acme/webapp; set `path` under its [[repo]]"
    );
    assert_eq!(fs::read_to_string(calls).unwrap().lines().count(), 1);
}

#[test]
fn checkout_where_there_is_no_repository_at_all_says_the_same() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_dir(&dir);
    let (gh, _calls) = shims(
        &dir,
        "echo 'fatal: not a git repository (or any of the parent directories): .git' >&2; exit 128",
        "",
    );

    let err = gh.checkout("acme/webapp", 42, Some(&clone)).unwrap_err();

    assert_eq!(
        err.to_string(),
        format!("{} is not a clone of acme/webapp", clone.display())
    );
}

#[test]
fn checkout_reports_any_other_failure_to_read_the_remotes() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_dir(&dir);
    let (gh, calls) = shims(
        &dir,
        "echo 'fatal: detected dubious ownership in repository' >&2; exit 128",
        "",
    );

    let err = gh.checkout("acme/webapp", 42, Some(&clone)).unwrap_err();

    assert_eq!(
        err.to_string(),
        "fatal: detected dubious ownership in repository"
    );
    assert_eq!(fs::read_to_string(calls).unwrap().lines().count(), 1);
}

#[test]
fn checkout_without_git_says_git_is_missing() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_dir(&dir);
    let gh = shim(&dir, "exit 0").with_git(dir.path().join("no-git"));

    let err = gh.checkout("acme/webapp", 42, Some(&clone)).unwrap_err();

    assert_eq!(err.to_string(), "git not found on PATH");
}

#[test]
fn checkout_into_a_directory_that_is_missing_names_it() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let (gh, _calls) = shims(&dir, ORIGIN, "");
    let missing = dir.path().join("gone");

    let err = gh.checkout("acme/webapp", 42, Some(&missing)).unwrap_err();

    assert_eq!(
        err.to_string(),
        format!("{} is not a directory", missing.display())
    );
}

#[test]
fn checkout_never_lets_git_prompt_for_credentials() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_dir(&dir);
    let seen = dir.path().join("prompt");
    let (gh, _calls) = shims(
        &dir,
        ORIGIN,
        &format!("echo \"$GIT_TERMINAL_PROMPT\" > '{}'", seen.display()),
    );

    gh.checkout("acme/webapp", 42, Some(&clone)).unwrap();

    assert_eq!(fs::read_to_string(seen).unwrap(), "0\n");
}

#[test]
fn a_refused_checkout_reports_gits_fatal_line_not_its_progress() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_dir(&dir);
    let (gh, _calls) = shims(
        &dir,
        ORIGIN,
        r#"printf '%s\n' 'From https://github.com/acme/webapp' ' * [new ref] refs/pull/42/head -> retry' "fatal: 'retry' is already checked out at '/work/trees/retry'" 'failed to run git: exit status 128' >&2; exit 1"#,
    );

    let err = gh.checkout("acme/webapp", 42, Some(&clone)).unwrap_err();

    assert_eq!(
        err.to_string(),
        "fatal: 'retry' is already checked out at '/work/trees/retry'"
    );
}

#[test]
fn a_checkout_over_local_changes_reports_gits_error_line() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_dir(&dir);
    let (gh, _calls) = shims(
        &dir,
        ORIGIN,
        r#"printf '%s\n' 'error: Your local changes to the following files would be overwritten by checkout:' '	src/app.rs' 'Aborting' 'failed to run git: exit status 1' >&2; exit 1"#,
    );

    let err = gh.checkout("acme/webapp", 42, Some(&clone)).unwrap_err();

    assert_eq!(
        err.to_string(),
        "error: Your local changes to the following files would be overwritten by checkout:"
    );
}

#[test]
fn a_refused_checkout_with_no_reason_from_git_reports_the_last_line_gh_printed() {
    let _serialized = serialized();
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_dir(&dir);
    let (gh, _calls) = shims(
        &dir,
        ORIGIN,
        r#"printf '%s\n' 'GraphQL: Could not resolve to a PullRequest with the number of 42.' '' >&2; exit 1"#,
    );

    let err = gh.checkout("acme/webapp", 42, Some(&clone)).unwrap_err();

    assert_eq!(
        err.to_string(),
        "GraphQL: Could not resolve to a PullRequest with the number of 42."
    );
}

const ORIGIN: &str = r"printf 'origin\tgit@github.com:acme/webapp.git (fetch)\norigin\tgit@github.com:acme/webapp.git (push)\n'";

fn shims(dir: &tempfile::TempDir, git: &str, gh: &str) -> (CliGh, std::path::PathBuf) {
    let calls = dir.path().join("calls.log");
    let logged = |program: &str, script: &str| {
        format!(
            "echo \"$(pwd -P): {program} $*\" >> '{}'\n{script}",
            calls.display()
        )
    };
    let git_shim = dir.path().join("bin").join("git");
    let cli = shim(dir, &logged("gh", gh));
    fs::write(&git_shim, format!("#!/bin/sh\n{}\n", logged("git", git))).unwrap();
    fs::set_permissions(&git_shim, fs::Permissions::from_mode(0o755)).unwrap();
    (cli.with_git(git_shim), calls)
}

fn clone_dir(dir: &tempfile::TempDir) -> std::path::PathBuf {
    let clone = dir.path().join("webapp");
    fs::create_dir_all(&clone).unwrap();
    clone.canonicalize().unwrap()
}
